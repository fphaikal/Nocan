use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub autostart: bool,
    pub mascot_enabled: bool,
    pub minimal_notch: bool,
    pub auto_minimize_on_maximize: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            autostart: true,
            mascot_enabled: false,
            minimal_notch: false,
            auto_minimize_on_maximize: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiUsageSettings {
    pub show_antigravity: bool,
    pub show_claude: bool,
    pub show_codex: bool,
    pub show_opencode: bool,
    pub default_provider: u32, // 0: Antigravity, 1: Claude, 2: Codex, 3: OpenCode
    pub poll_interval_secs: u64,
    pub show_proportional_bar: bool,
}

impl Default for AiUsageSettings {
    fn default() -> Self {
        Self {
            show_antigravity: true,
            show_claude: true,
            show_codex: true,
            show_opencode: true,
            default_provider: 0,
            poll_interval_secs: 3,
            show_proportional_bar: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicSettings {
    pub ambient_glow_enabled: bool,
    pub ambient_opacity: f32,
    pub dynamic_lyrics_width: bool,
    pub lyrics_max_width: f32,
    pub discord_rpc_enabled: bool,
}

impl Default for MusicSettings {
    fn default() -> Self {
        Self {
            ambient_glow_enabled: true,
            ambient_opacity: 0.85,
            dynamic_lyrics_width: true,
            lyrics_max_width: 520.0,
            discord_rpc_enabled: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductivitySettings {
    pub focus_work_mins: u32,
    pub focus_break_mins: u32,
    pub water_interval_mins: u32,
    pub water_daily_target_glasses: u32,
}

impl Default for ProductivitySettings {
    fn default() -> Self {
        Self {
            focus_work_mins: 25,
            focus_break_mins: 5,
            water_interval_mins: 45,
            water_daily_target_glasses: 8,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub accent_mode: u32, // 0: Auto, 1: Gemini Blue, 2: Coral, 3: Emerald, 4: Cyan, 5: Violet
    pub acrylic_opacity: f32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            accent_mode: 0,
            acrylic_opacity: 0.90,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub ai: AiUsageSettings,
    pub music: MusicSettings,
    pub productivity: ProductivitySettings,
    pub appearance: AppearanceSettings,
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| "C:\\Users\\abufa\\AppData\\Local".to_string());
        let nocan_dir = PathBuf::from(&base).join("Nocan");
        let _ = std::fs::create_dir_all(&nocan_dir);
        nocan_dir.join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                return settings;
            }
        }
        let default_settings = Self::default();
        let _ = default_settings.save();
        default_settings
    }

    pub fn save(&self) -> bool {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            return std::fs::write(&path, json).is_ok();
        }
        false
    }
}

pub fn debug_log(msg: &str) {
    use std::io::Write;
    let base = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:\\Users\\abufa\\AppData\\Local".to_string());
    let log_dir = PathBuf::from(base).join("Nocan");
    let _ = std::fs::create_dir_all(&log_dir);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("nocan_debug.log"))
    {
        let _ = writeln!(
            f,
            "[{}] {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            msg
        );
    }
}
