use std::time::Instant;
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, ScreenToClient, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, Shell_NotifyIconW, HDROP, NIF_ICON, NIF_MESSAGE,
    NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetClassNameW, GetCursorPos, GetForegroundWindow, GetMessageW, IsIconic,
    IsWindowVisible, LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassExW,
    SetCursor, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, TrackPopupMenu,
    TranslateMessage, GWLP_USERDATA, HTCLIENT, HWND_TOPMOST, IDC_ARROW, IDC_HAND, IDI_APPLICATION,
    MB_ICONINFORMATION, MB_OK, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG,
    SWP_NOACTIVATE, SW_SHOWNOACTIVATE, TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_DESTROY,
    WM_DROPFILES, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONUP, WM_SETCURSOR,
    WM_TIMER, WM_USER, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};

use crate::graphics::composition::CompositionTarget;
use crate::graphics::d2d::D2DContext;
use crate::graphics::d3d::D3DContext;
use crate::graphics::dwrite::DWriteContext;
use crate::integrations::media::MediaService;
use crate::integrations::system_info::{get_system_status, SystemStatus};
use crate::notch::carousel::Carousel;
use crate::notch::dots::TOTAL_SECTIONS;
use crate::sections::focus::FocusSection;
use crate::sections::music::MusicSection;
use crate::sections::openmaus::OpenMausSection;
use crate::sections::shelf::ShelfSection;
use crate::sections::todo::TodoSection;
use crate::sections::water::WaterSection;

const MINIMIZED_W: f32 = 44.0;
const MINIMIZED_H: f32 = 4.0;
const MINIMIZED_R: f32 = 2.0;
const MINIMIZED_EAR: f32 = 2.0;

const COLLAPSED_W: f32 = 150.0;
const COLLAPSED_H: f32 = 30.0;
const COLLAPSED_R: f32 = 14.0;
const COLLAPSED_EAR: f32 = 10.0;

const EXPANDED_W: f32 = 420.0;
const LYRICS_W: f32 = 520.0;
const EXPANDED_H: f32 = 140.0;
const EXPANDED_R: f32 = 32.0;
const EXPANDED_EAR: f32 = 22.0;

const WM_TRAYICON: u32 = WM_USER + 101;
const IDM_AUTOSTART: usize = 1001;
const IDM_MASCOT: usize = 1002;
const IDM_MINIMAL: usize = 1003;
const IDM_ABOUT: usize = 1004;
const IDM_EXIT: usize = 1005;

const WM_MOUSELEAVE: u32 = 0x02A3;

