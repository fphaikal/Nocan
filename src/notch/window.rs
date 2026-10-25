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
    DispatchMessageW, GetCursorPos, GetMessageW, LoadCursorW, LoadIconW, MessageBoxW,
    PostQuitMessage, RegisterClassExW, SetCursor, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, TrackPopupMenu, TranslateMessage, GWLP_USERDATA, HTCLIENT,
    HWND_TOPMOST, IDC_ARROW, IDC_HAND, IDI_APPLICATION, MB_ICONINFORMATION, MB_OK, MF_CHECKED,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, SWP_NOACTIVATE, SW_SHOWNOACTIVATE, TPM_BOTTOMALIGN,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_DESTROY, WM_DROPFILES, WM_LBUTTONDOWN, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_RBUTTONUP, WM_SETCURSOR, WM_TIMER, WM_USER, WNDCLASSEXW,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
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
    pub autostart_enabled: bool,
    current_body_w: f32,
    current_h: f32,
    current_r: f32,
    current_ear: f32,
    screen_w: i32,
    screen_x: i32,
    screen_y: i32,
    system_status: SystemStatus,
    last_status_poll: u64,
    start_instant: Instant,
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
            autostart_enabled: true,
            current_body_w: COLLAPSED_W,
            current_h: COLLAPSED_H,
            current_r: COLLAPSED_R,
            current_ear: COLLAPSED_EAR,
            screen_w,
            screen_x,
            screen_y,
            system_status: get_system_status(),
            last_status_poll: 0,
            start_instant: Instant::now(),
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
            self.carousel.set_section(sec);
            self.music_sec.show_lyrics = false;
            self.resize_and_dump(&format!("{}\\dump_sec_{}.raw", dir, sec));
        }

        // 4. Expanded Lyrics Mode
        self.carousel.set_section(0);
        self.music_sec.show_lyrics = true;
        self.current_body_w = LYRICS_W;
        self.resize_and_dump(&format!("{}\\dump_lyrics.raw", dir));
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

    pub fn tick_frame(&mut self) {
        let (target_body_w, target_h, target_r, target_ear) = if self.is_hovered {
            let w = if self.carousel.current_section == 0 && self.music_sec.show_lyrics {
                LYRICS_W
            } else {
                EXPANDED_W
            };
            (w, EXPANDED_H, EXPANDED_R, EXPANDED_EAR)
        } else if self.minimal_notch {
            (COLLAPSED_W, 8.0, 4.0, 4.0)
        } else {
            (COLLAPSED_W, COLLAPSED_H, COLLAPSED_R, COLLAPSED_EAR)
        };

        let old_win_w = (self.current_body_w + 2.0 * self.current_ear).round() as i32;
        let old_win_h = self.current_h.round() as i32;

        self.current_body_w += (target_body_w - self.current_body_w) * 0.24;
        self.current_h += (target_h - self.current_h) * 0.24;
        self.current_r += (target_r - self.current_r) * 0.24;
        self.current_ear += (target_ear - self.current_ear) * 0.24;

        if (self.current_body_w - target_body_w).abs() < 0.5 {
            self.current_body_w = target_body_w;
        }
        if (self.current_h - target_h).abs() < 0.5 {
            self.current_h = target_h;
        }
        if (self.current_r - target_r).abs() < 0.3 {
            self.current_r = target_r;
        }
        if (self.current_ear - target_ear).abs() < 0.3 {
            self.current_ear = target_ear;
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

            if !self.is_hovered {
                if !self.minimal_notch && body_h >= 24.0 {
                    if self.mascot_enabled && !track.is_playing {
                        self.draw_mascot_bar(body_left, body_w, body_h, anim_time, white_b);
                    } else {
                        self.draw_collapsed_bar(body_left, body_w, body_h, &track, anim_time, white_b);
                    }
                }
            } else if body_h >= 90.0 {
                let sec_bounds = D2D_RECT_F {
                    left: body_left,
                    top: 0.0,
                    right: body_left + body_w,
                    bottom: body_h,
                };

                let in_lyrics_mode = self.carousel.current_section == 0 && self.music_sec.show_lyrics;

                // 2. Top Status Header (l-hdr): Battery Ring + Date & Time + 5-Bar Equalizer
                if !in_lyrics_mode {
                    self.draw_expanded_top_bar(body_left, body_w, &track, anim_time, white_b);
                }

                // 3. Active Section Content
                match self.carousel.current_section {
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

                // 4. Circular Navigation Arrows (.arw) & Page Dots (.dots)
                if !in_lyrics_mode {
                    self.carousel.nav.draw(
                        &self.d2d,
                        body_left,
                        body_w,
                        body_h,
                        true,
                        true,
                        nav_bg,
                        nav_hov,
                        white_b,
                    );
                    self.carousel
                        .dots
                        .draw(&self.d2d, body_left, body_w, body_h, white_b, dot_inactive);
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
        anim_time: f32,
        white_brush: &ID2D1SolidColorBrush,
    ) {
        let cx = body_left + body_w * 0.5;
        let cy = body_h * 0.5;

        // Ambient bottom glow (.ambient: cyan -> indigo -> violet)
        if let Ok(amb) = self.d2d.create_linear_gradient(
            (body_left, body_h),
            (body_left + body_w, body_h),
            &[
                (0.0, D2D1_COLOR_F { r: 25.0 / 255.0, g: 184.0 / 255.0, b: 1.0, a: 0.45 }),
                (0.55, D2D1_COLOR_F { r: 45.0 / 255.0, g: 91.0 / 255.0, b: 1.0, a: 0.45 }),
                (1.0, D2D1_COLOR_F { r: 123.0 / 255.0, g: 75.0 / 255.0, b: 1.0, a: 0.45 }),
            ],
        ) {
            let r = D2D_RECT_F {
                left: body_left + 6.0,
                top: body_h * 0.45,
                right: body_left + body_w - 6.0,
                bottom: body_h,
            };
            self.d2d.fill_rounded_rect_linear(r, 12.0, 12.0, &amb);
        }

        // Periodic blink every 4.2s (92%..100% scaleY drops to 0.1)
        let phase = (anim_time % 4.2) / 4.2;
        let blink_scale: f32 = if phase > 0.92 && phase < 0.98 {
            0.12
        } else {
            1.0
        };
        let rx = 4.2f32;
        let ry = (6.2f32 * blink_scale).max(0.6f32);

        // Soft eye glow
        if let Ok(eye_glow) = self.d2d.create_solid_brush(D2D1_COLOR_F {
            r: 120.0 / 255.0,
            g: 190.0 / 255.0,
            b: 1.0,
            a: 0.32,
        }) {
            self.d2d.fill_ellipse(cx - 11.7, cy, rx + 3.0, ry + 3.0, &eye_glow);
            self.d2d.fill_ellipse(cx + 11.7, cy, rx + 3.0, ry + 3.0, &eye_glow);
        }

        self.d2d.fill_ellipse(cx - 11.7, cy, rx, ry, white_brush);
        self.d2d.fill_ellipse(cx + 11.7, cy, rx, ry, white_brush);
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
            if self.carousel.nav.hit_test_prev(body_left, body_w, body_h, x, y, true)
                || self.carousel.nav.hit_test_next(body_left, body_w, body_h, x, y, true)
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
                notch.carousel.nav.hovered_prev =
                    notch.carousel.nav.hit_test_prev(body_left, body_w, body_h, x, y, true);
                notch.carousel.nav.hovered_next =
                    notch.carousel.nav.hit_test_next(body_left, body_w, body_h, x, y, true);
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
                    if notch.carousel.nav.hit_test_prev(body_left, body_w, body_h, x, y, true) {
                        notch.carousel.prev();
                        return LRESULT(0);
                    }
                    if notch.carousel.nav.hit_test_next(body_left, body_w, body_h, x, y, true) {
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
