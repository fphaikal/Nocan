use std::sync::atomic::{AtomicIsize, Ordering};
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_IGNORE, D2D1_PIXEL_FORMAT, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory1, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
    D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, BringWindowToTop, CreateWindowExW, DefWindowProcW, GetSystemMetrics,
    GetWindowLongPtrW, IsWindow, LoadCursorW, LoadIconW, PostMessageW, RegisterClassExW,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA,
    IDC_ARROW, IDI_APPLICATION, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_RESTORE, SW_SHOW, WM_CLOSE,
    WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_PAINT, WM_USER, WNDCLASSEXW,
    WS_CAPTION, WS_EX_APPWINDOW, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

use crate::config::AppSettings;
use crate::graphics::dwrite::DWriteContext;
use crate::settings::ui::{ClickRegion, SettingsAction, SettingsRenderer, COLOR_BG};

pub const WM_SETTINGS_CHANGED: u32 = WM_USER + 102;
const WM_MOUSELEAVE: u32 = 0x02A3;
static SETTINGS_HWND: AtomicIsize = AtomicIsize::new(0);

const CLIENT_W: f32 = 780.0;
const CLIENT_H: f32 = 560.0;

pub struct SettingsWindow {
    pub hwnd: HWND,
    pub notch_hwnd: HWND,
    pub render_target: ID2D1HwndRenderTarget,
    pub brush: ID2D1SolidColorBrush,
    pub dwrite: DWriteContext,
    pub settings: AppSettings,
    pub active_tab: usize,
    pub mouse_pos: (f32, f32),
    pub click_regions: Vec<ClickRegion>,
    pub mouse_tracking: bool,
}

impl SettingsWindow {
    pub fn show_or_create(notch_hwnd: HWND) -> Result<HWND> {
        crate::config::debug_log("SettingsWindow::show_or_create called");
        let existing = SETTINGS_HWND.load(Ordering::SeqCst);
        if existing != 0 {
            let hwnd = HWND(existing as *mut _);
            unsafe {
                if IsWindow(hwnd).as_bool() {
                    crate::config::debug_log("SettingsWindow exists, showing existing");
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self;
                    if !ptr.is_null() {
                        (*ptr).settings = AppSettings::load();
                        (*ptr).notch_hwnd = notch_hwnd;
                        let _ = InvalidateRect(hwnd, None, false);
                    }
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = BringWindowToTop(hwnd);
                    let _ = SetForegroundWindow(hwnd);
                    return Ok(hwnd);
                }
            }
        }

        Self::create(notch_hwnd)
    }

