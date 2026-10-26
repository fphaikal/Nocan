use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
use serde::Deserialize;
use windows::Win32::System::SystemInformation::GetLocalTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiTab {
    Antigravity = 0,
    Claude = 1,
    Codex = 2,
}

impl AiTab {
    pub fn next(&self) -> Self {
        match self {
            AiTab::Antigravity => AiTab::Claude,
            AiTab::Claude => AiTab::Codex,
            AiTab::Codex => AiTab::Antigravity,
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
    pub last_updated: String,
}

pub struct AiUsageService {
    data: Arc<RwLock<AiUsageSummary>>,
    _worker: Option<thread::JoinHandle<()>>,
}

impl AiUsageService {
    pub fn new() -> Self {
        let initial = Arc::new(RwLock::new(Self::fetch_all()));
        let data_clone = Arc::clone(&initial);

        // Spawn background polling thread to avoid any UI frame drops
        let handle = thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(3));
            let fresh = Self::fetch_all();
            if let Ok(mut lock) = data_clone.write() {
                *lock = fresh;
            }
        });

        Self {
            data: initial,
            _worker: Some(handle),
        }
    }

    pub fn get_summary(&self) -> AiUsageSummary {
        self.data.read().map(|d| d.clone()).unwrap_or_default()
    }

    pub fn fetch_all() -> AiUsageSummary {
        let today_prefix = get_today_date_prefix();
        let now_time = get_now_time_string();

        let antigravity = scan_antigravity(&today_prefix);
        let codex = scan_codex(&today_prefix);
        let claude = scan_claude(&today_prefix);

        let total_today_tokens = antigravity.today_tokens + codex.today_tokens + claude.today_tokens;
        let total_today_turns = antigravity.today_turns + codex.today_turns + claude.today_turns;
        let total_lifetime_tokens = antigravity.total_tokens + codex.total_tokens + claude.total_tokens;

        AiUsageSummary {
            total_today_tokens,
            total_today_turns,
            total_lifetime_tokens,
            antigravity,
            claude,
            codex,
            last_updated: now_time,
        }
    }
}

