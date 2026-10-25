use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, ShellExecuteW, Shell_NotifyIconW, HDROP,
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostQuitMessage,
    RegisterClassExW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TrackPopupMenu, TranslateMessage, GWLP_USERDATA, HWND_TOPMOST, IDI_APPLICATION,
    MF_SEPARATOR, MF_STRING, MSG, SWP_NOACTIVATE, SW_SHOW, SW_SHOWNOACTIVATE, TPM_BOTTOMALIGN,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_DESTROY, WM_DROPFILES, WM_LBUTTONDOWN,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONUP, WM_TIMER,
    WM_USER, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::graphics::composition::CompositionTarget;
use crate::graphics::d2d::D2DContext;
use crate::graphics::d3d::D3DContext;
use crate::graphics::dwrite::DWriteContext;
use crate::integrations::media::MediaService;
use crate::integrations::system_info::{get_system_status, SystemStatus};
use crate::notch::carousel::Carousel;
use crate::sections::focus::FocusSection;
use crate::sections::music::MusicSection;
use crate::sections::openmaus::OpenMausSection;
use crate::sections::shelf::ShelfSection;
use crate::sections::todo::TodoSection;
use crate::sections::water::WaterSection;

const COLLAPSED_W: f32 = 220.0;
const COLLAPSED_H: f32 = 32.0;
const EXPANDED_W: f32 = 420.0;
const EXPANDED_H: f32 = 140.0;

const WM_TRAYICON: u32 = WM_USER + 101;
const IDM_OPEN: usize = 1001;
const IDM_FOLDER: usize = 1002;
const IDM_EXIT: usize = 1003;

const WM_MOUSELEAVE: u32 = 0x02A3;

pub struct NotchWindow {
    pub hwnd: HWND,
    d3d: D3DContext,
    d2d: D2DContext,
    dwrite: DWriteContext,
    _comp: CompositionTarget,
    carousel: Carousel,
    media_service: MediaService,
    focus_sec: FocusSection,
    water_sec: WaterSection,
    todo_sec: TodoSection,
    openmaus_sec: OpenMausSection,
    shelf_sec: ShelfSection,
    pub is_hovered: bool,
    current_w: f32,
    current_h: f32,
    screen_w: i32,
    screen_x: i32,
    screen_y: i32,
    system_status: SystemStatus,
    last_status_poll: u64,
}

impl NotchWindow {
    pub fn create(media_service: MediaService) -> Result<HWND> {
        let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)?.into() };
        let class_name: Vec<u16> = "NotchWindow\0".encode_utf16().collect();

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: windows::Win32::UI::WindowsAndMessaging::CS_HREDRAW
                | windows::Win32::UI::WindowsAndMessaging::CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: windows::Win32::UI::WindowsAndMessaging::HICON::default(),
            hCursor: windows::Win32::UI::WindowsAndMessaging::HCURSOR::default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: windows::Win32::UI::WindowsAndMessaging::HICON::default(),
        };

        unsafe {
            RegisterClassExW(&wnd_class);
        }

        // Get Primary Monitor info
        let (screen_x, screen_y, screen_w) = get_monitor_bounds();
        let init_x = screen_x + (screen_w - COLLAPSED_W as i32) / 2;
        let init_y = screen_y;

        let ex_style = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP;
        let style = WS_POPUP;

        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(class_name.as_ptr()),
                style,
                init_x,
                init_y,
                COLLAPSED_W as i32,
                COLLAPSED_H as i32,
                None,
                None,
                hinstance,
                None,
            )?
        };

        unsafe {
            DragAcceptFiles(hwnd, true);

            // Add System Tray Icon
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
                ..Default::default()
            };
            let tip: Vec<u16> = "Nocan Living Notch\0".encode_utf16().collect();
            for (i, &c) in tip.iter().enumerate().take(127) {
                nid.szTip[i] = c;
            }
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }

        let d3d = D3DContext::new(hwnd, COLLAPSED_W as u32, COLLAPSED_H as u32)?;
        let mut d2d = D2DContext::new(&d3d.dxgi_device)?;
        let surface = d3d.get_surface()?;
        d2d.set_target_surface(&surface)?;

        let dwrite = DWriteContext::new()?;
        let comp = CompositionTarget::new(hwnd, &d3d.dxgi_device, &d3d.swapchain)?;

        let notch = Box::new(Self {
            hwnd,
            d3d,
            d2d,
            dwrite,
            _comp: comp,
            carousel: Carousel::new(),
            media_service,
            focus_sec: FocusSection::new(),
            water_sec: WaterSection::new(),
            todo_sec: TodoSection::new(),
            openmaus_sec: OpenMausSection::new(),
            shelf_sec: ShelfSection::new(),
            is_hovered: false,
            current_w: COLLAPSED_W,
            current_h: COLLAPSED_H,
            screen_w,
            screen_x,
            screen_y,
            system_status: get_system_status(),
            last_status_poll: 0,
        });

        let raw_ptr = Box::into_raw(notch);
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            windows::Win32::UI::WindowsAndMessaging::SetTimer(hwnd, 1, 16, None); // ~60 FPS
        }

        Ok(hwnd)
    }
}