    fn create(notch_hwnd: HWND) -> Result<HWND> {
        crate::config::debug_log("SettingsWindow::create starting");
        let class_name: Vec<u16> = "NocanSettingsWindowClass\0".encode_utf16().collect();
        let title: Vec<u16> = "Pengaturan Nocan Living Notch\0".encode_utf16().collect();

        unsafe {
            let hinstance = GetModuleHandleW(None)?;
            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(Self::wndproc),
                hInstance: hinstance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&wnd_class);

            let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: CLIENT_W as i32,
                bottom: CLIENT_H as i32,
            };
            let _ = AdjustWindowRectEx(&mut rect, style, false, WS_EX_APPWINDOW);
            let win_w = rect.right - rect.left;
            let win_h = rect.bottom - rect.top;

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let pos_x = (screen_w - win_w) / 2;
            let pos_y = (screen_h - win_h) / 2;

            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
                pos_x,
                pos_y,
                win_w,
                win_h,
                None,
                None,
                hinstance,
                None,
            )?;
            crate::config::debug_log(&format!("SettingsWindow HWND created: {:?}", hwnd));

            // Windows 11 Fluent Dark Title Bar
            let dark_mode = BOOL::from(true);
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const _ as *const std::ffi::c_void,
                std::mem::size_of_val(&dark_mode) as u32,
            );

            // D2D Factory & Render Target
            let factory_options = D2D1_FACTORY_OPTIONS::default();
            let factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&factory_options))?;

            let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };

            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: D2D_SIZE_U {
                    width: CLIENT_W as u32,
                    height: CLIENT_H as u32,
                },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };

            let render_target = factory.CreateHwndRenderTarget(&rt_props, &hwnd_props)?;
            let brush = render_target.CreateSolidColorBrush(&COLOR_BG, None)?;
            let dwrite = DWriteContext::new()?;
            let settings = AppSettings::load();

            let window_state = Box::new(Self {
                hwnd,
                notch_hwnd,
                render_target,
                brush,
                dwrite,
                settings,
                active_tab: 0,
                mouse_pos: (-1000.0, -1000.0),
                click_regions: Vec::new(),
                mouse_tracking: false,
            });

            let raw_ptr = Box::into_raw(window_state);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
            SETTINGS_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
            crate::config::debug_log("SettingsWindow state initialized, calling ShowWindow");

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);

            Ok(hwnd)
        }
    }

    fn notify_notch(&self) {
        unsafe {
            if IsWindow(self.notch_hwnd).as_bool() {
                let _ = PostMessageW(self.notch_hwnd, WM_SETTINGS_CHANGED, WPARAM(0), LPARAM(0));
            }
        }
    }

    fn handle_action(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::SelectTab(tab) => {
                self.active_tab = tab;
            }
            SettingsAction::ToggleAutostart => {
                self.settings.general.autostart = !self.settings.general.autostart;
            }
            SettingsAction::ToggleMascot => {
                self.settings.general.mascot_enabled = !self.settings.general.mascot_enabled;
            }
            SettingsAction::ToggleMinimalNotch => {
                self.settings.general.minimal_notch = !self.settings.general.minimal_notch;
            }
            SettingsAction::ToggleAutoMinimizeOnMaximize => {
                self.settings.general.auto_minimize_on_maximize =
                    !self.settings.general.auto_minimize_on_maximize;
            }
            SettingsAction::ToggleSectionMusic => {
                self.settings.sections.show_music = !self.settings.sections.show_music;
            }
            SettingsAction::ToggleSectionFocus => {
                self.settings.sections.show_focus = !self.settings.sections.show_focus;
            }
            SettingsAction::ToggleSectionWater => {
                self.settings.sections.show_water = !self.settings.sections.show_water;
            }
            SettingsAction::ToggleSectionShelf => {
                self.settings.sections.show_shelf = !self.settings.sections.show_shelf;
            }
            SettingsAction::ToggleSectionTodo => {
                self.settings.sections.show_todo = !self.settings.sections.show_todo;
            }
            SettingsAction::ToggleSectionOpenMaus => {
                self.settings.sections.show_openmaus = !self.settings.sections.show_openmaus;
            }
            SettingsAction::ToggleSectionAiUsage => {
                self.settings.sections.show_ai_usage = !self.settings.sections.show_ai_usage;
            }
            SettingsAction::ToggleSectionSaham => {
                self.settings.sections.show_saham = !self.settings.sections.show_saham;
            }
            SettingsAction::ToggleAiAntigravity => {
                self.settings.ai.show_antigravity = !self.settings.ai.show_antigravity;
            }
            SettingsAction::ToggleAiClaude => {
                self.settings.ai.show_claude = !self.settings.ai.show_claude;
            }
            SettingsAction::ToggleAiCodex => {
                self.settings.ai.show_codex = !self.settings.ai.show_codex;
            }
            SettingsAction::ToggleAiOpenCode => {
                self.settings.ai.show_opencode = !self.settings.ai.show_opencode;
            }
            SettingsAction::SetAiDefaultProvider(p) => {
                self.settings.ai.default_provider = p;
            }
            SettingsAction::AdjustAiPoll(delta) => {
                let new_val = (self.settings.ai.poll_interval_secs as i64 + delta as i64)
                    .clamp(1, 60) as u64;
                self.settings.ai.poll_interval_secs = new_val;
            }
            SettingsAction::ToggleAiProportionBar => {
                self.settings.ai.show_proportional_bar = !self.settings.ai.show_proportional_bar;
            }
            SettingsAction::ToggleMusicGlow => {
                self.settings.music.ambient_glow_enabled =
                    !self.settings.music.ambient_glow_enabled;
            }
            SettingsAction::ToggleDynamicLyricsWidth => {
                self.settings.music.dynamic_lyrics_width =
                    !self.settings.music.dynamic_lyrics_width;
            }
            SettingsAction::ToggleDiscordRpc => {
                self.settings.music.discord_rpc_enabled =
                    !self.settings.music.discord_rpc_enabled;
            }
            SettingsAction::AdjustFocusWorkMins(delta) => {
                let new_val = (self.settings.productivity.focus_work_mins as i32 + delta)
                    .clamp(5, 90) as u32;
                self.settings.productivity.focus_work_mins = new_val;
            }
            SettingsAction::AdjustFocusBreakMins(delta) => {
                let new_val = (self.settings.productivity.focus_break_mins as i32 + delta)
                    .clamp(1, 30) as u32;
                self.settings.productivity.focus_break_mins = new_val;
            }
            SettingsAction::AdjustWaterIntervalMins(delta) => {
                let new_val = (self.settings.productivity.water_interval_mins as i32 + delta)
                    .clamp(15, 120) as u32;
                self.settings.productivity.water_interval_mins = new_val;
            }
            SettingsAction::AdjustWaterTargetGlasses(delta) => {
                let new_val = (self.settings.productivity.water_daily_target_glasses as i32 + delta)
                    .clamp(1, 24) as u32;
                self.settings.productivity.water_daily_target_glasses = new_val;
            }
            SettingsAction::SetAccentMode(mode) => {
                self.settings.appearance.accent_mode = mode;
            }
            SettingsAction::AdjustAcrylicOpacity(delta) => {
                let new_val = (self.settings.appearance.acrylic_opacity + delta).clamp(0.5, 1.0);
                self.settings.appearance.acrylic_opacity = (new_val * 100.0).round() / 100.0;
            }
            SettingsAction::ResetToDefaults => {
                self.settings = AppSettings::default();
            }
        }

        // Save immediately and notify notch live
        self.settings.save();
        self.notify_notch();

        unsafe {
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self;

        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let _ = BeginPaint(hwnd, &mut ps);
                if !ptr.is_null() {
                    let s = &mut *ptr;
                    s.render_target.BeginDraw();
                    s.click_regions = SettingsRenderer::render(
                        &s.render_target,
                        &s.brush,
                        &s.dwrite,
                        &s.settings,
                        s.active_tab,
                        s.mouse_pos,
                        CLIENT_W,
                        CLIENT_H,
                    );
                    let _ = s.render_target.EndDraw(None, None);
                }
                let _ = EndPaint(hwnd, &ps);
                return LRESULT(0);
            }
            WM_ERASEBKGND => {
                return LRESULT(1);
            }
            WM_MOUSEMOVE => {
                if !ptr.is_null() {
                    let s = &mut *ptr;
                    let x = (lparam.0 & 0xFFFF) as i16 as f32;
                    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
                    s.mouse_pos = (x, y);

                    if !s.mouse_tracking {
                        let mut tme = TRACKMOUSEEVENT {
                            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        let _ = TrackMouseEvent(&mut tme);
                        s.mouse_tracking = true;
                    }

                    let _ = InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            WM_MOUSELEAVE => {
                if !ptr.is_null() {
                    let s = &mut *ptr;
                    s.mouse_pos = (-1000.0, -1000.0);
                    s.mouse_tracking = false;
                    let _ = InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                if !ptr.is_null() {
                    let s = &mut *ptr;
                    let x = (lparam.0 & 0xFFFF) as i16 as f32;
                    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

                    let mut matched_action = None;
                    for r in &s.click_regions {
                        if x >= r.rect.left && x <= r.rect.right && y >= r.rect.top && y <= r.rect.bottom {
                            matched_action = Some(r.action);
                            break;
                        }
                    }

                    if let Some(action) = matched_action {
                        s.handle_action(action);
                    }
                }
                return LRESULT(0);
            }
            WM_CLOSE => {
                crate::config::debug_log("SettingsWindow WM_CLOSE triggered");
                // Hide instead of destroy so reopening is instantaneous
                let _ = ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            }
            WM_DESTROY => {
                crate::config::debug_log("SettingsWindow WM_DESTROY triggered");
                SETTINGS_HWND.store(0, Ordering::SeqCst);
                if !ptr.is_null() {
                    let _ = Box::from_raw(ptr);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                return LRESULT(0);
            }
            _ => {}
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
