use std::sync::atomic::{AtomicIsize, Ordering};
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_IGNORE, D2D1_PIXEL_FORMAT, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory1, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
    D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, ScreenToClient, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, GetAsyncKeyState, VK_ESCAPE, VK_TAB, VK_SHIFT, VK_RETURN, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, BringWindowToTop, CreateWindowExW, DefWindowProcW, GetSystemMetrics,
    GetWindowLongPtrW, IsWindow, LoadCursorW, PostMessageW, RegisterClassExW,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA,
    HTCAPTION, HTCLIENT, IDC_ARROW, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_RESTORE, SW_SHOW,
    WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT,
    WM_USER, WM_KEYDOWN, WM_TIMER, SetTimer, KillTimer, WNDCLASSEXW,
    WS_EX_APPWINDOW, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
};

use crate::config::AppSettings;
use crate::graphics::dwrite::DWriteContext;
use crate::settings::ui::{dropdown_menu, ClickRegion, DropdownId, SettingsAction, SettingsRenderer};

pub const WM_SETTINGS_CHANGED: u32 = WM_USER + 102;
pub const WM_TRIGGER_WATER_REMINDER: u32 = WM_USER + 103;
const WM_MOUSELEAVE: u32 = 0x02A3;
static SETTINGS_HWND: AtomicIsize = AtomicIsize::new(0);