pub fn run_message_loop() {
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

impl NotchWindow {
    pub fn tick_frame(&mut self) {
        // Animation smoothing
        let target_w = if self.is_hovered { EXPANDED_W } else { COLLAPSED_W };
        let target_h = if self.is_hovered { EXPANDED_H } else { COLLAPSED_H };

        let dw = (target_w - self.current_w) * 0.22;
        let dh = (target_h - self.current_h) * 0.22;

        let old_w = self.current_w as i32;
        let old_h = self.current_h as i32;

        self.current_w += dw;
        self.current_h += dh;

        if (self.current_w - target_w).abs() < 0.5 {
            self.current_w = target_w;
        }
        if (self.current_h - target_h).abs() < 0.5 {
            self.current_h = target_h;
        }

        let new_w = self.current_w as i32;
        let new_h = self.current_h as i32;

        // Position window centered at top of monitor
        let new_x = self.screen_x + (self.screen_w - new_w) / 2;
        let new_y = self.screen_y;

        if new_w != old_w || new_h != old_h {
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    new_x,
                    new_y,
                    new_w,
                    new_h,
                    SWP_NOACTIVATE,
                );
            }
            // Resize swapchain
            self.d2d.reset_target();
            let _ = self.d3d.resize(new_w as u32, new_h as u32);
            if let Ok(surface) = self.d3d.get_surface() {
                let _ = self.d2d.set_target_surface(&surface);
            }
        }

        // Update active sections
        self.focus_sec.update();
        self.openmaus_sec.refresh_if_needed();

        // Poll system status periodically
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now_secs.saturating_sub(self.last_status_poll) >= 2 {
            self.last_status_poll = now_secs;
            self.system_status = get_system_status();
        }

        self.render();
    }

    fn render(&mut self) {
        let w = self.current_w;
        let h = self.current_h;

        self.d2d.begin_draw();
        // Clear transparent surface
        self.d2d.clear(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });

        // Pure Jet Black Background: RGB(0, 0, 0)
        let notch_bg = self.d2d.create_solid_brush(D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }).ok();

        // Typography and Accent Brushes
        let white_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.95 }).ok();
        let dim_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 180.0 / 255.0, g: 180.0 / 255.0, b: 180.0 / 255.0, a: 0.70 }).ok();
        let inactive_dot_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.25 }).ok();
        let chevron_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 120.0 / 255.0, g: 120.0 / 255.0, b: 120.0 / 255.0, a: 0.65 }).ok();
        let green_dot_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 39.0 / 255.0, g: 195.0 / 255.0, b: 74.0 / 255.0, a: 1.0 }).ok();
        let tomato_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 249.0 / 255.0, g: 83.0 / 255.0, b: 83.0 / 255.0, a: 1.0 }).ok();
        let water_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 52.0 / 255.0, g: 152.0 / 255.0, b: 219.0 / 255.0, a: 1.0 }).ok();
        let btn_dark_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 67.0 / 255.0, g: 67.0 / 255.0, b: 67.0 / 255.0, a: 1.0 }).ok();
        let subtle_bg_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.12 }).ok();
        let bot_circle_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 70.0 / 255.0, g: 50.0 / 255.0, b: 120.0 / 255.0, a: 1.0 }).ok();
        let green_badge_bg = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 34.0 / 255.0, g: 61.0 / 255.0, b: 30.0 / 255.0, a: 0.90 }).ok();
        let green_badge_text = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 130.0 / 255.0, g: 240.0 / 255.0, b: 130.0 / 255.0, a: 1.0 }).ok();

        if let (Some(bg), Some(white_b), Some(dim_b), Some(dot_inactive)) = (
            &notch_bg,
            &white_brush,
            &dim_brush,
            &inactive_dot_brush,
        ) {
            // Draw Authentic Pure Black Notch Body
            let notch_rect = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: w,
                bottom: h,
            };

            let corner_r = if self.is_hovered { 32.0 } else { 16.0 };
            self.d2d.fill_rounded_rect(notch_rect, corner_r, corner_r, bg);

            if !self.is_hovered {
                // Collapsed State: Dynamic Island compact pill
                self.draw_collapsed_bar(w, h, white_b, dim_b);
            } else {
                // Expanded State:
                // 1. Top bar: Green indicator dot + Date & Time + Battery
                self.draw_expanded_top_bar(w, dim_b, green_dot_brush.as_ref().unwrap_or(white_b));

                // 2. Render Active Section
                let sec_bounds = D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: w,
                    bottom: h,
                };

                let track = self.media_service.get_info();
                match self.carousel.current_section {
                    0 => {
                        MusicSection::draw(
                            &self.d2d,
                            &self.dwrite,
                            sec_bounds,
                            &track,
                            white_b,
                            dim_b,
                            subtle_bg_brush.as_ref().unwrap_or(dim_b),
                            subtle_bg_brush.as_ref().unwrap_or(dim_b),
                        );
                    }
                    1 => {
                        self.focus_sec.draw(
                            &self.d2d,
                            &self.dwrite,
                            sec_bounds,
                            white_b,
                            dim_b,
                            tomato_brush.as_ref().unwrap_or(white_b),
                            btn_dark_brush.as_ref().unwrap_or(dim_b),
                        );
                    }
                    2 => {
                        self.water_sec.draw(
                            &self.d2d,
                            &self.dwrite,
                            sec_bounds,
                            white_b,
                            dim_b,
                            water_brush.as_ref().unwrap_or(white_b),
                            btn_dark_brush.as_ref().unwrap_or(dim_b),
                        );
                    }
                    3 => {
                        self.todo_sec.draw(
                            &self.d2d,
                            &self.dwrite,
                            sec_bounds,
                            white_b,
                            dim_b,
                            green_dot_brush.as_ref().unwrap_or(white_b),
                            subtle_bg_brush.as_ref().unwrap_or(dim_b),
                        );
                    }
                    4 => {
                        self.openmaus_sec.draw(
                            &self.d2d,
                            &self.dwrite,
                            sec_bounds,
                            white_b,
                            dim_b,
                            bot_circle_brush.as_ref().unwrap_or(white_b),
                            green_badge_bg.as_ref().unwrap_or(dim_b),
                            green_badge_text.as_ref().unwrap_or(white_b),
                        );
                    }
                    5 => {
                        self.shelf_sec.draw(
                            &self.d2d,
                            &self.dwrite,
                            sec_bounds,
                            white_b,
                            dim_b,
                            subtle_bg_brush.as_ref().unwrap_or(dim_b),
                            subtle_bg_brush.as_ref().unwrap_or(dim_b),
                        );
                    }
                    _ => {}
                }

                // 3. Navigation Chevrons ‹ and › on the left and right
                self.carousel.nav.draw(
                    &self.d2d,
                    &self.dwrite,
                    w,
                    h,
                    chevron_brush.as_ref().unwrap_or(dim_b),
                    white_b,
                );

                // 4. Carousel 6-Dots Indicator at the bottom
                self.carousel.dots.draw(&self.d2d, w, h, white_b, dot_inactive);
            }
        }

        let _ = self.d2d.end_draw();
        let _ = self.d3d.present();
    }

    fn draw_collapsed_bar(
        &self,
        w: f32,
        h: f32,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
    ) {
        let center_x = w / 2.0;
        let track = self.media_service.get_info();

        if track.is_playing && !track.title.is_empty() {
            let icon_rect = D2D_RECT_F {
                left: 14.0,
                top: 6.0,
                right: 32.0,
                bottom: h - 6.0,
            };
            self.dwrite.draw_text(&self.d2d.context, "🎵", icon_rect, white_brush, &self.dwrite.format_center);

            let title_rect = D2D_RECT_F {
                left: 36.0,
                top: 6.0,
                right: w - 46.0,
                bottom: h - 6.0,
            };
            self.dwrite.draw_text(&self.d2d.context, &track.title, title_rect, white_brush, &self.dwrite.format_regular);
        } else {
            let time_rect = D2D_RECT_F {
                left: center_x - 70.0,
                top: 6.0,
                right: center_x + 70.0,
                bottom: h - 6.0,
            };
            self.dwrite.draw_text(&self.d2d.context, &self.system_status.time_str, time_rect, white_brush, &self.dwrite.format_center);
        }

        // Battery percentage on right
        if let Some(pct) = self.system_status.battery_percent {
            let bat_str = if self.system_status.is_charging {
                format!("⚡{}%", pct)
            } else {
                format!("{}%", pct)
            };
            let bat_rect = D2D_RECT_F {
                left: w - 46.0,
                top: 6.0,
                right: w - 10.0,
                bottom: h - 6.0,
            };
            self.dwrite.draw_text(&self.d2d.context, &bat_str, bat_rect, dim_brush, &self.dwrite.format_small);
        }
    }

    fn draw_expanded_top_bar(
        &self,
        w: f32,
        dim_brush: &ID2D1SolidColorBrush,
        green_brush: &ID2D1SolidColorBrush,
    ) {
        let center_x = w / 2.0;

        // Tiny green online/status dot at x=138, y=14
        self.d2d.fill_ellipse(center_x - 72.0, 14.0, 2.5, 2.5, green_brush);

        // Date & Time in the center: "@ Sun 25 Oct 2026 16:40"
        let date_time_str = format!("@ {}", self.system_status.time_str);
        let time_rect = D2D_RECT_F {
            left: center_x - 66.0,
            top: 7.0,
            right: center_x + 90.0,
            bottom: 23.0,
        };
        self.dwrite.draw_text(&self.d2d.context, &date_time_str, time_rect, dim_brush, &self.dwrite.format_small);

        // Battery status on right
        if let Some(pct) = self.system_status.battery_percent {
            let bat_str = if self.system_status.is_charging {
                format!("⚡{}%", pct)
            } else {
                format!("🔋{}%", pct)
            };
            let bat_rect = D2D_RECT_F {
                left: w - 50.0,
                top: 7.0,
                right: w - 12.0,
                bottom: 23.0,
            };
            self.dwrite.draw_text(&self.d2d.context, &bat_str, bat_rect, dim_brush, &self.dwrite.format_small);
        }
    }
}

