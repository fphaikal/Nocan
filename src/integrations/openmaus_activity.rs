use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::Value;
use windows::Win32::Foundation::{CloseHandle, BOOL, FILETIME, STILL_ACTIVE, SYSTEMTIME};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

const TAIL_BYTES: usize = 1024 * 1024;
const METADATA_BYTES: u64 = 4 * 1024 * 1024;
const HISTORY_LIMIT: usize = 60;
const STATE_LIMIT: usize = 4096;
const RECENT_MS: u64 = 120_000;
const FUTURE_MS: u64 = 30_000;
const FILETIME_EPOCH: u64 = 116_444_736_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityKind {
    AgentStarted,
    AgentCompleted,
    AgentFailed,
    NeedsAttention,
    AutomationStarted,
    AutomationCompleted,
    AutomationFailed,
}

#[derive(Clone, Debug)]
pub struct ActivityEvent {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub at_ms: u64,
    pub kind: ActivityKind,
}

#[derive(Clone, Debug)]
pub struct ScheduledRoutine {
    pub id: String,
    pub name: String,
    pub bot_name: String,
    pub schedule: String,
    pub next_run_at: Option<u64>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ActivitySnapshot {
    pub events: Vec<ActivityEvent>,
    pub routines: Vec<ScheduledRoutine>,
    pub running_agents: usize,
    pub running_automations: usize,
    pub server_online: bool,
    pub source_found: bool,
    pub sync_error: Option<String>,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ActivityPoll {
    pub snapshot: ActivitySnapshot,
    pub notifications: Vec<ActivityEvent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bot {
    id: String,
    name: Option<String>,
    title: Option<String>,
    thread_id: Option<String>,
    #[serde(default)]
    tasks: Vec<Task>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    thread_id: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
struct RoutineFile {
    routines: Vec<Routine>,
    runs: Vec<Run>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Routine {
    id: String,
    name: String,
    bot_id: Option<String>,
    enabled: bool,
    schedule: Value,
    next_run_at: Option<Value>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Run {
    id: String,
    routine_id: Option<String>,
    routine_name: Option<String>,
    bot_id: Option<String>,
    thread_id: Option<String>,
    status: String,
    created_at: Option<Value>,
    started_at: Option<Value>,
    finished_at: Option<Value>,
}

impl Run {
    fn started(&self) -> Option<u64> {
        self.started_at.as_ref().and_then(parse_timestamp)
            .or_else(|| self.created_at.as_ref().and_then(parse_timestamp))
    }

    fn finished(&self) -> Option<u64> {
        self.finished_at.as_ref().and_then(parse_timestamp).or_else(|| self.started())
    }

    fn active(&self) -> bool {
        matches!(self.status.as_str(), "running" | "started" | "starting" | "in_progress" | "executing")
    }

    fn terminal_kind(&self) -> Option<ActivityKind> {
        match self.status.as_str() {
            "completed" => Some(ActivityKind::AutomationCompleted),
            "failed" | "missed" => Some(ActivityKind::AutomationFailed),
            _ => None,
        }
    }
}

// Unknown JSON fields are skipped by serde; message bodies, tool output, and lease tokens
// never enter the monitor's state or error messages.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventRecord {
    event_id: Option<String>,
    thread_id: Option<String>,
    turn_id: Option<String>,
    #[serde(rename = "type")]
    event_type: String,
    created_at: Option<Value>,
    ok: Option<bool>,
    terminal: Option<bool>,
    request_id: Option<String>,
    request_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lease {
    pid: u32,
    created_at: Option<Value>,
}

#[derive(Default)]
struct ServerState {
    online: bool,
    started_at_ms: Option<u64>,
}

impl ServerState {
    fn current_activity(&self, at_ms: u64, now: u64) -> bool {
        self.online && at_ms <= now.saturating_add(FUTURE_MS)
            && self.started_at_ms.map(|birth| at_ms >= birth).unwrap_or_else(|| fresh(at_ms, now))
    }
}

#[derive(Clone)]
struct HistoryEntry {
    event: ActivityEvent,
    agent_thread: Option<String>,
}

#[derive(Default)]
struct TurnState {
    at_ms: u64,
    active: bool,
    terminal: bool,
}

#[derive(Default)]
struct Tail {
    offset: u64,
    initialized: bool,
    creation: u64,
    modified: Option<SystemTime>,
    anchor: Vec<u8>,
    pending: Vec<u8>,
    skip_fragment: bool,
}

pub struct ActivityMonitor {
    root: PathBuf,
    bots: HashMap<String, String>,
    threads: HashMap<String, String>,
    routines: Vec<ScheduledRoutine>,
    runs: HashMap<String, Run>,
    automation_threads: HashSet<String>,
    tails: HashMap<PathBuf, Tail>,
    turns: HashMap<(String, String), TurnState>,
    requests: HashMap<String, (u64, bool)>,
    history: Vec<HistoryEntry>,
    notified: HashMap<String, u64>,
    events_baselined: bool,
    routines_baselined: bool,
}

impl ActivityMonitor {
    /// `root` is the .openmausbot directory, not its parent. This monitor never writes to it.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            bots: HashMap::new(),
            threads: HashMap::new(),
            routines: Vec::new(),
            runs: HashMap::new(),
            automation_threads: HashSet::new(),
            tails: HashMap::new(),
            turns: HashMap::new(),
            requests: HashMap::new(),
            history: Vec::new(),
            notified: HashMap::new(),
            events_baselined: false,
            routines_baselined: false,
        }
    }

    pub fn poll(&mut self) -> ActivityPoll {
        self.poll_at(timestamp_ms())
    }

    fn poll_at(&mut self, now: u64) -> ActivityPoll {
        let source_found = self.root.is_dir();
        let mut errors = Vec::new();
        let mut notifications = Vec::new();
        self.notified.retain(|_, at| fresh(*at, now));
        if source_found {
            self.read_bots(&mut errors);
            self.read_routines(now, &mut notifications, &mut errors);
            self.read_events(now, &mut notifications, &mut errors);
        }
        let server = if source_found { lease_status(&self.root, &mut errors) } else { ServerState::default() };
        notifications.retain(|event| event.kind != ActivityKind::NeedsAttention
            || !self.requests.get(&event.id).is_some_and(|(_, open)| !open));
        let active_threads: HashSet<&str> = self.turns.iter()
            .filter(|((thread, _), state)| state.active && server.current_activity(state.at_ms, now)
                && !self.automation_threads.contains(thread))
            .map(|((thread, _), _)| thread.as_str()).collect();
        let mut events: Vec<_> = self.history.iter()
            .filter(|entry| !entry.agent_thread.as_ref().is_some_and(|t| self.automation_threads.contains(t)))
            .map(|entry| entry.event.clone()).collect();
        sort_events(&mut events);
        events.truncate(HISTORY_LIMIT);
        sort_events(&mut notifications);
        let snapshot = ActivitySnapshot {
            events,
            routines: self.routines.clone(),
            running_agents: active_threads.len(),
            running_automations: self.runs.values().filter(|r| r.active()
                && r.started().is_some_and(|at| server.current_activity(at, now))).count(),
            server_online: server.online,
            source_found,
            sync_error: if errors.is_empty() { None } else { Some(errors.join("; ")) },
            updated_at_ms: now,
        };
        ActivityPoll { snapshot, notifications }
    }

    fn read_bots(&mut self, errors: &mut Vec<String>) {
        match read_json::<Vec<Bot>>(&self.root.join("bots.json")) {
            Ok(bots) => {
                let mut names = HashMap::new();
                let mut threads = HashMap::new();
                for bot in bots {
                    let name = label(bot.name.as_deref().or(bot.title.as_deref()), "Agent");
                    names.insert(bot.id, name.clone());
                    if let Some(thread) = bot.thread_id.filter(|t| !t.is_empty()) {
                        let title = label(bot.title.as_deref(), &name);
                        threads.insert(thread, if title == name { name.clone() } else { format!("{name} · {title}") });
                    }
                    for task in bot.tasks {
                        if let Some(thread) = task.thread_id.filter(|t| !t.is_empty()) {
                            let task_name = label(task.title.as_deref(), &name);
                            threads.insert(thread, if task_name == name { name.clone() } else { format!("{name} · {task_name}") });
                        }
                    }
                }
                self.bots = names;
                self.threads = threads;
            }
            Err(error) => errors.push(format!("bots.json: {error}")),
        }
    }

    fn read_routines(&mut self, now: u64, notifications: &mut Vec<ActivityEvent>, errors: &mut Vec<String>) {
        let data = match read_json::<RoutineFile>(&self.root.join("routines.json")) {
            Ok(data) => data,
            Err(error) => {
                errors.push(format!("routines.json: {error}"));
                return;
            }
        };
        let baseline = !self.routines_baselined;
        let routine_names: HashMap<_, _> = data.routines.iter()
            .map(|r| (r.id.clone(), label(Some(&r.name), "Automation"))).collect();
        self.routines = data.routines.into_iter().map(|r| ScheduledRoutine {
            id: r.id,
            name: label(Some(&r.name), "Automation"),
            bot_name: r.bot_id.as_ref().and_then(|id| self.bots.get(id)).cloned().unwrap_or_else(|| "Agent".into()),
            schedule: schedule_text(&r.schedule),
            next_run_at: r.next_run_at.as_ref().and_then(parse_timestamp),
            enabled: r.enabled,
        }).collect();
        self.routines.sort_by(|a, b| b.enabled.cmp(&a.enabled)
            .then_with(|| a.next_run_at.unwrap_or(u64::MAX).cmp(&b.next_run_at.unwrap_or(u64::MAX)))
            .then_with(|| a.id.cmp(&b.id)));
        // Keep mappings from runs removed by the server's history retention as well.
        // Otherwise their thread tails could reappear as duplicate agent completions.
        for run in &data.runs {
            if let Some(thread) = run.thread_id.as_ref().filter(|s| !s.is_empty()) {
                self.automation_threads.insert(thread.clone());
            }
        }
        for run in &data.runs {
            let previous = self.runs.get(&run.id);
            let transitioned = previous.map(|p| p.status != run.status).unwrap_or(true);
            let name = label(run.routine_name.as_deref().or_else(|| run.routine_id.as_ref()
                .and_then(|id| routine_names.get(id).map(String::as_str))), "Automation");
            let bot = run.bot_id.as_ref().and_then(|id| self.bots.get(id))
                .cloned().unwrap_or_else(|| "Agent".into());
            let mut additions = Vec::new();
            if run.active() || run.started_at.as_ref().and_then(parse_timestamp).is_some() {
                if let Some(at_ms) = run.started() {
                    additions.push((ActivityEvent {
                        id: format!("automation:{}:started", run.id),
                        title: "Automation started".into(),
                        detail: format!("{name} · {bot}"), at_ms, kind: ActivityKind::AutomationStarted,
                    }, run.active() && transitioned));
                }
            }
            if let (Some(kind), Some(at_ms)) = (run.terminal_kind(), run.finished()) {
                let (suffix, status) = match run.status.as_str() {
                    "completed" => ("completed", "completed"),
                    "missed" => ("failed", "missed"),
                    _ => ("failed", "failed"),
                };
                additions.push((ActivityEvent {
                    id: format!("automation:{}:{suffix}", run.id),
                    title: format!("Automation {status}"), detail: format!("{name} · {bot}"), at_ms, kind,
                }, transitioned && run.finished_at.as_ref().and_then(parse_timestamp).is_some()));
            }
            for (event, transitioned) in additions {
                self.record(event, None, !baseline && transitioned, now, notifications);
            }
        }
        self.runs = data.runs.into_iter().map(|r| (r.id.clone(), r)).collect();
        self.routines_baselined = true;
    }

    fn read_events(&mut self, now: u64, notifications: &mut Vec<ActivityEvent>, errors: &mut Vec<String>) {
        let directory = match fs::read_dir(self.root.join("events")) {
            Ok(directory) => directory,
            Err(_) => {
                errors.push("events: unable to read the activity folder".into());
                return;
            }
        };
        let mut paths = Vec::new();
        for entry in directory {
            match entry {
                Ok(entry) => {
                    if entry.path().extension().is_some_and(|s| s == "ndjson")
                        && entry.file_type().is_ok_and(|kind| kind.is_file()) {
                        paths.push(entry.path());
                    }
                }
                Err(_) => errors.push("events: unable to read the file list".into()),
            }
        }
        paths.sort();
        let mut records = Vec::new();
        let mut successful = true;
        for path in &paths {
            let tail = self.tails.entry(path.clone()).or_default();
            match tail.read(path) {
                Ok((events, malformed)) => {
                    if malformed {
                        successful = false;
                        errors.push("events: invalid activity entries found".into());
                    }
                    for record in events {
                        let Some(at_ms) = record.created_at.as_ref().and_then(parse_timestamp) else { continue; };
                        let thread = record.thread_id.clone().filter(|t| !t.is_empty())
                            .unwrap_or_else(|| path.file_stem().unwrap_or_default().to_string_lossy().into_owned());
                        records.push((at_ms, thread, record));
                    }
                }
                Err(_) => {
                    successful = false;
                    errors.push("events: unable to read the activity file".into());
                }
            }
        }
        let allow_notify = self.events_baselined && self.routines_baselined;
        records.sort_by(|a, b| a.0.cmp(&b.0)
            .then_with(|| event_order(&a.2.event_type).cmp(&event_order(&b.2.event_type)))
            .then_with(|| a.2.event_id.cmp(&b.2.event_id)));
        for (at_ms, thread, record) in records {
            self.ingest_event(record, thread, at_ms, allow_notify, now, notifications);
        }
        if successful {
            self.events_baselined = true;
        }
        self.tails.retain(|path, _| paths.binary_search(path).is_ok());
        if self.turns.len() > STATE_LIMIT {
            let mut oldest: Vec<_> = self.turns.iter().map(|(key, state)| (key.clone(), state.at_ms)).collect();
            oldest.sort_by_key(|(_, time)| *time);
            for (key, _) in oldest.into_iter().take(self.turns.len() - STATE_LIMIT) {
                self.turns.remove(&key);
            }
        }
        if self.requests.len() > STATE_LIMIT {
            let mut oldest: Vec<_> = self.requests.iter().map(|(key, (at, _))| (key.clone(), *at)).collect();
            oldest.sort_by_key(|(_, at)| *at);
            for (key, _) in oldest.into_iter().take(self.requests.len() - STATE_LIMIT) {
                self.requests.remove(&key);
            }
        }
    }

    fn ingest_event(&mut self, record: EventRecord, thread: String, at_ms: u64, allow_notify: bool,
        now: u64, notifications: &mut Vec<ActivityEvent>) {
        let turn = record.turn_id.as_deref().filter(|s| !s.is_empty());
        if matches!(record.event_type.as_str(), "request.opened" | "request.resolved") {
            if let Some(request) = record.request_id.as_deref().or(record.event_id.as_deref()) {
                let id = format!("agent:{thread}:{request}:attention");
                let previous = self.requests.entry(id).or_default();
                if at_ms >= previous.0 {
                    *previous = (at_ms, record.event_type == "request.opened");
                }
            }
        }
        let (kind, phase, title, reason) = match record.event_type.as_str() {
            "turn.started" => (ActivityKind::AgentStarted, "started", "Agent started working", ""),
            "turn.completed" => match record.ok {
                Some(true) => (ActivityKind::AgentCompleted, "completed", "Agent finished responding", ""),
                Some(false) => (ActivityKind::AgentFailed, "failed", "Agent failed to respond", ""),
                None => return,
            },
            "runtime.error" if record.terminal == Some(true) =>
                (ActivityKind::AgentFailed, "failed", "Agent stopped due to an error", ""),
            "request.opened" => (ActivityKind::NeedsAttention, "attention", "Agent needs attention",
                match record.request_type.as_deref() {
                    Some("approval") | Some("permission") => "Waiting for approval",
                    _ => "Waiting for your response",
                }),
            "cap.exhausted" => (ActivityKind::NeedsAttention, "limit", "Agent needs attention",
                "Activity limit reached"),
            // Resolution is not completion of the agent's turn.
            "request.resolved" | "item.completed" => return,
            _ => return,
        };
        if kind != ActivityKind::NeedsAttention {
            if let Some(turn) = turn {
                let state = self.turns.entry((thread.clone(), turn.to_owned())).or_default();
                let terminal = kind != ActivityKind::AgentStarted;
                if at_ms >= state.at_ms && !(state.terminal && !terminal) {
                    state.at_ms = at_ms;
                    state.active = !terminal;
                    state.terminal = terminal;
                }
            }
        }
        let identity = if record.event_type == "request.opened" {
            record.request_id.as_deref().or(record.event_id.as_deref())
        } else { turn.or(record.event_id.as_deref()) };
        let Some(identity) = identity.filter(|s| !s.is_empty()) else { return; };
        let name = self.threads.get(&thread).map(String::as_str).unwrap_or("Agent");
        let event = ActivityEvent {
            id: format!("agent:{thread}:{identity}:{phase}"),
            title: title.into(), detail: if reason.is_empty() { name.into() } else { format!("{name} · {reason}") }, at_ms, kind,
        };
        let mapped = self.automation_threads.contains(&thread) && kind != ActivityKind::NeedsAttention;
        self.record(event, if kind == ActivityKind::NeedsAttention { None } else { Some(thread) },
            allow_notify && !mapped, now, notifications);
    }

    fn record(&mut self, event: ActivityEvent, agent_thread: Option<String>, allow_notify: bool,
        now: u64, notifications: &mut Vec<ActivityEvent>) {
        if agent_thread.as_ref().is_some_and(|thread| self.automation_threads.contains(thread)) {
            return;
        }
        let existing = self.history.iter().position(|entry| entry.event.id == event.id);
        let is_new = existing.is_none();
        if let Some(index) = existing {
            // Keep the first terminal signal when runtime.error and turn.completed
            // describe the same failed turn.
            if self.history[index].event.at_ms > event.at_ms {
                self.history[index] = HistoryEntry { event: event.clone(), agent_thread };
            }
        } else {
            self.history.push(HistoryEntry { event: event.clone(), agent_thread });
        }
        if fresh(event.at_ms, now) {
            let seen = self.notified.insert(event.id.clone(), event.at_ms).is_some();
            if allow_notify && is_new && !seen {
                notifications.push(event);
            }
        }
        self.history.sort_by(|a, b| b.event.at_ms.cmp(&a.event.at_ms).then_with(|| a.event.id.cmp(&b.event.id)));
        // Retain extra hidden agent entries while an automation mapping is being loaded.
        self.history.truncate(HISTORY_LIMIT * 2);
    }
}

impl Tail {
    fn read(&mut self, path: &Path) -> std::io::Result<(Vec<EventRecord>, bool)> {
        let mut file = File::open(path)?;
        let metadata = file.metadata()?;
        #[cfg(windows)]
        let creation = {
            use std::os::windows::fs::MetadataExt;
            metadata.creation_time()
        };
        #[cfg(not(windows))]
        let creation = 0;
        let len = metadata.len();
        let modified = metadata.modified().ok();
        let mut reset = !self.initialized || len < self.offset || creation != self.creation
            || (len == self.offset && modified != self.modified);
        if !reset && !self.anchor.is_empty() {
            file.seek(SeekFrom::Start(self.offset - self.anchor.len() as u64))?;
            let mut anchor = vec![0; self.anchor.len()];
            if file.read_exact(&mut anchor).is_err() || anchor != self.anchor {
                reset = true;
            }
        }
        if reset {
            self.offset = len.saturating_sub(TAIL_BYTES as u64);
            self.pending.clear();
            self.skip_fragment = false;
            self.anchor.clear();
            if self.offset > 0 {
                file.seek(SeekFrom::Start(self.offset - 1))?;
                let mut preceding = [0];
                file.read_exact(&mut preceding)?;
                self.skip_fragment = preceding[0] != b'\n';
            }
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut bytes = Vec::new();
        (&mut file).take(TAIL_BYTES as u64).read_to_end(&mut bytes)?;
        self.offset += bytes.len() as u64;
        self.creation = creation;
        self.modified = modified;
        self.initialized = true;
        let mut records = Vec::new();
        let mut malformed = false;
        for part in bytes.split_inclusive(|byte| *byte == b'\n') {
            let complete = part.last() == Some(&b'\n');
            if self.skip_fragment {
                if complete { self.skip_fragment = false; }
                continue;
            }
            if self.pending.len() + part.len() > TAIL_BYTES {
                self.pending.clear();
                self.skip_fragment = !complete;
                malformed = true;
                continue;
            }
            self.pending.extend_from_slice(part);
            if complete {
                if !self.pending.iter().all(u8::is_ascii_whitespace) {
                    match serde_json::from_slice::<EventRecord>(&self.pending) {
                        Ok(record) => records.push(record),
                        Err(_) => malformed = true,
                    }
                }
                self.pending.clear();
            }
        }
        let anchor_len = self.offset.min(64) as usize;
        file.seek(SeekFrom::Start(self.offset - anchor_len as u64))?;
        let mut anchor = vec![0; anchor_len];
        file.read_exact(&mut anchor)?;
        self.anchor = anchor;
        Ok((records, malformed))
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, &'static str> {
    let mut file = File::open(path).map_err(|_| "unable to read the file")?;
    if file.metadata().map_err(|_| "unable to read file metadata")?.len() > METADATA_BYTES {
        return Err("file exceeds the read limit");
    }
    let mut bytes = Vec::new();
    (&mut file).take(METADATA_BYTES + 1).read_to_end(&mut bytes).map_err(|_| "unable to read the file")?;
    if bytes.len() as u64 > METADATA_BYTES { return Err("file exceeds the read limit"); }
    serde_json::from_slice(&bytes).map_err(|_| "invalid JSON format")
}

fn lease_status(root: &Path, errors: &mut Vec<String>) -> ServerState {
    let path = root.join("openmausbot-server.lease");
    if !path.exists() { return ServerState::default(); }
    let lease = match read_json::<Lease>(&path) {
        Ok(lease) => lease,
        Err(error) => {
            errors.push(format!("server lease: {error}"));
            return ServerState::default();
        }
    };
    if lease.pid == 0 { return ServerState::default(); }
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, lease.pid) else { return ServerState::default(); };
        let mut exit_code = 0;
        let mut online = GetExitCodeProcess(process, &mut exit_code).is_ok() && exit_code == STILL_ACTIVE.0 as u32;
        let lease_at = lease.created_at.as_ref().and_then(parse_timestamp);
        let mut started_at_ms = lease_at;
        if online {
            let (mut created, mut exited, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
            if GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user).is_ok() {
                let birth = filetime_ticks(created).saturating_sub(FILETIME_EPOCH) / 10_000;
                started_at_ms = Some(birth);
                // The lease is written after process creation; a newer process with
                // the same PID must not bring a stale lease back online.
                if let Some(at) = lease_at { online = birth <= at.saturating_add(1000); }
            }
        }
        let _ = CloseHandle(process);
        ServerState { online, started_at_ms }
    }
}

fn filetime_ticks(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

fn fresh(at: u64, now: u64) -> bool {
    at >= now.saturating_sub(RECENT_MS) && at <= now.saturating_add(FUTURE_MS)
}

fn event_order(kind: &str) -> u8 {
    match kind {
        "turn.started" => 0,
        "request.opened" => 1,
        "request.resolved" => 2,
        _ => 3,
    }
}

fn sort_events(events: &mut [ActivityEvent]) {
    events.sort_by(|a, b| b.at_ms.cmp(&a.at_ms).then_with(|| a.id.cmp(&b.id)));
}

fn label(value: Option<&str>, fallback: &str) -> String {
    let clean: String = value.unwrap_or("").chars().filter(|c| !c.is_control()).take(160).collect();
    if clean.trim().is_empty() { fallback.to_owned() } else { clean.trim().to_owned() }
}

pub fn timestamp_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(u64::MAX as u128) as u64
}

/// Numeric timestamps are epoch milliseconds. String timestamps accept RFC 3339 offsets.
pub fn parse_timestamp(value: &Value) -> Option<u64> {
    if let Some(ms) = value.as_u64() { return Some(ms); }
    let text = value.as_str()?;
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) { return text.parse().ok(); }
    parse_iso(text)
}

fn digits(bytes: &[u8]) -> Option<i64> {
    bytes.iter().try_fold(0i64, |n, b| if b.is_ascii_digit() { Some(n * 10 + i64::from(b - b'0')) } else { None })
}

fn parse_iso(text: &str) -> Option<u64> {
    let b = text.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b't')
        || b[13] != b':' || b[16] != b':' { return None; }
    let (year, month, day) = (digits(&b[0..4])?, digits(&b[5..7])?, digits(&b[8..10])?);
    let (hour, minute, second) = (digits(&b[11..13])?, digits(&b[14..16])?, digits(&b[17..19])?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month)
        || !(1..=month_days[(month - 1) as usize]).contains(&day)
        || hour > 23 || minute > 59 || second > 59 { return None; }
    let mut index = 19;
    let mut milliseconds = 0;
    if b.get(index) == Some(&b'.') {
        index += 1;
        let start = index;
        while b.get(index).is_some_and(u8::is_ascii_digit) {
            if index - start < 3 { milliseconds = milliseconds * 10 + i64::from(b[index] - b'0'); }
            index += 1;
        }
        if index == start || index - start > 9 { return None; }
        for _ in index - start..3 { milliseconds *= 10; }
    }
    let offset = match b.get(index) {
        Some(b'Z' | b'z') if index + 1 == b.len() => 0,
        Some(sign @ (b'+' | b'-')) if index + 6 == b.len() && b[index + 3] == b':' => {
            let h = digits(&b[index + 1..index + 3])?;
            let m = digits(&b[index + 4..index + 6])?;
            if h > 23 || m > 59 { return None; }
            (h * 60 + m) * if *sign == b'+' { 1 } else { -1 }
        }
        _ => return None,
    };
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    let ms = (((days * 24 + hour) * 60 + minute - offset) * 60 + second) * 1000 + milliseconds;
    u64::try_from(ms).ok()
}

// These kernel32 conversions are declared locally because the existing windows
// feature list does not include Win32_System_Time. No Cargo feature change is needed.
#[link(name = "kernel32")]
extern "system" {
    fn FileTimeToSystemTime(filetime: *const FILETIME, systemtime: *mut SYSTEMTIME) -> BOOL;
    fn SystemTimeToTzSpecificLocalTime(timezone: *const core::ffi::c_void, utc: *const SYSTEMTIME, local: *mut SYSTEMTIME) -> BOOL;
}

fn local_system_time(ms: u64) -> Option<SYSTEMTIME> {
    let ticks = ms.checked_mul(10_000)?.checked_add(FILETIME_EPOCH)?;
    let ft = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let (mut utc, mut local) = (SYSTEMTIME::default(), SYSTEMTIME::default());
    unsafe {
        if !FileTimeToSystemTime(&ft, &mut utc).as_bool()
            || !SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local).as_bool() { return None; }
    }
    Some(local)
}

pub fn local_time(ms: u64) -> String {
    local_system_time(ms).map(|t| format!("{:02}:{:02}", t.wHour, t.wMinute)).unwrap_or_else(|| "--:--".into())
}

pub fn local_date(ms: u64) -> String {
    local_system_time(ms).map(|t| format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay)).unwrap_or_default()
}

