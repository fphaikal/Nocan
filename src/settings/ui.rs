use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_POINT_2F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    ID2D1RenderTarget, ID2D1SolidColorBrush, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{IDWriteTextFormat, DWRITE_MEASURING_MODE_NATURAL};
use crate::{config::{AppSettings, ThemeMode}, graphics::dwrite::DWriteContext};

#[derive(Clone, Copy, PartialEq)]
enum Paint { Background, Sidebar, Edge, Surface, Divider, Text, Muted, Control, ControlBorder,
    Hover, Menu, Track, Segment, Link, Success, Danger, Literal(D2D1_COLOR_F) }

impl Paint {
    fn resolve(self, dark: bool) -> D2D1_COLOR_F {
        let rgb = match self {
            Self::Literal(c) => return c,
            Self::Background => return super::theme::background(dark),
            Self::Sidebar => if dark { (0.16,0.16,0.17) } else { (0.955,0.955,0.963) },
            Self::Edge => if dark { (0.26,0.26,0.28) } else { (0.88,0.88,0.90) },
            Self::Surface => if dark { (0.17,0.17,0.18) } else { (0.975,0.975,0.980) },
            Self::Divider => if dark { (0.26,0.26,0.28) } else { (0.91,0.91,0.925) },
            Self::Text => if dark { (0.94,0.94,0.95) } else { (0.17,0.17,0.19) },
            Self::Muted => if dark { (0.68,0.68,0.71) } else { (0.40,0.40,0.42) },
            Self::Control => if dark { (0.25,0.25,0.27) } else { (1.0,1.0,1.0) },
            Self::ControlBorder => if dark { (0.46,0.46,0.49) } else { (0.68,0.68,0.70) },
            Self::Hover => if dark { (0.28,0.30,0.34) } else { (0.93,0.95,0.99) },
            Self::Menu => if dark { (0.21,0.21,0.23) } else { (0.985,0.985,0.99) },
            Self::Track => if dark { (0.22,0.22,0.24) } else { (0.90,0.90,0.92) },
            Self::Segment => if dark { (0.38,0.38,0.41) } else { (1.0,1.0,1.0) },
            Self::Link => if dark { (0.40,0.69,1.0) } else { (0.0,0.35,0.80) },
            Self::Success => if dark { (0.38,0.82,0.55) } else { (0.12,0.46,0.28) },
            Self::Danger => if dark { (0.40,0.16,0.18) } else { (1.0,0.88,0.88) },
        };
        D2D1_COLOR_F { r: rgb.0, g: rgb.1, b: rgb.2, a: 1.0 }
    }
}
impl From<D2D1_COLOR_F> for Paint { fn from(value: D2D1_COLOR_F) -> Self { Self::Literal(value) } }
const COLOR_BG: Paint = Paint::Background;
const SIDEBAR_BG: Paint = Paint::Sidebar;
const SIDEBAR_BORDER: Paint = Paint::Edge;
const SIDEBAR_ACTIVE: Paint = color(0.0,0.37,0.88);
const SIDEBAR_HOVER: Paint = Paint::Hover;
const SIDEBAR_MUTED: Paint = Paint::Muted;
const SIDEBAR_TEXT: Paint = Paint::Text;
const SIDEBAR_TEXT_ACTIVE: Paint = color(1.0,1.0,1.0);
const CARD_BG: Paint = Paint::Surface;
const CARD_BORDER: Paint = Paint::Edge;
const CARD_DIVIDER: Paint = Paint::Divider;
const TEXT_TITLE: Paint = Paint::Text;
const TEXT_MUTED: Paint = Paint::Muted;
const TEXT_ITEM: Paint = Paint::Text;
const TEXT_DESC: Paint = Paint::Muted;
const LINK_BLUE: Paint = Paint::Link;
const CTRL_BG: Paint = Paint::Control;
const CTRL_BORDER: Paint = Paint::ControlBorder;
const CTRL_HOVER: Paint = Paint::Hover;
const CTRL_TEXT: Paint = Paint::Text;
const CTRL_CARET: Paint = Paint::Muted;
const MENU_BG: Paint = Paint::Menu;
const MENU_BORDER: Paint = Paint::ControlBorder;
const BADGE_BG: Paint = Paint::Track;
const BADGE_TEXT: Paint = Paint::Muted;
const CODE_BG: Paint = Paint::Control;
const CODE_BORDER: Paint = Paint::ControlBorder;
const CODE_TEXT: Paint = Paint::Text;
const SEG_TRACK: Paint = Paint::Track;
const SEG_BORDER: Paint = Paint::Edge;
const SEG_ACTIVE: Paint = Paint::Segment;
const SEG_ACTIVE_TEXT: Paint = Paint::Text;
const SEG_INACTIVE_TEXT: Paint = Paint::Muted;
const SUCCESS: Paint = Paint::Success;
const CLOSE_HOVER: Paint = Paint::Danger;
const CLOSE_X: Paint = Paint::Muted;

const fn color(r: f32, g: f32, b: f32) -> Paint { Paint::Literal(D2D1_COLOR_F { r, g, b, a: 1.0 }) }
fn rect(x: f32, y: f32, w: f32, h: f32) -> D2D_RECT_F { D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h } }

pub const OPACITY_SLIDER: D2D_RECT_F = D2D_RECT_F { left: 686.0, top: 570.0, right: 866.0, bottom: 608.0 };
pub fn opacity_at(x: f32) -> f32 {
    (0.5 + (x - OPACITY_SLIDER.left) / (OPACITY_SLIDER.right - OPACITY_SLIDER.left) * 0.5).clamp(0.5, 1.0)
}

const BADGE_PALETTE: [Paint; 6] = [
    color(0.176, 0.322, 0.545),
    color(0.373, 0.235, 0.514),
    color(0.153, 0.431, 0.361),
    color(0.478, 0.310, 0.161),
    color(0.435, 0.220, 0.259),
    color(0.208, 0.302, 0.412),
];

fn stock_badge_color(symbol: &str) -> Paint {
    let hash = symbol.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    BADGE_PALETTE[(hash % BADGE_PALETTE.len() as u32) as usize]
}

