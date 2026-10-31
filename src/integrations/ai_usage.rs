use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};
use serde::Deserialize;
use windows::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiTab {
    Antigravity = 0,
    Claude = 1,
    Codex = 2,
    OpenCode = 3,
}

impl AiTab {
    pub fn next(&self) -> Self {
        match self {
            AiTab::Antigravity => AiTab::Claude,
            AiTab::Claude => AiTab::Codex,
            AiTab::Codex => AiTab::OpenCode,
            AiTab::OpenCode => AiTab::Antigravity,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProviderUsage {
    pub name: String,
    pub title: String,
    pub active_model: String,
    pub today_tokens: u64,
    pub today_turns: u64,
    pub total_tokens: u64,
    pub total_turns: u64,
    pub input_tokens_today: u64,
    pub output_tokens_today: u64,
    pub cached_tokens_today: u64,
    pub quota_percent: Option<u32>,
    pub is_active: bool,
    pub status_text: String,
}

#[derive(Clone, Debug, Default)]
pub struct AiUsageSummary {
    pub total_today_tokens: u64,
    pub total_today_turns: u64,
    pub total_lifetime_tokens: u64,
    pub antigravity: ProviderUsage,
    pub claude: ProviderUsage,
    pub codex: ProviderUsage,
    pub opencode: ProviderUsage,
    pub last_updated: String,
}

#[derive(Clone, Debug)]
pub struct TimeContext {
    pub now_sec: u64,
    pub today_local: String,
    pub today_utc: String,
    pub yesterday_local: String,
    pub utc_cutoff_hour: u32,
    pub local_midnight_epoch_sec: u64,
}

pub struct AiUsageService {
    data: Arc<RwLock<AiUsageSummary>>,
    _worker: Option<thread::JoinHandle<()>>,
    poll_interval: Arc<AtomicU64>,
}

impl AiUsageService {
    pub fn new() -> Self {
        let initial = Arc::new(RwLock::new(Self::fetch_all()));
        let data_clone = Arc::clone(&initial);

        let poll_interval = Arc::new(AtomicU64::new(30));
        let interval = Arc::clone(&poll_interval);
        let handle = thread::spawn(move || loop {
            thread::park_timeout(Duration::from_secs(interval.load(Ordering::Relaxed).clamp(1,60)));
            let fresh = Self::fetch_all();
            if let Ok(mut lock) = data_clone.write() {
                *lock = fresh;
            }
        });

        Self {
            data: initial,
            _worker: Some(handle),
            poll_interval,
        }
    }

    pub fn set_poll_interval(&self, secs: u64) {
        let next = secs.clamp(1,60);
        if self.poll_interval.swap(next, Ordering::Relaxed) != next {
            if let Some(worker) = &self._worker { worker.thread().unpark(); }
        }
    }

    pub fn get_summary(&self) -> AiUsageSummary {
        self.data.read().map(|d| d.clone()).unwrap_or_default()
    }

    pub fn fetch_all() -> AiUsageSummary {
        let ctx = get_time_context();
        let now_time = get_now_time_string();

        let antigravity = scan_antigravity(&ctx);
        let codex = scan_codex(&ctx);
        let claude = scan_claude(&ctx);
        let opencode = scan_opencode(&ctx);

        let total_today_tokens = antigravity.today_tokens + codex.today_tokens + claude.today_tokens + opencode.today_tokens;
        let total_today_turns = antigravity.today_turns + codex.today_turns + claude.today_turns + opencode.today_turns;
        let total_lifetime_tokens = antigravity.total_tokens + codex.total_tokens + claude.total_tokens + opencode.total_tokens;

        AiUsageSummary {
            total_today_tokens,
            total_today_turns,
            total_lifetime_tokens,
            antigravity,
            claude,
            codex,
            opencode,
            last_updated: now_time,
        }
    }
}

// ---------------------------------------------------------------------------
// Time & Midnight Helpers
// ---------------------------------------------------------------------------
pub fn get_time_context() -> TimeContext {
    unsafe {
        let st_loc = GetLocalTime();
        let st_utc = GetSystemTime();

        let today_local = format!("{:04}-{:02}-{:02}", st_loc.wYear, st_loc.wMonth, st_loc.wDay);
        let today_utc = format!("{:04}-{:02}-{:02}", st_utc.wYear, st_utc.wMonth, st_utc.wDay);

        let loc_mins = st_loc.wHour as i32 * 60 + st_loc.wMinute as i32;
        let utc_mins = st_utc.wHour as i32 * 60 + st_utc.wMinute as i32;
        let mut diff_mins = loc_mins - utc_mins;
        if diff_mins < -720 { diff_mins += 1440; }
        if diff_mins > 720 { diff_mins -= 1440; }
        let diff_hours = diff_mins / 60;

        let utc_cutoff_hour = ((24 - diff_hours).rem_euclid(24)) as u32;

        let now_sec = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let sec_since_midnight = st_loc.wHour as u64 * 3600 + st_loc.wMinute as u64 * 60 + st_loc.wSecond as u64;
        let local_midnight_epoch_sec = now_sec.saturating_sub(sec_since_midnight);

        // Approximate yesterday's date string for late night sessions
        let yesterday_day = if st_loc.wDay > 1 { st_loc.wDay - 1 } else { 28 };
        let yesterday_local = format!("{:04}-{:02}-{:02}", st_loc.wYear, st_loc.wMonth, yesterday_day);

        TimeContext {
            now_sec,
            today_local,
            today_utc,
            yesterday_local,
            utc_cutoff_hour,
            local_midnight_epoch_sec,
        }
    }
}

pub fn is_recent_iso(iso: &str, ctx: &TimeContext) -> bool {
    if iso.starts_with(&ctx.today_local) || iso.starts_with(&ctx.today_utc) {
        return true;
    }
    if iso.starts_with(&ctx.yesterday_local) {
        if let Some(t_idx) = iso.find('T') {
            if iso.len() >= t_idx + 3 {
                if let Ok(hour) = iso[t_idx + 1..t_idx + 3].parse::<u32>() {
                    return hour >= ctx.utc_cutoff_hour;
                }
            }
        }
    }
    false
}

pub fn is_recent_epoch_ms(ts_ms: u64, ctx: &TimeContext) -> bool {
    let sec = ts_ms / 1000;
    sec >= ctx.local_midnight_epoch_sec || (ctx.now_sec.saturating_sub(sec) <= 86400)
}

// ---------------------------------------------------------------------------
// 1. Antigravity Scanner
// ---------------------------------------------------------------------------
fn scan_antigravity(ctx: &TimeContext) -> ProviderUsage {
    let mut usage = ProviderUsage {
        name: "Antigravity".to_string(),
        title: "Google Gemini".to_string(),
        active_model: "Gemini 3.8 Flash".to_string(),
        status_text: "Standby".to_string(),
        ..Default::default()
    };

    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let ag_cli_dir = PathBuf::from(&user_profile).join(".gemini").join("antigravity-cli");
    let ag_app_dir = PathBuf::from(&user_profile).join(".gemini").join("antigravity");

    // Check active presence
    let presence_dir = ag_cli_dir.join("presence");
    if let Ok(entries) = std::fs::read_dir(&presence_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("lock") {
                usage.is_active = true;
                usage.status_text = "Active Agent".to_string();
                break;
            }
        }
    }

    // Check recent history or active session
    let hist_path = ag_cli_dir.join("history.jsonl");
    if let Ok(meta) = std::fs::metadata(&hist_path) {
        if let Ok(mod_time) = meta.modified() {
            if let Ok(elapsed) = SystemTime::now().duration_since(mod_time) {
                if elapsed.as_secs() < 300 {
                    usage.is_active = true;
                    usage.status_text = "Active Session".to_string();
                } else if !usage.is_active {
                    usage.status_text = format!("Idle ({}m)", elapsed.as_secs() / 60);
                }
            }
        }
    }

    // Read active model from settings.json
    let settings_path = PathBuf::from(&user_profile).join(".gemini").join("settings.json");
    if let Ok(content) = std::fs::read_to_string(&settings_path) {
        if let Some(pos) = content.find("\"modelSelection\":") {
            let rest = &content[pos + 17..];
            if let Some(quote1) = rest.find('"') {
                if let Some(quote2) = rest[quote1 + 1..].find('"') {
                    let m = &rest[quote1 + 1..quote1 + 1 + quote2];
                    if !m.is_empty() {
                        usage.active_model = m.to_string();
                    }
                }
            }
        }
    }

    // Scan transcripts in both brain directories
    let brain_dirs = [ag_cli_dir.join("brain"), ag_app_dir.join("brain")];
    for b_dir in &brain_dirs {
        let Ok(dirs) = std::fs::read_dir(b_dir) else { continue; };
        for d in dirs.flatten() {
            let t_path = d.path().join(".system_generated").join("logs").join("transcript.jsonl");
            if !t_path.exists() {
                continue;
            }

            if let Ok(file) = File::open(&t_path) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || !trimmed.contains("\"PLANNER_RESPONSE\"") {
                        continue;
                    }

                    usage.total_turns += 1;

                    let inp = extract_json_int(trimmed, "\"input_tokens\":");
                    let outp = extract_json_int(trimmed, "\"output_tokens\":");
                    let cache = extract_json_int(trimmed, "\"cache_read_tokens\":");
                    let step_tot = inp + outp;

                    usage.total_tokens += step_tot;

                    let is_today = if let Some(idx) = trimmed.find("\"created_at\":\"") {
                        let sub = &trimmed[idx + 14..];
                        if let Some(end_q) = sub.find('"') {
                            is_recent_iso(&sub[..end_q], ctx)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if is_today {
                        usage.today_turns += 1;
                        usage.today_tokens += step_tot;
                        usage.input_tokens_today += inp;
                        usage.output_tokens_today += outp;
                        usage.cached_tokens_today += cache;
                    }
                }
            }
        }
    }

    if usage.today_tokens > 0 && !usage.is_active {
        usage.status_text = "Synced Today".to_string();
    }

    usage
}

// ---------------------------------------------------------------------------
// 2. Codex Scanner
// ---------------------------------------------------------------------------
fn scan_codex(ctx: &TimeContext) -> ProviderUsage {
    let mut usage = ProviderUsage {
        name: "Codex".to_string(),
        title: "OpenAI Codex".to_string(),
        active_model: "gpt-5.6-luna".to_string(),
        status_text: "Standby".to_string(),
        ..Default::default()
    };

    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let codex_dir = PathBuf::from(&user_profile).join(".codex");
    let opencodex_dir = PathBuf::from(&user_profile).join(".opencodex");

    // 1. Read config.toml for model
    let config_toml = codex_dir.join("config.toml");
    if let Ok(content) = std::fs::read_to_string(&config_toml) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("model = ") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    let cleaned = val.trim().trim_matches('"').trim_matches('\'');
                    if !cleaned.is_empty() {
                        usage.active_model = cleaned.to_string();
                    }
                }
                break;
            }
        }
    }

    // 2. Read quota from .opencodex/codex-quota-cache.json
    let quota_json = opencodex_dir.join("codex-quota-cache.json");
    if let Ok(content) = std::fs::read_to_string(&quota_json) {
        #[derive(Deserialize)]
        struct QuotaFile {
            quotas: Option<HashMap<String, QuotaMain>>,
        }
        #[derive(Deserialize)]
        struct QuotaMain {
            #[serde(rename = "monthlyPercent")]
            monthly_percent: Option<u32>,
        }
        if let Ok(parsed) = serde_json::from_str::<QuotaFile>(&content) {
            if let Some(map) = parsed.quotas {
                if let Some(m) = map.get("__main__") {
                    usage.quota_percent = m.monthly_percent;
                }
            }
        }
    }

    // 3. Check OpenCodex service active
    let pid_file = opencodex_dir.join("ocx.pid");
    if pid_file.exists() {
        usage.is_active = true;
        usage.status_text = "Proxy Online (10100)".to_string();
    } else {
        usage.status_text = "Standby".to_string();
    }

    // 4. Scan Codex CLI sessions in ~/.codex/sessions
    let sessions_dir = codex_dir.join("sessions");
    if sessions_dir.exists() {
        scan_codex_sessions_recursive(&sessions_dir, &mut usage, ctx);
    }

    // 5. Scan .opencodex/usage.jsonl for provider == "openai"
    let ocx_usage = opencodex_dir.join("usage.jsonl");
    if let Ok(file) = File::open(&ocx_usage) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.contains("\"provider\":\"openai\"") {
                let tot = extract_json_int(trimmed, "\"totalTokens\":");
                let inp = extract_json_int(trimmed, "\"inputTokens\":");
                let outp = extract_json_int(trimmed, "\"outputTokens\":");
                let cache = extract_json_int(trimmed, "\"cachedInputTokens\":");
                let ts_ms = extract_json_int(trimmed, "\"timestamp\":");

                usage.total_tokens += tot;
                usage.total_turns += 1;

                if is_recent_epoch_ms(ts_ms, ctx) {
                    usage.today_tokens += tot;
                    usage.today_turns += 1;
                    usage.input_tokens_today += inp;
                    usage.output_tokens_today += outp;
                    usage.cached_tokens_today += cache;

                    if let Some(m_idx) = trimmed.find("\"model\":\"") {
                        let sub = &trimmed[m_idx + 9..];
                        if let Some(end_q) = sub.find('"') {
                            let m = &sub[..end_q];
                            if !m.is_empty() && m != "unknown" {
                                usage.active_model = m.to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    if usage.active_model.starts_with("tokenrouter/") {
        usage.active_model = usage.active_model.replace("tokenrouter/", "");
    }

    usage
}

fn scan_codex_sessions_recursive(dir: &Path, usage: &mut ProviderUsage, ctx: &TimeContext) {
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan_codex_sessions_recursive(&p, usage, ctx);
        } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            if let Ok(file) = File::open(&p) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || !trimmed.contains("\"last_token_usage\"") {
                        continue;
                    }

                    // Look inside "last_token_usage" to get turn incremental tokens
                    if let Some(pos) = trimmed.find("\"last_token_usage\":") {
                        let sub = &trimmed[pos..];
                        let tot = extract_json_int(sub, "\"total_tokens\":");
                        let inp = extract_json_int(sub, "\"input_tokens\":");
                        let outp = extract_json_int(sub, "\"output_tokens\":");
                        let cache = extract_json_int(sub, "\"cached_input_tokens\":");

                        usage.total_tokens += tot;
                        usage.total_turns += 1;

                        if let Some(idx) = trimmed.find("\"timestamp\":\"") {
                            let ts_sub = &trimmed[idx + 13..];
                            if let Some(end_q) = ts_sub.find('"') {
                                if is_recent_iso(&ts_sub[..end_q], ctx) {
                                    usage.today_tokens += tot;
                                    usage.today_turns += 1;
                                    usage.input_tokens_today += inp;
                                    usage.output_tokens_today += outp;
                                    usage.cached_tokens_today += cache;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Claude Scanner
// ---------------------------------------------------------------------------
fn scan_claude(ctx: &TimeContext) -> ProviderUsage {
    let mut usage = ProviderUsage {
        name: "Claude".to_string(),
        title: "Anthropic Claude".to_string(),
        active_model: "Claude 3.7 Sonnet".to_string(),
        status_text: "Standby".to_string(),
        ..Default::default()
    };

    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let claude_dir = PathBuf::from(&user_profile).join(".claude");
    let om_dir = PathBuf::from(&user_profile).join(".openmausbot").join("usage");

    // 1. Inspect gateway-models.json
    let gw_models = claude_dir.join("cache").join("gateway-models.json");
    if let Ok(content) = std::fs::read_to_string(&gw_models) {
        if let Some(pos) = content.find("\"display_name\":\"") {
            let sub = &content[pos + 16..];
            if let Some(end_q) = sub.find('"') {
                usage.active_model = sub[..end_q].to_string();
                usage.is_active = true;
                usage.status_text = "Gateway Ready (10100)".to_string();
            }
        }
    }

    // 2. Scan OpenMausbot for any Claude
    if let Ok(entries) = std::fs::read_dir(&om_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Ok(file) = File::open(&p) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    let lower = trimmed.to_lowercase();
                    if lower.contains("claude") || lower.contains("anthropic") {
                        let inp = extract_json_int(trimmed, "\"input\":");
                        let outp = extract_json_int(trimmed, "\"output\":");
                        let tot = inp + outp;
                        usage.total_turns += 1;
                        usage.total_tokens += tot;

                        if let Some(idx) = trimmed.find("\"at\":\"") {
                            let sub = &trimmed[idx + 6..];
                            if let Some(end_q) = sub.find('"') {
                                if is_recent_iso(&sub[..end_q], ctx) {
                                    usage.today_turns += 1;
                                    usage.today_tokens += tot;
                                    usage.input_tokens_today += inp;
                                    usage.output_tokens_today += outp;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Scan OpenCodex for claude/anthropic requests
    let opencodex_dir = PathBuf::from(&user_profile).join(".opencodex");
    let ocx_usage = opencodex_dir.join("usage.jsonl");
    if let Ok(file) = File::open(&ocx_usage) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if trimmed.contains("claude") || trimmed.contains("anthropic") {
                let tot = extract_json_int(trimmed, "\"totalTokens\":");
                let inp = extract_json_int(trimmed, "\"inputTokens\":");
                let outp = extract_json_int(trimmed, "\"outputTokens\":");
                let ts_ms = extract_json_int(trimmed, "\"timestamp\":");

                usage.total_tokens += tot;
                usage.total_turns += 1;

                if is_recent_epoch_ms(ts_ms, ctx) {
                    usage.today_tokens += tot;
                    usage.today_turns += 1;
                    usage.input_tokens_today += inp;
                    usage.output_tokens_today += outp;
                }
            }
        }
    }

    if usage.status_text == "Standby" {
        usage.status_text = "Ready (Standby)".to_string();
    }

    usage
}

// ---------------------------------------------------------------------------
// 4. OpenCode Scanner
// ---------------------------------------------------------------------------
fn scan_opencode(ctx: &TimeContext) -> ProviderUsage {
    let mut usage = ProviderUsage {
        name: "OpenCode".to_string(),
        title: "OpenCode & WorkBuddy".to_string(),
        active_model: "deepseek-v4.1-flash".to_string(),
        status_text: "Online Ready".to_string(),
        ..Default::default()
    };

    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let opencodex_dir = PathBuf::from(&user_profile).join(".opencodex");
    let ocx_usage = opencodex_dir.join("usage.jsonl");

    if let Ok(file) = File::open(&ocx_usage) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let is_opencode = trimmed.contains("\"provider\":\"workbuddy-ai\"")
                || trimmed.contains("\"provider\":\"tokenrouter\"")
                || trimmed.contains("\"provider\":\"opencode-free\"");

            if is_opencode {
                let tot = extract_json_int(trimmed, "\"totalTokens\":");
                let inp = extract_json_int(trimmed, "\"inputTokens\":");
                let outp = extract_json_int(trimmed, "\"outputTokens\":");
                let cache = extract_json_int(trimmed, "\"cachedInputTokens\":");
                let ts_ms = extract_json_int(trimmed, "\"timestamp\":");

                usage.total_tokens += tot;
                usage.total_turns += 1;

                if is_recent_epoch_ms(ts_ms, ctx) {
                    usage.today_tokens += tot;
                    usage.today_turns += 1;
                    usage.input_tokens_today += inp;
                    usage.output_tokens_today += outp;
                    usage.cached_tokens_today += cache;

                    if let Some(m_idx) = trimmed.find("\"model\":\"") {
                        let sub = &trimmed[m_idx + 9..];
                        if let Some(end_q) = sub.find('"') {
                            let m = &sub[..end_q];
                            if !m.is_empty() && m != "unknown" {
                                usage.active_model = m.to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    if usage.today_tokens > 0 {
        usage.is_active = true;
        usage.status_text = "Active Today".to_string();
    }

    if usage.active_model.starts_with("opencode/") {
        usage.active_model = usage.active_model.replace("opencode/", "");
    }
    if usage.active_model.starts_with("tokenrouter/") {
        usage.active_model = usage.active_model.replace("tokenrouter/", "");
    }

    usage
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
fn extract_json_int(line: &str, key: &str) -> u64 {
    if let Some(pos) = line.find(key) {
        let rest = &line[pos + key.len()..];
        let trimmed = rest.trim_start();
        let end = trimmed.find(|c: char| !c.is_ascii_digit()).unwrap_or(trimmed.len());
        if end > 0 {
            return trimmed[..end].parse::<u64>().unwrap_or(0);
        }
    }
    0
}

pub fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000_000 {
        format!("{:.1}B", tokens as f64 / 1_000_000_000.0)
    } else if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

pub fn get_today_date_prefix() -> String {
    unsafe {
        let st = GetLocalTime();
        format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay)
    }
}

pub fn get_now_time_string() -> String {
    unsafe {
        let st = GetLocalTime();
        format!("{:02}:{:02}:{:02} UTC+07", st.wHour, st.wMinute, st.wSecond)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_ai_summary_has_data() {
        let summary = AiUsageService::fetch_all();
        println!("Fetched Summary:");
        println!("  Antigravity: today={}, total={}", summary.antigravity.today_tokens, summary.antigravity.total_tokens);
        println!("  Codex:       today={}, total={}", summary.codex.today_tokens, summary.codex.total_tokens);
        println!("  OpenCode:    today={}, total={}", summary.opencode.today_tokens, summary.opencode.total_tokens);
        println!("  Total Today: {}", summary.total_today_tokens);
        println!("  Total Lifetime: {}", summary.total_lifetime_tokens);
        assert!(summary.total_lifetime_tokens > 0, "Lifetime tokens should be > 0");
    }
}