pub struct NotchWindow {
    pub hwnd: HWND,
    d3d: D3DContext,
    d2d: D2DContext,
    dwrite: DWriteContext,
    _comp: CompositionTarget,
    carousel: Carousel,
    media_service: MediaService,
    music_sec: MusicSection,
    focus_sec: FocusSection,
    water_sec: WaterSection,
    shelf_sec: ShelfSection,
    todo_sec: TodoSection,
    openmaus_sec: OpenMausSection,
    pub is_hovered: bool,
    pub mascot_enabled: bool,
    pub minimal_notch: bool,
    pub auto_minimized: bool,
    pub autostart_enabled: bool,
    current_body_w: f32,
    current_h: f32,
    current_r: f32,
    current_ear: f32,
    vel_w: f32,
    vel_h: f32,
    vel_r: f32,
    vel_ear: f32,
    screen_w: i32,
    screen_x: i32,
    screen_y: i32,
    system_status: SystemStatus,
    last_status_poll: u64,
    start_instant: Instant,
    last_frame_instant: Instant,
    dump_next_frame: Option<String>,
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
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: windows::Win32::UI::WindowsAndMessaging::HICON::default(),
        };

        unsafe {
            RegisterClassExW(&wnd_class);
        }

        let (screen_x, screen_y, screen_w) = get_monitor_bounds();
        let init_win_w = (COLLAPSED_W + 2.0 * COLLAPSED_EAR) as i32;
        let init_win_h = COLLAPSED_H as i32;
        let init_x = screen_x + (screen_w - init_win_w) / 2;
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
                init_win_w,
                init_win_h,
                None,
                None,
                hinstance,
                None,
            )?
        };

        unsafe {
            DragAcceptFiles(hwnd, true);

            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
                ..Default::default()
            };
            let tip: Vec<u16> = "Nocan\0".encode_utf16().collect();
            for (i, &c) in tip.iter().enumerate().take(127) {
                nid.szTip[i] = c;
            }
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }

        let d3d = D3DContext::new(hwnd, init_win_w as u32, init_win_h as u32)?;
        let mut d2d = D2DContext::new(&d3d.dxgi_device)?;
        {
            let surface = d3d.get_surface()?;
            d2d.set_target_surface(&surface)?;
        }

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
            music_sec: MusicSection::new(),
            focus_sec: FocusSection::new(),
            water_sec: WaterSection::new(),
            shelf_sec: ShelfSection::new(),
            todo_sec: TodoSection::new(),
            openmaus_sec: OpenMausSection::new(),
            is_hovered: false,
            mascot_enabled: false,
            minimal_notch: false,
            auto_minimized: false,
            autostart_enabled: true,
            current_body_w: COLLAPSED_W,
            current_h: COLLAPSED_H,
            current_r: COLLAPSED_R,
            current_ear: COLLAPSED_EAR,
            vel_w: 0.0,
            vel_h: 0.0,
            vel_r: 0.0,
            vel_ear: 0.0,
            screen_w,
            screen_x,
            screen_y,
            system_status: get_system_status(),
            last_status_poll: 0,
            start_instant: Instant::now(),
            last_frame_instant: Instant::now(),
            dump_next_frame: None,
        });

        let raw_ptr = Box::into_raw(notch);
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
            if std::env::args().any(|a| a == "--dump-previews") {
                (*raw_ptr).dump_all_previews();
                let _ = DestroyWindow(hwnd);
                return Ok(hwnd);
            }
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            windows::Win32::UI::WindowsAndMessaging::SetTimer(hwnd, 1, 16, None);
        }

        Ok(hwnd)
    }

    pub fn dump_all_previews(&mut self) {
        let dir = "C:\\Users\\abufa\\.gemini\\antigravity-cli\\brain\\d3fb3b2e-6db7-4ab4-b33b-d2c255fd3495\\scratch";
        let _ = std::fs::create_dir_all(dir);

        // 1. Closed Idle
        self.is_hovered = false;
        self.mascot_enabled = false;
        self.current_body_w = COLLAPSED_W;
        self.current_h = COLLAPSED_H;
        self.current_r = COLLAPSED_R;
        self.current_ear = COLLAPSED_EAR;
        self.resize_and_dump(&format!("{}\\dump_idle.raw", dir));

        // 2. Closed Mascot
        self.mascot_enabled = true;
        self.resize_and_dump(&format!("{}\\dump_mascot.raw", dir));
        self.mascot_enabled = false;

        // 3. Expanded Sections 0..5
        self.is_hovered = true;
        self.current_body_w = EXPANDED_W;
        self.current_h = EXPANDED_H;
        self.current_r = EXPANDED_R;
        self.current_ear = EXPANDED_EAR;
        for sec in 0..6 {
            self.carousel.snap_to_section(sec);
            self.music_sec.show_lyrics = false;
            self.resize_and_dump(&format!("{}\\dump_sec_{}.raw", dir, sec));
        }

        // 4. Expanded Lyrics Mode
        self.carousel.snap_to_section(0);
        self.music_sec.show_lyrics = true;
        self.current_body_w = LYRICS_W;
        self.resize_and_dump(&format!("{}\\dump_lyrics.raw", dir));

        // 5. Mid-transition preview between Section 0 (Music) and Section 1 (Focus)
        self.music_sec.show_lyrics = false;
        self.current_body_w = EXPANDED_W;
        self.carousel.current_section = 1;
        self.carousel.page_pos = 0.45;
        self.resize_and_dump(&format!("{}\\dump_transition_0_1.raw", dir));

        // 6. Minimized Notch (when inside an application: 44x4, corner_r=2, ear_r=2)
        self.is_hovered = false;
        self.auto_minimized = true;
        self.current_body_w = MINIMIZED_W;
        self.current_h = MINIMIZED_H;
        self.current_r = MINIMIZED_R;
        self.current_ear = MINIMIZED_EAR;
        self.resize_and_dump(&format!("{}\\dump_minimized.raw", dir));
        self.auto_minimized = false;
    }

    fn resize_and_dump(&mut self, path: &str) {
        let win_w = (self.current_body_w + 2.0 * self.current_ear).round() as u32;
        let win_h = self.current_h.round() as u32;
        self.d2d.reset_target();
        let _ = self.d3d.resize(win_w, win_h);
        if let Ok(surface) = self.d3d.get_surface() {
            let _ = self.d2d.set_target_surface(&surface);
        }
        self.dump_next_frame = Some(path.to_string());
        self.render();
    }

    /// Detects whether the user is currently inside an active application window
    /// (as opposed to the Windows Desktop `Progman`/`WorkerW` or Taskbar `Shell_TrayWnd`),
    /// matching `nocan.original.exe` (`0x140025f53`).
    fn check_foreground_app(&self) -> Option<bool> {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return Some(false);
            }
            if fg == self.hwnd {
                return None;
            }
            if !IsWindowVisible(fg).as_bool() || IsIconic(fg).as_bool() {
                return Some(false);
            }
            let mut class_buf = [0u16; 64];
            let len = GetClassNameW(fg, &mut class_buf);
            if len > 0 {
                let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
                if class_name == "Progman"
                    || class_name == "WorkerW"
                    || class_name == "Shell_TrayWnd"
                    || class_name == "Shell_SecondaryTrayWnd"
                {
                    return Some(false);
                }
            }
            Some(true)
        }
    }

    pub fn tick_frame(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_instant).as_secs_f32().clamp(0.001, 0.050);
        self.last_frame_instant = now;

        // 1. Check if user is in an application to auto-minimize the notch
        if let Some(in_app) = self.check_foreground_app() {
            self.auto_minimized = in_app;
        }
        let is_minimized_mode = self.minimal_notch || self.auto_minimized;

        // 2. Screen-space cursor hover check (matches nocan.original.exe 0x140024db4..0x140024e86:
        //    when minimized to 44x4, the hover hit-box at top-center is widened to 100x10 + 4px margin)
        let mut cursor_pt = POINT::default();
        if unsafe { GetCursorPos(&mut cursor_pt).is_ok() } {
            let screen_cx = (self.screen_x as f32) + (self.screen_w as f32) * 0.5;
            let dx = (cursor_pt.x as f32 - screen_cx).abs();
            let dy = cursor_pt.y as f32 - (self.screen_y as f32);

            if !self.is_hovered {
                let min_hit_w = if is_minimized_mode { 100.0f32 } else { COLLAPSED_W };
                let min_hit_h = if is_minimized_mode { 10.0f32 } else { COLLAPSED_H };
                let half_w = self.current_body_w.max(min_hit_w) * 0.5 + 4.0;
                let max_y = self.current_h.max(min_hit_h) + 4.0;
                if dx <= half_w && dy >= -2.0 && dy <= max_y {
                    self.is_hovered = true;
                }
            } else {
                let target_w = if self.carousel.current_section == 0 && self.music_sec.show_lyrics {
                    LYRICS_W
                } else {
                    EXPANDED_W
                };
                let half_w = self.current_body_w.max(target_w) * 0.5 + EXPANDED_EAR + 6.0;
                let max_y = self.current_h.max(EXPANDED_H) + 6.0;
                if dx > half_w || dy < -5.0 || dy > max_y {
                    self.is_hovered = false;
                    self.music_sec.show_lyrics = false;
                }
            }
        }

        let prev_sec = self.carousel.current_section;
        self.carousel.update(dt);
        if self.carousel.current_section == 2 && prev_sec != 2 {
            self.water_sec.trigger_grow_anim();
        }
        self.water_sec.update(dt);

        let (target_body_w, target_h, target_r, target_ear) = if self.is_hovered {
            let w = if self.carousel.current_section == 0 && self.music_sec.show_lyrics {
                LYRICS_W
            } else {
                EXPANDED_W
            };
            (w, EXPANDED_H, EXPANDED_R, EXPANDED_EAR)
        } else if is_minimized_mode {
            (MINIMIZED_W, MINIMIZED_H, MINIMIZED_R, MINIMIZED_EAR)
        } else {
            (COLLAPSED_W, COLLAPSED_H, COLLAPSED_R, COLLAPSED_EAR)
        };

        let old_win_w = (self.current_body_w + 2.0 * self.current_ear).round() as i32;
        let old_win_h = self.current_h.round() as i32;

        // Second-order spring physics from nocan.original.exe (0x1400242cd): stiffness = 220.0, damping = 26.0
        let k = 220.0f32;
        let d = 26.0f32;

        let fw = k * (target_body_w - self.current_body_w) - d * self.vel_w;
        self.vel_w += fw * dt;
        self.current_body_w += self.vel_w * dt;
        if (self.current_body_w - target_body_w).abs() < 0.25 && self.vel_w.abs() < 1.0 {
            self.current_body_w = target_body_w;
            self.vel_w = 0.0;
        }

        let fh = k * (target_h - self.current_h) - d * self.vel_h;
        self.vel_h += fh * dt;
        self.current_h += self.vel_h * dt;
        if (self.current_h - target_h).abs() < 0.25 && self.vel_h.abs() < 1.0 {
            self.current_h = target_h;
            self.vel_h = 0.0;
        }

        let fr = k * (target_r - self.current_r) - d * self.vel_r;
        self.vel_r += fr * dt;
        self.current_r += self.vel_r * dt;
        if (self.current_r - target_r).abs() < 0.15 && self.vel_r.abs() < 0.5 {
            self.current_r = target_r;
            self.vel_r = 0.0;
        }

        let fe = k * (target_ear - self.current_ear) - d * self.vel_ear;
        self.vel_ear += fe * dt;
        self.current_ear += self.vel_ear * dt;
        if (self.current_ear - target_ear).abs() < 0.15 && self.vel_ear.abs() < 0.5 {
            self.current_ear = target_ear;
            self.vel_ear = 0.0;
        }

        let new_win_w = (self.current_body_w + 2.0 * self.current_ear).round() as i32;
        let new_win_h = self.current_h.round() as i32;

        let new_x = self.screen_x + (self.screen_w - new_win_w) / 2;
        let new_y = self.screen_y;

        if new_win_w != old_win_w || new_win_h != old_win_h {
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    new_x,
                    new_y,
                    new_win_w.max(10),
                    new_win_h.max(4),
                    SWP_NOACTIVATE,
                );
            }
            self.d2d.reset_target();
            let _ = self.d3d.resize(new_win_w.max(10) as u32, new_win_h.max(4) as u32);
            if let Ok(surface) = self.d3d.get_surface() {
                let _ = self.d2d.set_target_surface(&surface);
            }
        }

        // Update active sections
        self.focus_sec.update();
        self.openmaus_sec.refresh_if_needed();

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
        let body_w = self.current_body_w;
        let body_h = self.current_h;
        let ear = self.current_ear;
        let corner_r = self.current_r;
        let body_left = ear;
        let anim_time = self.start_instant.elapsed().as_secs_f32();

        self.d2d.begin_draw();
        self.d2d.clear(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });

        let notch_bg = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
            .ok();
        let white_brush = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 })
            .ok();
        let dim_brush = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.50 })
            .ok();
        let inactive_dot_brush = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.45 })
            .ok();
        let nav_circle_brush = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.28 })
            .ok();
        let nav_hover_brush = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.42 })
            .ok();
        let pill_bg_brush = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.12 })
            .ok();
        let bar_bg_brush = self
            .d2d
            .create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.22 })
            .ok();

        if let (
            Some(bg),
            Some(white_b),
            Some(dim_b),
            Some(dot_inactive),
            Some(nav_bg),
            Some(nav_hov),
            Some(pill_b),
            Some(bar_b),
        ) = (
            &notch_bg,
            &white_brush,
            &dim_brush,
            &inactive_dot_brush,
            &nav_circle_brush,
            &nav_hover_brush,
            &pill_bg_brush,
            &bar_bg_brush,
        ) {
            // 1. Draw Authentic Pure Black Notch Silhouette with Concave Top Ears
            self.d2d
                .fill_notch_body(body_left, 0.0, body_w, body_h, corner_r, ear, bg);

            let track = self.media_service.get_info();

            let open_t = ((body_h - COLLAPSED_H) / (EXPANDED_H - COLLAPSED_H)).clamp(0.0, 1.0);
            // Smoothly fade collapsed bar content as body_h shrinks below 30px toward 4px (0x14003149c: (body_h - 18.0) / 12.0)
            let min_fade = ((body_h - 18.0) / 12.0).clamp(0.0, 1.0);
            let closed_alpha = min_fade * (1.0 - open_t * 2.8).clamp(0.0, 1.0);
            let open_alpha = ((open_t - 0.28) / 0.72).clamp(0.0, 1.0);

            if closed_alpha > 0.01 && body_h >= 18.0 {
                let pushed = if closed_alpha < 0.99 {
                    self.d2d.push_clipped_opacity_layer(body_left, body_w, body_h, corner_r, ear, closed_alpha)
                } else {
                    false
                };
                if self.mascot_enabled {
                    self.draw_mascot_bar(body_left, body_w, body_h, corner_r, anim_time, white_b);
                } else {
                    self.draw_collapsed_bar(body_left, body_w, body_h, &track, anim_time, white_b);
                }
                if pushed {
                    self.d2d.pop_layer();
                }
            }

            if open_alpha > 0.01 {
                let in_lyrics_mode = self.carousel.current_section == 0 && self.music_sec.show_lyrics;

                // 2. Top Status Header (l-hdr): Battery Ring + Date & Time + 5-Bar Equalizer
                if !in_lyrics_mode {
                    let pushed_hdr = if open_alpha < 0.99 {
                        self.d2d.push_clipped_opacity_layer(body_left, body_w, body_h, corner_r, ear, open_alpha)
                    } else {
                        false
                    };
                    self.draw_expanded_top_bar(body_left, body_w, &track, anim_time, white_b);
                    if pushed_hdr {
                        self.d2d.pop_layer();
                    }
                }

                // 3. Spring-Animated Section Transitions (slide_x = delta * 90.0 + smoothstep crossfade)
                for sec_idx in 0..TOTAL_SECTIONS {
                    let delta = (sec_idx as f32) - self.carousel.page_pos;
                    let raw_a = (1.0 - delta.abs()).clamp(0.0, 1.0);
                    if raw_a <= 0.005 {
                        continue;
                    }
                    let sec_alpha = raw_a * raw_a * (3.0 - 2.0 * raw_a) * open_alpha;
                    if sec_alpha <= 0.01 {
                        continue;
                    }
                    let slide_x = delta * 90.0;
                    let sec_bounds = D2D_RECT_F {
                        left: body_left + slide_x,
                        top: 0.0,
                        right: body_left + body_w + slide_x,
                        bottom: body_h,
                    };

                    let pushed_sec = if sec_alpha < 0.995 || slide_x.abs() > 0.5 {
                        self.d2d.push_clipped_opacity_layer(body_left, body_w, body_h, corner_r, ear, sec_alpha)
                    } else {
                        false
                    };

                    match sec_idx {
                        0 => {
                            self.music_sec.update_lyrics(&track);
                            self.music_sec.draw(
                                &self.d2d,
                                &self.dwrite,
                                sec_bounds,
                                &track,
                                white_b,
                                dim_b,
                                bar_b,
                            );
                        }
                        1 => {
                            self.focus_sec.draw(
                                &self.d2d,
                                &self.dwrite,
                                sec_bounds,
                                white_b,
                                dim_b,
                                pill_b,
                            );
                        }
                        2 => {
                            self.water_sec.draw(
                                &self.d2d,
                                &self.dwrite,
                                sec_bounds,
                                white_b,
                                dim_b,
                                pill_b,
                            );
                        }
                        3 => {
                            self.shelf_sec.draw(
                                &self.d2d,
                                &self.dwrite,
                                sec_bounds,
                                white_b,
                                dim_b,
                                pill_b,
                            );
                        }
                        4 => {
                            self.todo_sec.draw(
                                &self.d2d,
                                &self.dwrite,
                                sec_bounds,
                                white_b,
                                dim_b,
                                pill_b,
                            );
                        }
                        5 => {
                            self.openmaus_sec.draw(
                                &self.d2d,
                                &self.dwrite,
                                sec_bounds,
                                white_b,
                                dim_b,
                                pill_b,
                            );
                        }
                        _ => {}
                    }

                    if pushed_sec {
                        self.d2d.pop_layer();
                    }
                }

                // 4. Circular Navigation Arrows (.arw) & Page Dots (.dots)
                if !in_lyrics_mode {
                    let prev_a = self.carousel.prev_alpha() * open_alpha;
                    let next_a = self.carousel.next_alpha() * open_alpha;
                    self.carousel.nav.draw(
                        &self.d2d,
                        body_left,
                        body_w,
                        body_h,
                        prev_a,
                        next_a,
                        nav_bg,
                        nav_hov,
                        white_b,
                    );
                    let pushed_dots = if open_alpha < 0.99 {
                        self.d2d.push_clipped_opacity_layer(body_left, body_w, body_h, corner_r, ear, open_alpha)
                    } else {
                        false
                    };
                    self.carousel.dots.draw(
                        &self.d2d,
                        body_left,
                        body_w,
                        body_h,
                        self.carousel.page_pos,
                        white_b,
                        dot_inactive,
                    );
                    if pushed_dots {
                        self.d2d.pop_layer();
                    }
                }
            }
        }

        let _ = self.d2d.end_draw();
        if let Some(path) = self.dump_next_frame.take() {
            let _ = self.d3d.dump_backbuffer_raw(&path);
        }
        let _ = self.d3d.present();
    }

    fn draw_battery_ring(&self, cx: f32, cy: f32, white_brush: &ID2D1SolidColorBrush) {
        let pct = self.system_status.battery_percent.unwrap_or(72) as f32 / 100.0;
        if let (Ok(track_b), Ok(arc_b)) = (
            self.d2d.create_solid_brush(D2D1_COLOR_F {
                r: 51.0 / 255.0,
                g: 224.0 / 255.0,
                b: 122.0 / 255.0,
                a: 0.28,
            }),
            self.d2d.create_solid_brush(D2D1_COLOR_F {
                r: 51.0 / 255.0,
                g: 224.0 / 255.0,
                b: 122.0 / 255.0,
                a: 1.0,
            }),
        ) {
            self.d2d
                .draw_progress_ring(cx, cy, 6.1, 2.6, pct.clamp(0.05, 1.0), &track_b, &arc_b);
        }

        // Small white lightning bolt inside ring when charging (as seen in original_music.png & original_pomo.png)
        if self.system_status.is_charging {
            let bolt = [
                (cx + 0.6, cy - 3.6),
                (cx - 2.2, cy + 0.3),
                (cx + 0.1, cy + 0.3),
                (cx - 0.6, cy + 3.6),
                (cx + 2.2, cy - 0.3),
                (cx - 0.1, cy - 0.3),
            ];
            self.d2d.fill_polygon(&bolt, white_brush);
        }
    }

    fn draw_collapsed_bar(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        track: &crate::integrations::media::MediaTrackInfo,
        anim_time: f32,
        white_brush: &ID2D1SolidColorBrush,
    ) {
        let cy = body_h * 0.5;

        // 1. Battery Ring at left: 20px, top: 15px
        self.draw_battery_ring(body_left + 20.0, cy, white_brush);

        // 2. Clock Time at left: 75px, top: 15px, font-size: 16px Bold
        let clock_rect = D2D_RECT_F {
            left: body_left + 36.0,
            top: 0.0,
            right: body_left + body_w - 36.0,
            bottom: body_h,
        };
        self.dwrite.draw_text(
            &self.d2d.context,
            &self.system_status.clock_str,
            clock_rect,
            white_brush,
            &self.dwrite.format_idle_time,
        );

        // 3. 5-bar Equalizer at left: 131px, top: 15px
        self.d2d
            .draw_equalizer_bars(body_left + 131.0, cy, track.is_playing, anim_time, white_brush);
    }

    fn draw_mascot_bar(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        corner_r: f32,
        anim_time: f32,
        white_brush: &ID2D1SolidColorBrush,
    ) {
        let cx = body_left + body_w * 0.5;
        let cy = body_h * 0.5;

        // Exact nocan.original.exe (0x140038f38, 0x140039002, 0x140039b99, 0x140039c49) ambient glow:
        // Horizontal gradient (#1ab8ff -> #2970ff -> #754dff) masked vertically by 6-stop alpha gradient
        // from bottom (y = body_h) up to y = body_h * 0.10, clipped to the notch body rounded silhouette.
        let breathe_opacity = if self.dump_next_frame.is_some() {
            0.85
        } else {
            0.75 + 0.12 * (anim_time * 1.6).sin()
        };
        let shift_x = if self.dump_next_frame.is_some() {
            0.0
        } else {
            (anim_time * 0.60).sin() * (body_w * 0.12)
        };

        self.d2d.draw_ambient_glow(
            body_left,
            body_w,
            body_h,
            corner_r,
            0.90,
            breathe_opacity,
            shift_x,
            &[
                (0.00, D2D1_COLOR_F { r: 0.10, g: 0.74, b: 1.00, a: 1.0 }),
                (0.50, D2D1_COLOR_F { r: 0.16, g: 0.44, b: 1.00, a: 1.0 }),
                (1.00, D2D1_COLOR_F { r: 0.46, g: 0.30, b: 1.00, a: 1.0 }),
            ],
        );

        // Subtle cursor eye-tracking (clamped to [-3.0, 3.0] horizontally, [-2.0, 2.0] vertically)
        let (look_x, look_y) = if self.dump_next_frame.is_some() {
            (0.0f32, 0.0f32)
        } else {
            let mut pt = POINT::default();
            unsafe {
                let _ = GetCursorPos(&mut pt);
            }
            let notch_screen_cx = (self.screen_x + self.screen_w / 2) as f32;
            let notch_screen_cy = (self.screen_y as f32) + cy;
            let dx = ((pt.x as f32 - notch_screen_cx) / 220.0).clamp(-1.0, 1.0) * 2.6;
            let dy = ((pt.y as f32 - notch_screen_cy) / 180.0).clamp(-1.0, 1.0) * 1.4;
            (dx, dy)
        };

        // Smooth sine blink every 4.2s (0x140039591: 0.18s blink window)
        let cycle = anim_time % 4.2;
        let blink_scale = if self.dump_next_frame.is_none() && cycle > 3.86 && cycle < 4.04 {
            let t = (cycle - 3.86) / 0.18;
            (1.0 - 0.90 * (t * std::f32::consts::PI).sin()).max(0.10)
        } else {
            1.0
        };

        let rx = 4.2f32;
        let ry = (6.2f32 * blink_scale).max(0.6f32);
        let glow_rx = rx * 2.4;
        let glow_ry = ry * 1.9 + 2.0;

        let eye_positions = [
            (cx - 11.0 + look_x, cy + look_y),
            (cx + 11.0 + look_x, cy + look_y),
        ];

        // Radial eye glow (0x1400390ce: 3 stops) + solid white eye oval
        for &(ex, ey) in &eye_positions {
            if let Ok(rad_glow) = self.d2d.create_radial_gradient(
                (ex, ey),
                glow_rx,
                glow_ry,
                &[
                    (0.00, D2D1_COLOR_F { r: 0.55, g: 0.85, b: 1.00, a: 0.55 }),
                    (0.60, D2D1_COLOR_F { r: 0.35, g: 0.65, b: 1.00, a: 0.15 }),
                    (1.00, D2D1_COLOR_F { r: 0.30, g: 0.55, b: 1.00, a: 0.00 }),
                ],
            ) {
                self.d2d.fill_ellipse_radial(ex, ey, glow_rx, glow_ry, &rad_glow);
            }
            self.d2d.fill_ellipse(ex, ey, rx, ry, white_brush);
        }
    }

    fn draw_expanded_top_bar(
        &self,
        body_left: f32,
        _body_w: f32,
        track: &crate::integrations::media::MediaTrackInfo,
        anim_time: f32,
        white_brush: &ID2D1SolidColorBrush,
    ) {
        let cy = 18.0;

        // 1. Battery Ring at left: 146px, top: 18px
        self.draw_battery_ring(body_left + 146.0, cy, white_brush);

        // 2. Date (13px Normal) + Clock Time (13px Bold) at left: 163px, top: 18px
        let date_rect = D2D_RECT_F {
            left: body_left + 163.0,
            top: cy - 10.0,
            right: body_left + 280.0,
            bottom: cy + 10.0,
        };
        self.dwrite.draw_text(
            &self.d2d.context,
            &self.system_status.date_str,
            date_rect,
            white_brush,
            &self.dwrite.format_regular,
        );

        let date_w = (self.system_status.date_str.len() as f32) * 6.45 + 6.0;
        let clock_rect = D2D_RECT_F {
            left: body_left + 163.0 + date_w,
            top: cy - 10.0,
            right: body_left + 350.0,
            bottom: cy + 10.0,
        };
        self.dwrite.draw_text(
            &self.d2d.context,
            &self.system_status.clock_str,
            clock_rect,
            white_brush,
            &self.dwrite.format_regular_bold,
        );

        // 3. 5-bar Equalizer at left: 396px, top: 18px
        self.d2d
            .draw_equalizer_bars(body_left + 396.0, cy, track.is_playing, anim_time, white_brush);
    }

    pub fn is_interactive_at(&self, x: f32, y: f32) -> bool {
        if !self.is_hovered {
            return false;
        }

        let body_left = self.current_ear;
        let body_w = self.current_body_w;
        let body_h = self.current_h;
        let in_lyrics_mode = self.carousel.current_section == 0 && self.music_sec.show_lyrics;

        if !in_lyrics_mode {
            if self.carousel.nav.hit_test_prev(body_left, body_w, body_h, x, y, self.carousel.show_prev())
                || self.carousel.nav.hit_test_next(body_left, body_w, body_h, x, y, self.carousel.show_next())
            {
                return true;
            }
            if self.carousel.dots.hit_test(body_left, body_w, body_h, x, y).is_some() {
                return true;
            }
        }

        let sec_bounds = D2D_RECT_F {
            left: body_left,
            top: 0.0,
            right: body_left + body_w,
            bottom: body_h,
        };

        match self.carousel.current_section {
            0 => self.music_sec.is_interactive(sec_bounds, x, y),
            1 => self.focus_sec.is_interactive(sec_bounds, x, y),
            2 => self.water_sec.is_interactive(sec_bounds, x, y),
            3 => self.shelf_sec.is_interactive(sec_bounds, x, y),
            4 => self.todo_sec.is_interactive(sec_bounds, x, y),
            _ => false,
        }
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
            WM_SETCURSOR => {
                let hit_test = (lparam.0 & 0xFFFF) as u32;
                if hit_test == HTCLIENT {
                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    let _ = ScreenToClient(hwnd, &mut pt);
                    let cursor_id = if notch.is_interactive_at(pt.x as f32, pt.y as f32) {
                        IDC_HAND
                    } else {
                        IDC_ARROW
                    };
                    let cursor = LoadCursorW(None, cursor_id).unwrap_or_default();
                    SetCursor(cursor);
                    return LRESULT(1);
                }
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

                let x = (lparam.0 & 0xFFFF) as i16 as f32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
                let body_left = notch.current_ear;
                let body_w = notch.current_body_w;
                let body_h = notch.current_h;
                let show_prev = notch.carousel.show_prev();
                let show_next = notch.carousel.show_next();
                notch.carousel.nav.hovered_prev =
                    notch.carousel.nav.hit_test_prev(body_left, body_w, body_h, x, y, show_prev);
                notch.carousel.nav.hovered_next =
                    notch.carousel.nav.hit_test_next(body_left, body_w, body_h, x, y, show_next);
                return LRESULT(0);
            }
            WM_MOUSELEAVE => {
                notch.is_hovered = false;
                notch.music_sec.show_lyrics = false;
                return LRESULT(0);
            }
            WM_MOUSEWHEEL => {
                let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
                notch.music_sec.show_lyrics = false;
                notch.carousel.on_wheel(delta);
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xFFFF) as i16 as f32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

                let body_left = notch.current_ear;
                let body_w = notch.current_body_w;
                let body_h = notch.current_h;
                let in_lyrics_mode = notch.carousel.current_section == 0 && notch.music_sec.show_lyrics;

                if !in_lyrics_mode {
                    let show_prev = notch.carousel.show_prev();
                    let show_next = notch.carousel.show_next();
                    if notch.carousel.nav.hit_test_prev(body_left, body_w, body_h, x, y, show_prev) {
                        notch.carousel.prev();
                        return LRESULT(0);
                    }
                    if notch.carousel.nav.hit_test_next(body_left, body_w, body_h, x, y, show_next) {
                        notch.carousel.next();
                        return LRESULT(0);
                    }
                    if let Some(idx) = notch.carousel.dots.hit_test(body_left, body_w, body_h, x, y) {
                        notch.carousel.set_section(idx);
                        return LRESULT(0);
                    }
                }

                let sec_bounds = D2D_RECT_F {
                    left: body_left,
                    top: 0.0,
                    right: body_left + body_w,
                    bottom: body_h,
                };

                match notch.carousel.current_section {
                    0 => {
                        let _ = notch.music_sec.hit_test_controls(
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
                        let _ = notch.shelf_sec.on_click(sec_bounds, x, y);
                    }
                    4 => {
                        let _ = notch.todo_sec.on_click(sec_bounds, x, y);
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
                        notch.carousel.set_section(3); // Switch to Shelf (index 3)
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
                        let auto_w: Vec<u16> = "Start with Windows\0".encode_utf16().collect();
                        let mascot_w: Vec<u16> = "Nocan Mascot\0".encode_utf16().collect();
                        let min_w: Vec<u16> = "Minimal notch\0".encode_utf16().collect();
                        let about_w: Vec<u16> = "About Nocan\0".encode_utf16().collect();
                        let exit_w: Vec<u16> = "Exit\0".encode_utf16().collect();

                        let auto_flag = if notch.autostart_enabled { MF_CHECKED } else { MF_UNCHECKED };
                        let mascot_flag = if notch.mascot_enabled { MF_CHECKED } else { MF_UNCHECKED };
                        let min_flag = if notch.minimal_notch { MF_CHECKED } else { MF_UNCHECKED };

                        let _ = AppendMenuW(hmenu, MF_STRING | auto_flag, IDM_AUTOSTART, PCWSTR(auto_w.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_STRING | mascot_flag, IDM_MASCOT, PCWSTR(mascot_w.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_STRING | min_flag, IDM_MINIMAL, PCWSTR(min_w.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_ABOUT, PCWSTR(about_w.as_ptr()));
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
                            IDM_AUTOSTART => {
                                notch.autostart_enabled = !notch.autostart_enabled;
                            }
                            IDM_MASCOT => {
                                notch.mascot_enabled = !notch.mascot_enabled;
                            }
                            IDM_MINIMAL => {
                                notch.minimal_notch = !notch.minimal_notch;
                            }
                            IDM_ABOUT => {
                                let msg: Vec<u16> = "Nocan v1.1.0\nMusic, lyrics, focus timer, water reminder, to-do, file shelf & OpenMausBot.\nLyrics provided by LRCLIB (lrclib.net).\0".encode_utf16().collect();
                                let cap: Vec<u16> = "About Nocan\0".encode_utf16().collect();
                                let _ = MessageBoxW(hwnd, PCWSTR(msg.as_ptr()), PCWSTR(cap.as_ptr()), MB_OK | MB_ICONINFORMATION);
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