pub fn schedule_text(schedule: &Value) -> String {
    match schedule.get("type").and_then(Value::as_str) {
        Some("interval") => match schedule.get("everyMinutes").and_then(Value::as_u64).filter(|m| *m > 0) {
            Some(minutes) if minutes % 1440 == 0 => format!("Every {} day{}", minutes / 1440, if minutes == 1440 { "" } else { "s" }),
            Some(minutes) if minutes % 60 == 0 => format!("Every {} hour{}", minutes / 60, if minutes == 60 { "" } else { "s" }),
            Some(minutes) => format!("Every {minutes} minute{}", if minutes == 1 { "" } else { "s" }),
            None => "Interval not set".into(),
        },
        Some("daily") => {
            let Some(time) = schedule.get("time").and_then(Value::as_str).filter(|t| valid_clock(t)) else {
                return "Daily time not set".into();
            };
            let days = schedule.get("weekdays").and_then(Value::as_array).map(|days| {
                let mut days: Vec<_> = days.iter().filter_map(Value::as_u64).filter(|d| *d < 7).collect();
                days.sort_unstable(); days.dedup(); days
            });
            let days = match days.as_deref() {
                None | Some([0, 1, 2, 3, 4, 5, 6]) => "Every day".into(),
                Some([1, 2, 3, 4, 5]) => "Monday–Friday".into(),
                Some([0, 6]) => "Saturday–Sunday".into(),
                Some([]) => "No active days".into(),
                Some(days) => days.iter().map(|d| ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][*d as usize])
                    .collect::<Vec<_>>().join(", "),
            };
            format!("{days} · {time}")
        }
        Some("once") => schedule.get("at").and_then(parse_timestamp)
            .map(|at| format!("Once · {} {}", local_date(at), local_time(at)))
            .unwrap_or_else(|| "Time not set".into()),
        _ => "Unrecognized schedule".into(),
    }
}