fn stock_badge_label(symbol: &str) -> &str {
    let base = symbol.split('.').next().unwrap_or(symbol);
    base.char_indices().nth(2).map(|(idx, _)| &base[..idx]).unwrap_or(base)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropdownId {
    NotchPreset,
    AutoMinimize,
    FontFamily,
    AccentMode,
    WaterInterval,
    WaterGlasses,
    FocusWork,
    FocusBreak,
    AiProvider,
    AiPoll,
}

pub const FONT_FAMILIES: [&str; 6] = [
    "Inter",
    "SF Pro Text",
    "Plus Jakarta Sans",
    "Poppins",
    "Segoe UI Variable Text",
    "Segoe UI",
];

pub const ACCENT_MODES: [&str; 6] = [
    "Auto (Windows)",
    "Gemini Blue",
    "Coral Red",
    "Emerald",
    "Cyan",
    "Violet",
];

pub const AI_PROVIDERS: [&str; 4] = ["Gemini", "Claude", "Codex", "OpenCode"];

const WATER_INTERVALS: [u32; 6] = [15, 30, 45, 60, 90, 120];
const WATER_GLASSES: [u32; 8] = [2, 4, 6, 8, 10, 12, 16, 20];
const FOCUS_WORK: [u32; 11] = [15, 20, 25, 30, 35, 40, 45, 50, 60, 75, 90];
const FOCUS_BREAK: [u32; 8] = [3, 4, 5, 6, 8, 10, 15, 20];
const AI_POLL: [u64; 7] = [1, 2, 3, 5, 10, 30, 60];

impl DropdownId {
    pub fn anchor(self) -> D2D_RECT_F {
        match self {
            Self::NotchPreset => rect(774.0, 222.0, 140.0, 32.0),
            Self::AutoMinimize => rect(744.0, 498.0, 170.0, 32.0),
            Self::FontFamily => rect(704.0, 439.0, 210.0, 32.0),
            Self::AccentMode => rect(774.0, 324.0, 140.0, 32.0),
            Self::WaterInterval => rect(774.0, 194.0, 140.0, 32.0),
            Self::WaterGlasses => rect(774.0, 260.0, 140.0, 32.0),
            Self::FocusWork => rect(774.0, 460.0, 140.0, 32.0),
            Self::FocusBreak => rect(774.0, 530.0, 140.0, 32.0),
            Self::AiProvider => rect(774.0, 194.0, 140.0, 32.0),
            Self::AiPoll => rect(774.0, 330.0, 140.0, 32.0),
        }
    }

    pub fn tab(self) -> usize {
        match self {
            Self::NotchPreset | Self::AutoMinimize => 0,
            Self::FontFamily | Self::AccentMode => 2,
            Self::WaterInterval | Self::WaterGlasses | Self::FocusWork | Self::FocusBreak => 3,
            Self::AiProvider | Self::AiPoll => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DropdownOption {
    pub label: String,
    pub action: SettingsAction,
}

#[derive(Clone, Debug)]
pub struct DropdownMenu {
    pub options: Vec<DropdownOption>,
    pub selected: usize,
    pub rect: D2D_RECT_F,
    pub item_h: f32,
}

impl DropdownMenu {
    pub fn option_at(&self, x: f32, y: f32) -> Option<usize> {
        (0..self.options.len()).find(|&index| {
            let item = self.item_rect(index);
            x >= item.left && x <= item.right && y >= item.top && y <= item.bottom
        })
    }

    pub fn item_rect(&self, index: usize) -> D2D_RECT_F {
        let width = self.rect.right - self.rect.left;
        rect(
            self.rect.left + 5.0,
            self.rect.top + 5.0 + index as f32 * self.item_h,
            width - 10.0,
            self.item_h - 2.0,
        )
    }
}

#[cfg(test)]
mod dropdown_tests {
    use super::*;

    fn menu() -> DropdownMenu {
        dropdown_menu(DropdownId::NotchPreset, &AppSettings::default(), "Segoe UI")
    }

    #[test]
    fn every_menu_row_is_selectable_where_it_is_drawn() {
        let menu = menu();
        for index in 0..menu.options.len() {
            let item = menu.item_rect(index);
            let x = (item.left + item.right) * 0.5;
            let y = (item.top + item.bottom) * 0.5;
            assert_eq!(menu.option_at(x, y), Some(index));
        }
        assert_eq!(menu.option_at(menu.rect.left + 1.0, menu.rect.top + 1.0), None);
        assert_eq!(menu.option_at(menu.rect.right + 1.0, menu.rect.top + 1.0), None);
    }

    #[test]
    fn anchors_match_the_controls_drawn_on_their_tab() {
        for id in [DropdownId::NotchPreset, DropdownId::AutoMinimize, DropdownId::FontFamily,
            DropdownId::AccentMode, DropdownId::WaterInterval, DropdownId::WaterGlasses,
            DropdownId::FocusWork, DropdownId::FocusBreak, DropdownId::AiProvider, DropdownId::AiPoll] {
            let a = id.anchor();
            assert_eq!(a.right - a.left, match id { DropdownId::FontFamily => 210.0, DropdownId::AutoMinimize => 170.0, _ => 140.0 });
            assert_eq!(a.bottom - a.top, 32.0);
            assert!(a.top >= 0.0 && a.bottom <= 735.0);
        }
    }
}

fn nearest_index<T: Copy + std::fmt::Display>(values: &[T], current: T) -> usize {
    let current_num = current.to_string().parse::<f64>().unwrap_or(0.0);
    let mut best = 0;
    let mut best_diff = f64::MAX;
    for (idx, value) in values.iter().enumerate() {
        let value_num = value.to_string().parse::<f64>().unwrap_or(0.0);
        let diff = (value_num - current_num).abs();
        if diff < best_diff {
            best_diff = diff;
            best = idx;
        }
    }
    best
}

pub fn dropdown_menu(id: DropdownId, settings: &AppSettings, font_name: &str) -> DropdownMenu {
    use SettingsAction::*;

    let (options, selected): (Vec<DropdownOption>, usize) = match id {
        DropdownId::NotchPreset => (
            vec![
                DropdownOption { label: "Standard".into(), action: SetMinimalNotch(false) },
                DropdownOption { label: "Minimal".into(), action: SetMinimalNotch(true) },
            ],
            if settings.general.minimal_notch { 1 } else { 0 },
        ),
        DropdownId::AutoMinimize => (
            vec![
                DropdownOption { label: "Keep Visible".into(), action: SetAutoMinimizeOnMaximize(false) },
                DropdownOption { label: "Auto Minimize".into(), action: SetAutoMinimizeOnMaximize(true) },
            ],
            if settings.general.auto_minimize_on_maximize { 1 } else { 0 },
        ),
        DropdownId::FontFamily => {
            let selected = FONT_FAMILIES.iter().position(|f| *f == font_name).unwrap_or(0);
            (
                FONT_FAMILIES
                    .iter()
                    .enumerate()
                    .map(|(i, f)| DropdownOption { label: (*f).into(), action: SetFontFamily(i) })
                    .collect(),
                selected,
            )
        }
        DropdownId::AccentMode => (
            ACCENT_MODES
                .iter()
                .enumerate()
                .map(|(i, a)| DropdownOption { label: (*a).into(), action: SetAccentMode(i as u32) })
                .collect(),
            (settings.appearance.accent_mode as usize).min(ACCENT_MODES.len() - 1),
        ),
        DropdownId::WaterInterval => (
            WATER_INTERVALS
                .iter()
                .map(|v| DropdownOption { label: format!("{} min", v), action: SetWaterIntervalMins(*v) })
                .collect(),
            nearest_index(&WATER_INTERVALS, settings.productivity.water_interval_mins),
        ),
        DropdownId::WaterGlasses => (
            WATER_GLASSES
                .iter()
                .map(|v| DropdownOption { label: format!("{} glasses", v), action: SetWaterTargetGlasses(*v) })
                .collect(),
            nearest_index(&WATER_GLASSES, settings.productivity.water_daily_target_glasses),
        ),
        DropdownId::FocusWork => (
            FOCUS_WORK
                .iter()
                .map(|v| DropdownOption { label: format!("{} min", v), action: SetFocusWorkMins(*v) })
                .collect(),
            nearest_index(&FOCUS_WORK, settings.productivity.focus_work_mins),
        ),
        DropdownId::FocusBreak => (
            FOCUS_BREAK
                .iter()
                .map(|v| DropdownOption { label: format!("{} min", v), action: SetFocusBreakMins(*v) })
                .collect(),
            nearest_index(&FOCUS_BREAK, settings.productivity.focus_break_mins),
        ),
        DropdownId::AiProvider => (
            AI_PROVIDERS
                .iter()
                .enumerate()
                .map(|(i, p)| DropdownOption { label: (*p).into(), action: SetAiDefaultProvider(i as u32) })
                .collect(),
            (settings.ai.default_provider as usize).min(AI_PROVIDERS.len() - 1),
        ),
        DropdownId::AiPoll => (
            AI_POLL
                .iter()
                .map(|v| DropdownOption { label: format!("{} sec", v), action: SetAiPoll(*v) })
                .collect(),
            nearest_index(&AI_POLL, settings.ai.poll_interval_secs),
        ),
    };

    let anchor = id.anchor();
    let longest = options.iter().map(|o| o.label.chars().count()).max().unwrap_or(0) as f32;
    let width = (longest * 7.6 + 52.0)
        .max(anchor.right - anchor.left)
        .min(320.0);
    let item_h = 32.0;
    let height = options.len() as f32 * item_h + 10.0;

    let mut x = anchor.right - width;
    if x < 214.0 {
        x = 214.0;
    }

    let mut y = anchor.bottom + 6.0;
    if y + height > 735.0 - 10.0 {
        y = anchor.top - 6.0 - height;
    }
    if y < 10.0 {
        y = 10.0;
    }

    DropdownMenu {
        options,
        selected,
        rect: D2D_RECT_F { left: x, top: y, right: x + width, bottom: y + height },
        item_h,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsAction {
    SetTheme(ThemeMode),
    SelectTab(usize),
    CloseWindow,
    MinimizeWindow,
    FocusSearch,
    Navigate(i32),
    OpenSectionsTab,
    AddStock,
    RemoveStock(usize),
    SetStockMarket(bool),
    FocusStockInput,
    ToggleAutostart,
    ToggleMascot,
    ToggleMinimalNotch,
    ToggleAutoMinimizeOnMaximize,
    ToggleSectionMusic,
    ToggleSectionFocus,
    ToggleSectionWater,
    ToggleSectionShelf,
    ToggleSectionTodo,
    ToggleSectionOpenMaus,
    ToggleOpenMausNotifications,
    ToggleSectionAiUsage,
    ToggleSectionSaham,
    ToggleAiAntigravity,
    ToggleAiClaude,
    ToggleAiCodex,
    ToggleAiOpenCode,
    SetAiDefaultProvider(u32),
    SetAiPoll(u64),
    AdjustAiPoll(i32),
    ToggleAiProportionBar,
    ToggleMusicGlow,
    ToggleDynamicLyricsWidth,
    ToggleDiscordRpc,
    AdjustFocusWorkMins(i32),
    AdjustFocusBreakMins(i32),
    AdjustWaterIntervalMins(i32),
    AdjustWaterTargetGlasses(i32),
    SetFocusWorkMins(u32),
    SetFocusBreakMins(u32),
    SetWaterIntervalMins(u32),
    SetWaterTargetGlasses(u32),
    SetMinimalNotch(bool),
    SetAutoMinimizeOnMaximize(bool),
    TestWaterReminder,
    SetAccentMode(u32),
    AdjustAcrylicOpacity(f32),
    SetAcrylicOpacity(f32),
    SetFontFamily(usize),
    ToggleDropdown(DropdownId),
    ResetToDefaults,
}

#[derive(Clone, Copy)]
pub struct ClickRegion {
    pub rect: D2D_RECT_F,
    pub action: SettingsAction,
}

pub struct SettingsRenderer;

struct Canvas<'a> {
    stock_focus:Option<bool>,
    dark: bool,
    target: &'a ID2D1RenderTarget,
    brush: &'a ID2D1SolidColorBrush,
    font: &'a DWriteContext,
    mouse: (f32, f32),
    regions: Vec<ClickRegion>,
    open_dropdown: Option<DropdownId>,
    toggle_anim: Option<(SettingsAction, f32)>,
    search_query: &'a str,
    search_index: usize,
    focused_action: Option<SettingsAction>,
}

impl Canvas<'_> {
    fn hovered(&self, r: D2D_RECT_F) -> bool {
        self.mouse.0 >= r.left && self.mouse.0 <= r.right && self.mouse.1 >= r.top && self.mouse.1 <= r.bottom
    }

    fn box_fill(&self, r: D2D_RECT_F, radius: f32, col: impl Into<Paint>) {
        let col = col.into();
        let radius = if radius == 8.0 && col == CARD_BG { 12.0 } else { radius };
        unsafe {
            self.brush.SetColor(&col.resolve(self.dark));
            self.target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT { rect: r, radiusX: radius, radiusY: radius },
                self.brush,
            );
        }
    }

    fn outline(&self, r: D2D_RECT_F, radius: f32, col: Paint) {
        let radius = if radius == 8.0 && col == CARD_BORDER { 12.0 } else { radius };
        unsafe {
            self.brush.SetColor(&col.resolve(self.dark));
            self.target.DrawRoundedRectangle(
                &D2D1_ROUNDED_RECT { rect: r, radiusX: radius, radiusY: radius },
                self.brush,
                1.0,
                None,
            );
        }
    }

    fn text(&self, value: &str, r: D2D_RECT_F, format: &IDWriteTextFormat, col: Paint) {
        let r = if r.left == 306.0 && r.top == 41.0 { rect(394.0, r.top, r.right - 394.0, r.bottom - r.top) } else { r };
        unsafe {
            self.brush.SetColor(&col.resolve(self.dark));
            let utf16: Vec<u16> = value.encode_utf16().collect();
            self.target.DrawText(
                &utf16,
                format,
                &r,
                self.brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, col: Paint) {
        let (x1, x2) = if col == CARD_DIVIDER { (x1 + 12.0, x2 - 12.0) } else { (x1, x2) };
        unsafe {
            self.brush.SetColor(&col.resolve(self.dark));
            self.target.DrawLine(
                D2D_POINT_2F { x: x1, y: y1 },
                D2D_POINT_2F { x: x2, y: y2 },
                self.brush,
                1.0,
                None,
            );
        }
    }

    fn push(&mut self, r: D2D_RECT_F, action: SettingsAction) {
        self.regions.push(ClickRegion { rect: r, action });
    }

    fn badge(&self, x: f32, y: f32, w: f32, label: &str) {
        let r = rect(x, y, w, 16.0);
        self.box_fill(r, 8.0, BADGE_BG);
        self.text(label, r, &self.font.format_badge, BADGE_TEXT);
    }

    fn code_pill(&self, x: f32, y: f32, w: f32, label: &str) {
        let r = rect(x, y, w, 18.0);
        self.box_fill(r, 4.0, CODE_BG);
        self.outline(r, 4.0, CODE_BORDER);
        self.text(label, r, &self.font.format_code_pill, CODE_TEXT);
    }

    fn dropdown_button(&mut self, r: D2D_RECT_F, label: &str, id: DropdownId) {
        let is_open = self.open_dropdown == Some(id);
        if self.hovered(r) || is_open { self.box_fill(r, 6.0, CTRL_HOVER); }
        let text_r = rect(r.left + 4.0, r.top, r.right - r.left - 34.0, r.bottom - r.top);
        self.text(label, text_r, &self.font.format_sidebar_item, CTRL_TEXT);
        let cx = r.right - 13.0;
        let cy = (r.top + r.bottom) * 0.5;
        self.box_fill(rect(cx - 9.0, cy - 10.0, 18.0, 20.0), 8.0, if is_open { SIDEBAR_ACTIVE } else { SEG_TRACK });
        let ink = if is_open { color(1.0,1.0,1.0) } else { CTRL_CARET };
        self.chevron(cx, cy - 4.0, true, ink);
        self.chevron(cx, cy + 4.0, false, ink);
        self.push(r, SettingsAction::ToggleDropdown(id));
    }

    fn chevron(&self, x: f32, y: f32, up: bool, ink: Paint) {
        let dy = if up { 2.0 } else { -2.0 };
        self.line(x - 3.0, y + dy, x, y - dy, ink);
        self.line(x, y - dy, x + 3.0, y + dy, ink);
    }

    fn render_dropdown(&mut self, menu: &DropdownMenu) {
        // Soft drop shadow
        let shadow = D2D_RECT_F {
            left: menu.rect.left + 2.0,
            top: menu.rect.top + 5.0,
            right: menu.rect.right + 2.0,
            bottom: menu.rect.bottom + 5.0,
        };
        self.box_fill(shadow, 10.0, D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.12 });

        self.box_fill(menu.rect, 8.0, MENU_BG);
        self.outline(menu.rect, 8.0, MENU_BORDER);

        for (idx, option) in menu.options.iter().enumerate() {
            let item = menu.item_rect(idx);

            let highlighted = self.hovered(item) || self.focused_action == Some(option.action);
            if highlighted {
                self.box_fill(item, 5.0, SIDEBAR_ACTIVE);
            }

            let is_selected = idx == menu.selected;
            let text_r = rect(item.left + 28.0, item.top, item.right - item.left - 36.0, item.bottom - item.top);
            self.text(
                &option.label,
                text_r,
                if is_selected { &self.font.format_sidebar_active } else { &self.font.format_sidebar_item },
                if highlighted { color(1.0,1.0,1.0) } else { CTRL_TEXT },
            );

            if is_selected {
                let check_r = rect(item.left + 5.0, item.top, 18.0, item.bottom - item.top);
                self.text("✓", check_r, &self.font.format_control_text, if highlighted { color(1.0,1.0,1.0) } else { TEXT_TITLE });
            }

            // Register the visible menu row so clicks select the option it represents.
            self.push(item, option.action);
        }
    }

    fn action_button(&mut self, r: D2D_RECT_F, label: &str, action: SettingsAction) {
        if matches!(label, "On" | "Off" | "Show" | "Hide") {
            let on = label == "On" || label == "Show";
            let track = rect(r.right - 36.0, r.top + 6.0, 36.0, 20.0);
            let amount = self.toggle_anim.filter(|(a, _)| *a == action).map(|(_, t)| {
                let t = t * t * (3.0 - 2.0 * t);
                if on { t } else { 1.0 - t }
            }).unwrap_or(if on { 1.0 } else { 0.0 });
            self.box_fill(track, 10.0, if on { SIDEBAR_ACTIVE } else { SEG_TRACK });
            self.outline(track, 10.0, if on { SIDEBAR_ACTIVE } else { CTRL_BORDER });
            let thumb = rect(track.left + 2.0 + 16.0 * amount, track.top + 2.0, 16.0, 16.0);
            self.box_fill(rect(thumb.left, thumb.top + 1.0, 16.0, 16.0), 8.0, color(0.72, 0.72, 0.74));
            self.box_fill(thumb, 8.0, color(1.0,1.0,1.0));
            self.push(r, action);
            return;
        }
        let is_h = self.hovered(r);
        let bg = if is_h { CTRL_HOVER } else { CTRL_BG };
        let face = rect(r.left, r.top + 3.0, r.right - r.left, r.bottom - r.top - 6.0);
        self.box_fill(rect(face.left, face.top + 1.0, face.right - face.left, face.bottom - face.top), 5.0, SEG_BORDER);
        self.box_fill(face, 5.0, bg);
        self.outline(face, 5.0, CTRL_BORDER);
        self.text(label, face, &self.font.format_control_text, if action == SettingsAction::ResetToDefaults {
            if self.dark { color(1.0, 0.62, 0.64) } else { color(0.76, 0.12, 0.15) }
        } else { CTRL_TEXT });
        self.push(r, action);
    }

    fn radio(&mut self, r: D2D_RECT_F, label: &str, selected: bool, action: SettingsAction) {
        let dot = rect(r.left, r.top + 7.0, 16.0, 16.0);
        self.box_fill(dot, 8.0, if selected { SIDEBAR_ACTIVE } else { CTRL_BG });
        self.outline(dot, 8.0, if selected { SIDEBAR_ACTIVE } else { CTRL_BORDER });
        if selected { self.box_fill(rect(dot.left + 5.0, dot.top + 5.0, 6.0, 6.0), 3.0, color(1.0,1.0,1.0)); }
        self.text(label, rect(r.left + 24.0, r.top, r.right - r.left - 24.0, r.bottom - r.top), &self.font.format_sidebar_item, TEXT_ITEM);
        if !selected { self.push(r, action); }
    }

    fn segmented_control(&mut self, r: D2D_RECT_F, opt1: &str, opt2: &str, is_opt1: bool, action: SettingsAction) {
        self.box_fill(r, 6.0, SEG_TRACK);
        self.outline(r, 6.0, SEG_BORDER);

        let h = r.bottom - r.top;
        let w = r.right - r.left;
        let half_w = (w - 6.0) / 2.0;

        let r1 = rect(r.left + 2.0, r.top + 2.0, half_w, h - 4.0);
        let r2 = rect(r.left + 4.0 + half_w, r.top + 2.0, half_w, h - 4.0);
        let selected=if is_opt1{r1}else{r2};
        self.box_fill(selected,5.0,SEG_ACTIVE);
        self.outline(selected,5.0,SEG_BORDER);
        self.text(opt1,r1,&self.font.format_control_text,if is_opt1{SEG_ACTIVE_TEXT}else{SEG_INACTIVE_TEXT});
        self.text(opt2,r2,&self.font.format_control_text,if is_opt1{SEG_INACTIVE_TEXT}else{SEG_ACTIVE_TEXT});

        // Selecting the current segment must leave the preference unchanged.
        self.push(if is_opt1 { r2 } else { r1 }, action);
    }

    fn sidebar_header(&self, x: f32, y: f32, label: &str) {
        if !self.search_query.is_empty() { return; }
        let r = rect(x, y, 180.0, 16.0);
        self.text(label, r, &self.font.format_sidebar_header, SIDEBAR_MUTED);
    }

    fn sidebar_item(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, is_active: bool, action: SettingsAction) {
        let y = if self.search_query.is_empty() { y + 28.0 } else {
            if !label.to_lowercase().contains(&self.search_query.to_lowercase()) { return; }
            let y = 191.0 + self.search_index as f32 * 38.0;
            self.search_index += 1;
            y
        };
        let r = rect(x, y, w, h);
        let is_h = self.hovered(r);

        if is_active {
            self.box_fill(r, 6.0, SIDEBAR_ACTIVE);
            let text_r = rect(x + 44.0, y, w - 50.0, h);
            self.text(label, text_r, &self.font.format_sidebar_active, SIDEBAR_TEXT_ACTIVE);
        } else {
            if is_h {
                self.box_fill(r, 6.0, SIDEBAR_HOVER);
            }
            let text_r = rect(x + 44.0, y, w - 50.0, h);
            self.text(label, text_r, &self.font.format_sidebar_item, SIDEBAR_TEXT);
        }

        let (glyph, tint) = match action {
            SettingsAction::SelectTab(0) => ("⚙", color(0.49, 0.50, 0.54)),
            SettingsAction::SelectTab(1) => ("▦", color(0.0, 0.56, 0.93)),
            SettingsAction::SelectTab(2) => ("◐", color(0.14, 0.15, 0.18)),
            SettingsAction::SelectTab(3) => ("◷", color(0.91, 0.24, 0.32)),
            SettingsAction::SelectTab(4) => ("▥", color(0.46, 0.32, 0.83)),
            SettingsAction::SelectTab(5) => ("♫", color(0.90, 0.21, 0.35)),
            SettingsAction::SelectTab(6) => ("↗", color(0.20, 0.64, 0.35)),
            SettingsAction::SelectTab(7) => ("⇄", color(0.0, 0.50, 0.85)),
            SettingsAction::SelectTab(8) => ("▱", color(0.0, 0.59, 0.89)),
            SettingsAction::SelectTab(9) => ("i", color(0.49, 0.50, 0.54)),
            SettingsAction::SelectTab(10) => ("⌘", color(0.49, 0.50, 0.54)),
            _ => ("✉", color(0.0, 0.50, 0.85)),
        };
        let icon = rect(x + 9.0, y + (h - 25.0) * 0.5, 25.0, 25.0);
        self.box_fill(icon, 6.0, tint);
        self.text(glyph, icon, &self.font.format_control_text, color(1.0, 1.0, 1.0));

        self.push(r, action);
    }
}

impl SettingsRenderer {
    pub fn render(
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
        dwrite: &DWriteContext,
        settings: &AppSettings,
        active_tab: usize,
        mouse_pos: (f32, f32),
        _save_ok: bool,
        _nav_pos: f32,
        toggle_anim: Option<(SettingsAction, f32)>,
        open_dropdown: Option<DropdownId>,
        stock_market_idx: bool,
        _w: f32,
        _h: f32,
        search_query: &str,
        search_focused: bool,
        navigation: (bool, bool),
        focused_action: Option<SettingsAction>,
        stock_focus: Option<bool>,
    ) -> Vec<ClickRegion> {
        let dark = super::theme::is_dark(settings);
        let mut c = Canvas {
            stock_focus,
            dark,
            target,
            brush,
            font: dwrite,
            mouse: mouse_pos,
            regions: vec![],
            open_dropdown,
            toggle_anim,
            search_query,
            search_index: 0,
            focused_action,
        };

        let h = 735.0;

        // 1. Draw Backgrounds (Split Left Sidebar & Right Content)
        unsafe { target.Clear(Some(&COLOR_BG.resolve(dark))); }

        let sidebar_w = 280.0;
        let sidebar_rect = rect(0.0, 0.0, sidebar_w, h);
        c.box_fill(sidebar_rect, 0.0, SIDEBAR_BG);
        c.line(sidebar_w, 0.0, sidebar_w, h, SIDEBAR_BORDER);
        for (x, enabled, direction) in [(312.0, navigation.0, -1), (352.0, navigation.1, 1)] {
            let r = rect(x - 8.0, 37.0, 32.0, 34.0);
            if enabled && c.hovered(r) { c.box_fill(r, 8.0, SIDEBAR_HOVER); }
            let ink = if enabled { TEXT_MUTED } else { SIDEBAR_BORDER };
            let dx = if direction < 0 { 4.0 } else { -4.0 };
            c.line(x + 8.0 + dx, 47.0, x + 8.0 - dx, 54.0, ink);
            c.line(x + 8.0 - dx, 54.0, x + 8.0 + dx, 61.0, ink);
            if enabled { c.push(r, SettingsAction::Navigate(direction)); }
        }

        for (x, tint, action) in [
            (22.0, color(1.0, 0.36, 0.39), Some(SettingsAction::CloseWindow)),
            (46.0, color(1.0, 0.74, 0.17), Some(SettingsAction::MinimizeWindow)),
            (70.0, color(0.78, 0.78, 0.80), None),
        ] {
            let hit = rect(x - 4.0, 20.0, 24.0, 24.0);
            c.box_fill(rect(x, 24.0, 16.0, 16.0), 8.0, tint);
            if let Some(action) = action {
                if c.hovered(hit) { c.text(if x < 40.0 { "×" } else { "−" }, rect(x, 24.0, 16.0, 16.0), &dwrite.format_control_text, TEXT_TITLE); }
                c.push(hit, action);
            }
        }
        let search = rect(22.0, 64.0, 236.0, 30.0);
        c.box_fill(search, 9.0, SEG_TRACK);
        if search_focused { c.outline(search, 9.0, SIDEBAR_ACTIVE); }
        c.outline(rect(32.0, 72.0, 11.0, 11.0), 5.5, TEXT_MUTED);
        c.line(41.0, 81.0, 46.0, 86.0, TEXT_MUTED);
        c.text(if search_query.is_empty() { "Search" } else { search_query }, rect(53.0, 64.0, 192.0, 30.0), &dwrite.format_sidebar_item, TEXT_MUTED);
        c.push(search, SettingsAction::FocusSearch);
        c.box_fill(rect(22.0, 110.0, 44.0, 44.0), 12.0, color(0.18, 0.19, 0.22));
        c.text("N", rect(22.0, 110.0, 44.0, 44.0), &dwrite.format_big_center, color(1.0, 1.0, 1.0));
        c.text("Nocan", rect(78.0, 112.0, 170.0, 22.0), &dwrite.format_sidebar_active, TEXT_TITLE);
        c.text("Living Notch", rect(78.0, 134.0, 170.0, 18.0), &dwrite.format_page_sub, TEXT_MUTED);
        c.sidebar_header(25.0, 169.0, "Settings");

        let settings_items = [
            (0, "General", 163.0),
            (1, "Sections", 201.0),
            (2, "Appearance", 239.0),
            (3, "Notifications", 277.0),
            (4, "AI Monitor", 315.0),
        ];
        for (tab_id, name, item_y) in settings_items {
            c.sidebar_item(14.0, item_y, 252.0, 36.0, name, active_tab == tab_id, SettingsAction::SelectTab(tab_id));
        }

        c.sidebar_header(25.0, 397.0, "Modules");

        let module_items = [
            (5, "Music & Lyrics", 391.0),
            (6, "Stock Portfolio", 429.0),
            (7, "OpenMaus Bot", 467.0),
            (8, "File Shelf", 505.0),
        ];
        for (tab_id, name, item_y) in module_items {
            c.sidebar_item(14.0, item_y, 252.0, 36.0, name, active_tab == tab_id, SettingsAction::SelectTab(tab_id));
        }

        c.sidebar_header(25.0, 587.0, "System");
        c.sidebar_item(14.0, 581.0, 252.0, 36.0, "Specifications", active_tab == 9, SettingsAction::SelectTab(9));

        // Bottom Sidebar Items
        c.sidebar_item(14.0, 619.0, 252.0, 36.0, "Shortcuts", active_tab == 10, SettingsAction::SelectTab(10));
        c.sidebar_item(14.0, 657.0, 252.0, 36.0, "Give Feedback", active_tab == 11, SettingsAction::SelectTab(11));
        if !search_query.is_empty() && c.search_index == 0 {
            c.text("No settings found.", rect(25.0, 191.0, 230.0, 30.0), &dwrite.format_item_desc, TEXT_MUTED);
        }

        // 4. Main Content Area Rendering
        let content_x = 306.0;
        let content_w = 623.0;

        match active_tab {
            // General
            0 => {
                c.text("General", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Customize notch behavior, the mascot, and Windows integration.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Notch Appearance", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                c.text("Choose the notch size and desktop mascot animation.", rect(content_x,174.0,content_w,18.0),&dwrite.format_section_desc,TEXT_DESC);

                // Card Container 1 (3 Rows: 204.0 to 424.0)
                let card1 = rect(content_x, 204.0, content_w, 220.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);

                // Row 1: Ukuran Siluet Notch (204 to 276)
                c.text("Notch Size", rect(content_x + 16.0, 220.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Choose the notch size when idle.", rect(content_x + 16.0, 242.0, 420.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);

                let preset_label = if settings.general.minimal_notch { "Minimal" } else { "Standard" };
                c.dropdown_button(DropdownId::NotchPreset.anchor(), preset_label, DropdownId::NotchPreset);

                c.line(content_x, 276.0, content_x + content_w, 276.0, CARD_DIVIDER);

                // Row 2: Maskot Nocan (with badge 60 FPS) (276 to 348)
                c.text("Nocan Mascot", rect(content_x + 16.0, 292.0, 94.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.badge(content_x + 114.0, 294.0, 52.0, "60 FPS");
                c.text("Interactive tiger eyes and whiskers in the notch.", rect(content_x + 16.0, 314.0, 420.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                let mascot_label = if settings.general.mascot_enabled { "On" } else { "Off" };
                c.action_button(rect(836.0, 296.0, 78.0, 32.0), mascot_label, SettingsAction::ToggleMascot);

                c.line(content_x, 348.0, content_x + content_w, 348.0, CARD_DIVIDER);

                // Row 3: Seksi Notch Aktif (with badge 8 Modul) (348 to 424)
                c.text("Notch Sections", rect(content_x + 16.0, 364.0, 114.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.badge(content_x + 134.0, 366.0, 58.0, "8 Modules");
                c.text("Manage music, focus, hydration, tasks, and AI monitoring.", rect(content_x + 16.0, 386.0, 420.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.action_button(rect(846.0, 368.0, 68.0, 32.0), "Manage", SettingsAction::OpenSectionsTab);

                c.text("Desktop & Window Behavior", rect(content_x, 456.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card2 = rect(content_x, 486.0, content_w, 92.0);
                c.box_fill(card2, 8.0, CARD_BG);
                c.outline(card2, 8.0, CARD_BORDER);

                c.text("Minimize on Maximize", rect(content_x + 16.0, 500.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Shrink the notch when another window is maximized.", rect(content_x + 16.0, 522.0, 420.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);

                c.text("Shortcut:", rect(content_x + 16.0, 546.0, 56.0, 18.0), &dwrite.format_item_desc, TEXT_DESC);
                c.code_pill(content_x + 76.0, 546.0, 34.0, "Win");
                c.text("+", rect(content_x + 114.0, 546.0, 10.0, 18.0), &dwrite.format_item_desc, TEXT_DESC);
                c.code_pill(content_x + 128.0, 546.0, 38.0, "Shift");
                c.text("+", rect(content_x + 170.0, 546.0, 10.0, 18.0), &dwrite.format_item_desc, TEXT_DESC);
                c.code_pill(content_x + 184.0, 546.0, 20.0, "N");
                c.text("to toggle notch visibility.", rect(content_x + 210.0, 546.0, 240.0, 18.0), &dwrite.format_item_desc, TEXT_DESC);

                let auto_min_label = if settings.general.auto_minimize_on_maximize { "Auto Minimize" } else { "Keep Visible" };
                c.dropdown_button(DropdownId::AutoMinimize.anchor(), auto_min_label, DropdownId::AutoMinimize);

                c.text("Startup", rect(content_x, 604.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card3 = rect(content_x, 634.0, content_w, 76.0);
                c.box_fill(card3, 8.0, CARD_BG);
                c.outline(card3, 8.0, CARD_BORDER);

                c.text("Start with Windows", rect(content_x + 16.0, 646.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Launch Nocan automatically when you sign in to Windows.", rect(content_x + 16.0, 666.0, 400.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.text("Windows Registry Run entry ⓘ", rect(content_x + 16.0, 684.0, 240.0, 16.0), &dwrite.format_item_desc, SIDEBAR_MUTED);

                c.radio(rect(744.0, 638.0, 170.0, 30.0), "Automatic", settings.general.autostart, SettingsAction::ToggleAutostart);
                c.radio(rect(744.0, 672.0, 170.0, 30.0), "Manual", !settings.general.autostart, SettingsAction::ToggleAutostart);
            },

            // TAB 1: "Sections" (Modul Bilah Notch)
            1 => {
                c.text("Sections", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Choose which of the 8 modules appear in the notch.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Notch Modules", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let sections = [
                    ("Music & Sync Lyrics", "Media playback with synchronized lyrics & dynamic ambience.", settings.sections.show_music, SettingsAction::ToggleSectionMusic),
                    ("Focus Pomodoro", "Pomodoro work timer with break countdowns.", settings.sections.show_focus, SettingsAction::ToggleSectionFocus),
                    ("Water Reminder", "Daily hydration logging with automated alerts.", settings.sections.show_water, SettingsAction::ToggleSectionWater),
                    ("File Shelf", "Quick drag-and-drop storage shelf.", settings.sections.show_shelf, SettingsAction::ToggleSectionShelf),
                    ("Daily Tasks", "Interactive task checklist for productivity.", settings.sections.show_todo, SettingsAction::ToggleSectionTodo),
                    ("OpenMaus Bot", "Agent activity, automation schedules, and model usage.", settings.sections.show_openmaus, SettingsAction::ToggleSectionOpenMaus),
                    ("AI Usage Monitor", "Multi-provider AI quota & token trackers.", settings.sections.show_ai_usage, SettingsAction::ToggleSectionAiUsage),
                    ("Stock Portfolio", "Market tracking for BEI/IDX and global tickers.", settings.sections.show_saham, SettingsAction::ToggleSectionSaham),
                ];

                let card_h = sections.len() as f32 * 64.0;
                let card = rect(content_x, 175.0, content_w, card_h);
                c.box_fill(card, 8.0, CARD_BG);
                c.outline(card, 8.0, CARD_BORDER);

                for (idx, (sec_title, sec_desc, is_on, action)) in sections.iter().enumerate() {
                    let ry = 175.0 + idx as f32 * 64.0;
                    c.text(sec_title, rect(content_x + 16.0, ry + 12.0, 260.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                    c.text(sec_desc, rect(content_x + 16.0, ry + 32.0, content_w - 180.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);

                    let status_str = if *is_on { "On" } else { "Off" };
                    c.action_button(rect(736.0, ry + 16.0, 78.0, 32.0), status_str, *action);

                    if idx + 1 < sections.len() {
                        c.line(content_x, ry + 64.0, content_x + content_w, ry + 64.0, CARD_DIVIDER);
                    }
                }
            },

            // TAB 2: "Appearance"
            2 => {
                c.text("Appearance", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Customize Nocan's colors, fonts, and appearance.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                let preview = rect(content_x, 120.0, content_w, 105.0);
                c.box_fill(preview, 14.0, CARD_BG);
                c.outline(preview, 14.0, CARD_BORDER);
                c.text("Appearance", rect(content_x + 16.0, 138.0, 180.0, 22.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text(if settings.appearance.theme == ThemeMode::System { "Follows Windows theme" } else { "Settings window theme" }, rect(content_x + 16.0, 167.0, 205.0, 20.0), &dwrite.format_item_desc, TEXT_DESC);
                for (x, mode, label) in [(574.0, ThemeMode::Light, "Light"), (688.0, ThemeMode::Dark, "Dark"), (802.0, ThemeMode::System, "System OS")] {
                    let tile = rect(x, 133.0, 94.0, 55.0);
                    let tile_dark = mode == ThemeMode::Dark;
                    c.box_fill(tile, 8.0, if tile_dark { color(0.13,0.14,0.17) } else { color(0.88,0.93,0.98) });
                    c.box_fill(rect(x + 5.0, 138.0, 24.0, 45.0), 4.0, if tile_dark { color(0.25,0.26,0.29) } else { color(0.76,0.82,0.88) });
                    c.box_fill(rect(x + 34.0, 148.0, 54.0, 35.0), 4.0, if tile_dark { color(0.22,0.23,0.26) } else { color(1.0,1.0,1.0) });
                    if mode == ThemeMode::System { c.box_fill(rect(x + 47.0,133.0,47.0,55.0), 6.0, color(0.16,0.17,0.20)); }
                    if settings.appearance.theme == mode {
                        c.outline(rect(x - 3.0, 130.0, 100.0, 61.0), 10.0, SIDEBAR_ACTIVE);
                        c.outline(rect(x - 2.0, 131.0, 98.0, 59.0), 9.0, SIDEBAR_ACTIVE);
                    }
                    c.text(label, rect(x, 194.0, 94.0, 22.0), &dwrite.format_control_text, TEXT_ITEM);
                    c.push(rect(x - 3.0, 130.0, 100.0, 86.0), SettingsAction::SetTheme(mode));
                }

                c.text("Theme", rect(content_x + 12.0, 255.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card1 = rect(content_x, 420.0, content_w, 70.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);
                c.text("Interface Font", rect(content_x + 16.0, 435.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Applies to the notch and settings.", rect(content_x + 16.0, 457.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.dropdown_button(DropdownId::FontFamily.anchor(), &dwrite.font_name, DropdownId::FontFamily);


                let accents = [("Auto (Windows)", 0), ("Gemini Blue", 1), ("Coral Red", 2), ("Emerald", 3), ("Cyan", 4), ("Violet", 5)];
                let card2 = rect(content_x, 285.0, content_w, 120.0);
                c.box_fill(card2, 8.0, CARD_BG);
                c.outline(card2, 8.0, CARD_BORDER);

                let curr_accent = accents.iter().find(|(_, id)| *id == settings.appearance.accent_mode).map(|(n, _)| *n).unwrap_or("Auto");
                c.text("Accent Color", rect(content_x + 16.0, 310.0, 160.0, 22.0), &dwrite.format_item_title, TEXT_ITEM);
                let swatches = [color(0.40, 0.43, 0.48), color(0.0, 0.48, 1.0), color(1.0, 0.27, 0.30), color(0.22, 0.68, 0.34), color(0.0, 0.68, 0.78), color(0.64, 0.28, 0.73)];
                for (index, tint) in swatches.iter().enumerate() {
                    let x = 530.0 + index as f32 * 62.0;
                    let selected = settings.appearance.accent_mode == index as u32;
                    if selected { c.outline(rect(x - 4.0, 301.0, 40.0, 40.0), 20.0, SIDEBAR_ACTIVE); }
                    c.box_fill(rect(x, 305.0, 32.0, 32.0), 16.0, *tint);
                    if selected { c.text("✓", rect(x, 305.0, 32.0, 32.0), &dwrite.format_control_text, color(1.0, 1.0, 1.0)); }
                    c.push(rect(x - 6.0, 299.0, 44.0, 44.0), SettingsAction::SetAccentMode(index as u32));
                }
                c.line(content_x + 16.0, 356.0, content_x + content_w - 16.0, 356.0, CARD_DIVIDER);
                c.text("Selected Color", rect(content_x + 16.0, 368.0, 260.0, 22.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text(curr_accent, rect(744.0, 368.0, 170.0, 22.0), &dwrite.format_regular_right, TEXT_MUTED);

                c.text("Window Material", rect(content_x + 12.0, 525.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card3 = rect(content_x, 555.0, content_w, 70.0);
                c.box_fill(card3, 8.0, CARD_BG);
                c.outline(card3, 8.0, CARD_BORDER);

                let pct = (settings.appearance.acrylic_opacity * 100.0).round();
                c.text("Notch Opacity", rect(content_x + 16.0, 570.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Adjust the notch background opacity.", rect(content_x + 16.0, 592.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);

                let slider = OPACITY_SLIDER;
                let thumb_x = slider.left + (settings.appearance.acrylic_opacity - 0.5) * 2.0 * (slider.right - slider.left);
                c.box_fill(rect(slider.left, 586.0, slider.right - slider.left, 5.0), 2.5, SEG_TRACK);
                c.box_fill(rect(slider.left, 586.0, thumb_x - slider.left, 5.0), 2.5, SIDEBAR_ACTIVE);
                c.box_fill(rect(thumb_x - 9.0, 581.0, 18.0, 18.0), 9.0, color(0.76, 0.76, 0.79));
                c.box_fill(rect(thumb_x - 9.0, 579.0, 18.0, 18.0), 9.0, color(1.0,1.0,1.0));
                c.outline(rect(thumb_x - 9.0, 579.0, 18.0, 18.0), 9.0, CTRL_BORDER);
                c.text(&format!("{}%", pct), rect(877.0, 574.0, 42.0, 32.0), &dwrite.format_control_text, TEXT_TITLE);
                c.push(slider, SettingsAction::SetAcrylicOpacity(settings.appearance.acrylic_opacity));
            },

            // TAB 3: "Notifications"
            3 => {
                c.text("Notifications", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Set hydration reminders and Pomodoro work and break intervals.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Daily Hydration", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card1 = rect(content_x, 175.0, content_w, 212.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);

                c.text("Reminder Interval", rect(content_x + 16.0, 190.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Time between hydration reminders in the notch.", rect(content_x + 16.0, 212.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.dropdown_button(DropdownId::WaterInterval.anchor(), &format!("{} min", settings.productivity.water_interval_mins), DropdownId::WaterInterval);

                c.line(content_x, 245.0, content_x + content_w, 245.0, CARD_DIVIDER);

                c.text("Daily Water Goal", rect(content_x + 16.0, 260.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Daily goal in glasses (250 ml each).", rect(content_x + 16.0, 282.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.dropdown_button(DropdownId::WaterGlasses.anchor(), &format!("{} glasses", settings.productivity.water_daily_target_glasses), DropdownId::WaterGlasses);

                c.line(content_x, 315.0, content_x + content_w, 315.0, CARD_DIVIDER);

                c.text("Test Hydration Reminder", rect(content_x + 16.0, 330.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Preview the hydration reminder animation in the notch.", rect(content_x + 16.0, 352.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.action_button(rect(814.0, 334.0, 100.0, 32.0), "Test Reminder", SettingsAction::TestWaterReminder);

                c.text("Pomodoro Focus", rect(content_x, 415.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card2 = rect(content_x, 445.0, content_w, 142.0);
                c.box_fill(card2, 8.0, CARD_BG);
                c.outline(card2, 8.0, CARD_BORDER);

                c.text("Work Duration", rect(content_x + 16.0, 460.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Focus time before each break.", rect(content_x + 16.0, 482.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.dropdown_button(DropdownId::FocusWork.anchor(), &format!("{} min", settings.productivity.focus_work_mins), DropdownId::FocusWork);

                c.line(content_x, 515.0, content_x + content_w, 515.0, CARD_DIVIDER);

                c.text("Short Break Duration", rect(content_x + 16.0, 530.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Rest time before the next focus session.", rect(content_x + 16.0, 552.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.dropdown_button(DropdownId::FocusBreak.anchor(), &format!("{} min", settings.productivity.focus_break_mins), DropdownId::FocusBreak);
            },

            // TAB 4: "AI Monitor"
            4 => {
                c.text("AI Monitor", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Choose AI providers, daily token limits, and refresh intervals.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("AI Providers", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card1 = rect(content_x, 175.0, content_w, 212.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);

                let cur_prov = AI_PROVIDERS.get(settings.ai.default_provider as usize).copied().unwrap_or("Gemini");

                c.text("Default Provider", rect(content_x + 16.0, 190.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Provider tab shown when you open the AI section.", rect(content_x + 16.0, 212.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.dropdown_button(DropdownId::AiProvider.anchor(), cur_prov, DropdownId::AiProvider);

                c.line(content_x, 245.0, content_x + content_w, 245.0, CARD_DIVIDER);

                c.text("Token Usage Bar", rect(content_x + 16.0, 260.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Show each provider's share of daily token usage.", rect(content_x + 16.0, 282.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.segmented_control(
                    rect(744.0, 264.0, 170.0, 32.0),
                    "Show",
                    "Hide",
                    settings.ai.show_proportional_bar,
                    SettingsAction::ToggleAiProportionBar,
                );

                c.line(content_x, 315.0, content_x + content_w, 315.0, CARD_DIVIDER);

                c.text("Refresh Interval", rect(content_x + 16.0, 330.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("How often token usage syncs with providers.", rect(content_x + 16.0, 352.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.dropdown_button(DropdownId::AiPoll.anchor(), &format!("{} sec", settings.ai.poll_interval_secs), DropdownId::AiPoll);

                c.text("Provider Visibility", rect(content_x, 415.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let m_items = [
                    ("Google Gemini", settings.ai.show_antigravity, SettingsAction::ToggleAiAntigravity),
                    ("Anthropic Claude", settings.ai.show_claude, SettingsAction::ToggleAiClaude),
                    ("OpenAI Codex", settings.ai.show_codex, SettingsAction::ToggleAiCodex),
                    ("OpenCode Local", settings.ai.show_opencode, SettingsAction::ToggleAiOpenCode),
                ];
                let card2 = rect(content_x, 445.0, content_w, 240.0);
                c.box_fill(card2, 8.0, CARD_BG);
                c.outline(card2, 8.0, CARD_BORDER);

                for (idx, (m_title, is_on, action)) in m_items.iter().enumerate() {
                    let ry = 445.0 + idx as f32 * 60.0;
                    c.text(m_title, rect(content_x + 16.0, ry + 16.0, 260.0, 20.0), &dwrite.format_item_title, TEXT_ITEM);
                    let status = if *is_on { "Show" } else { "Hide" };
                    c.action_button(rect(836.0, ry + 14.0, 78.0, 32.0), status, *action);
                    if idx + 1 < m_items.len() {
                        c.line(content_x, ry + 60.0, content_x + content_w, ry + 60.0, CARD_DIVIDER);
                    }
                }
            },

            // TAB 5: "Music & Lyrics"
            5 => {
                c.text("Music & Lyrics", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Windows media controls, synced lyrics, and ambient lighting.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Music Appearance", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card1 = rect(content_x, 175.0, content_w, 212.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);

                c.text("Ambient Glow", rect(content_x + 16.0, 190.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Match the ambient glow to the current album artwork.", rect(content_x + 16.0, 212.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                let glow_label = if settings.music.ambient_glow_enabled { "On" } else { "Off" };
                c.action_button(rect(836.0, 194.0, 78.0, 32.0), glow_label, SettingsAction::ToggleMusicGlow);

                c.line(content_x, 245.0, content_x + content_w, 245.0, CARD_DIVIDER);

                c.text("Dynamic Lyrics Width", rect(content_x + 16.0, 260.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Resize the notch smoothly to fit the current lyric.", rect(content_x + 16.0, 282.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                let lyr_label = if settings.music.dynamic_lyrics_width { "On" } else { "Off" };
                c.action_button(rect(836.0, 264.0, 78.0, 32.0), lyr_label, SettingsAction::ToggleDynamicLyricsWidth);

                c.line(content_x, 315.0, content_x + content_w, 315.0, CARD_DIVIDER);

                c.text("Discord Rich Presence (RPC)", rect(content_x + 16.0, 330.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Show your Nocan music activity on your Discord profile.", rect(content_x + 16.0, 352.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                let rpc_label = if settings.music.discord_rpc_enabled { "On" } else { "Off" };
                c.action_button(rect(836.0, 334.0, 78.0, 32.0), rpc_label, SettingsAction::ToggleDiscordRpc);
            },

            // TAB 6: "Stock Portfolio"
            6 => {
                c.text("Stock Portfolio", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Track Indonesian (IDX) and global stock markets.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Stock Watchlist", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);
                c.text(
                    &format!("{} / 6 stocks", settings.stock_symbols.len()),
                    rect(content_x + content_w - 140.0, 145.0, 140.0, 20.0),
                    &dwrite.format_regular_right,
                    if settings.stock_symbols.len() >= 6 { SUCCESS } else { TEXT_MUTED },
                );

                let card_top = 175.0;
                let input_y = card_top + 16.0;
                let row_h = 46.0;
                let rows_top = input_y + 34.0 + 16.0;
                let visible_rows = settings.stock_symbols.len().max(1) as f32;
                let card_h = (rows_top - card_top) + visible_rows * row_h + 12.0;

                let card = rect(content_x, card_top, content_w, card_h);
                c.box_fill(card, 8.0, CARD_BG);
                c.outline(card, 8.0, CARD_BORDER);

                let input_r = super::stock_input::FIELD;
                c.box_fill(input_r,7.0,if dark{color(35.0/255.0,35.0/255.0,38.0/255.0)}else{color(1.0,1.0,1.0)});
                c.outline(input_r,7.0,if dark{color(0.32,0.32,0.35)}else{color(0.74,0.74,0.77)});
                if c.stock_focus==Some(true) {
                    c.outline(rect(input_r.left-1.0,input_r.top-1.0,input_r.right-input_r.left+2.0,38.0),8.0,color(0.04,0.52,1.0));
                    c.outline(input_r,7.0,color(0.04,0.52,1.0));
                }
                if c.stock_focus.is_none() {
                    c.text(if stock_market_idx{super::stock_input::PLACEHOLDER}else{super::stock_input::GLOBAL_PLACEHOLDER},rect(input_r.left+12.0,input_r.top,input_r.right-input_r.left-24.0,36.0),&dwrite.format_sidebar_item,TEXT_MUTED);
                }
                c.push(input_r,SettingsAction::FocusStockInput);

                c.segmented_control(
                    super::stock_input::MARKET,
                    "IDX",
                    "Global",
                    stock_market_idx,
                    SettingsAction::SetStockMarket(!stock_market_idx),
                );

                let add=super::stock_input::ADD;
                c.box_fill(rect(add.left,add.top+1.0,90.0,35.0),7.0,if dark{color(0.0,0.24,0.58)}else{color(0.0,0.27,0.66)});
                c.box_fill(add,7.0,if c.hovered(add){color(0.04,0.41,0.85)}else{color(0.0,0.36,0.80)});
                c.text("+ Add",add,&dwrite.format_control_text,color(1.0,1.0,1.0));
                c.push(add,SettingsAction::AddStock);

                if settings.stock_symbols.is_empty() {
                    c.text(
                        "Your watchlist is empty. Enter a ticker above and click + Add.",
                        rect(content_x + 16.0, rows_top + 10.0, content_w - 32.0, 20.0),
                        &dwrite.format_item_desc,
                        TEXT_DESC,
                    );
                } else {
                    for (idx, symbol) in settings.stock_symbols.iter().enumerate() {
                        let ry = rows_top + idx as f32 * row_h;

                        if idx > 0 {
                            c.line(content_x + 16.0, ry, content_x + content_w - 16.0, ry, CARD_DIVIDER);
                        }

                        let avatar = rect(content_x + 16.0, ry + 8.0, 30.0, 30.0);
                        c.box_fill(avatar, 8.0, stock_badge_color(symbol));
                        c.text(stock_badge_label(symbol), avatar, &dwrite.format_badge, color(0.96, 0.96, 0.96));

                        c.text(symbol, rect(content_x + 58.0, ry + 6.0, 340.0, 20.0), &dwrite.format_item_title, TEXT_ITEM);
                        c.text(
                            if symbol.ends_with(".JK") { "Indonesia Stock Exchange (IDX)" } else { "Global market" },
                            rect(content_x + 58.0, ry + 25.0, 340.0, 16.0),
                            &dwrite.format_item_desc,
                            TEXT_DESC,
                        );

                        let remove = rect(content_x + content_w - 46.0, ry + 8.0, 30.0, 30.0);
                        let hovering = c.hovered(remove);
                        c.box_fill(remove, 6.0, if hovering { CLOSE_HOVER } else { CTRL_BG });
                        c.text("✕", remove, &dwrite.format_close_x, if hovering { TEXT_TITLE } else { CLOSE_X });
                        c.push(remove, SettingsAction::RemoveStock(idx));
                    }
                }

                let market_y = card_top + card_h + 28.0;
                c.text("Market Data", rect(content_x, market_y, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card2 = rect(content_x, market_y + 30.0, content_w, 70.0);
                c.box_fill(card2, 8.0, CARD_BG);
                c.outline(card2, 8.0, CARD_BORDER);

                c.text("Yahoo Finance Data Feed", rect(content_x + 16.0, market_y + 45.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Latest quotes for Indonesian and global stocks.", rect(content_x + 16.0, market_y + 67.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.text("Connected ●", rect(814.0, market_y + 55.0, 100.0, 20.0), &dwrite.format_control_text, SUCCESS);
            },

            // TAB 7: "OpenMaus LLM"
            7 => {
                c.text("OpenMaus Bot", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Track OpenMaus agents, automations, and usage in the notch.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("OpenMaus Integration", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card1 = rect(content_x, 175.0, content_w, 142.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);

                c.text("Show OpenMaus Section", rect(content_x + 16.0, 190.0, 300.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Activity, upcoming runs, and bot, model, and provider details.", rect(content_x + 16.0, 212.0, 440.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.action_button(rect(836.0,190.0,78.0,32.0),if settings.sections.show_openmaus {"On"}else{"Off"},SettingsAction::ToggleSectionOpenMaus);

                c.line(content_x, 245.0, content_x + content_w, 245.0, CARD_DIVIDER);

                c.text("Activity Notifications", rect(content_x + 16.0, 260.0, 300.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Alerts for automation runs, agent replies, and attention needed.", rect(content_x + 16.0, 282.0, 450.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.action_button(rect(836.0,260.0,78.0,32.0),if settings.openmaus_notifications {"On"}else{"Off"},SettingsAction::ToggleOpenMausNotifications);
                c.text("Local Data Source",rect(content_x,350.0,content_w,20.0),&dwrite.format_section_header,TEXT_TITLE);
                c.box_fill(rect(content_x,380.0,content_w,92.0),8.0,CARD_BG);
                c.outline(rect(content_x,380.0,content_w,92.0),8.0,CARD_BORDER);
                c.text("~/.openmausbot",rect(content_x+16.0,394.0,content_w-32.0,20.0),&dwrite.format_item_title,TEXT_ITEM);
                c.text("Updates automatically. Past activity won't trigger alerts on startup.",rect(content_x+16.0,424.0,content_w-32.0,30.0),&dwrite.format_item_desc,TEXT_DESC);
            },

            // TAB 8: "File Shelf"
            8 => {
                c.text("File Shelf", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Drag and drop files into a temporary shelf in the notch.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Shelf Settings", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card1 = rect(content_x, 175.0, content_w, 142.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);

                c.text("File Capacity", rect(content_x + 16.0, 190.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Maximum number of files held in the shelf.", rect(content_x + 16.0, 212.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.text("5 Files", rect(774.0, 194.0, 140.0, 32.0), &dwrite.format_control_text, CTRL_TEXT);

                c.line(content_x, 245.0, content_x + content_w, 245.0, CARD_DIVIDER);

                c.text("Clear on Restart", rect(content_x + 16.0, 260.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Clear temporary file history when your computer restarts.", rect(content_x + 16.0, 282.0, 380.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.action_button(rect(836.0, 264.0, 78.0, 32.0), if settings.sections.show_shelf { "On" } else { "Off" }, SettingsAction::ToggleSectionShelf);
            },

            // TAB 9: "Specifications & System"
            9 => {
                c.text("Specifications", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("View graphics engine details and restore default settings.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Software Architecture", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let specs = [
                    ("Graphics Engine", "Direct2D 1.1 + DirectWrite GPU Hardware Accelerated"),
                    ("Animation Pipeline", "60 FPS Adaptive Frame Rate with Elastic Spring Physics"),
                    ("OS Integration", "Windows 11 Fluent DWM Immersive Dark Architecture"),
                    ("Application Version", concat!("v", env!("CARGO_PKG_VERSION"), " • Local-First Zero Telemetry")),
                ];

                let card1 = rect(content_x, 175.0, content_w, 240.0);
                c.box_fill(card1, 8.0, CARD_BG);
                c.outline(card1, 8.0, CARD_BORDER);

                for (idx, (s_title, s_desc)) in specs.iter().enumerate() {
                    let ry = 175.0 + idx as f32 * 60.0;
                    c.text(s_title, rect(content_x + 16.0, ry + 12.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                    c.text(s_desc, rect(content_x + 16.0, ry + 32.0, content_w - 32.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                    if idx + 1 < specs.len() {
                        c.line(content_x, ry + 60.0, content_x + content_w, ry + 60.0, CARD_DIVIDER);
                    }
                }

                c.text("Restore Defaults", rect(content_x, 445.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card2 = rect(content_x, 475.0, content_w, 70.0);
                c.box_fill(card2, 8.0, CARD_BG);
                c.outline(card2, 8.0, CARD_BORDER);

                c.text("Reset to Defaults", rect(content_x + 16.0, 490.0, 240.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Restore default appearance, sections, and quota settings.", rect(content_x + 16.0, 510.0, 420.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.action_button(rect(764.0, 494.0, 150.0, 32.0), "Reset Settings", SettingsAction::ResetToDefaults);
            },

            // TAB 10: "Shortcuts"
            10 => {
                c.text("Shortcuts", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Keyboard shortcuts for navigating the notch and settings.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Keyboard Shortcuts", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let shortcuts = [
                    ("Toggle Settings", "Win + Shift + N"),
                    ("Navigate Settings", "Tab / Shift + Tab"),
                    ("Activate Control", "Enter / Space"),
                    ("Close Settings", "Esc / ✕ button"),
                ];

                let card = rect(content_x, 175.0, content_w, 240.0);
                c.box_fill(card, 8.0, CARD_BG);
                c.outline(card, 8.0, CARD_BORDER);

                for (idx, (s_label, s_key)) in shortcuts.iter().enumerate() {
                    let ry = 175.0 + idx as f32 * 60.0;
                    c.text(s_label, rect(content_x + 16.0, ry + 20.0, 260.0, 20.0), &dwrite.format_item_title, TEXT_ITEM);
                    c.text(s_key, rect(744.0, ry + 20.0, 170.0, 20.0), &dwrite.format_sidebar_item, TEXT_MUTED);
                    if idx + 1 < shortcuts.len() {
                        c.line(content_x, ry + 60.0, content_x + content_w, ry + 60.0, CARD_DIVIDER);
                    }
                }
            },

            // TAB 11: "Give Feedback"
            _ => {
                c.text("Give Feedback", rect(content_x, 41.0, content_w, 30.0), &dwrite.format_page_title, TEXT_TITLE);
                c.text("Share ideas, suggestions, and feedback about Nocan.", rect(content_x, 77.0, content_w, 20.0), &dwrite.format_page_sub, TEXT_MUTED);

                c.text("Developer & Community", rect(content_x, 145.0, content_w, 20.0), &dwrite.format_section_header, TEXT_TITLE);

                let card = rect(content_x, 175.0, content_w, 142.0);
                c.box_fill(card, 8.0, CARD_BG);
                c.outline(card, 8.0, CARD_BORDER);

                c.text("Developer Support", rect(content_x + 16.0, 190.0, 260.0, 18.0), &dwrite.format_item_title, TEXT_ITEM);
                c.text("Send questions, bug reports, or ideas for notch sections to:", rect(content_x + 16.0, 212.0, 420.0, 16.0), &dwrite.format_item_desc, TEXT_DESC);
                c.text("scorush.id@gmail.com", rect(content_x + 16.0, 240.0, 240.0, 20.0), &dwrite.format_item_title, LINK_BLUE);
            }
        }

        // Dropdown popup is always rendered last so it stays above cards and controls.
        if let Some(id) = open_dropdown {
            if id.tab() == active_tab {
                let menu = dropdown_menu(id, settings, &dwrite.font_name);
                c.render_dropdown(&menu);
            }
        }

        c.regions
    }
}

pub fn dump_settings_previews(out_dir: &std::path::Path) -> windows::core::Result<()> {
    use windows::core::Interface;
    std::fs::create_dir_all(out_dir).map_err(|_| windows::core::Error::from_win32())?;
    let d3d = crate::graphics::d3d::D3DContext::new(windows::Win32::Foundation::HWND::default(), super::window::CLIENT_W as u32, super::window::CLIENT_H as u32)?;
    let dxgi_dev: windows::Win32::Graphics::Dxgi::IDXGIDevice = d3d.device.cast()?;
    let mut d2d = crate::graphics::d2d::D2DContext::new(&dxgi_dev)?;
    let surface = d3d.get_surface()?;
    d2d.set_target_surface(&surface)?;
    let dwrite = DWriteContext::new()?;
    let brush = d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 })?;
    let mut settings = AppSettings::load();
    settings.appearance.theme = ThemeMode::Light;

    for tab in 0..12 {
        d2d.begin_draw();
        let target: &ID2D1RenderTarget = &d2d.context;
        let _ = SettingsRenderer::render(
            target,
            &brush,
            &dwrite,
            &settings,
            tab,
            (-1000.0, -1000.0),
            true,
            tab as f32,
            None,
            None,
            true,
            1036.0,
            735.0,
            "",
            false,
            (false, false),
            None,
            None,
        );
        let _ = d2d.end_draw();
        let raw_path = out_dir.join(format!("settings-tab-{:02}.raw", tab));
        d3d.dump_backbuffer_raw(&raw_path.to_string_lossy())?;
    }

    // Extra frames with an open dropdown to eyeball the popup rendering.
    let dropdowns = [
        (0, DropdownId::NotchPreset),
        (2, DropdownId::FontFamily),
        (3, DropdownId::FocusWork),
        (4, DropdownId::AiProvider),
    ];
    for (tab, id) in dropdowns {
        d2d.begin_draw();
        let target: &ID2D1RenderTarget = &d2d.context;
        let _ = SettingsRenderer::render(
            target,
            &brush,
            &dwrite,
            &settings,
            tab,
            (-1000.0, -1000.0),
            true,
            tab as f32,
            None,
            Some(id),
            true,
            1036.0,
            735.0,
            "",
            false,
            (false, false),
            None,
            None,
        );
        let _ = d2d.end_draw();
        let raw_path = out_dir.join(format!("settings-dropdown-{:02}.raw", tab));
        d3d.dump_backbuffer_raw(&raw_path.to_string_lossy())?;
    }
    settings.appearance.theme = ThemeMode::Dark;
    for tab in 0..12 {
        d2d.begin_draw();
        let target: &ID2D1RenderTarget = &d2d.context;
        let _ = SettingsRenderer::render(target, &brush, &dwrite, &settings, tab,
            (-1000.0, -1000.0), true, tab as f32, None, None, true,
            super::window::CLIENT_W, super::window::CLIENT_H, "", false, (false,false), None,None);
        d2d.end_draw()?;
        d3d.dump_backbuffer_raw(&out_dir.join(format!("settings-dark-tab-{tab:02}.raw")).to_string_lossy())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_remains_readable_on_both_themes() {
        let luminance = |c: D2D1_COLOR_F| {
            let channel = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
        };
        for dark in [false, true] {
            for foreground in [TEXT_ITEM, TEXT_DESC, LINK_BLUE, SUCCESS] {
                for surface in [COLOR_BG, CARD_BG, SIDEBAR_BG] {
                    let a = luminance(foreground.resolve(dark));
                    let b = luminance(surface.resolve(dark));
                    assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5);
                }
            }
        }
    }

    #[test]
    fn test_dump_settings_previews() {
        let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED) };
        let dir = std::path::Path::new("target/settings-preview-test");
        let res = dump_settings_previews(dir);
        if let Err(ref e) = res {
            eprintln!("dump_settings_previews failed: {:?}", e);
        }
        assert!(res.is_ok());
    }
}
