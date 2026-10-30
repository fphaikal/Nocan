use crate::config::AppSettings;
use super::ui::{SettingsAction, FONT_FAMILIES};

pub fn apply(settings: &mut AppSettings, action: SettingsAction) {
    use SettingsAction::*;
    match action {
        SetTheme(mode) => settings.appearance.theme = mode,
        SelectTab(_) | Navigate(_) | CloseWindow | MinimizeWindow | FocusSearch | FocusStockInput | OpenSectionsTab | AddStock | RemoveStock(_) | SetStockMarket(_) | TestWaterReminder | ToggleDropdown(_) => {},
        ToggleAutostart => settings.general.autostart = !settings.general.autostart,
        ToggleMascot => settings.general.mascot_enabled = !settings.general.mascot_enabled,
        ToggleMinimalNotch => settings.general.minimal_notch = !settings.general.minimal_notch,
        SetMinimalNotch(value) => settings.general.minimal_notch = value,
        ToggleAutoMinimizeOnMaximize => settings.general.auto_minimize_on_maximize = !settings.general.auto_minimize_on_maximize,
        SetAutoMinimizeOnMaximize(value) => settings.general.auto_minimize_on_maximize = value,
        ToggleSectionMusic => settings.sections.show_music = !settings.sections.show_music,
        ToggleSectionFocus => settings.sections.show_focus = !settings.sections.show_focus,
        ToggleSectionWater => settings.sections.show_water = !settings.sections.show_water,
        ToggleSectionShelf => settings.sections.show_shelf = !settings.sections.show_shelf,
        ToggleSectionTodo => settings.sections.show_todo = !settings.sections.show_todo,
        ToggleSectionOpenMaus => settings.sections.show_openmaus = !settings.sections.show_openmaus,
        ToggleOpenMausNotifications => settings.openmaus_notifications = !settings.openmaus_notifications,
        ToggleSectionAiUsage => settings.sections.show_ai_usage = !settings.sections.show_ai_usage,
        ToggleSectionSaham => settings.sections.show_saham = !settings.sections.show_saham,
        ToggleAiAntigravity => settings.ai.show_antigravity = !settings.ai.show_antigravity,
        ToggleAiClaude => settings.ai.show_claude = !settings.ai.show_claude,
        ToggleAiCodex => settings.ai.show_codex = !settings.ai.show_codex,
        ToggleAiOpenCode => settings.ai.show_opencode = !settings.ai.show_opencode,
        SetAiDefaultProvider(value) => settings.ai.default_provider = value.min(3),
        SetAiPoll(value) => settings.ai.poll_interval_secs = value.clamp(1, 60),
        AdjustAiPoll(delta) => settings.ai.poll_interval_secs = (settings.ai.poll_interval_secs as i64 + delta as i64).clamp(1,60) as u64,
        ToggleAiProportionBar => settings.ai.show_proportional_bar = !settings.ai.show_proportional_bar,
        ToggleMusicGlow => settings.music.ambient_glow_enabled = !settings.music.ambient_glow_enabled,
        ToggleDynamicLyricsWidth => settings.music.dynamic_lyrics_width = !settings.music.dynamic_lyrics_width,
        ToggleDiscordRpc => settings.music.discord_rpc_enabled = !settings.music.discord_rpc_enabled,
        AdjustFocusWorkMins(delta) => settings.productivity.focus_work_mins = (settings.productivity.focus_work_mins as i32 + delta).clamp(5,90) as u32,
        AdjustFocusBreakMins(delta) => settings.productivity.focus_break_mins = (settings.productivity.focus_break_mins as i32 + delta).clamp(1,30) as u32,
        AdjustWaterIntervalMins(delta) => settings.productivity.water_interval_mins = (settings.productivity.water_interval_mins as i32 + delta).clamp(15,120) as u32,
        AdjustWaterTargetGlasses(delta) => settings.productivity.water_daily_target_glasses = (settings.productivity.water_daily_target_glasses as i32 + delta).clamp(1,24) as u32,
        SetFocusWorkMins(value) => settings.productivity.focus_work_mins = value.clamp(5, 90),
        SetFocusBreakMins(value) => settings.productivity.focus_break_mins = value.clamp(1, 30),
        SetWaterIntervalMins(value) => settings.productivity.water_interval_mins = value.clamp(15, 120),
        SetWaterTargetGlasses(value) => settings.productivity.water_daily_target_glasses = value.clamp(1, 24),
        SetAccentMode(value) => settings.appearance.accent_mode = value.min(5),
        AdjustAcrylicOpacity(delta) => settings.appearance.acrylic_opacity = ((settings.appearance.acrylic_opacity + delta).clamp(0.5,1.0) * 100.0).round() / 100.0,
        SetAcrylicOpacity(value) => settings.appearance.acrylic_opacity = (value.clamp(0.5, 1.0) * 100.0).round() / 100.0,
        SetFontFamily(index) => if let Some(name) = FONT_FAMILIES.get(index) {
            settings.appearance.font_family = (*name).to_string();
        },
        ResetToDefaults => *settings = AppSettings::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_toggle_and_clamp() {
        let mut settings = AppSettings::default();
        apply(&mut settings, SettingsAction::ToggleMascot);
        assert!(!settings.general.mascot_enabled);
        apply(&mut settings, SettingsAction::ToggleOpenMausNotifications);
        assert!(!settings.openmaus_notifications);
        apply(&mut settings, SettingsAction::SetAiDefaultProvider(2));
        assert_eq!(settings.ai.default_provider, 2);
        apply(&mut settings, SettingsAction::AdjustFocusWorkMins(-200));
        assert_eq!(settings.productivity.focus_work_mins, 5);
        apply(&mut settings, SettingsAction::SetFontFamily(5));
        assert_eq!(settings.appearance.font_family, "Segoe UI");
        apply(&mut settings, SettingsAction::ResetToDefaults);
        assert!(settings.general.mascot_enabled);
    }
}
