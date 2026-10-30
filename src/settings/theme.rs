use windows::core::w;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use crate::config::AppSettings;

pub fn system_is_dark() -> bool {
    let mut light = 1u32;
    let mut size = std::mem::size_of_val(&light) as u32;
    let status = unsafe { RegGetValueW(HKEY_CURRENT_USER,
        w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
        w!("AppsUseLightTheme"), RRF_RT_REG_DWORD, None,
        Some(&mut light as *mut _ as *mut std::ffi::c_void), Some(&mut size)) };
    status == ERROR_SUCCESS && light == 0
}

pub fn is_dark(settings: &AppSettings) -> bool {
    settings.appearance.theme.is_dark(system_is_dark())
}

pub fn background(dark: bool) -> D2D1_COLOR_F {
    let value = if dark { 0.12 } else { 0.995 };
    D2D1_COLOR_F { r: value, g: value, b: value, a: 1.0 }
}

#[cfg(test)]
mod tests {
    use crate::config::{AppearanceSettings, ThemeMode};

    #[test]
    fn system_tracks_windows_while_explicit_choices_override_it() {
        for os_dark in [false, true] {
            assert!(!ThemeMode::Light.is_dark(os_dark));
            assert!(ThemeMode::Dark.is_dark(os_dark));
            assert_eq!(ThemeMode::System.is_dark(os_dark), os_dark);
        }
    }

    #[test]
    fn theme_persists_and_older_settings_keep_their_light_appearance() {
        let old: AppearanceSettings = serde_json::from_str(r#"{"accent_mode":0,"acrylic_opacity":0.9}"#).unwrap();
        assert_eq!(old.theme, ThemeMode::Light);
        for theme in [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System] {
            let mut settings = AppearanceSettings::default();
            settings.theme = theme;
            let saved = serde_json::to_string(&settings).unwrap();
            assert_eq!(serde_json::from_str::<AppearanceSettings>(&saved).unwrap().theme, theme);
        }
    }
}
