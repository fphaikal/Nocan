use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
            mascot_enabled: true,
            minimal_notch: false,
            auto_minimize_on_maximize: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SectionSettings {
    pub show_music: bool,
    pub show_focus: bool,
    pub show_water: bool,
    pub show_shelf: bool,
    pub show_todo: bool,
    pub show_openmaus: bool,
    pub show_ai_usage: bool,
    pub show_saham: bool,
}

impl Default for SectionSettings {
    fn default() -> Self {
        Self {
            show_music: true,
            show_focus: true,
            show_water: true,
            show_shelf: true,
            show_todo: true,
            show_openmaus: true,
            show_ai_usage: true,
            show_saham: true,
        }
    }
}

impl SectionSettings {
    pub fn active_sections(&self) -> Vec<crate::notch::carousel::SectionId> {
        use crate::notch::carousel::SectionId;
        let mut list = Vec::new();
        if self.show_music { list.push(SectionId::Music); }
        if self.show_focus { list.push(SectionId::Focus); }
        if self.show_water { list.push(SectionId::Water); }
        if self.show_shelf { list.push(SectionId::Shelf); }
        if self.show_todo { list.push(SectionId::Todo); }
        if self.show_openmaus { list.push(SectionId::OpenMaus); }
        if self.show_ai_usage { list.push(SectionId::AiUsage); }
        if self.show_saham { list.push(SectionId::Saham); }
        if list.is_empty() {
            list.push(SectionId::Music);
        }
        list
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
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
#[serde(default)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    System,
}

impl ThemeMode {
    pub fn is_dark(self, system_dark: bool) -> bool {
        match self { Self::Light => false, Self::Dark => true, Self::System => system_dark }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    #[serde(default)]
    pub theme: ThemeMode,
    pub accent_mode: u32, // 0: Auto, 1: Gemini Blue, 2: Coral, 3: Emerald, 4: Cyan, 5: Violet
    pub acrylic_opacity: f32,
    #[serde(default = "default_font_family")]
    pub font_family: String,
}

fn default_font_family() -> String {
    "SF Pro Text".to_string()
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::default(),
            accent_mode: 0,
            acrylic_opacity: 0.90,
            font_family: default_font_family(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub general: GeneralSettings,
    #[serde(default)]
    pub sections: SectionSettings,
    pub ai: AiUsageSettings,
    pub music: MusicSettings,
    pub productivity: ProductivitySettings,
    pub appearance: AppearanceSettings,
    #[serde(default = "default_openmaus_notifications")]
    pub openmaus_notifications: bool,
    #[serde(default = "default_stock_symbols")]
    pub stock_symbols: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { general: GeneralSettings::default(), sections: SectionSettings::default(),
            ai: AiUsageSettings::default(), music: MusicSettings::default(),
            productivity: ProductivitySettings::default(), appearance: AppearanceSettings::default(),
            stock_symbols: default_stock_symbols(), openmaus_notifications:true }
    }
}

fn default_openmaus_notifications() -> bool { true }

#[cfg(test)]
mod openmaus_preferences_tests {
    use super::*;
    #[test]
    fn existing_preferences_keep_their_values_when_notifications_are_added() {
        let mut value=serde_json::to_value(AppSettings::default()).unwrap();
        value.as_object_mut().unwrap().remove("openmaus_notifications");
        value["general"]["autostart"]=false.into();
        let loaded:AppSettings=serde_json::from_value(value).unwrap();
        assert!(loaded.openmaus_notifications);assert!(!loaded.general.autostart);
    }
    #[test]
    fn preferences_from_older_versions_keep_known_fields() {
        let loaded:AppSettings=serde_json::from_str(r#"{"general":{"autostart":false,"mascot_enabled":false},"productivity":{"water_daily_target_glasses":9}}"#).unwrap();
        assert!(!loaded.general.autostart);assert!(!loaded.general.mascot_enabled);
        assert_eq!(loaded.productivity.water_daily_target_glasses,9);
        assert_eq!(loaded.productivity.focus_work_mins,25);assert!(loaded.openmaus_notifications);
    }
}

fn default_stock_symbols() -> Vec<String> {
    ["BBCA.JK", "BBRI.JK", "TLKM.JK", "ASII.JK"].iter().map(|s| s.to_string()).collect()
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        crate::storage::data_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        crate::storage::load_json(&path).ok().flatten().unwrap_or_default()
    }

    pub fn save(&self) -> bool {
        let path = Self::config_path();
        crate::storage::save_json(&path,self).is_ok()
    }
}

pub fn debug_log(msg: &str) {
    use std::io::Write;
    let log_dir = crate::storage::data_dir();
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