pub const CLIENT_W: f32 = 960.0;
pub const CLIENT_H: f32 = 735.0;

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
    pub focus_index: Option<usize>,
    pub save_ok: bool,
    pub open_dropdown: Option<DropdownId>,
    pub stock_market_idx: bool,
    stock_edit: HWND,
    stock_input:super::stock_input::StockInput,
    theme_dark: bool,
    nav_pos: f32,
    toggle_anim: Option<(SettingsAction, f32)>,
    dragging_opacity: bool,
    search_query: String,
    search_focused: bool,
    history: Vec<usize>,
    history_index: usize,
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
                        let empty: Vec<u16> = "\0".encode_utf16().collect();
                        let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowTextW((*ptr).stock_edit, PCWSTR(empty.as_ptr()));
                        (*ptr).settings.general.autostart = super::autostart::enabled();
                        (*ptr).refresh_theme();
                        (*ptr).notch_hwnd = notch_hwnd;
                        (*ptr).open_dropdown = None;
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
        let title: Vec<u16> = "Settings\0".encode_utf16().collect();

        unsafe {
            let hinstance = GetModuleHandleW(None)?;
            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(Self::wndproc),
                hInstance: hinstance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hIcon: crate::graphics::branding::app_icon(32)?,
                hIconSm: crate::graphics::branding::app_icon(16)?,
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&wnd_class);

            // Borderless popup: the native title bar is hidden and the app draws its own chrome.
            let style = WS_POPUP | WS_SYSMENU | WS_MINIMIZEBOX;
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
            let corner = windows::Win32::Graphics::Dwm::DWMWCP_ROUND;
            let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
                hwnd,
                windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner as *const _ as *const std::ffi::c_void,
                std::mem::size_of_val(&corner) as u32,
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
            let brush = render_target.CreateSolidColorBrush(&super::theme::background(false), None)?;
            let dwrite = DWriteContext::new()?;
            let mut settings = AppSettings::load();
            settings.general.autostart = super::autostart::enabled();

            let stock_input=super::stock_input::StockInput::new(hwnd,&dwrite.font_name)?;
            let stock_edit=stock_input.hwnd;

            // Default on General Tab (Tab 0) -> hide stock input edit box
            let _ = ShowWindow(stock_edit, SW_HIDE);

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
                focus_index: None,
                save_ok: true,
                open_dropdown: None,
                stock_market_idx: true,
                stock_edit,
                stock_input,
                theme_dark: false,
                nav_pos: 0.0,
                toggle_anim: None,
                dragging_opacity: false,
                search_query: String::new(),
                search_focused: false,
                history: vec![0],
                history_index: 0,
            });

            let raw_ptr = Box::into_raw(window_state);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
            (*raw_ptr).refresh_theme();
            SETTINGS_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
            crate::config::debug_log("SettingsWindow state initialized, calling ShowWindow");

            let _ = ShowWindow(hwnd, SW_SHOW);
            if std::env::args().any(|arg| arg == "--preview-settings") { let _ = ShowWindow(hwnd, SW_RESTORE); }
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);

            Ok(hwnd)
        }
    }

    fn refresh_theme(&mut self) {
        let dark = super::theme::is_dark(&self.settings);
        unsafe {
            self.theme_dark=dark;
            self.stock_input.update(dark,&self.dwrite.font_name);
            let value = i32::from(dark);
            let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(self.hwnd,
                windows::Win32::Graphics::Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE,
                &value as *const _ as *const std::ffi::c_void, std::mem::size_of_val(&value) as u32);
            let _ = InvalidateRect(self.hwnd, None, false);
            let _ = InvalidateRect(self.stock_edit, None, true);
        }
    }

    fn notify_notch(&self) {
        unsafe {
            if IsWindow(self.notch_hwnd).as_bool() {
                let _ = PostMessageW(self.notch_hwnd, WM_SETTINGS_CHANGED, WPARAM(0), LPARAM(0));
            }
        }
    }

    fn show_message(&self, title: &str, message: &str) {
        let message: Vec<u16> = format!("{}\0", message).encode_utf16().collect();
        let title: Vec<u16> = format!("{}\0", title).encode_utf16().collect();
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                self.hwnd,
                PCWSTR(message.as_ptr()),
                PCWSTR(title.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
            );
        }
    }

    fn activate_tab(&mut self, tab: usize, record: bool) {
        if record && tab != self.active_tab {
            self.history.truncate(self.history_index + 1);
            self.history.push(tab);
            self.history_index = self.history.len() - 1;
        }
        self.active_tab = tab;
        self.open_dropdown = None;
        self.focus_index = None;
        unsafe {
            if tab!=6 && self.stock_input.focused() {
                let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.hwnd);
            }
            let _ = ShowWindow(self.stock_edit, if tab == 6 { SW_SHOW } else { SW_HIDE });
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }

    fn handle_action(&mut self, action: SettingsAction) {
        if action==SettingsAction::FocusStockInput {
            self.search_focused=false;self.open_dropdown=None;
            unsafe{let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.stock_edit);}
            return;
        }
        self.search_focused = action == SettingsAction::FocusSearch;
        if self.search_focused {
            unsafe{let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.hwnd);}
            self.open_dropdown = None;
            unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            return;
        }
        if let SettingsAction::Navigate(delta) = action {
            let next = self.history_index as i32 + delta;
            if next >= 0 && (next as usize) < self.history.len() {
                self.history_index = next as usize;
                self.activate_tab(self.history[self.history_index], false);
            }
            return;
        }
        if action == SettingsAction::MinimizeWindow {
            unsafe { let _ = ShowWindow(self.hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE); }
            return;
        }
        if let SettingsAction::ToggleDropdown(id) = action {
            self.open_dropdown = if self.open_dropdown == Some(id) { None } else { Some(id) };
            unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            return;
        }
        if action == SettingsAction::CloseWindow {
            self.open_dropdown = None;
            unsafe {
                let _ = PostMessageW(self.hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            return;
        }
        if action == SettingsAction::OpenSectionsTab {
            self.activate_tab(1, true);
            return;
        }
        if action == SettingsAction::ToggleAutostart {
            let target = !self.settings.general.autostart;
            let _ = super::autostart::set(target);
            self.settings.general.autostart = target;
            self.save_ok = self.settings.save();
            if self.save_ok { self.notify_notch(); }
            unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            return;
        }
        if action == SettingsAction::ToggleMinimalNotch {
            self.settings.general.minimal_notch = !self.settings.general.minimal_notch;
            self.save_ok = self.settings.save();
            if self.save_ok { self.notify_notch(); }
            unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            return;
        }
        if action == SettingsAction::ToggleMascot {
            self.toggle_anim = Some((action, 0.0));
            unsafe { SetTimer(self.hwnd, 2, 16, None); }
            self.settings.general.mascot_enabled = !self.settings.general.mascot_enabled;
            self.save_ok = self.settings.save();
            if self.save_ok { self.notify_notch(); }
            unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            return;
        }
        if action == SettingsAction::ToggleAutoMinimizeOnMaximize {
            self.settings.general.auto_minimize_on_maximize = !self.settings.general.auto_minimize_on_maximize;
            self.save_ok = self.settings.save();
            if self.save_ok { self.notify_notch(); }
            unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            return;
        }
        if action == SettingsAction::TestWaterReminder {
            unsafe {
                if IsWindow(self.notch_hwnd).as_bool() {
                    let _ = PostMessageW(self.notch_hwnd, WM_TRIGGER_WATER_REMINDER, WPARAM(0), LPARAM(0));
                }
            }
            return;
        }
        if let SettingsAction::SelectTab(tab) = action {
            self.activate_tab(tab, true);
            return;
        }
        if let SettingsAction::SetStockMarket(idx_market) = action {
            self.stock_market_idx = idx_market;
            self.stock_input.set_market(idx_market);
            unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            return;
        }
        if action == SettingsAction::AddStock {
            let mut buffer = [0u16; 256];
            let count = unsafe {
                windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    self.stock_edit,
                    windows::Win32::UI::WindowsAndMessaging::WM_GETTEXT,
                    WPARAM(buffer.len()),
                    LPARAM(buffer.as_mut_ptr() as isize),
                )
                .0 as usize
            };
            let text = String::from_utf16_lossy(&buffer[..count]);
            let text = text.trim();
            if text.is_empty() {
                unsafe { let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.stock_edit); }
                return;
            }

            let parsed = match crate::sections::saham::normalize_symbols(text, self.stock_market_idx) {
                Ok(symbols) => symbols,
                Err(message) => {
                    self.show_message("Stock Ticker", message);
                    unsafe { let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.stock_edit); }
                    return;
                }
            };

            let mut merged = self.settings.stock_symbols.clone();
            for symbol in parsed {
                if !merged.contains(&symbol) {
                    merged.push(symbol);
                }
            }

            if merged.len() > 6 {
                self.show_message("Watchlist Full", "Your watchlist can hold up to 6 stocks. Remove one first.");
                unsafe { let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.stock_edit); }
                return;
            }

            if merged == self.settings.stock_symbols {
                self.show_message("Already Added", "All entered tickers are already in your watchlist.");
                unsafe { let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.stock_edit); }
                return;
            }

            self.settings.stock_symbols = merged;
            self.save_ok = self.settings.save();
            if self.save_ok { self.notify_notch(); }

            let empty: Vec<u16> = "\0".encode_utf16().collect();
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowTextW(self.stock_edit, PCWSTR(empty.as_ptr()));
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.stock_edit);
                let _ = InvalidateRect(self.hwnd, None, false);
            }
            return;
        }
        if let SettingsAction::RemoveStock(index) = action {
            if index < self.settings.stock_symbols.len() {
                self.settings.stock_symbols.remove(index);
                self.save_ok = self.settings.save();
                if self.save_ok { self.notify_notch(); }
                unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
            }
            return;
        }
        if action == SettingsAction::ResetToDefaults {
            let message: Vec<u16> = "Restore all settings to their defaults?\0".encode_utf16().collect();
            let title: Vec<u16> = "Reset Settings\0".encode_utf16().collect();
            let answer = unsafe {
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    self.hwnd, PCWSTR(message.as_ptr()), PCWSTR(title.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::MB_YESNO | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
                )
            };
            if answer != windows::Win32::UI::WindowsAndMessaging::IDYES { return; }
        }
        if matches!(action, SettingsAction::ToggleAutostart | SettingsAction::ToggleMascot |
            SettingsAction::ToggleMinimalNotch | SettingsAction::ToggleAutoMinimizeOnMaximize |
            SettingsAction::ToggleSectionMusic | SettingsAction::ToggleSectionFocus |
            SettingsAction::ToggleSectionWater | SettingsAction::ToggleSectionShelf |
            SettingsAction::ToggleSectionTodo | SettingsAction::ToggleSectionOpenMaus |
            SettingsAction::ToggleOpenMausNotifications |
            SettingsAction::ToggleSectionAiUsage | SettingsAction::ToggleSectionSaham |
            SettingsAction::ToggleAiAntigravity | SettingsAction::ToggleAiClaude |
            SettingsAction::ToggleAiCodex | SettingsAction::ToggleAiOpenCode |
            SettingsAction::ToggleAiProportionBar | SettingsAction::ToggleMusicGlow |
            SettingsAction::ToggleDynamicLyricsWidth | SettingsAction::ToggleDiscordRpc) {
            self.toggle_anim = Some((action, 0.0));
            unsafe { SetTimer(self.hwnd, 2, 16, None); }
        }
        if action == SettingsAction::ToggleAutostart || action == SettingsAction::ResetToDefaults {
            let target = if action == SettingsAction::ResetToDefaults { AppSettings::default().general.autostart } else { !self.settings.general.autostart };
            if super::autostart::set(target).is_err() {
                self.save_ok = false;
                unsafe { let _ = InvalidateRect(self.hwnd, None, false); }
                return;
            }
        }
        super::actions::apply(&mut self.settings, action);
        if matches!(action, SettingsAction::SetTheme(_) | SettingsAction::ResetToDefaults) { self.refresh_theme(); }
        self.open_dropdown = None;

        if self.dwrite.font_name != self.settings.appearance.font_family {
            if let Ok(new_dw) = DWriteContext::with_font_name(Some(&self.settings.appearance.font_family)) {
                self.dwrite = new_dw;
            }
        }

        // Save immediately and notify notch live
        self.stock_input.update(self.theme_dark,&self.dwrite.font_name);
        self.save_ok = self.settings.save();
        if self.save_ok { self.notify_notch(); }

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
            super::stock_input::WM_STOCK_INPUT if !ptr.is_null()=>{
                let s=&mut *ptr;
                match wparam.0 {
                    1 if s.active_tab==6=>s.handle_action(SettingsAction::AddStock),
                    2|3=>{
                        let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(hwnd);
                        let field=s.click_regions.iter().position(|r|r.action==SettingsAction::FocusStockInput);
                        s.focus_index=if wparam.0==2 {s.click_regions.iter().position(|r|matches!(r.action,SettingsAction::SetStockMarket(_)))}else{field.and_then(|i|i.checked_sub(1))};
                    },
                    5=>{let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(hwnd);s.focus_index=None;},
                    _=>{},
                }
                let _=InvalidateRect(hwnd,None,false);return LRESULT(0);
            }
            windows::Win32::UI::WindowsAndMessaging::WM_COMMAND if !ptr.is_null() && lparam.0==(*ptr).stock_edit.0 as isize=>{
                let _=InvalidateRect(hwnd,None,false);return LRESULT(0);
            }
            windows::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE | windows::Win32::UI::WindowsAndMessaging::WM_THEMECHANGED if !ptr.is_null() => {
                (*ptr).refresh_theme();
                return LRESULT(0);
            }
            windows::Win32::UI::WindowsAndMessaging::WM_CTLCOLOREDIT if !ptr.is_null() && lparam.0 == (*ptr).stock_edit.0 as isize => {
                return (*ptr).stock_input.color(windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut _));
            }
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
                        s.save_ok,
                        s.nav_pos,
                        s.toggle_anim,
                        s.open_dropdown,
                        s.stock_market_idx,
                        CLIENT_W,
                        CLIENT_H,
                        &s.search_query,
                        s.search_focused,
                        (s.history_index > 0, s.history_index + 1 < s.history.len()),
                        s.focus_index.and_then(|i| s.click_regions.get(i)).map(|r| r.action),
                        Some(s.stock_input.focused()),
                    );
                    if let Some(region) = s.focus_index.and_then(|i| s.click_regions.get(i)) {
                        s.brush.SetColor(&windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F { r: 0.24, g: 0.52, b: 0.96, a: 1.0 });
                        s.render_target.DrawRoundedRectangle(&windows::Win32::Graphics::Direct2D::D2D1_ROUNDED_RECT {
                            rect: region.rect, radiusX: 6.0, radiusY: 6.0,
                        }, &s.brush, 1.5, None);
                    }
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
                    if s.dragging_opacity && wparam.0 & 1 != 0 {
                        s.handle_action(SettingsAction::SetAcrylicOpacity(super::ui::opacity_at(x)));
                    }

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
            WM_TIMER if wparam.0 == 2 => {
                if !ptr.is_null() {
                    let s = &mut *ptr;
                    let target = s.active_tab as f32;
                    s.nav_pos += (target - s.nav_pos) * 0.22;
                    if (target - s.nav_pos).abs() < 0.005 { s.nav_pos = target; }
                    if let Some((action, progress)) = s.toggle_anim {
                        let next = (progress + 0.10).min(1.0);
                        s.toggle_anim = if next >= 1.0 { None } else { Some((action, next)) };
                    }
                    if s.nav_pos == target && s.toggle_anim.is_none() { let _ = KillTimer(hwnd, 2); }
                    let _ = InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                if !ptr.is_null() {
                    let s = &mut *ptr;
                    if s.search_focused {
                        if wparam.0 == 0x56 && GetAsyncKeyState(0x11) < 0 {
                            let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(hwnd, windows::Win32::UI::WindowsAndMessaging::WM_PASTE, WPARAM(0), LPARAM(0));
                            return LRESULT(0);
                        }
                        if wparam.0 == VK_ESCAPE.0 as usize {
                            s.search_query.clear();
                            s.search_focused = false;
                            let _ = InvalidateRect(hwnd, None, false);
                            return LRESULT(0);
                        }
                        if wparam.0 != VK_TAB.0 as usize { return LRESULT(0); }
                        s.search_focused = false;
                    }
                    if let Some(id) = s.open_dropdown {
                        if wparam.0 == 0x26 || wparam.0 == 0x28 {
                            let menu = dropdown_menu(id, &s.settings, &s.dwrite.font_name);
                            let start = s.click_regions.len().saturating_sub(menu.options.len());
                            let current = s.focus_index.filter(|i| *i >= start).map(|i| i - start).unwrap_or(menu.selected);
                            let next = if wparam.0 == 0x26 { (current + menu.options.len() - 1) % menu.options.len() } else { (current + 1) % menu.options.len() };
                            let row = menu.item_rect(next);
                            s.focus_index = Some(start + next);
                            s.mouse_pos = ((row.left + row.right) * 0.5, (row.top + row.bottom) * 0.5);
                            let _ = InvalidateRect(hwnd, None, false);
                            return LRESULT(0);
                        }
                    }
                    if wparam.0 == 0x25 || wparam.0 == 0x27 {
                        if s.focus_index.and_then(|i| s.click_regions.get(i)).map_or(false, |r| matches!(r.action, SettingsAction::SetAcrylicOpacity(_))) {
                            s.handle_action(SettingsAction::AdjustAcrylicOpacity(if wparam.0 == 0x25 { -0.01 } else { 0.01 }));
                            return LRESULT(0);
                        }
                    }
                    if wparam.0 == VK_ESCAPE.0 as usize && s.open_dropdown.is_some() {
                        s.open_dropdown = None;
                        let _ = InvalidateRect(hwnd, None, false);
                        return LRESULT(0);
                    }
                    if wparam.0 == VK_TAB.0 as usize && !s.click_regions.is_empty() {
                        let count = s.click_regions.len();
                        let reverse = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
                        s.focus_index = Some(match s.focus_index {
                            Some(i) if reverse => (i + count - 1) % count,
                            Some(i) => (i + 1) % count,
                            None if reverse => count - 1,
                            None => 0,
                        });
                        if s.focus_index.and_then(|i|s.click_regions.get(i)).map_or(false,|r|r.action==SettingsAction::FocusStockInput) {
                            let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(s.stock_edit);
                        }
                        let _ = InvalidateRect(hwnd, None, false);
                        return LRESULT(0);
                    }
                    if wparam.0 == VK_RETURN.0 as usize || wparam.0 == VK_SPACE.0 as usize {
                        if let Some(id) = s.open_dropdown {
                            let menu = dropdown_menu(id, &s.settings, &s.dwrite.font_name);
                            let start = s.click_regions.len().saturating_sub(menu.options.len());
                            let index = s.focus_index.filter(|i| *i >= start).map(|i| i - start).unwrap_or(menu.selected).min(menu.options.len() - 1);
                            s.handle_action(menu.options[index].action);
                            return LRESULT(0);
                        }
                        if let Some(region) = s.focus_index.and_then(|i| s.click_regions.get(i)).copied() {
                            s.handle_action(region.action);
                            return LRESULT(0);
                        }
                    }
                }
            }
            windows::Win32::UI::WindowsAndMessaging::WM_PASTE => {
                if !ptr.is_null() && (*ptr).search_focused {
                    use windows::Win32::System::{DataExchange::*, Memory::*};
                    if OpenClipboard(hwnd).is_ok() {
                        if let Ok(handle) = GetClipboardData(13) {
                            let memory = windows::Win32::Foundation::HGLOBAL(handle.0);
                            let data = GlobalLock(memory) as *const u16;
                            if !data.is_null() {
                                let units = std::slice::from_raw_parts(data, GlobalSize(memory) / 2);
                                let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
                                let pasted = String::from_utf16_lossy(&units[..end]);
                                let s = &mut *ptr;
                                let remaining = 40usize.saturating_sub(s.search_query.chars().count());
                                s.search_query.extend(pasted.chars().filter(|c| !c.is_control()).take(remaining));
                                let _ = GlobalUnlock(memory);
                            }
                        }
                        let _ = CloseClipboard();
                    }
                    let _ = InvalidateRect(hwnd, None, false);
                    return LRESULT(0);
                }
            }
            windows::Win32::UI::WindowsAndMessaging::WM_CHAR => {
                if !ptr.is_null() && (*ptr).search_focused {
                    let s = &mut *ptr;
                    if wparam.0 == 8 { s.search_query.pop(); }
                    else if let Some(ch) = char::from_u32(wparam.0 as u32) {
                        if !ch.is_control() && s.search_query.chars().count() < 40 { s.search_query.push(ch); }
                    }
                    s.focus_index = None;
                    let _ = InvalidateRect(hwnd, None, false);
                    return LRESULT(0);
                }
            }
            WM_LBUTTONDOWN => {
                if !ptr.is_null() {
                    let s = &mut *ptr;
                    s.focus_index = None;
                    s.search_focused = false;
                    let x = (lparam.0 & 0xFFFF) as i16 as f32;
                    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

                    let slider = super::ui::OPACITY_SLIDER;
                    if s.active_tab == 2 && s.open_dropdown.is_none() && x >= slider.left && x <= slider.right && y >= slider.top && y <= slider.bottom {
                        s.dragging_opacity = true;
                        windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
                        s.handle_action(SettingsAction::SetAcrylicOpacity(super::ui::opacity_at(x)));
                        return LRESULT(0);
                    }
                    // An open dropdown captures the click: select an option, close, or
                    // hand over to another dropdown button in a single click.
                    if let Some(id) = s.open_dropdown {
                        let anchor = id.anchor();
                        let anchor_hit = x >= anchor.left && x <= anchor.right && y >= anchor.top && y <= anchor.bottom;
                        if anchor_hit {
                            s.open_dropdown = None;
                            let _ = InvalidateRect(hwnd, None, false);
                            return LRESULT(0);
                        }

                        let menu = dropdown_menu(id, &s.settings, &s.dwrite.font_name);
                        let picked = menu.option_at(x, y).map(|idx| menu.options[idx].action);
                        if let Some(action) = picked {
                            s.open_dropdown = None;
                            s.handle_action(action);
                            return LRESULT(0);
                        }
                        if x >= menu.rect.left && x <= menu.rect.right && y >= menu.rect.top && y <= menu.rect.bottom {
                            s.open_dropdown = None;
                            let _ = InvalidateRect(hwnd, None, false);
                            return LRESULT(0);
                        }

                        s.open_dropdown = None;
                        let mut matched_action = None;
                        for r in &s.click_regions {
                            // Ignore the control under the menu itself; those rows were
                            // already tested above and must not re-open the dropdown.
                            let over_menu = x >= menu.rect.left && x <= menu.rect.right
                                && y >= menu.rect.top && y <= menu.rect.bottom;
                            if !over_menu && x >= r.rect.left && x <= r.rect.right && y >= r.rect.top && y <= r.rect.bottom {
                                matched_action = Some(r.action);
                                break;
                            }
                        }
                        if let Some(action) = matched_action {
                            s.handle_action(action);
                        } else {
                            let _ = InvalidateRect(hwnd, None, false);
                        }
                        return LRESULT(0);
                    }

                    let mut matched_action = None;
                    // The current page is rendered after the sidebar, so prefer its
                    // controls when a control rect happens to overlap navigation.
                    for r in s.click_regions.iter().rev() {
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
            windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP => {
                if !ptr.is_null() && (*ptr).dragging_opacity {
                    (*ptr).dragging_opacity = false;
                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                }
                return LRESULT(0);
            }
            WM_NCHITTEST => {
                // Borderless window: let the empty header strip act as a drag handle.
                let mut pt = POINT {
                    x: (lparam.0 & 0xFFFF) as i16 as i32,
                    y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
                };
                let _ = ScreenToClient(hwnd, &mut pt);
                let x = pt.x as f32;
                let y = pt.y as f32;
                let over_navigation = x >= 300.0 && x <= 380.0 && y >= 37.0;
                if y >= 0.0 && y < 64.0 && x >= 100.0 && !over_navigation {
                    return LRESULT(HTCAPTION as isize);
                }
                return LRESULT(HTCLIENT as isize);
            }
            WM_CLOSE => {
                if std::env::args().any(|arg| arg == "--preview-settings") {
                    let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                    windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
                    return LRESULT(0);
                }
                crate::config::debug_log("SettingsWindow WM_CLOSE triggered");
                let _ = ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            }
            WM_DESTROY => {
                crate::config::debug_log("SettingsWindow WM_DESTROY triggered");
                SETTINGS_HWND.store(0, Ordering::SeqCst);
                if !ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    let _ = Box::from_raw(ptr);
                }
                return LRESULT(0);
            }
            _ => {}
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