fn get_monitor_bounds() -> (i32, i32, i32) {
    unsafe {
        let pt = POINT { x: 0, y: 0 };
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            let w = mi.rcMonitor.right - mi.rcMonitor.left;
            (mi.rcMonitor.left, mi.rcMonitor.top, w)
        } else {
            (0, 0, 1920)
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if ptr != 0 {
        let notch = &mut *(ptr as *mut NotchWindow);

        match msg {
            WM_TIMER => {
                notch.tick_frame();
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                if !notch.is_hovered {
                    notch.is_hovered = true;
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut tme);
                }

                let x = (lparam.0 & 0xFFFF) as f32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as f32;
                notch.carousel.on_mouse_move(notch.current_w, notch.current_h, x, y);
                return LRESULT(0);
            }
            WM_MOUSELEAVE => {
                notch.is_hovered = false;
                return LRESULT(0);
            }
            WM_MOUSEWHEEL => {
                let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
                notch.carousel.on_wheel(delta);
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xFFFF) as f32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as f32;

                if notch.carousel.on_click(notch.current_w, notch.current_h, x, y) {
                    return LRESULT(0);
                }

                let sec_bounds = D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: notch.current_w,
                    bottom: notch.current_h,
                };

                match notch.carousel.current_section {
                    0 => {
                        let _ = MusicSection::hit_test_controls(
                            sec_bounds,
                            x,
                            y,
                            &notch.media_service,
                        );
                    }
                    1 => {
                        let _ = notch.focus_sec.on_click(sec_bounds, x, y);
                    }
                    2 => {
                        let _ = notch.water_sec.on_click(sec_bounds, x, y);
                    }
                    3 => {
                        let _ = notch.todo_sec.on_click(sec_bounds, x, y);
                    }
                    5 => {
                        let _ = notch.shelf_sec.on_click(sec_bounds, x, y);
                    }
                    _ => {}
                }
                return LRESULT(0);
            }
            WM_DROPFILES => {
                let hdrop = HDROP(wparam.0 as *mut _);
                let count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
                for i in 0..count {
                    let mut buf = [0u16; 512];
                    let len = DragQueryFileW(hdrop, i, Some(&mut buf));
                    if len > 0 {
                        let path_str = String::from_utf16_lossy(&buf[..len as usize]);
                        let path = std::path::PathBuf::from(path_str);
                        notch.shelf_sec.add_file(&path);
                        notch.carousel.set_section(5); // switch to shelf
                    }
                }
                DragFinish(hdrop);
                return LRESULT(0);
            }
            WM_TRAYICON => {
                if lparam.0 as u32 == WM_RBUTTONUP {
                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    if let Ok(hmenu) = CreatePopupMenu() {
                        let title_w: Vec<u16> = "Show Notch\0".encode_utf16().collect();
                        let folder_w: Vec<u16> = "Open Data Folder\0".encode_utf16().collect();
                        let exit_w: Vec<u16> = "Exit Nocan\0".encode_utf16().collect();
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_OPEN, PCWSTR(title_w.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_FOLDER, PCWSTR(folder_w.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_EXIT, PCWSTR(exit_w.as_ptr()));
                        let _ = SetForegroundWindow(hwnd);
                        let cmd = TrackPopupMenu(
                            hmenu,
                            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD,
                            pt.x,
                            pt.y,
                            0,
                            hwnd,
                            None,
                        );
                        let _ = DestroyMenu(hmenu);
                        match cmd.0 as usize {
                            IDM_OPEN => {
                                notch.is_hovered = true;
                            }
                            IDM_FOLDER => {
                                let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
                                let dir = format!("{}\\Nocan\0", local_appdata);
                                let dir_w: Vec<u16> = dir.encode_utf16().collect();
                                let op: Vec<u16> = "open\0".encode_utf16().collect();
                                let _ = ShellExecuteW(None, PCWSTR(op.as_ptr()), PCWSTR(dir_w.as_ptr()), None, None, SW_SHOW);
                            }
                            IDM_EXIT => {
                                let _ = DestroyWindow(hwnd);
                            }
                            _ => {}
                        }
                    }
                }
                return LRESULT(0);
            }
            WM_DESTROY => {
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: 1,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                let _ = Box::from_raw(ptr as *mut NotchWindow);
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
