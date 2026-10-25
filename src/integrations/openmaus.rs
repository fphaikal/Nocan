use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use serde::Deserialize;
use windows::Win32::System::SystemInformation::GetLocalTime;

#[derive(Clone, Debug, Default)]
pub struct UsageItem {
    pub name: String,
    pub subtext: String,
    pub turns: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub prompt_bytes: u64,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Default)]
pub struct UsageReport {
    pub total_turns: u64,
    pub total_tokens: u64,
    pub total_input: u64,
    pub total_output: u64,
    pub total_cost_usd: f64,
    pub today_turns: u64,
    pub today_tokens: u64,
    pub bots: Vec<UsageItem>,
    pub models: Vec<UsageItem>,
    pub providers: Vec<UsageItem>,
    pub last_updated: String,
    pub is_active: bool,
}

#[derive(Deserialize, Debug)]
struct RawUsageRow {
    #[serde(rename = "botId")]
    bot_id: Option<String>,
    #[serde(rename = "botName")]
    bot_name: Option<String>,
    model: Option<String>,
    #[serde(rename = "instanceId")]
    instance_id: Option<String>,
    #[serde(rename = "driverKind")]
    driver_kind: Option<String>,
    input: Option<u64>,
    output: Option<u64>,
    #[serde(rename = "costUsd")]
    cost_usd: Option<f64>,
    #[serde(rename = "promptBytes")]
    prompt_bytes: Option<RawPromptBytes>,
    at: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RawPromptBytes {
    stable: Option<u64>,
    volatile: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct RawConfig {
    instances: Option<HashMap<String, RawInstance>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RawInstance {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    driver: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RawBot {
    id: Option<String>,
    name: Option<String>,
    title: Option<String>,
}

pub fn get_openmaus_data_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\abufa".to_string());
    let mut p = PathBuf::from(home);
    p.push(".openmausbot");
    p
}

fn resolve_provider_name(
    instance_id: Option<&str>,
    driver_kind: Option<&str>,
    instances_cfg: &HashMap<String, RawInstance>,
) -> String {
    if let Some(inst) = instance_id {
        if let Some(cfg) = instances_cfg.get(inst) {
            if let Some(dn) = &cfg.display_name {
                return dn.clone();
            }
        }
        if inst == "opencodeGo" {
            return "OpenCode Go".to_string();
        }
        if inst == "openaiCompat" {
            return "DeepSeek".to_string();
        }
    }

    if let Some(dk) = driver_kind {
        if dk == "antigravityAgent" {
            return "Antigravity (Gemini)".to_string();
        }
        if dk == "openai-compat" {
            return "OpenAI Compatible".to_string();
        }
        return dk.to_string();
    }

    instance_id.unwrap_or("Unknown Provider").to_string()
}

pub fn load_usage_report() -> UsageReport {
    let data_dir = get_openmaus_data_dir();
    let usage_dir = data_dir.join("usage");

    // Load configs
    let mut instances_cfg: HashMap<String, RawInstance> = HashMap::new();
    let config_path = data_dir.join("config.json");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(parsed) = serde_json::from_str::<RawConfig>(&content) {
            if let Some(insts) = parsed.instances {
                instances_cfg = insts;
            }
        }
    }

    // Load bots
    let mut bot_titles: HashMap<String, String> = HashMap::new();
    let mut bot_names: HashMap<String, String> = HashMap::new();
    let bots_path = data_dir.join("bots.json");
    if let Ok(content) = std::fs::read_to_string(&bots_path) {
        if let Ok(parsed) = serde_json::from_str::<HashMap<String, RawBot>>(&content) {
            for (_, b) in parsed {
                if let Some(id) = b.id {
                    if let Some(n) = b.name {
                        bot_names.insert(id.clone(), n);
                    }
                    if let Some(t) = b.title {
                        bot_titles.insert(id, t);
                    }
                }
            }
        }
    }

    let mut report = UsageReport::default();
    report.last_updated = get_now_time_string();
    report.is_active = data_dir.exists();

    if !usage_dir.exists() {
        return report;
    }

    let today_prefix = get_today_date_prefix();

    let mut bot_map: HashMap<String, UsageItem> = HashMap::new();
    let mut model_map: HashMap<String, UsageItem> = HashMap::new();
    let mut prov_map: HashMap<String, UsageItem> = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(usage_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            if let Ok(file) = File::open(&path) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Ok(row) = serde_json::from_str::<RawUsageRow>(trimmed) {
                        let inp = row.input.unwrap_or(0);
                        let outp = row.output.unwrap_or(0);
                        let cost = row.cost_usd.unwrap_or(0.0);
                        let tot = inp + outp;

                        let pb = if let Some(bytes) = row.prompt_bytes {
                            bytes.stable.unwrap_or(0) + bytes.volatile.unwrap_or(0)
                        } else {
                            0
                        };

                        report.total_turns += 1;
                        report.total_input += inp;
                        report.total_output += outp;
                        report.total_tokens += tot;
                        report.total_cost_usd += cost;

                        let is_today = if let Some(ref at_str) = row.at {
                            at_str.starts_with(&today_prefix)
                        } else {
                            false
                        };

                        if is_today {
                            report.today_turns += 1;
                            report.today_tokens += tot;
                        }

                        // Aggregate by Bot
                        let b_name = row.bot_name.clone().unwrap_or_else(|| {
                            if let Some(ref id) = row.bot_id {
                                bot_names.get(id).cloned().unwrap_or_else(|| id.clone())
                            } else {
                                "Unknown Bot".to_string()
                            }
                        });
                        let b_title = row
                            .bot_id
                            .as_ref()
                            .and_then(|id| bot_titles.get(id))
                            .cloned()
                            .unwrap_or_default();

                        let b_item = bot_map.entry(b_name.clone()).or_insert_with(|| UsageItem {
                            name: b_name,
                            subtext: b_title,
                            ..Default::default()
                        });
                        b_item.turns += 1;
                        b_item.input_tokens += inp;
                        b_item.output_tokens += outp;
                        b_item.total_tokens += tot;
                        b_item.prompt_bytes += pb;
                        b_item.cost_usd += cost;

                        // Aggregate by Model
                        let m_name = row.model.clone().unwrap_or_else(|| "Unknown Model".to_string());
                        let m_item = model_map.entry(m_name.clone()).or_insert_with(|| UsageItem {
                            name: m_name,
                            ..Default::default()
                        });
                        m_item.turns += 1;
                        m_item.input_tokens += inp;
                        m_item.output_tokens += outp;
                        m_item.total_tokens += tot;
                        m_item.prompt_bytes += pb;
                        m_item.cost_usd += cost;

                        // Aggregate by Provider
                        let p_name = resolve_provider_name(
                            row.instance_id.as_deref(),
                            row.driver_kind.as_deref(),
                            &instances_cfg,
                        );
                        let p_item = prov_map.entry(p_name.clone()).or_insert_with(|| UsageItem {
                            name: p_name,
                            ..Default::default()
                        });
                        p_item.turns += 1;
                        p_item.input_tokens += inp;
                        p_item.output_tokens += outp;
                        p_item.total_tokens += tot;
                        p_item.prompt_bytes += pb;
                        p_item.cost_usd += cost;
                    }
                }
            }
        }
    }

    let mut bots: Vec<UsageItem> = bot_map.into_values().collect();
    bots.sort_by(|a, b| b.turns.cmp(&a.turns).then_with(|| b.total_tokens.cmp(&a.total_tokens)));

    let mut models: Vec<UsageItem> = model_map.into_values().collect();
    models.sort_by(|a, b| b.turns.cmp(&a.turns).then_with(|| b.total_tokens.cmp(&a.total_tokens)));

    let mut providers: Vec<UsageItem> = prov_map.into_values().collect();
    providers.sort_by(|a, b| b.turns.cmp(&a.turns).then_with(|| b.total_tokens.cmp(&a.total_tokens)));

    report.bots = bots;
    report.models = models;
    report.providers = providers;

    report
}

fn get_today_date_prefix() -> String {
    unsafe {
        let st = GetLocalTime();
        format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay)
    }
}

fn get_now_time_string() -> String {
    unsafe {
        let st = GetLocalTime();
        format!("{:02}:{:02}:{:02} WIB", st.wHour, st.wMinute, st.wSecond)
    }
}