// ---------------------------------------------------------------------------
// Antigravity Scanner
// ---------------------------------------------------------------------------
fn scan_antigravity(today_prefix: &str) -> ProviderUsage {
    let mut usage = ProviderUsage {
        name: "Antigravity".to_string(),
        title: "Google Gemini".to_string(),
        active_model: "Gemini 3.8 Flash".to_string(),
        status_text: "Standby".to_string(),
        ..Default::default()
    };

    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let ag_dir = PathBuf::from(&user_profile).join(".gemini").join("antigravity-cli");

    // 1. Check active presence
    let presence_dir = ag_dir.join("presence");
    if let Ok(entries) = std::fs::read_dir(&presence_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("lock") {
                usage.is_active = true;
                usage.status_text = "Active Agent".to_string();
                break;
            }
        }
    }

    // Also check recent modification of history.jsonl
    let hist_path = ag_dir.join("history.jsonl");
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

    // 2. Scan brain transcripts
    let brain_dir = ag_dir.join("brain");
    if let Ok(dirs) = std::fs::read_dir(&brain_dir) {
        for d in dirs.flatten() {
            let t_path = d.path().join(".system_generated").join("logs").join("transcript.jsonl");
            if !t_path.exists() {
                continue;
            }

            // Optimization: check file modification time
            let is_modified_recently = if let Ok(meta) = std::fs::metadata(&t_path) {
                if let Ok(mtime) = meta.modified() {
                    SystemTime::now().duration_since(mtime).map(|d| d.as_secs() < 86400 * 2).unwrap_or(true)
                } else {
                    true
                }
            } else {
                true
            };

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

                    let is_today = if is_modified_recently {
                        trimmed.contains(&format!("\"created_at\":\"{}", today_prefix))
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

    // 3. Scan OpenMausbot usage for any antigravityAgent runs
    let om_dir = PathBuf::from(&user_profile).join(".openmausbot").join("usage");
    if let Ok(entries) = std::fs::read_dir(om_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Ok(file) = File::open(p) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.contains("\"driverKind\":\"antigravityAgent\"") {
                        let inp = extract_json_int(trimmed, "\"input\":");
                        let outp = extract_json_int(trimmed, "\"output\":");
                        let tot = inp + outp;
                        if trimmed.contains(&format!("\"at\":\"{}", today_prefix)) {
                            usage.today_turns += 1;
                            usage.today_tokens += tot;
                            usage.input_tokens_today += inp;
                            usage.output_tokens_today += outp;
                        }
                        usage.total_turns += 1;
                        usage.total_tokens += tot;
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
// Codex Scanner
// ---------------------------------------------------------------------------
fn scan_codex(today_prefix: &str) -> ProviderUsage {
    let mut usage = ProviderUsage {
        name: "Codex".to_string(),
        title: "OpenAI Codex".to_string(),
        active_model: "z-ai/glm-5.3-free".to_string(),
        status_text: "Offline".to_string(),
        ..Default::default()
    };

    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let codex_dir = PathBuf::from(&user_profile).join(".codex");
    let opencodex_dir = PathBuf::from(&user_profile).join(".opencodex");
    let orca_dir = PathBuf::from(&user_profile).join("AppData").join("Roaming").join("orca");

    // 1. Read active model from .codex/config.toml
    let config_toml = codex_dir.join("config.toml");
    if let Ok(content) = std::fs::read_to_string(config_toml) {
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
    if let Ok(content) = std::fs::read_to_string(quota_json) {
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

    // 4. Read Orca codex usage if available
    let orca_codex = orca_dir.join("orca-codex-usage.json");
    if let Ok(content) = std::fs::read_to_string(orca_codex) {
        #[derive(Deserialize)]
        struct OrcaUsage {
            #[serde(rename = "dailyAggregates")]
            daily_aggregates: Option<Vec<DailyAgg>>,
            sessions: Option<Vec<SessionAgg>>,
        }
        #[derive(Deserialize)]
        struct DailyAgg {
            day: Option<String>,
            #[serde(rename = "totalTokens")]
            total_tokens: Option<u64>,
            #[serde(rename = "inputTokens")]
            input_tokens: Option<u64>,
            #[serde(rename = "outputTokens")]
            output_tokens: Option<u64>,
            #[serde(rename = "cachedInputTokens")]
            cached_input_tokens: Option<u64>,
            #[serde(rename = "eventCount")]
            event_count: Option<u64>,
        }
        #[derive(Deserialize)]
        struct SessionAgg {
            #[serde(rename = "totalTokens")]
            total_tokens: Option<u64>,
            #[serde(rename = "eventCount")]
            event_count: Option<u64>,
        }

        if let Ok(parsed) = serde_json::from_str::<OrcaUsage>(&content) {
            if let Some(dailies) = parsed.daily_aggregates {
                for d in dailies {
                    let tot = d.total_tokens.unwrap_or(0);
                    let inp = d.input_tokens.unwrap_or(0);
                    let outp = d.output_tokens.unwrap_or(0);
                    let cache = d.cached_input_tokens.unwrap_or(0);
                    let count = d.event_count.unwrap_or(0);

                    if let Some(day_str) = d.day {
                        if day_str == today_prefix {
                            usage.today_tokens += tot;
                            usage.today_turns += count;
                            usage.input_tokens_today += inp;
                            usage.output_tokens_today += outp;
                            usage.cached_tokens_today += cache;
                        }
                    }
                }
            }
            if let Some(sessions) = parsed.sessions {
                for s in sessions {
                    usage.total_tokens += s.total_tokens.unwrap_or(0);
                    usage.total_turns += s.event_count.unwrap_or(0);
                }
            }
        }
    }

    // 5. Also read .opencodex/usage.jsonl for all-time / loopback usage
    let ocx_usage = opencodex_dir.join("usage.jsonl");
    if let Ok(file) = File::open(ocx_usage) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let tot = extract_json_int(trimmed, "\"totalTokens\":");
            usage.total_tokens += tot;
            usage.total_turns += 1;
        }
    }

    // Fallback model formatting
    if usage.active_model.starts_with("tokenrouter/") {
        usage.active_model = usage.active_model.replace("tokenrouter/", "");
    }

    usage
}

// ---------------------------------------------------------------------------
// Claude Scanner
// ---------------------------------------------------------------------------
fn scan_claude(today_prefix: &str) -> ProviderUsage {
    let mut usage = ProviderUsage {
        name: "Claude".to_string(),
        title: "Anthropic Claude".to_string(),
        active_model: "Claude 3.7 Sonnet".to_string(),
        status_text: "Standby".to_string(),
        ..Default::default()
    };

    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let claude_dir = PathBuf::from(&user_profile).join(".claude");
    let orca_dir = PathBuf::from(&user_profile).join("AppData").join("Roaming").join("orca");
    let om_dir = PathBuf::from(&user_profile).join(".openmausbot").join("usage");

    // 1. Inspect gateway-models.json
    let gw_models = claude_dir.join("cache").join("gateway-models.json");
    if let Ok(content) = std::fs::read_to_string(gw_models) {
        #[derive(Deserialize)]
        struct GwFile {
            models: Option<Vec<GwModel>>,
        }
        #[derive(Deserialize)]
        struct GwModel {
            display_name: Option<String>,
        }
        if let Ok(parsed) = serde_json::from_str::<GwFile>(&content) {
            if let Some(list) = parsed.models {
                for m in list {
                    if let Some(dn) = m.display_name {
                        if dn.contains("claude") {
                            usage.active_model = dn;
                            usage.is_active = true;
                            usage.status_text = "Gateway Ready".to_string();
                            break;
                        }
                    }
                }
            }
        }
    }

    // 2. Read Orca claude usage
    let orca_claude = orca_dir.join("orca-claude-usage.json");
    if let Ok(content) = std::fs::read_to_string(orca_claude) {
        #[derive(Deserialize)]
        struct OrcaClaude {
            #[serde(rename = "dailyAggregates")]
            daily_aggregates: Option<Vec<DailyAgg>>,
            sessions: Option<Vec<SessionAgg>>,
        }
        #[derive(Deserialize)]
        struct DailyAgg {
            day: Option<String>,
            #[serde(rename = "totalTokens")]
            total_tokens: Option<u64>,
            #[serde(rename = "inputTokens")]
            input_tokens: Option<u64>,
            #[serde(rename = "outputTokens")]
            output_tokens: Option<u64>,
            #[serde(rename = "eventCount")]
            event_count: Option<u64>,
        }
        #[derive(Deserialize)]
        struct SessionAgg {
            #[serde(rename = "totalTokens")]
            total_tokens: Option<u64>,
            #[serde(rename = "eventCount")]
            event_count: Option<u64>,
        }

        if let Ok(parsed) = serde_json::from_str::<OrcaClaude>(&content) {
            if let Some(dailies) = parsed.daily_aggregates {
                for d in dailies {
                    let tot = d.total_tokens.unwrap_or(0);
                    let count = d.event_count.unwrap_or(0);
                    if let Some(day_str) = d.day {
                        if day_str == today_prefix {
                            usage.today_tokens += tot;
                            usage.today_turns += count;
                            usage.input_tokens_today += d.input_tokens.unwrap_or(0);
                            usage.output_tokens_today += d.output_tokens.unwrap_or(0);
                        }
                    }
                }
            }
            if let Some(sessions) = parsed.sessions {
                for s in sessions {
                    usage.total_tokens += s.total_tokens.unwrap_or(0);
                    usage.total_turns += s.event_count.unwrap_or(0);
                }
            }
        }
    }

    // 3. Scan OpenMausbot for any Claude models
    if let Ok(entries) = std::fs::read_dir(om_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Ok(file) = File::open(p) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    let lower = trimmed.to_lowercase();
                    if lower.contains("claude") || lower.contains("anthropic") {
                        let inp = extract_json_int(trimmed, "\"input\":");
                        let outp = extract_json_int(trimmed, "\"output\":");
                        let tot = inp + outp;
                        if trimmed.contains(&format!("\"at\":\"{}", today_prefix)) {
                            usage.today_turns += 1;
                            usage.today_tokens += tot;
                            usage.input_tokens_today += inp;
                            usage.output_tokens_today += outp;
                        }
                        usage.total_turns += 1;
                        usage.total_tokens += tot;
                    }
                }
            }
        }
    }

    if usage.total_tokens == 0 && usage.today_tokens == 0 {
        // If no standalone claude tokens yet, display ready status
        if usage.status_text == "Standby" {
            usage.status_text = "Ready (Standby)".to_string();
        }
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
        format!("{:02}:{:02}:{:02} WIB", st.wHour, st.wMinute, st.wSecond)
    }
}
