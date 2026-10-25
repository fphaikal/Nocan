use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, GetMonitorInfoW, MonitorFromPoint, ScreenToClient, SetWindowRgn, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::{RegisterPowerSettingNotification, POWERBROADCAST_SETTING};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_VOLUME_DOWN, VK_VOLUME_MUTE,
    VK_VOLUME_UP,
};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, Shell_NotifyIconW, HDROP, NIF_ICON, NIF_MESSAGE,
    NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetClassNameW, GetCursorPos, GetForegroundWindow, GetMessageW, IsIconic,
    GetWindowRect, IsWindowVisible, IsZoomed, LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassExW,
    SetCursor, SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TrackPopupMenu,
    TranslateMessage, DEVICE_NOTIFY_WINDOW_HANDLE, GWLP_USERDATA, HTCLIENT, IDC_ARROW, IDC_HAND,
    IDI_APPLICATION, MB_ICONINFORMATION, MB_OK, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    MSG, PBT_POWERSETTINGCHANGE, SW_SHOWNOACTIVATE, TPM_BOTTOMALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_DESTROY, WM_DROPFILES, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_POWERBROADCAST, WM_RBUTTONUP, WM_SETCURSOR, WM_TIMER, WM_USER, WNDCLASSEXW,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::graphics::composition::CompositionTarget;
use crate::graphics::d2d::D2DContext;
use crate::graphics::d3d::D3DContext;
use crate::graphics::dwrite::DWriteContext;
use crate::integrations::media::MediaService;
use crate::integrations::system_info::{
    get_system_status, query_audio_state, query_connected_bluetooth_devices,
     SystemStatus, GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS,
};
use crate::notch::carousel::Carousel;
use crate::notch::dots::TOTAL_SECTIONS;
use crate::sections::focus::FocusSection;
use crate::sections::music::MusicSection;
use crate::sections::openmaus::OpenMausSection;
use crate::sections::shelf::ShelfSection;
use crate::sections::todo::TodoSection;
use crate::sections::water::WaterSection;

/// Fixed DirectComposition canvas size matching `nocan.original.exe` (`0x140001a54`).
/// Using a fixed canvas avoids per-frame `IDXGISwapChain1::ResizeBuffers` & `SetWindowPos` calls,
/// eliminating GPU stalls and integer-rounding horizontal jitter during animations.
const CANVAS_W: i32 = 760;
const CANVAS_H: i32 = 160;

const MINIMIZED_W: f32 = 44.0;
const MINIMIZED_H: f32 = 4.0;

const COLLAPSED_W: f32 = 150.0;
const COLLAPSED_H: f32 = 30.0;
const COLLAPSED_R: f32 = 14.0;
const COLLAPSED_EAR: f32 = 10.0;

const PEEK_BAR_W: f32 = 300.0;
const PEEK_BAR_H: f32 = 34.0;

const PEEK_CARD_W: f32 = 310.0;
const PEEK_CARD_H: f32 = 60.0;

const EXPANDED_W: f32 = 420.0;
const LYRIC_BAR_MIN_W: f32 = 190.0;
const LYRIC_BAR_MAX_W: f32 = 560.0;
const LYRIC_BAR_PAD: f32 = 22.0;
const LYRICS_MIN_W: f32 = 440.0;
const LYRICS_MAX_W: f32 = 700.0;
const LYRICS_PANE_LEFT: f32 = 264.0;
const LYRICS_PANE_PAD: f32 = 26.0;
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

#[derive(Clone, Debug)]
pub enum PeekKind {
    Volume {
        level: f32,
        muted: bool,
    },
    Brightness {
        level: f32,
    },
    DeviceConnect {
        name: String,
        connected: bool,
        battery_pct: Option<u32>,
    },
    Charging {
        is_charging: bool,
        pct: u32,
    },
}