fn valid_clock(time: &str) -> bool {
    let b = time.as_bytes();
    b.len() == 5 && b[2] == b':' && digits(&b[..2]).is_some_and(|n| n < 24)
        && digits(&b[3..]).is_some_and(|n| n < 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("nocan-activity-{}-{}-{}",
                std::process::id(), timestamp_ms(), FIXTURE_ID.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(root.join("events")).unwrap();
            let fixture = Self(root);
            fixture.write("bots.json", &json!([{
                "id":"bot", "name":"Asisten", "threadId":"chat",
                "tasks":[{"threadId":"task", "title":"Uji tugas"}]
            }]));
            fixture.write("routines.json", &json!({"routines":[], "runs":[]}));
            fixture
        }

        fn write(&self, name: &str, value: &Value) {
            fs::write(self.0.join(name), serde_json::to_vec(value).unwrap()).unwrap();
        }

        fn append(&self, thread: &str, value: &Value) {
            let mut file = fs::OpenOptions::new().create(true).append(true)
                .open(self.0.join("events").join(format!("{thread}.ndjson"))).unwrap();
            serde_json::to_writer(&mut file, value).unwrap();
            file.write_all(b"\n").unwrap();
        }

        fn monitor(&self) -> ActivityMonitor { ActivityMonitor::new(self.0.clone()) }

        fn runs(&self, runs: Value) {
            self.write("routines.json", &json!({
                "routines":[{"id":"routine", "name":"Cek berkala", "botId":"bot", "enabled":true,
                    "schedule":{"type":"interval", "everyMinutes":15}, "nextRunAt":1234}],
                "runs":runs
            }));
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    fn event(kind: &str, id: &str, turn: &str, at: u64) -> Value {
        json!({"eventId":id, "threadId":"chat", "turnId":turn, "type":kind,
            "createdAt":at, "ok":true})
    }

    fn run(status: &str, at: u64) -> Value {
        let mut run = json!({"id":"run", "routineId":"routine", "routineName":"Cek berkala",
            "botId":"bot", "threadId":"auto-thread", "status":status,
            "createdAt":at.saturating_sub(1000), "startedAt":at});
        if matches!(status, "completed" | "failed" | "missed") {
            run["finishedAt"] = json!(at);
        }
        run
    }

    #[test]
    fn startup_populates_history_without_replaying_recent_events_or_runs() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        fixture.runs(json!([run("completed", now)]));
        fixture.append("chat", &event("turn.started", "start", "turn", now - 1000));
        fixture.append("chat", &event("turn.completed", "done", "turn", now));
        let mut monitor = fixture.monitor();
        let first = monitor.poll_at(now);
        assert!(first.notifications.is_empty());
        assert_eq!(first.snapshot.events.len(), 4);
        assert!(first.snapshot.events.iter().any(|e| e.kind == ActivityKind::AgentCompleted));
        assert_eq!(first.snapshot.routines[0].schedule, "Every 15 minutes");
        assert_eq!(first.snapshot.routines[0].bot_name, "Asisten");
        assert!(monitor.poll_at(now + 1000).notifications.is_empty());
    }

    #[test]
    fn new_completion_notifies_once_including_rewritten_and_evicted_records() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        fixture.append("chat", &event("turn.completed", "done", "turn", now));
        let poll = monitor.poll_at(now + 1000);
        assert_eq!(poll.notifications.len(), 1);
        assert_eq!(poll.notifications[0].kind, ActivityKind::AgentCompleted);
        fixture.append("chat", &event("turn.completed", "duplicate-id", "turn", now));
        assert!(monitor.poll_at(now + 2000).notifications.is_empty());
        for i in 0..150 {
            fixture.append("chat", &event("turn.completed", &format!("e{i}"), &format!("t{i}"), now + 3000));
        }
        let poll = monitor.poll_at(now + 4000);
        assert_eq!(poll.snapshot.events.len(), HISTORY_LIMIT);
        fs::write(fixture.0.join("events/chat.ndjson"),
            format!("{}\n", event("turn.completed", "different-id", "turn", now))).unwrap();
        assert!(monitor.poll_at(now + 5000).notifications.is_empty());
    }

    #[test]
    fn startup_events_do_not_notify_after_truncation_even_if_evicted_from_history() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        for i in 0..150 {
            fixture.append("chat", &event("turn.completed", &format!("e{i}"), &format!("t{i}"), now));
        }
        let mut monitor = fixture.monitor();
        assert!(monitor.poll_at(now).notifications.is_empty());
        fs::write(fixture.0.join("events/chat.ndjson"),
            format!("{}\n", event("turn.completed", "replayed", "t0", now))).unwrap();
        assert!(monitor.poll_at(now + 1000).notifications.is_empty());
    }

    #[test]
    fn item_completed_and_nonterminal_errors_do_not_complete_turns() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        fixture.append("chat", &event("turn.started", "start", "turn", now));
        fixture.append("chat", &event("item.completed", "item", "turn", now));
        let mut error = event("runtime.error", "retry", "turn", now);
        error["terminal"] = json!(false);
        fixture.append("chat", &error);
        let poll = monitor.poll_at(now);
        assert_eq!(poll.snapshot.events.len(), 1);
        assert_eq!(poll.notifications[0].kind, ActivityKind::AgentStarted);
        assert!(monitor.turns.get(&("chat".into(), "turn".into())).unwrap().active);
        assert_eq!(poll.snapshot.running_agents, 0, "offline must not expose stale active turns");
    }

    #[test]
    fn terminal_runtime_error_and_failed_turn_are_one_failure() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        let mut error = event("runtime.error", "error", "turn", now);
        error["terminal"] = json!(true);
        fixture.append("chat", &error);
        let mut completion = event("turn.completed", "done", "turn", now + 1);
        completion["ok"] = json!(false);
        fixture.append("chat", &completion);
        let poll = monitor.poll_at(now + 1000);
        assert_eq!(poll.notifications.len(), 1);
        assert_eq!(poll.notifications[0].kind, ActivityKind::AgentFailed);
        assert_eq!(poll.snapshot.events.len(), 1);
        assert!(!monitor.turns.get(&("chat".into(), "turn".into())).unwrap().active);
    }

    #[test]
    fn automation_transitions_deduplicate_agent_activity_on_mapped_thread() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        fixture.runs(json!([run("running", now)]));
        let mut started = event("turn.started", "start", "auto", now);
        started["threadId"] = json!("auto-thread");
        fixture.append("auto-thread", &started);
        let first = monitor.poll_at(now + 1000);
        assert_eq!(first.notifications.len(), 1);
        assert_eq!(first.notifications[0].kind, ActivityKind::AutomationStarted);
        fixture.runs(json!([run("completed", now + 2000)]));
        let mut completed = event("turn.completed", "done", "auto", now + 2000);
        completed["threadId"] = json!("auto-thread");
        fixture.append("auto-thread", &completed);
        let poll = monitor.poll_at(now + 3000);
        assert_eq!(poll.notifications.len(), 1);
        assert_eq!(poll.notifications[0].kind, ActivityKind::AutomationCompleted);
        assert!(poll.snapshot.events.iter().all(|e| matches!(e.kind,
            ActivityKind::AutomationStarted | ActivityKind::AutomationCompleted)));
        assert!(monitor.poll_at(now + 4000).notifications.is_empty());
    }

    #[test]
    fn automation_failed_missed_and_missing_finish_time() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        let mut failed = run("failed", now);
        failed["id"] = json!("failed-run");
        let mut missed = run("missed", now);
        missed["id"] = json!("missed-run");
        let mut unfinished = run("missed", now + 2000);
        unfinished["id"] = json!("unfinished");
        unfinished.as_object_mut().unwrap().remove("finishedAt");
        let mut future = run("missed", now + FUTURE_MS + 1);
        future["id"] = json!("future");
        fixture.runs(json!([failed, missed, unfinished, future]));
        let poll = monitor.poll_at(now);
        assert_eq!(poll.notifications.len(), 2);
        assert!(poll.notifications.iter().all(|e| e.kind == ActivityKind::AutomationFailed));
        assert!(poll.notifications.iter().all(|e| e.id.contains("failed-run") || e.id.contains("missed-run")));
    }

    #[test]
    fn partial_lines_utf8_and_truncation_are_handled_without_replay() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        let mut first = event("turn.completed", "one", "one", now);
        first["ignored"] = json!("tulisan 日本語");
        let bytes = format!("{first}\n").into_bytes();
        let split = bytes.iter().position(|b| *b >= 0xe0).unwrap() + 1;
        let path = fixture.0.join("events/chat.ndjson");
        fs::write(&path, &bytes[..split]).unwrap();
        let partial = monitor.poll_at(now);
        assert!(partial.notifications.is_empty());
        assert!(partial.snapshot.sync_error.is_none());
        fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(&bytes[split..]).unwrap();
        assert_eq!(monitor.poll_at(now).notifications.len(), 1);
        fs::write(&path, b"{\"eventId\":\"cut").unwrap();
        assert!(monitor.poll_at(now).notifications.is_empty());
        fs::write(&path, format!("{}\n", event("turn.completed", "two", "two", now))).unwrap();
        let replacement = monitor.poll_at(now);
        assert_eq!(replacement.notifications.len(), 1);
        assert!(replacement.notifications[0].id.contains(":two:"));
        assert!(replacement.snapshot.sync_error.is_none());
    }

    #[test]
    fn initial_tail_is_bounded_and_skips_leading_fragment() {
        let fixture = Fixture::new();
        let path = fixture.0.join("events/large.ndjson");
        let mut bytes = vec![b'x'; TAIL_BYTES + 200];
        bytes.push(b'\n');
        bytes.extend_from_slice(format!("{}\n", event("turn.completed", "last", "last", timestamp_ms())).as_bytes());
        fs::write(&path, bytes).unwrap();
        let mut tail = Tail::default();
        let (records, malformed) = tail.read(&path).unwrap();
        assert!(!malformed);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].event_id.as_deref(), Some("last"));
        assert_eq!(tail.offset, fs::metadata(&path).unwrap().len());
        assert!(tail.pending.is_empty());
        assert!(tail.read(&path).unwrap().0.is_empty());
    }

    #[test]
    fn same_length_replacement_and_oversized_incremental_line_recover() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let path = fixture.0.join("events/chat.ndjson");
        fs::write(&path, format!("{}\n", event("turn.started", "aaa", "aaa", now))).unwrap();
        let mut tail = Tail::default();
        assert_eq!(tail.read(&path).unwrap().0.len(), 1);
        fs::write(&path, format!("{}\n", event("turn.started", "bbb", "bbb", now))).unwrap();
        assert_eq!(tail.read(&path).unwrap().0[0].event_id.as_deref(), Some("bbb"));
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&vec![b'x'; TAIL_BYTES + 20]).unwrap();
        assert!(tail.read(&path).unwrap().0.is_empty());
        let (_, malformed) = tail.read(&path).unwrap();
        assert!(malformed);
        assert!(tail.pending.len() <= TAIL_BYTES);
        file.write_all(format!("\n{}\n", event("turn.completed", "ccc", "ccc", now)).as_bytes()).unwrap();
        let (records, malformed) = tail.read(&path).unwrap();
        assert!(!malformed);
        assert_eq!(records[0].event_id.as_deref(), Some("ccc"));
    }

    #[test]
    fn malformed_metadata_retains_last_good_state_and_does_not_leak_contents() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        fixture.runs(json!([run("completed", now)]));
        let mut monitor = fixture.monitor();
        let good = monitor.poll_at(now);
        fs::write(fixture.0.join("bots.json"), b"{PRIVATE_CONTENT").unwrap();
        fs::write(fixture.0.join("routines.json"), b"{PRIVATE_CONTENT").unwrap();
        let malformed = monitor.poll_at(now + 1000);
        assert_eq!(malformed.snapshot.routines.len(), good.snapshot.routines.len());
        assert_eq!(malformed.snapshot.events.len(), good.snapshot.events.len());
        let error = malformed.snapshot.sync_error.unwrap();
        assert!(error.contains("invalid JSON format"));
        assert!(!error.contains("PRIVATE_CONTENT"));
        assert_eq!(monitor.bots.get("bot").map(String::as_str), Some("Asisten"));
    }

    #[test]
    fn initial_routines_failure_does_not_backfill_notifications_after_recovery() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        fs::write(fixture.0.join("routines.json"), b"{").unwrap();
        fixture.append("chat", &event("turn.completed", "old", "old", now));
        let mut monitor = fixture.monitor();
        assert!(monitor.poll_at(now).notifications.is_empty());
        fixture.runs(json!([run("completed", now)]));
        let recovered = monitor.poll_at(now + 1000);
        assert!(recovered.notifications.is_empty());
        assert!(recovered.snapshot.sync_error.is_none());
        assert!(monitor.poll_at(now + 2000).notifications.is_empty());
        fixture.append("chat", &event("turn.completed", "fresh", "fresh", now + 3000));
        assert_eq!(monitor.poll_at(now + 3000).notifications.len(), 1);
    }

    #[test]
    fn malformed_event_does_not_erase_good_history_or_notify_baseline() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        fixture.append("chat", &event("turn.completed", "old", "old", now));
        let path = fixture.0.join("events/chat.ndjson");
        fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"{broken}\n").unwrap();
        let mut monitor = fixture.monitor();
        let first = monitor.poll_at(now);
        assert!(first.notifications.is_empty());
        assert!(first.snapshot.sync_error.is_some());
        let second = monitor.poll_at(now + 1000);
        assert!(second.notifications.is_empty());
        assert_eq!(second.snapshot.events.len(), 1);
        assert!(second.snapshot.sync_error.is_none());
    }

    #[test]
    fn only_recent_events_with_bounded_clock_skew_notify() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        for (id, at) in [("old", now - RECENT_MS - 1), ("boundary", now - RECENT_MS),
            ("future", now + FUTURE_MS), ("too-future", now + FUTURE_MS + 1)] {
            fixture.append("chat", &event("turn.completed", id, id, at));
        }
        let poll = monitor.poll_at(now);
        assert_eq!(poll.snapshot.events.len(), 4);
        assert_eq!(poll.notifications.len(), 2);
        assert!(poll.notifications.iter().all(|e| e.id.contains(":boundary:") || e.id.contains(":future:")));
        assert!(poll.snapshot.events.windows(2).all(|w| w[0].at_ms >= w[1].at_ms));
    }

    #[test]
    fn attention_requests_resolve_without_completing_agents() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = fixture.monitor();
        monitor.poll_at(now);
        let mut opened = event("request.opened", "open", "turn", now);
        opened["requestId"] = json!("request");
        opened["requestType"] = json!("approval");
        fixture.append("chat", &opened);
        let mut resolved = event("request.resolved", "resolved", "turn", now + 1);
        resolved["requestId"] = json!("request");
        fixture.append("chat", &resolved);
        let poll = monitor.poll_at(now + 1000);
        assert!(poll.notifications.is_empty());
        assert_eq!(poll.snapshot.events[0].kind, ActivityKind::NeedsAttention);
        assert!(poll.snapshot.events.iter().all(|e| e.kind != ActivityKind::AgentCompleted));
        let limit = event("cap.exhausted", "limit", "limit-turn", now + 2000);
        fixture.append("chat", &limit);
        let poll = monitor.poll_at(now + 2000);
        assert_eq!(poll.notifications.len(), 1);
        assert_eq!(poll.notifications[0].kind, ActivityKind::NeedsAttention);
    }

    #[test]
    fn live_lease_pid_is_validated_and_offline_counts_are_zero() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        fixture.write("openmausbot-server.lease", &json!({
            "pid":std::process::id(), "createdAt":now, "token":"PRIVATE_LEASE_TOKEN"
        }));
        fixture.append("chat", &event("turn.started", "start", "turn", now));
        fixture.runs(json!([run("running", now)]));
        let mut monitor = fixture.monitor();
        let online = monitor.poll_at(now);
        assert!(online.snapshot.server_online);
        assert_eq!(online.snapshot.running_agents, 1);
        assert_eq!(online.snapshot.running_automations, 1);
        fixture.write("openmausbot-server.lease", &json!({"pid":std::process::id(), "createdAt":1}));
        let stale = monitor.poll_at(now);
        assert!(!stale.snapshot.server_online, "a reused PID must not validate an older lease");
        assert_eq!(stale.snapshot.running_agents, 0);
        assert_eq!(stale.snapshot.running_automations, 0);
        fixture.write("openmausbot-server.lease", &json!({"pid":u32::MAX, "createdAt":now}));
        assert!(!monitor.poll_at(now).snapshot.server_online);
    }

    #[test]
    fn old_unmatched_turns_and_runs_do_not_survive_server_generation_change() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        fixture.write("openmausbot-server.lease", &json!({"pid":std::process::id(), "createdAt":now}));
        let server = lease_status(&fixture.0, &mut Vec::new());
        assert!(server.online);
        let before_birth = server.started_at_ms.unwrap().saturating_sub(10_000);
        fixture.append("chat", &event("turn.started", "old", "old", before_birth));
        fixture.runs(json!([run("running", before_birth)]));
        let mut monitor = fixture.monitor();
        let stale = monitor.poll_at(now);
        assert!(stale.snapshot.server_online);
        assert_eq!(stale.snapshot.running_agents, 0);
        assert_eq!(stale.snapshot.running_automations, 0);
        assert_eq!(stale.snapshot.events.len(), 2, "historical starts remain in history");
        fixture.append("chat", &event("turn.started", "new", "new", now));
        fixture.runs(json!([run("running", now)]));
        let current = monitor.poll_at(now);
        assert_eq!(current.snapshot.running_agents, 1);
        assert_eq!(current.snapshot.running_automations, 1);
        assert!(!ServerState { online:true, started_at_ms:None }.current_activity(now - RECENT_MS - 1, now));
    }

    #[test]
    fn same_size_rewrite_with_unchanged_suffix_is_rechecked() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let path = fixture.0.join("events/chat.ndjson");
        let common = format!("{}\n", event("turn.completed", "suffix", "suffix", now));
        fs::write(&path, format!("{}\n{common}", event("turn.completed", "aaa", "aaa", now))).unwrap();
        let mut tail = Tail::default();
        assert_eq!(tail.read(&path).unwrap().0.len(), 2);
        fs::write(&path, format!("{}\n{common}", event("turn.completed", "bbb", "bbb", now))).unwrap();
        // Make the changed metadata deterministic even on filesystems with coarse timestamps.
        tail.modified = Some(UNIX_EPOCH);
        let (records, malformed) = tail.read(&path).unwrap();
        assert!(!malformed);
        assert_eq!(records[0].event_id.as_deref(), Some("bbb"));
    }

    #[test]
    fn missing_source_can_appear_later_without_startup_replay() {
        let fixture = Fixture::new();
        let now = timestamp_ms();
        let mut monitor = ActivityMonitor::new(fixture.0.join("missing"));
        let missing = monitor.poll_at(now);
        assert!(!missing.snapshot.source_found);
        assert!(!missing.snapshot.server_online);
        assert_eq!(missing.snapshot.running_agents, 0);
        monitor.root = fixture.0.clone();
        fixture.append("chat", &event("turn.completed", "old", "old", now));
        assert!(monitor.poll_at(now).notifications.is_empty());
    }

    #[test]
    fn timestamp_offsets_fraction_leap_days_and_invalid_dates() {
        let utc = parse_timestamp(&json!("2026-10-29T17:30:45.123Z")).unwrap();
        assert_eq!(Some(utc), parse_timestamp(&json!("2026-10-30T00:30:45.123+07:00")));
        assert_eq!(Some(utc), parse_timestamp(&json!("2026-10-29T12:00:45.123-05:30")));
        assert_eq!(Some(utc), parse_timestamp(&json!(utc)));
        assert_eq!(Some(utc), parse_timestamp(&json!(utc.to_string())));
        assert_eq!(Some(0), parse_timestamp(&json!("1970-01-01T00:00:00Z")));
        assert_eq!(Some(100), parse_timestamp(&json!("1970-01-01T00:00:00.1Z")));
        assert_eq!(Some(123), parse_timestamp(&json!("1970-01-01T00:00:00.123456789Z")));
        assert!(parse_timestamp(&json!("2024-02-29T00:00:00Z")).is_some());
        for invalid in ["2026-02-29T00:00:00Z", "2026-04-31T00:00:00Z", "2026-01-01T24:00:00Z",
            "2026-01-01T00:60:00Z", "2026-01-01T00:00:00", "2026-01-01T00:00:00+24:00",
            "1969-12-31T23:59:59Z", "2026-01-01T00:00:00.Z", "🐭"] {
            assert_eq!(parse_timestamp(&json!(invalid)), None, "{invalid}");
        }
        assert_eq!(parse_timestamp(&json!(-1)), None);
        assert_eq!(parse_timestamp(&json!(12.5)), None);
        assert_eq!(parse_timestamp(&Value::Null), None);
    }

    #[test]
    fn local_formatting_uses_windows_timezone_and_handles_invalid_range() {
        let now = timestamp_ms();
        let expected = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        assert_eq!(local_date(now), format!("{:04}-{:02}-{:02}", expected.wYear, expected.wMonth, expected.wDay));
        assert_eq!(local_time(now), format!("{:02}:{:02}", expected.wHour, expected.wMinute));
        assert_eq!(local_time(u64::MAX), "--:--");
        assert_eq!(local_date(u64::MAX), "");
    }

    #[test]
    fn schedule_descriptions_cover_interval_daily_once_and_invalid_data() {
        assert_eq!(schedule_text(&json!({"type":"interval", "everyMinutes":60})), "Every 1 hour");
        assert_eq!(schedule_text(&json!({"type":"interval", "everyMinutes":1440})), "Every 1 day");
        assert_eq!(schedule_text(&json!({"type":"interval", "everyMinutes":1})), "Every 1 minute");
        assert_eq!(schedule_text(&json!({"type":"interval", "everyMinutes":15})), "Every 15 minutes");
        assert_eq!(schedule_text(&json!({"type":"interval", "everyMinutes":120})), "Every 2 hours");
        assert_eq!(schedule_text(&json!({"type":"interval", "everyMinutes":2880})), "Every 2 days");
        assert_eq!(schedule_text(&json!({"type":"interval", "everyMinutes":0})), "Interval not set");
        assert_eq!(schedule_text(&json!({"type":"daily", "time":"09:20", "weekdays":[6,5,4,3,2,1,0]})), "Every day · 09:20");
        assert_eq!(schedule_text(&json!({"type":"daily", "time":"09:20", "weekdays":[1,2,3,4,5]})), "Monday–Friday · 09:20");
        assert_eq!(schedule_text(&json!({"type":"daily", "time":"09:20", "weekdays":[1,3,3,5]})), "Mon, Wed, Fri · 09:20");
        assert_eq!(schedule_text(&json!({"type":"daily", "time":"09:20", "weekdays":[0,6]})), "Saturday–Sunday · 09:20");
        assert_eq!(schedule_text(&json!({"type":"daily", "time":"09:20", "weekdays":[]})), "No active days · 09:20");
        assert_eq!(schedule_text(&json!({"type":"daily", "time":"99:99"})), "Daily time not set");
        let at = parse_timestamp(&json!("2026-10-29T17:30:00Z")).unwrap();
        assert_eq!(schedule_text(&json!({"type":"once", "at":at})), format!("Once · {} {}", local_date(at), local_time(at)));
        assert_eq!(schedule_text(&json!({"type":"once"})), "Time not set");
        assert_eq!(schedule_text(&json!({"type":"unknown"})), "Unrecognized schedule");
    }
}