#[derive(Clone, Debug)]
pub struct PeekOverlay {
    pub kind: PeekKind,
    pub started_at: Instant,
    pub updated_at: Instant,
    pub duration_secs: f32,
    pub display_level: f32,
}

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
    lyrics_w: f32,
    lyric_line: Option<String>,
    lyric_text_w: f32,
    lyric_changed_at: Instant,
    current_h: f32,
    vel_w: f32,
    vel_h: f32,
    last_rgn_bounds: (i32, i32, i32),
    screen_w: i32,
    screen_x: i32,
    screen_y: i32,
    system_status: SystemStatus,
    prev_is_charging: Option<bool>,
    last_status_poll: u64,
    last_audio_poll: Instant,
    prev_volume: Option<(i32, bool)>,
    prev_audio_device: Option<(String, String, u32)>,
    initial_brightness_received: bool,
    prev_brightness: Option<u32>,
    bt_rx: Receiver<(String, bool)>,
    pub peek_overlay: Option<PeekOverlay>,
    pub last_peek: Option<PeekOverlay>,
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
        let init_x = screen_x + (screen_w - CANVAS_W) / 2;
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
                CANVAS_W,
                CANVAS_H,
                None,
                None,
                hinstance,
                None,
            )?
        };

        unsafe {
            DragAcceptFiles(hwnd, true);

            // Register for display brightness power notifications (GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS)
            let _ = RegisterPowerSettingNotification(
                HANDLE(hwnd.0),
                &GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS,
                DEVICE_NOTIFY_WINDOW_HANDLE,
            );

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

        let d3d = D3DContext::new(hwnd, CANVAS_W as u32, CANVAS_H as u32)?;
        let mut d2d = D2DContext::new(&d3d.dxgi_device)?;
        {
            let surface = d3d.get_surface()?;
            d2d.set_target_surface(&surface)?;
        }

        let dwrite = DWriteContext::new()?;
        let comp = CompositionTarget::new(hwnd, &d3d.dxgi_device, &d3d.swapchain)?;

        // Spawn background Bluetooth connection watcher thread
        let (bt_tx, bt_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut known = query_connected_bluetooth_devices();
            loop {
                std::thread::sleep(Duration::from_millis(1500));
                let current = query_connected_bluetooth_devices();
                for dev in current.difference(&known) {
                    let _ = bt_tx.send((dev.clone(), true));
                }
                for dev in known.difference(&current) {
                    let _ = bt_tx.send((dev.clone(), false));
                }
                known = current;
            }
        });

        let init_status = get_system_status();
        let init_charging = Some(init_status.is_charging);
        let init_audio = query_audio_state();
        let prev_volume = init_audio
            .as_ref()
            .map(|a| ((a.volume * 100.0).round() as i32, a.muted));
        let prev_audio_device = init_audio
            .map(|a| (a.device_id, a.device_name, a.active_count));

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
            lyrics_w: LYRICS_MIN_W,
            lyric_line: None,
            lyric_text_w: 0.0,
            lyric_changed_at: Instant::now(),
            current_h: COLLAPSED_H,
            vel_w: 0.0,
            vel_h: 0.0,
            last_rgn_bounds: (0, 0, 0),
            screen_w,
            screen_x,
            screen_y,
            system_status: init_status,
            prev_is_charging: init_charging,
            last_status_poll: 0,
            last_audio_poll: Instant::now(),
            prev_volume,
            prev_audio_device,
            initial_brightness_received: false,
            prev_brightness: None,
            bt_rx,
            peek_overlay: None,
            last_peek: None,
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
            (*raw_ptr).update_window_rgn();
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            windows::Win32::UI::WindowsAndMessaging::SetTimer(hwnd, 1, 10, None);
        }

        Ok(hwnd)
    }

    /// Computes `(corner_r, ear_r)` directly from `(body_w, body_h)` matching `nocan.original.exe` (`0x14002fd5a`).
    #[inline]
    fn compute_radii(body_w: f32, body_h: f32) -> (f32, f32) {
        let open_t = ((body_h - COLLAPSED_H) / (EXPANDED_H - COLLAPSED_H)).clamp(0.0, 1.0);
        let corner_r = (COLLAPSED_R + (EXPANDED_R - COLLAPSED_R) * open_t)
            .min(body_h * 0.5)
            .min(body_w * 0.5);
        let ear = (COLLAPSED_EAR + (EXPANDED_EAR - COLLAPSED_EAR) * open_t)
            .min((body_h - corner_r).max(0.0));
        (corner_r, ear)
    }

    #[inline]
    fn current_body_left(&self) -> f32 {
        if self.dump_next_frame.is_some() {
            let (_, ear) = Self::compute_radii(self.current_body_w, self.current_h);
            ear
        } else {
            (CANVAS_W as f32 - self.current_body_w) * 0.5
        }
    }

    /// Restricts the clickable HWND region to the active notch bounds (`CreateRectRgn` + `SetWindowRgn`),
    /// matching `nocan.original.exe` (`0x1400252ea..0x140025405`) so clicks outside the notch pass through
    /// while the fixed `580x160` DirectComposition surface never needs resizing.
    fn update_window_rgn(&mut self) {
        let is_minimized_mode = self.minimal_notch || self.auto_minimized;
        let min_hit_w = if is_minimized_mode && !self.is_hovered && self.peek_overlay.is_none() && self.lyric_line.is_none() {
            100.0f32
        } else {
            COLLAPSED_W
        };
        let min_hit_h = if is_minimized_mode && !self.is_hovered && self.peek_overlay.is_none() && self.lyric_line.is_none() {
            10.0f32
        } else {
            COLLAPSED_H
        };
        let (_, ear) = Self::compute_radii(self.current_body_w, self.current_h);
        let hit_w = self.current_body_w.max(min_hit_w) + 2.0 * ear + 8.0;
        let hit_h = self.current_h.max(min_hit_h) + 4.0;

        let rgn_left = ((CANVAS_W as f32 - hit_w) * 0.5).floor().max(0.0) as i32;
        let rgn_right = ((CANVAS_W as f32 + hit_w) * 0.5).ceil().min(CANVAS_W as f32) as i32;
        let rgn_bottom = hit_h.ceil().min(CANVAS_H as f32) as i32;

        let new_bounds = (rgn_left, rgn_right, rgn_bottom);
        if new_bounds != self.last_rgn_bounds {
            self.last_rgn_bounds = new_bounds;
            unsafe {
                let hrgn = CreateRectRgn(rgn_left, 0, rgn_right, rgn_bottom);
                if !hrgn.is_invalid() {
                    let _ = SetWindowRgn(self.hwnd, hrgn, false);
                }
            }
        }
    }

    pub fn trigger_peek(&mut self, kind: PeekKind) {
        let now = Instant::now();
        match &kind {
            PeekKind::Volume { level, muted } => {
                let target_lvl = if *muted { 0.0 } else { *level };
                if let Some(ref mut existing) = self.peek_overlay {
                    if matches!(existing.kind, PeekKind::Volume { .. }) {
                        existing.kind = kind;
                        existing.updated_at = now;
                        existing.duration_secs = 1.65;
                        self.last_peek = self.peek_overlay.clone();
                        return;
                    }
                }
                let p = PeekOverlay {
                    kind,
                    started_at: now,
                    updated_at: now,
                    duration_secs: 1.65,
                    display_level: target_lvl,
                };
                self.last_peek = Some(p.clone());
                self.peek_overlay = Some(p);
            }
            PeekKind::Brightness { level } => {
                let target_lvl = *level;
                if let Some(ref mut existing) = self.peek_overlay {
                    if matches!(existing.kind, PeekKind::Brightness { .. }) {
                        existing.kind = kind;
                        existing.updated_at = now;
                        existing.duration_secs = 1.65;
                        self.last_peek = self.peek_overlay.clone();
                        return;
                    }
                }
                let p = PeekOverlay {
                    kind,
                    started_at: now,
                    updated_at: now,
                    duration_secs: 1.65,
                    display_level: target_lvl,
                };
                self.last_peek = Some(p.clone());
                self.peek_overlay = Some(p);
            }
            PeekKind::DeviceConnect { .. } => {
                let p = PeekOverlay {
                    kind,
                    started_at: now,
                    updated_at: now,
                    duration_secs: 3.2,
                    display_level: 0.0,
                };
                self.last_peek = Some(p.clone());
                self.peek_overlay = Some(p);
            }
            PeekKind::Charging { .. } => {
                let p = PeekOverlay {
                    kind,
                    started_at: now,
                    updated_at: now,
                    duration_secs: 3.0,
                    display_level: 0.0,
                };
                self.last_peek = Some(p.clone());
                self.peek_overlay = Some(p);
            }
        }
    }

    pub fn dump_all_previews(&mut self) {
        let dir = "C:\\Users\\abufa\\.gemini\\antigravity-cli\\brain\\d3fb3b2e-6db7-4ab4-b33b-d2c255fd3495\\scratch";
        let _ = std::fs::create_dir_all(dir);

        // 1. Closed Idle
        self.is_hovered = false;
        self.mascot_enabled = false;
        self.peek_overlay = None;
        self.last_peek = None;
        self.current_body_w = COLLAPSED_W;
        self.current_h = COLLAPSED_H;
        self.resize_and_dump(&format!("{}\\dump_idle.raw", dir));

        // 2. Closed Mascot
        self.mascot_enabled = true;
        self.resize_and_dump(&format!("{}\\dump_mascot.raw", dir));
        self.mascot_enabled = false;

        // 3. Expanded Sections 0..5
        self.is_hovered = true;
        self.current_body_w = EXPANDED_W;
        self.current_h = EXPANDED_H;
        for sec in 0..6 {
            self.carousel.snap_to_section(sec);
            self.music_sec.show_lyrics = false;
            self.resize_and_dump(&format!("{}\\dump_sec_{}.raw", dir, sec));
        }

        // 4. Expanded Lyrics Mode
        self.carousel.snap_to_section(0);
        self.music_sec.show_lyrics = true;
        self.lyrics_w = 520.0;
        self.current_body_w = 520.0;
        self.resize_and_dump(&format!("{}\\dump_lyrics.raw", dir));

        // 5. Mid-transition preview between Section 0 (Music) and Section 1 (Focus)
        self.music_sec.show_lyrics = false;
        self.current_body_w = EXPANDED_W;
        self.carousel.current_section = 1;
        self.carousel.page_pos = 0.45;
        self.resize_and_dump(&format!("{}\\dump_transition_0_1.raw", dir));

        // 6. Minimized Notch (when inside an application: 44x4)
        self.is_hovered = false;
        self.auto_minimized = true;
        self.current_body_w = MINIMIZED_W;
        self.current_h = MINIMIZED_H;
        self.resize_and_dump(&format!("{}\\dump_minimized.raw", dir));
        self.auto_minimized = false;

        // 7. Volume Overlay (300x34)
        let now = Instant::now();
        self.current_body_w = PEEK_BAR_W;
        self.current_h = PEEK_BAR_H;
        let vol_peek = PeekOverlay {
            kind: PeekKind::Volume {
                level: 0.72,
                muted: false,
            },
            started_at: now,
            updated_at: now,
            duration_secs: 1.65,
            display_level: 0.72,
        };
        self.peek_overlay = Some(vol_peek.clone());
        self.last_peek = Some(vol_peek);
        self.resize_and_dump(&format!("{}\\dump_volume.raw", dir));

        // 8. Brightness Overlay (300x34)
        let br_peek = PeekOverlay {
            kind: PeekKind::Brightness { level: 0.80 },
            started_at: now,
            updated_at: now,
            duration_secs: 1.65,
            display_level: 0.80,
        };
        self.peek_overlay = Some(br_peek.clone());
        self.last_peek = Some(br_peek);
        self.resize_and_dump(&format!("{}\\dump_brightness.raw", dir));

        // 9. Device Connect / Headphones Overlay (.l-bt, 310x60)
        self.current_body_w = PEEK_CARD_W;
        self.current_h = PEEK_CARD_H;
        let bt_peek = PeekOverlay {
            kind: PeekKind::DeviceConnect {
                name: "AirBuds Pro".to_string(),
                connected: true,
                battery_pct: Some(70),
            },
            started_at: now,
            updated_at: now,
            duration_secs: 3.2,
            display_level: 0.70,
        };
        self.peek_overlay = Some(bt_peek.clone());
        self.last_peek = Some(bt_peek);
        self.resize_and_dump(&format!("{}\\dump_device_connect.raw", dir));

        // 10. Charging Overlay (.l-charge, 310x60)
        let chg_peek = PeekOverlay {
            kind: PeekKind::Charging {
                is_charging: true,
                pct: 70,
            },
            started_at: now,
            updated_at: now,
            duration_secs: 3.0,
            display_level: 0.70,
        };
        self.peek_overlay = Some(chg_peek.clone());
        self.last_peek = Some(chg_peek);
        self.resize_and_dump(&format!("{}\\dump_charging.raw", dir));
        self.peek_overlay = None;
        self.last_peek = None;
    }

    fn resize_and_dump(&mut self, path: &str) {
        let (_, ear) = Self::compute_radii(self.current_body_w, self.current_h);
        let win_w = (self.current_body_w + 2.0 * ear).round() as u32;
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
            // Only minimize for maximized / full-screen windows; restored (non-maximized) windows leave the notch as is.
            if !IsZoomed(fg).as_bool() {
                let mut rc = RECT::default();
                let covers_screen = GetWindowRect(fg, &mut rc).is_ok()
                    && rc.left <= self.screen_x
                    && rc.top <= self.screen_y
                    && rc.right >= self.screen_x + self.screen_w;
                if !covers_screen {
                    return Some(false);
                }
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

    fn poll_system_events(&mut self, now: Instant) {
        // 1. Check Bluetooth connection events from background watcher thread
        while let Ok((dev_name, connected)) = self.bt_rx.try_recv() {
            self.trigger_peek(PeekKind::DeviceConnect {
                name: dev_name,
                connected,
                battery_pct: Some(70),
            });
        }

        // 2. Check Volume keys or poll Audio Endpoint every 100ms
        let vol_key_pressed = unsafe {
            (GetAsyncKeyState(VK_VOLUME_UP.0 as i32) as u16 & 0x8000) != 0
                || (GetAsyncKeyState(VK_VOLUME_DOWN.0 as i32) as u16 & 0x8000) != 0
                || (GetAsyncKeyState(VK_VOLUME_MUTE.0 as i32) as u16 & 0x8000) != 0
        };

        if vol_key_pressed || now.duration_since(self.last_audio_poll).as_millis() >= 100 {
            self.last_audio_poll = now;
            if let Some(audio) = query_audio_state() {
                let vol_pct = (audio.volume * 100.0).round() as i32;
                if let Some((prev_pct, prev_muted)) = self.prev_volume {
                    if vol_pct != prev_pct || audio.muted != prev_muted || vol_key_pressed {
                        self.trigger_peek(PeekKind::Volume {
                            level: audio.volume,
                            muted: audio.muted,
                        });
                    }
                }
                self.prev_volume = Some((vol_pct, audio.muted));

                if let Some((ref prev_id, ref prev_name, prev_count)) = self.prev_audio_device {
                    if !audio.device_id.is_empty() && &audio.device_id != prev_id {
                        let connected = audio.active_count >= prev_count;
                        let display_name = if connected {
                            audio.device_name.clone()
                        } else {
                            prev_name.clone()
                        };
                        self.trigger_peek(PeekKind::DeviceConnect {
                            name: display_name,
                            connected,
                            battery_pct: if connected { Some(100) } else { None },
                        });
                    }
                }
                self.prev_audio_device =
                    Some((audio.device_id, audio.device_name, audio.active_count));
            }
        }
    }

    pub fn tick_frame(&mut self) {
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_frame_instant)
            .as_secs_f32()
            .clamp(0.001, 0.050);
        self.last_frame_instant = now;

        // 1. Check if user is in an application to auto-minimize the notch
        if let Some(in_app) = self.check_foreground_app() {
            self.auto_minimized = in_app;
        }
        let is_minimized_mode = self.minimal_notch || self.auto_minimized;

        // 2. Poll volume, audio device changes, and Bluetooth connections
        self.poll_system_events(now);

        // Per-sentence lyric bar: follows the active synced lyric line while music plays.
        {
            let track = self.media_service.get_info();
            self.music_sec.update_lyrics(&track);
            let new_line = self.music_sec.current_lyric_line(&track);
            if new_line != self.lyric_line {
                self.lyric_changed_at = now;
                if let Some(ref s) = new_line {
                    self.lyric_text_w = self.dwrite.measure_text_width(s, &self.dwrite.format_lyric_bar);
                }
                self.lyric_line = new_line;
            }
        }

        if self.carousel.current_section == 0 && self.music_sec.show_lyrics {
            let track = self.media_service.get_info();
            let pane = self.music_sec.lyrics_pane_width(&self.dwrite, &track);
            self.lyrics_w = (LYRICS_PANE_LEFT + pane + LYRICS_PANE_PAD).clamp(LYRICS_MIN_W, LYRICS_MAX_W);
        }

        // 3. Update active Peek Overlay timer & progress animation
        if let Some(ref mut peek) = self.peek_overlay {
            if now.duration_since(peek.updated_at).as_secs_f32() >= peek.duration_secs {
                self.peek_overlay = None;
            } else {
                match peek.kind {
                    PeekKind::Volume { level, muted } => {
                        let target = if muted { 0.0 } else { level };
                        peek.display_level += (target - peek.display_level) * (18.0 * dt).min(1.0);
                    }
                    PeekKind::Brightness { level } => {
                        peek.display_level += (level - peek.display_level) * (18.0 * dt).min(1.0);
                    }
                    PeekKind::DeviceConnect { battery_pct, .. } => {
                        let target = battery_pct.unwrap_or(100) as f32 / 100.0;
                        let elapsed = now.duration_since(peek.started_at).as_secs_f32();
                        let t = ((elapsed - 0.15) / 0.85).clamp(0.0, 1.0);
                        let ease = 1.0 - (1.0 - t).powi(3);
                        peek.display_level = target * ease;
                    }
                    PeekKind::Charging { pct, .. } => {
                        let target = (pct as f32 / 100.0).clamp(0.05, 1.0);
                        let elapsed = now.duration_since(peek.started_at).as_secs_f32();
                        let t = ((elapsed - 0.15) / 0.85).clamp(0.0, 1.0);
                        let ease = 1.0 - (1.0 - t).powi(3);
                        peek.display_level = target * ease;
                    }
                }
                self.last_peek = Some(peek.clone());
            }
        }

        // 4. Screen-space cursor hover check (matches nocan.original.exe 0x140024db4..0x140024e86)
        let mut cursor_pt = POINT::default();
        if unsafe { GetCursorPos(&mut cursor_pt).is_ok() } {
            let screen_cx = (self.screen_x as f32) + (self.screen_w as f32) * 0.5;
            let dx = (cursor_pt.x as f32 - screen_cx).abs();
            let dy = cursor_pt.y as f32 - (self.screen_y as f32);

            if !self.is_hovered {
                let min_hit_w = if is_minimized_mode && self.peek_overlay.is_none() && self.lyric_line.is_none() {
                    100.0f32
                } else {
                    COLLAPSED_W
                };
                let min_hit_h = if is_minimized_mode && self.peek_overlay.is_none() && self.lyric_line.is_none() {
                    10.0f32
                } else {
                    COLLAPSED_H
                };
                let half_w = self.current_body_w.max(min_hit_w) * 0.5 + 4.0;
                let max_y = self.current_h.max(min_hit_h) + 4.0;
                if dx <= half_w && dy >= -2.0 && dy <= max_y {
                    self.is_hovered = true;
                }
            } else {
                let target_w = if self.carousel.current_section == 0 && self.music_sec.show_lyrics {
                    self.lyrics_w
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

        let (target_body_w, target_h) = if self.is_hovered {
            let w = if self.carousel.current_section == 0 && self.music_sec.show_lyrics {
                self.lyrics_w
            } else {
                EXPANDED_W
            };
            (w, EXPANDED_H)
        } else if let Some(ref peek) = self.peek_overlay {
            match peek.kind {
                PeekKind::Volume { .. } | PeekKind::Brightness { .. } => (PEEK_BAR_W, PEEK_BAR_H),
                PeekKind::DeviceConnect { .. } | PeekKind::Charging { .. } => {
                    (PEEK_CARD_W, PEEK_CARD_H)
                }
            }
        } else if self.lyric_line.is_some() {
            ((self.lyric_text_w + 2.0 * LYRIC_BAR_PAD).clamp(LYRIC_BAR_MIN_W, LYRIC_BAR_MAX_W), COLLAPSED_H)
        } else if is_minimized_mode {
            (MINIMIZED_W, MINIMIZED_H)
        } else {
            (COLLAPSED_W, COLLAPSED_H)
        };

        // Sub-stepped second-order spring physics from nocan.original.exe (stiffness = 220.0, damping = 26.0)
        // Sub-stepping eliminates Euler integration overshoot/jitter when timer intervals vary.
        let k = 220.0f32;
        let d = 26.0f32;
        let mut rem_dt = dt;
        while rem_dt > 0.0001 {
            let step = rem_dt.min(0.004);
            rem_dt -= step;

            let fw = k * (target_body_w - self.current_body_w) - d * self.vel_w;
            self.vel_w += fw * step;
            self.current_body_w += self.vel_w * step;

            let fh = k * (target_h - self.current_h) - d * self.vel_h;
            self.vel_h += fh * step;
            self.current_h += self.vel_h * step;
        }

        if (self.current_body_w - target_body_w).abs() < 0.15 && self.vel_w.abs() < 0.8 {
            self.current_body_w = target_body_w;
            self.vel_w = 0.0;
        }
        if (self.current_h - target_h).abs() < 0.15 && self.vel_h.abs() < 0.8 {
            self.current_h = target_h;
            self.vel_h = 0.0;
        }

        if self.peek_overlay.is_none() && self.current_body_w <= 185.0 {
            self.last_peek = None;
        }

        self.update_window_rgn();

        // Update active sections
        self.focus_sec.update();
        self.openmaus_sec.refresh_if_needed();

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now_secs.saturating_sub(self.last_status_poll) >= 1 {
            self.last_status_poll = now_secs;
            let new_status = get_system_status();
            if let Some(was_charging) = self.prev_is_charging {
                if new_status.is_charging != was_charging {
                    self.trigger_peek(PeekKind::Charging {
                        is_charging: new_status.is_charging,
                        pct: new_status.battery_percent.unwrap_or(100) as u32,
                    });
                }
            }
            self.prev_is_charging = Some(new_status.is_charging);
            self.system_status = new_status;
        }

        self.render();
    }

    fn render(&mut self) {
        let body_w = self.current_body_w;
        let body_h = self.current_h;
        let (corner_r, ear) = Self::compute_radii(body_w, body_h);
        let body_left = self.current_body_left();
        let anim_time = self.start_instant.elapsed().as_secs_f32();

        self.d2d.begin_draw();
        self.d2d
            .clear(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });

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
            let min_h_t = ((body_h - 19.0) / 11.0).clamp(0.0, 1.0);
            let min_w_t = ((body_w - 115.0) / 35.0).clamp(0.0, 1.0);
            let min_fade = min_h_t.min(min_w_t);

            let has_peek = self.peek_overlay.is_some() || self.last_peek.is_some();
            let peek_suppress_idle = if has_peek {
                (1.0 - ((body_w - 160.0) / 35.0).clamp(0.0, 1.0)).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let lyric_active = self.lyric_line.is_some();
        let closed_alpha = if lyric_active { 0.0 } else { min_fade * peek_suppress_idle * (1.0 - open_t * 2.8).clamp(0.0, 1.0) };
            let open_alpha = ((open_t - 0.32) / 0.68).clamp(0.0, 1.0);

            // 1b. Draw Collapsed Bar (150x30) when visible
            if closed_alpha > 0.01 && body_h >= 19.0 {
                let needs_clip = closed_alpha < 0.995 || body_h < 29.8 || body_w < 149.5;
                let pushed = if needs_clip {
                    self.d2d.push_clipped_opacity_layer(
                        body_left,
                        body_w,
                        body_h,
                        corner_r,
                        ear,
                        closed_alpha,
                    )
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

            // 1b2. Draw the current lyric sentence (fades in on each change)
            if self.peek_overlay.is_none() && self.last_peek.is_none() && open_t < 0.2 {
                if let Some(ref line) = self.lyric_line {
                    let fade = (self.lyric_changed_at.elapsed().as_secs_f32() / 0.28).clamp(0.0, 1.0);
                    let w_t = ((body_w - 120.0) / 60.0).clamp(0.0, 1.0);
                    let alpha = fade * w_t;
                    if alpha > 0.01 {
                        let pushed = self.d2d.push_clipped_opacity_layer(body_left, body_w, body_h, corner_r, ear, alpha);
                        let rect = D2D_RECT_F {
                            left: body_left + LYRIC_BAR_PAD * 0.5,
                            top: 0.0,
                            right: body_left + body_w - LYRIC_BAR_PAD * 0.5,
                            bottom: body_h,
                        };
                        self.dwrite.draw_text(&self.d2d.context, line, rect, white_b, &self.dwrite.format_lyric_bar);
                        if pushed {
                            self.d2d.pop_layer();
                        }
                    }
                }
            }

            // 1c. Draw Animated Peek Overlays (Volume, Brightness, Device Connect, Charging)
            let active_peek = self.peek_overlay.as_ref().or(self.last_peek.as_ref()).cloned();
            if let Some(peek) = active_peek {
                let peek_w_t = ((body_w - 190.0) / 95.0).clamp(0.0, 1.0);
                let peek_h_fade = 1.0 - ((body_h - 68.0) / 35.0).clamp(0.0, 1.0);
                let peek_alpha = (peek_w_t * peek_h_fade).clamp(0.0, 1.0);
                if peek_alpha > 0.01 {
                    let pushed_peek = self.d2d.push_clipped_opacity_layer(
                        body_left,
                        body_w,
                        body_h,
                        corner_r,
                        ear,
                        peek_alpha,
                    );
                    self.draw_peek_overlay(
                        body_left,
                        body_w,
                        body_h,
                        &peek,
                        anim_time,
                        white_b,
                        dim_b,
                        bar_b,
                    );
                    if pushed_peek {
                        self.d2d.pop_layer();
                    }
                }
            }

            if open_alpha > 0.01 {
                let in_lyrics_mode =
                    self.carousel.current_section == 0 && self.music_sec.show_lyrics;

                // 2. Top Status Header (l-hdr): Battery Ring + Date & Time + 5-Bar Equalizer
                if !in_lyrics_mode {
                    let pushed_hdr = if open_alpha < 0.99 {
                        self.d2d.push_clipped_opacity_layer(
                            body_left,
                            body_w,
                            body_h,
                            corner_r,
                            ear,
                            open_alpha,
                        )
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
                        self.d2d.push_clipped_opacity_layer(
                            body_left,
                            body_w,
                            body_h,
                            corner_r,
                            ear,
                            sec_alpha,
                        )
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
                        self.d2d.push_clipped_opacity_layer(
                            body_left,
                            body_w,
                            body_h,
                            corner_r,
                            ear,
                            open_alpha,
                        )
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

    fn draw_peek_overlay(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        peek: &PeekOverlay,
        anim_time: f32,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
        bar_bg_brush: &ID2D1SolidColorBrush,
    ) {
        match &peek.kind {
            PeekKind::Volume { level, muted } => {
                let cy = body_h * 0.5;
                let icx = body_left + 22.0;

                // 1. Speaker Cone Polygon (exact nocan.original.exe 0x94180 vertices)
                let cone = [
                    (icx - 7.5, cy - 2.8),
                    (icx - 3.8, cy - 2.8),
                    (icx + 0.8, cy - 6.8),
                    (icx + 0.8, cy + 6.8),
                    (icx - 3.8, cy + 2.8),
                    (icx - 7.5, cy + 2.8),
                ];
                self.d2d.fill_polygon(&cone, white_brush);

                if *muted || *level <= 0.005 {
                    // Muted 'X' slash next to speaker cone (0x941bc..0x941ec)
                    self.d2d
                        .draw_line(icx + 3.0, cy - 3.0, icx + 7.4, cy + 3.0, white_brush, 1.7, true);
                    self.d2d
                        .draw_line(icx + 7.4, cy - 3.0, icx + 3.0, cy + 3.0, white_brush, 1.7, true);
                } else {
                    // Sound wave arcs
                    self.d2d
                        .draw_arc_segment(icx + 0.5, cy, 4.2, -42.0, 42.0, 1.6, white_brush);
                    if *level >= 0.40 {
                        self.d2d
                            .draw_arc_segment(icx + 0.5, cy, 7.2, -48.0, 48.0, 1.6, white_brush);
                    }
                }

                // 2. Smoothly Animated Progress Track & Fill Bar
                let bar_left = body_left + 42.0;
                let bar_w = (body_w - 94.0).max(40.0);
                let track_rect = D2D_RECT_F {
                    left: bar_left,
                    top: cy - 2.5,
                    right: bar_left + bar_w,
                    bottom: cy + 2.5,
                };
                self.d2d
                    .fill_rounded_rect(track_rect, 2.5, 2.5, bar_bg_brush);

                let fill_ratio = peek.display_level.clamp(0.0, 1.0);
                if fill_ratio > 0.005 {
                    let fill_rect = D2D_RECT_F {
                        left: bar_left,
                        top: cy - 2.5,
                        right: bar_left + bar_w * fill_ratio,
                        bottom: cy + 2.5,
                    };
                    self.d2d
                        .fill_rounded_rect(fill_rect, 2.5, 2.5, white_brush);
                }

                // 3. Percentage Label on Right
                let pct_str = if *muted {
                    "Mute".to_string()
                } else {
                    format!("{}%", (*level * 100.0).round() as i32)
                };
                let text_rect = D2D_RECT_F {
                    left: bar_left + bar_w + 4.0,
                    top: 0.0,
                    right: body_left + body_w - 15.0,
                    bottom: body_h,
                };
                self.dwrite.draw_text(
                    &self.d2d.context,
                    &pct_str,
                    text_rect,
                    white_brush,
                    &self.dwrite.format_regular_semibold_right,
                );
            }

            PeekKind::Brightness { level } => {
                let cy = body_h * 0.5;
                let icx = body_left + 22.0;

                // 1. Sun Icon (core + 8 radial rays)
                self.d2d.fill_ellipse(icx, cy, 3.6, 3.6, white_brush);
                let ray_outer = 6.8 + 1.0 * peek.display_level.clamp(0.0, 1.0);
                for i in 0..8 {
                    let angle = (i as f32) * std::f32::consts::FRAC_PI_4;
                    let c = angle.cos();
                    let s = angle.sin();
                    self.d2d.draw_line(
                        icx + 5.2 * c,
                        cy + 5.2 * s,
                        icx + ray_outer * c,
                        cy + ray_outer * s,
                        white_brush,
                        1.5,
                        true,
                    );
                }

                // 2. Smoothly Animated Progress Track & Fill Bar
                let bar_left = body_left + 42.0;
                let bar_w = (body_w - 94.0).max(40.0);
                let track_rect = D2D_RECT_F {
                    left: bar_left,
                    top: cy - 2.5,
                    right: bar_left + bar_w,
                    bottom: cy + 2.5,
                };
                self.d2d
                    .fill_rounded_rect(track_rect, 2.5, 2.5, bar_bg_brush);

                let fill_ratio = peek.display_level.clamp(0.0, 1.0);
                if fill_ratio > 0.005 {
                    let fill_rect = D2D_RECT_F {
                        left: bar_left,
                        top: cy - 2.5,
                        right: bar_left + bar_w * fill_ratio,
                        bottom: cy + 2.5,
                    };
                    self.d2d
                        .fill_rounded_rect(fill_rect, 2.5, 2.5, white_brush);
                }

                // 3. Percentage Label on Right
                let pct_str = format!("{}%", (*level * 100.0).round() as i32);
                let text_rect = D2D_RECT_F {
                    left: bar_left + bar_w + 4.0,
                    top: 0.0,
                    right: body_left + body_w - 15.0,
                    bottom: body_h,
                };
                self.dwrite.draw_text(
                    &self.d2d.context,
                    &pct_str,
                    text_rect,
                    white_brush,
                    &self.dwrite.format_regular_semibold_right,
                );
            }

            PeekKind::DeviceConnect {
                name,
                connected,
                battery_pct,
            } => {
                // Exact .l-bt layout from web_nocan.html (lines 798-805) & nocan.original.exe (0x14004347b):
                // 310x60 card, left ring at (35, 30), labels at left=64 (top 22 & 40), right headphones badge at (275, 30)
                let ring_cx = body_left + 35.0;
                let ring_cy = 30.0;

                // 1. Left .pk-ring (r=11, stroke=5, track rgba(255,255,255,0.22), arc #5ec2ff)
                if let (Ok(track_b), Ok(arc_b)) = (
                    self.d2d.create_solid_brush(D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 0.22,
                    }),
                    self.d2d.create_solid_brush(D2D1_COLOR_F {
                        r: 94.0 / 255.0,
                        g: 194.0 / 255.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                ) {
                    let progress = if self.dump_next_frame.is_some() {
                        0.70
                    } else if *connected {
                        peek.display_level.clamp(0.05, 1.0)
                    } else {
                        0.0
                    };
                    self.d2d.draw_progress_ring(
                        ring_cx,
                        ring_cy,
                        11.0,
                        4.8,
                        progress,
                        &track_b,
                        &arc_b,
                    );
                }

                // Bluetooth rune icon inside .pk-ring at (35, 30) (viewBox 0 0 24 24: m7 7 10 10-5 4V3l5 4L7 17, width 12x12)
                let bt_pts = [
                    (ring_cx - 2.6, ring_cy - 2.4),
                    (ring_cx + 2.6, ring_cy + 2.4),
                    (ring_cx, ring_cy + 4.8),
                    (ring_cx, ring_cy - 4.8),
                    (ring_cx + 2.6, ring_cy - 2.4),
                    (ring_cx - 2.6, ring_cy + 2.4),
                ];
                self.d2d.draw_polyline(&bt_pts, white_brush, 1.4, false);

                // 2. Center Device Name & Connection Status
                let title_rect = D2D_RECT_F {
                    left: body_left + 64.0,
                    top: 11.0,
                    right: body_left + body_w - 64.0,
                    bottom: 33.0,
                };
                self.dwrite.draw_text(
                    &self.d2d.context,
                    name,
                    title_rect,
                    white_brush,
                    &self.dwrite.format_title,
                );

                let sub_str = if *connected {
                    if let Some(pct) = battery_pct {
                        format!("Connected \u{00b7} {}%", pct)
                    } else {
                        "Connected".to_string()
                    }
                } else {
                    "Disconnected".to_string()
                };
                let sub_rect = D2D_RECT_F {
                    left: body_left + 64.0,
                    top: 30.0,
                    right: body_left + body_w - 64.0,
                    bottom: 50.0,
                };
                self.dwrite.draw_text(
                    &self.d2d.context,
                    &sub_str,
                    sub_rect,
                    dim_brush,
                    &self.dwrite.format_sub,
                );

                // 3. Right Headphones Badge at (body_left + body_w - 35.0, 30.0)
                let badge_cx = body_left + body_w - 35.0;
                let badge_cy = 30.0;
                let pulse = if self.dump_next_frame.is_some() {
                    0.0
                } else {
                    (anim_time * 3.2).sin() * 1.2
                };

                if let (Ok(outer_glow), Ok(mid_ring), Ok(inner_bg), Ok(inner_stroke)) = (
                    self.d2d.create_solid_brush(D2D1_COLOR_F {
                        r: 94.0 / 255.0,
                        g: 194.0 / 255.0,
                        b: 1.0,
                        a: 0.08,
                    }),
                    self.d2d.create_solid_brush(D2D1_COLOR_F {
                        r: 94.0 / 255.0,
                        g: 194.0 / 255.0,
                        b: 1.0,
                        a: 0.35,
                    }),
                    self.d2d.create_solid_brush(D2D1_COLOR_F {
                        r: 13.0 / 255.0,
                        g: 26.0 / 255.0,
                        b: 38.0 / 255.0,
                        a: 1.0,
                    }),
                    self.d2d.create_solid_brush(D2D1_COLOR_F {
                        r: 94.0 / 255.0,
                        g: 194.0 / 255.0,
                        b: 1.0,
                        a: 0.55,
                    }),
                ) {
                    self.d2d
                        .draw_ellipse(badge_cx, badge_cy, 23.0 + pulse, 23.0 + pulse, &outer_glow, 5.0);
                    self.d2d
                        .draw_ellipse(badge_cx, badge_cy, 20.0, 20.0, &mid_ring, 1.5);
                    self.d2d
                        .fill_ellipse(badge_cx, badge_cy, 15.5, 15.5, &inner_bg);
                    self.d2d
                        .draw_ellipse(badge_cx, badge_cy, 14.8, 14.8, &inner_stroke, 1.5);
                }

                // White Headphones SVG (16x16 at badge_cx, badge_cy)
                self.d2d.draw_arc_segment(
                    badge_cx,
                    badge_cy + 0.5,
                    5.3,
                    180.0,
                    360.0,
                    1.5,
                    white_brush,
                );
                self.d2d.draw_line(
                    badge_cx - 5.3,
                    badge_cy + 0.5,
                    badge_cx - 5.3,
                    badge_cy + 4.2,
                    white_brush,
                    1.5,
                    true,
                );
                self.d2d.draw_line(
                    badge_cx + 5.3,
                    badge_cy + 0.5,
                    badge_cx + 5.3,
                    badge_cy + 4.2,
                    white_brush,
                    1.5,
                    true,
                );
                let left_cup = D2D_RECT_F {
                    left: badge_cx - 5.6,
                    top: badge_cy + 1.2,
                    right: badge_cx - 3.1,
                    bottom: badge_cy + 5.2,
                };
                let right_cup = D2D_RECT_F {
                    left: badge_cx + 3.1,
                    top: badge_cy + 1.2,
                    right: badge_cx + 5.6,
                    bottom: badge_cy + 5.2,
                };
                self.d2d.fill_rounded_rect(left_cup, 1.0, 1.0, white_brush);
                self.d2d.fill_rounded_rect(right_cup, 1.0, 1.0, white_brush);
            }

            PeekKind::Charging { is_charging, pct } => {
                // Exact .l-charge layout from web_nocan.html (lines 808-814) & nocan.original.exe (0x14003b5dc)
                let ring_cx = body_left + 35.0;
                let ring_cy = 30.0;

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
                    let progress = if self.dump_next_frame.is_some() {
                        (*pct as f32 / 100.0).clamp(0.05, 1.0)
                    } else {
                        peek.display_level.clamp(0.05, 1.0)
                    };
                    self.d2d.draw_progress_ring(
                        ring_cx,
                        ring_cy,
                        11.0,
                        4.8,
                        progress,
                        &track_b,
                        &arc_b,
                    );
                }

                // Lightning bolt inside .pk-ring at (35, 30) (viewBox 0 0 10 14: M6 0 0 8h4l-1 6 6-8H5z)
                let bolt = [
                    (ring_cx + 1.1, ring_cy - 6.5),
                    (ring_cx - 4.5, ring_cy + 0.9),
                    (ring_cx - 0.5, ring_cy + 0.9),
                    (ring_cx - 1.5, ring_cy + 6.5),
                    (ring_cx + 4.5, ring_cy - 0.9),
                    (ring_cx + 0.3, ring_cy - 0.9),
                ];
                self.d2d.fill_polygon(&bolt, white_brush);

                // Center Title & Subtitle
                let title = if *is_charging {
                    "Charging"
                } else {
                    "Charger unplugged"
                };
                let subtitle = if *is_charging {
                    "Charger connected"
                } else {
                    "Running on battery"
                };
                let title_rect = D2D_RECT_F {
                    left: body_left + 64.0,
                    top: 11.0,
                    right: body_left + body_w - 82.0,
                    bottom: 33.0,
                };
                self.dwrite.draw_text(
                    &self.d2d.context,
                    title,
                    title_rect,
                    white_brush,
                    &self.dwrite.format_title,
                );
                let sub_rect = D2D_RECT_F {
                    left: body_left + 64.0,
                    top: 30.0,
                    right: body_left + body_w - 82.0,
                    bottom: 50.0,
                };
                self.dwrite.draw_text(
                    &self.d2d.context,
                    subtitle,
                    sub_rect,
                    dim_brush,
                    &self.dwrite.format_sub,
                );

                // Right 24px Battery Percentage at right: 22px, top: 30px
                let pct_str = format!("{}%", pct);
                let pct_rect = D2D_RECT_F {
                    left: body_left + body_w - 95.0,
                    top: 0.0,
                    right: body_left + body_w - 22.0,
                    bottom: body_h,
                };
                self.dwrite.draw_text(
                    &self.d2d.context,
                    &pct_str,
                    pct_rect,
                    white_brush,
                    &self.dwrite.format_peek_pct,
                );
            }
        }
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
        let cx = body_left + body_w * 0.5;

        // 1. Battery Ring at left: 20px, top: 15px
        self.draw_battery_ring(body_left + 20.0, cy, white_brush);

        // 2. Clock Time centered at cx with fixed ±36px bounds so shrinking never clips text into ellipsis
        let clock_rect = D2D_RECT_F {
            left: cx - 36.0,
            top: 0.0,
            right: cx + 36.0,
            bottom: body_h,
        };
        self.dwrite.draw_text(
            &self.d2d.context,
            &self.system_status.clock_str,
            clock_rect,
            white_brush,
            &self.dwrite.format_idle_time,
        );

        // 3. 5-bar Equalizer anchored 19px from right edge (body_left + 131px when body_w == 150)
        self.d2d.draw_equalizer_bars(
            body_left + body_w - 19.0,
            cy,
            track.is_playing,
            anim_time,
            white_brush,
        );
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
                self.d2d
                    .fill_ellipse_radial(ex, ey, glow_rx, glow_ry, &rad_glow);
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

        let body_left = self.current_body_left();
        let body_w = self.current_body_w;
        let body_h = self.current_h;
        let in_lyrics_mode = self.carousel.current_section == 0 && self.music_sec.show_lyrics;

        if !in_lyrics_mode {
            if self
                .carousel
                .nav
                .hit_test_prev(body_left, body_w, body_h, x, y, self.carousel.show_prev())
                || self
                    .carousel
                    .nav
                    .hit_test_next(body_left, body_w, body_h, x, y, self.carousel.show_next())
            {
                return true;
            }
            if self
                .carousel
                .dots
                .hit_test(body_left, body_w, body_h, x, y)
                .is_some()
            {
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
            WM_POWERBROADCAST => {
                if wparam.0 as u32 == PBT_POWERSETTINGCHANGE && lparam.0 != 0 {
                    let setting = &*(lparam.0 as *const POWERBROADCAST_SETTING);
                    if setting.PowerSetting == GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS
                        && setting.DataLength >= 4
                    {
                        let br_pct =
                            *(setting.Data.as_ptr() as *const u32).clamp(&0, &100);
                        if !notch.initial_brightness_received {
                            notch.initial_brightness_received = true;
                            notch.prev_brightness = Some(br_pct);
                        } else if notch.prev_brightness != Some(br_pct) {
                            notch.prev_brightness = Some(br_pct);
                            notch.trigger_peek(PeekKind::Brightness {
                                level: (br_pct as f32 / 100.0).clamp(0.0, 1.0),
                            });
                        }
                    }
                }
                return LRESULT(1);
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
                let body_left = notch.current_body_left();
                let body_w = notch.current_body_w;
                let body_h = notch.current_h;
                let show_prev = notch.carousel.show_prev();
                let show_next = notch.carousel.show_next();
                notch.carousel.nav.hovered_prev =
                    notch
                        .carousel
                        .nav
                        .hit_test_prev(body_left, body_w, body_h, x, y, show_prev);
                notch.carousel.nav.hovered_next =
                    notch
                        .carousel
                        .nav
                        .hit_test_next(body_left, body_w, body_h, x, y, show_next);
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

                let body_left = notch.current_body_left();
                let body_w = notch.current_body_w;
                let body_h = notch.current_h;
                let in_lyrics_mode =
                    notch.carousel.current_section == 0 && notch.music_sec.show_lyrics;

                if !in_lyrics_mode {
                    let show_prev = notch.carousel.show_prev();
                    let show_next = notch.carousel.show_next();
                    if notch
                        .carousel
                        .nav
                        .hit_test_prev(body_left, body_w, body_h, x, y, show_prev)
                    {
                        notch.carousel.prev();
                        return LRESULT(0);
                    }
                    if notch
                        .carousel
                        .nav
                        .hit_test_next(body_left, body_w, body_h, x, y, show_next)
                    {
                        notch.carousel.next();
                        return LRESULT(0);
                    }
                    if let Some(idx) = notch
                        .carousel
                        .dots
                        .hit_test(body_left, body_w, body_h, x, y)
                    {
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

                        let auto_flag = if notch.autostart_enabled {
                            MF_CHECKED
                        } else {
                            MF_UNCHECKED
                        };
                        let mascot_flag = if notch.mascot_enabled {
                            MF_CHECKED
                        } else {
                            MF_UNCHECKED
                        };
                        let min_flag = if notch.minimal_notch {
                            MF_CHECKED
                        } else {
                            MF_UNCHECKED
                        };

                        let _ = AppendMenuW(
                            hmenu,
                            MF_STRING | auto_flag,
                            IDM_AUTOSTART,
                            PCWSTR(auto_w.as_ptr()),
                        );
                        let _ = AppendMenuW(
                            hmenu,
                            MF_STRING | mascot_flag,
                            IDM_MASCOT,
                            PCWSTR(mascot_w.as_ptr()),
                        );
                        let _ = AppendMenuW(
                            hmenu,
                            MF_STRING | min_flag,
                            IDM_MINIMAL,
                            PCWSTR(min_w.as_ptr()),
                        );
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
                                let _ = MessageBoxW(
                                    hwnd,
                                    PCWSTR(msg.as_ptr()),
                                    PCWSTR(cap.as_ptr()),
                                    MB_OK | MB_ICONINFORMATION,
                                );
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
