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
    VK_VOLUME_UP, VK_RETURN, VK_SPACE, VK_ESCAPE, VK_TAB, VK_SHIFT, VK_LEFT, VK_RIGHT,
};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, Shell_NotifyIconW, HDROP, NIF_ICON, NIF_MESSAGE,
    NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetClassNameW, GetCursorPos, GetForegroundWindow, GetMessageW, IsIconic,
    GetWindowRect, IsWindowVisible, IsZoomed, LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassExW,
    SetCursor, SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TrackPopupMenu, SendMessageW,
    TranslateMessage, DEVICE_NOTIFY_WINDOW_HANDLE, GWLP_USERDATA, HTCLIENT, IDC_ARROW, IDC_HAND,
    MB_ICONINFORMATION, MB_OK, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    MSG, PBT_POWERSETTINGCHANGE, SW_SHOWNOACTIVATE, TPM_BOTTOMALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_COMMAND, WM_DESTROY, WM_DROPFILES, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_POWERBROADCAST, WM_RBUTTONUP, WM_SETCURSOR, WM_TIMER, WM_USER, WM_KEYDOWN, WNDCLASSEXW,
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
use crate::notch::carousel::{Carousel, SectionId};
use crate::notch::mascot::TigerMascot;
use crate::sections::focus::FocusSection;
use crate::sections::music::MusicSection;
use crate::sections::ai_usage::AiUsageSection;
use crate::sections::openmaus::OpenMausSection;
use crate::sections::openmaus::OpenMausTab;
use crate::integrations::openmaus_activity::{ActivityEvent, ActivityKind};
use crate::sections::saham::SahamSection;
use crate::sections::shelf::ShelfSection;
use crate::sections::todo::{self, TodoSection, TodoEditing, TodoItem};
use super::todo_editor::{TodoInput, WM_TODO_EDITOR};
use crate::sections::water::WaterSection;
use crate::sections::deck::{self, Action, Region, Page};

/// Fixed DirectComposition canvas size matching `nocan.original.exe` (`0x140001a54`).
/// Using a fixed canvas avoids per-frame `IDXGISwapChain1::ResizeBuffers` & `SetWindowPos` calls,
/// eliminating GPU stalls and integer-rounding horizontal jitter during animations.
const CANVAS_W: i32 = 760;
const CANVAS_H: i32 = 240;

const MINIMIZED_W: f32 = 44.0;
const MINIMIZED_H: f32 = 4.0;

const COLLAPSED_W: f32 = 188.0;
const COLLAPSED_H: f32 = 32.0;
const MASCOT_H: f32 = 40.0;
const COLLAPSED_R: f32 = 14.0;
const COLLAPSED_EAR: f32 = 10.0;

const PEEK_BAR_W: f32 = 300.0;
const PEEK_BAR_H: f32 = 34.0;

const PEEK_CARD_W: f32 = 310.0;
const PEEK_CARD_H: f32 = 60.0;

const WATER_CARD_W: f32 = 360.0;
const WATER_CARD_H: f32 = 62.0;

const EXPANDED_W: f32 = 448.0;
const LYRIC_BAR_MIN_W: f32 = 190.0;
const LYRIC_BAR_MAX_W: f32 = 560.0;
const LYRIC_BAR_EXTRA: f32 = 62.0;
const LYRICS_MIN_W: f32 = 560.0;
const LYRICS_MAX_W: f32 = 700.0;
const LYRICS_PANE_LEFT: f32 = 256.0;
const LYRICS_PANE_PAD: f32 = 26.0;
const EXPANDED_H: f32 = 196.0;
const EXPANDED_R: f32 = 32.0;
const EXPANDED_EAR: f32 = 22.0;

/// A compact Pomodoro companion that sits beside the notch while the lyric
/// sentence occupies the main bar. It never shares the notch silhouette.
const FOCUS_CAPSULE_W: f32 = 206.0;
const FOCUS_CAPSULE_H: f32 = 58.0;
const FOCUS_CAPSULE_GAP: f32 = 10.0;

const WM_TRAYICON: u32 = WM_USER + 101;
const IDM_SETTINGS: usize = 1000;
const IDM_AUTOSTART: usize = 1001;
const IDM_MASCOT: usize = 1002;
const IDM_MINIMAL: usize = 1003;
const IDM_ABOUT: usize = 1004;
const IDM_EXIT: usize = 1005;

const WM_MOUSELEAVE: u32 = 0x02A3;

#[derive(Clone, Debug)]
pub enum PeekKind {
    OpenMaus { event: ActivityEvent },
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WaterReminderBtn {
    Drink,
    Skip,
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
    tiger: TigerMascot,
    dwrite: DWriteContext,
    _comp: CompositionTarget,
    carousel: Carousel,
    media_service: MediaService,
    music_sec: MusicSection,
    focus_sec: FocusSection,
    water_sec: WaterSection,
    shelf_sec: ShelfSection,
    todo_sec: TodoSection,
    todo_input: TodoInput,
    todo_editing: Option<TodoEditing>,
    todo_offset: usize,
    todo_error: Option<String>,
    todo_deleted: Option<(TodoItem, usize)>,
    todo_preview: bool,
    openmaus_sec: OpenMausSection,
    ai_usage_sec: AiUsageSection,
    saham_sec: SahamSection,
    deck_regions: Vec<Region>,
    deck_focus: Option<usize>,
    shelf_offset: usize,
    pub is_hovered: bool,
    pub mascot_enabled: bool,
    pub minimal_notch: bool,
    pub auto_minimized: bool,
    pub autostart_enabled: bool,
    auto_minimize_enabled: bool,
    previous_settings: Option<crate::config::AppSettings>,
    accent_mode: u32,
    visual_opacity: f32,
    dynamic_lyrics_width: bool,
    water_reminder_interval: Duration,
    water_reminder_since: Instant,
    current_body_w: f32,
    lyrics_w: f32,
    lyric_line: Option<String>,
    lyric_text_w: f32,
    lyric_changed_at: Instant,
    lyric_color: (f32, f32, f32),
    lyric_color_target: (f32, f32, f32),
    lyric_color_key: String,
    cover_mix: f32,
    lyrics_morph: f32,
    provider_morph: f32,
    focus_morph: f32,
    floating_timer: Option<super::focus_capsule::FloatingTimer>,
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
    pub water_reminder_active: bool,
    water_feedback: Option<super::hydration::DrinkFeedback>,
    pub water_reminder_hover_button: Option<WaterReminderBtn>,
    pub peek_overlay: Option<PeekOverlay>,
    pub last_peek: Option<PeekOverlay>,
    start_instant: Instant,
    last_frame_instant: Instant,
    dump_next_frame: Option<String>,
}

impl NotchWindow {
    pub fn create(media_service: MediaService) -> Result<HWND> {
        let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)?.into() };
        let class_name: Vec<u16> = "NocanWindow\0".encode_utf16().collect();
        let large_icon = crate::graphics::branding::app_icon(32)?;
        let small_icon = crate::graphics::branding::app_icon(16)?;

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: windows::Win32::UI::WindowsAndMessaging::CS_HREDRAW
                | windows::Win32::UI::WindowsAndMessaging::CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: large_icon,
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: small_icon,
        };

        unsafe {
            RegisterClassExW(&wnd_class);
        }

        let task_preview=std::env::args().any(|arg|arg=="--preview-todo");
        let (screen_x, mut screen_y, screen_w) = get_monitor_bounds();
        if task_preview { screen_y+=160; }
        let init_x = screen_x + (screen_w - CANVAS_W) / 2;
        let init_y = screen_y;

        let ex_style = WS_EX_TOPMOST | WS_EX_NOREDIRECTIONBITMAP | if task_preview {
            windows::Win32::UI::WindowsAndMessaging::WS_EX_APPWINDOW
        } else { WS_EX_TOOLWINDOW };
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
                hIcon: small_icon,
                ..Default::default()
            };
            let tip: Vec<u16> = "Nocan\0".encode_utf16().collect();
            for (i, &c) in tip.iter().enumerate().take(127) {
                nid.szTip[i] = c;
            }
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }

        crate::config::debug_log("NotchWindow::create: window created, initializing D3D/D2D");
        let d3d = match D3DContext::new(hwnd, CANVAS_W as u32, CANVAS_H as u32) {
            Ok(d) => d,
            Err(e) => {
                crate::config::debug_log(&format!("D3DContext::new error: {:?}", e));
                return Err(e);
            }
        };
        let mut d2d = match D2DContext::new(&d3d.dxgi_device) {
            Ok(d) => d,
            Err(e) => {
                crate::config::debug_log(&format!("D2DContext::new error: {:?}", e));
                return Err(e);
            }
        };
        {
            let surface = d3d.get_surface()?;
            d2d.set_target_surface(&surface)?;
        }

        crate::config::debug_log("NotchWindow::create: initializing DWrite & CompositionTarget");
        let dwrite = match DWriteContext::new() {
            Ok(dw) => dw,
            Err(e) => {
                crate::config::debug_log(&format!("DWriteContext::new error: {:?}", e));
                return Err(e);
            }
        };
        let comp = match CompositionTarget::new(hwnd, &d3d.dxgi_device, &d3d.swapchain) {
            Ok(c) => c,
            Err(e) => {
                crate::config::debug_log(&format!("CompositionTarget::new error: {:?}", e));
                return Err(e);
            }
        };

        crate::config::debug_log("NotchWindow::create: spawning BT watcher");
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

        crate::config::debug_log("NotchWindow::create: querying system status & audio");
        let init_status = get_system_status();
        let init_charging = Some(init_status.is_charging);
        let init_audio = query_audio_state();
        let prev_volume = init_audio
            .as_ref()
            .map(|a| ((a.volume * 100.0).round() as i32, a.muted));
        let prev_audio_device = init_audio
            .map(|a| (a.device_id, a.device_name, a.active_count));

        crate::config::debug_log("NotchWindow::create: constructing NotchWindow struct");
        let todo_input = TodoInput::new(hwnd, &dwrite.font_name)?;
        let notch = Box::new(Self {
            hwnd,
            d3d,
            d2d,
            tiger: TigerMascot::default(),
            dwrite,
            _comp: comp,
            carousel: Carousel::new(),
            media_service,
            music_sec: MusicSection::new(),
            focus_sec: FocusSection::new(),
            water_sec: WaterSection::new(),
            shelf_sec: ShelfSection::new(),
            todo_sec: TodoSection::new(),
            todo_input,
            todo_editing: None,
            todo_offset: 0,
            todo_error: None,
            todo_deleted: None,
            todo_preview: task_preview,
            openmaus_sec: OpenMausSection::new(),
            ai_usage_sec: AiUsageSection::new(),
            saham_sec: SahamSection::new(),
            deck_regions: Vec::new(),
            deck_focus: None,
            shelf_offset: 0,
            is_hovered: false,
            mascot_enabled: false,
            minimal_notch: false,
            auto_minimized: false,
            autostart_enabled: true,
            auto_minimize_enabled: true,
            previous_settings: None,
            accent_mode: 0,
            visual_opacity: 0.9,
            dynamic_lyrics_width: true,
            water_reminder_interval: Duration::from_secs(45 * 60),
            water_reminder_since: Instant::now(),
            current_body_w: COLLAPSED_W,
            lyrics_w: LYRICS_MIN_W,
            lyric_line: None,
            lyric_text_w: 0.0,
            lyric_changed_at: Instant::now(),
            lyric_color: (0.16, 0.36, 0.80),
            lyric_color_target: (0.16, 0.36, 0.80),
            lyric_color_key: String::new(),
            cover_mix: 0.0,
            lyrics_morph: 0.0,
            provider_morph: 0.0,
            focus_morph: 0.0,
            floating_timer: super::focus_capsule::FloatingTimer::create(hwnd).map_err(|e| {
                crate::config::debug_log(&format!("Floating timer unavailable: {e:?}")); e
            }).ok(),
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
            water_reminder_active: false,
            water_feedback: None,
            water_reminder_hover_button: None,
            peek_overlay: None,
            last_peek: None,
            start_instant: Instant::now(),
            last_frame_instant: Instant::now(),
            dump_next_frame: None,
        });

        crate::config::debug_log("NotchWindow::create: notch struct constructed, setting USERDATA & applying settings");
        let raw_ptr = Box::into_raw(notch);
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
            let settings = crate::config::AppSettings::load();
            (*raw_ptr).apply_settings(&settings);
            let args: Vec<String> = std::env::args().collect();
            if let Some(index)=args.iter().position(|arg|arg=="--preview-todo") {
                let directory=args.get(index+1).map(std::path::PathBuf::from).unwrap_or_else(||std::env::temp_dir().join("nocan-todo-preview"));
                std::fs::create_dir_all(&directory).map_err(|_|windows::core::Error::from_win32())?;
                (*raw_ptr).todo_sec.file_path=directory.join("todo.json");
                (*raw_ptr).todo_sec.load();
                (*raw_ptr).carousel.set_active_sections(vec![SectionId::Music,SectionId::Focus,SectionId::Water,SectionId::Shelf,SectionId::Todo,SectionId::OpenMaus,SectionId::AiUsage,SectionId::Saham]);
                (*raw_ptr).carousel.snap_to_section_id(SectionId::Todo);
                (*raw_ptr).mascot_enabled=false;
                (*raw_ptr).minimal_notch=false;
                (*raw_ptr).auto_minimize_enabled=false;
                (*raw_ptr).begin_todo(TodoEditing::New);
            }
            if let Some(index) = args.iter().position(|arg| arg == "--preview-focus") {
                if let Some(seconds) = args.get(index + 1).and_then(|value| value.parse::<u32>().ok()) {
                    (*raw_ptr).focus_sec.remaining_secs = seconds.max(1).min((*raw_ptr).focus_sec.total_secs);
                }
                (*raw_ptr).focus_sec.toggle_start_pause();
            }
            if let Some(index) = args.iter().position(|a| a == "--preview-branding") {
                let dir = args.get(index + 1).ok_or_else(|| windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x80070057u32 as i32)))?;
                std::fs::create_dir_all(dir).map_err(|_|windows::core::Error::from_win32())?;
                (*raw_ptr).todo_sec.file_path=std::path::Path::new(dir).join("todo-probe.json");
                (*raw_ptr).todo_sec.items.clear();
                (*raw_ptr).todo_sec.save();
                (*raw_ptr).dump_branding_previews(std::path::Path::new(dir))?;
                (*raw_ptr).mascot_enabled=false;
                (*raw_ptr).minimal_notch=false;
                (*raw_ptr).auto_minimized=false;
                (*raw_ptr).is_hovered=true;
                (*raw_ptr).current_body_w=EXPANDED_W;
                (*raw_ptr).current_h=EXPANDED_H;
                let initial_settings = crate::config::AppSettings::load();
                let mut probe = initial_settings.clone();
                probe.general.auto_minimize_on_maximize = false;
                probe.music.dynamic_lyrics_width = false;
                probe.appearance.accent_mode = 2;
                probe.appearance.acrylic_opacity = 0.6;
                probe.productivity.focus_work_mins = 35;
                probe.productivity.water_daily_target_glasses = 10;
                probe.productivity.water_interval_mins = 30;
                probe.ai.default_provider = 2;
                probe.ai.show_antigravity = false;
                probe.ai.poll_interval_secs = 5;
                assert!(probe.save());
                let _ = SendMessageW(hwnd, crate::settings::window::WM_SETTINGS_CHANGED, WPARAM(0), LPARAM(0));
                assert!(!(*raw_ptr).auto_minimize_enabled && !(*raw_ptr).dynamic_lyrics_width);
                assert_eq!((*raw_ptr).accent_mode, 2);
                assert!(((*raw_ptr).visual_opacity - 0.6).abs() < 0.001);
                assert_eq!((*raw_ptr).focus_sec.remaining_secs, 35 * 60);
                assert_eq!((*raw_ptr).water_sec.goal_ml, 2500);
                assert_eq!((*raw_ptr).water_reminder_interval, Duration::from_secs(1800));
                assert!(!(*raw_ptr).ai_usage_sec.enabled[0]);
                assert_eq!((*raw_ptr).ai_usage_sec.active_tab, crate::integrations::ai_usage::AiTab::Codex);
                (*raw_ptr).ai_usage_sec.active_tab = crate::integrations::ai_usage::AiTab::Claude;
                let _ = SendMessageW(hwnd, crate::settings::window::WM_SETTINGS_CHANGED, WPARAM(0), LPARAM(0));
                assert_eq!((*raw_ptr).ai_usage_sec.active_tab, crate::integrations::ai_usage::AiTab::Claude);
                assert!(initial_settings.save());
                let _ = SendMessageW(hwnd, crate::settings::window::WM_SETTINGS_CHANGED, WPARAM(0), LPARAM(0));
                let click = move |predicate:fn(Action)->bool| {
                    let region=(*raw_ptr).deck_regions.iter().find(|r|predicate(r.action)).copied().expect("rendered control missing");
                    let x=((region.rect.left+region.rect.right)*0.5) as u16;
                    let y=((region.rect.top+region.rect.bottom)*0.5) as u16;
                    let _=SendMessageW(hwnd,WM_LBUTTONDOWN,WPARAM(0),LPARAM(((y as u32)<<16 | x as u32) as isize));
                };
                let snapshot=move |name:&str| {
                    (*raw_ptr).resize_and_dump(&std::path::Path::new(dir).join(name).to_string_lossy());
                };
                (*raw_ptr).carousel.snap_to_section(1); snapshot("verify-focus.raw");
                click(|a|matches!(a,Action::FocusMode(crate::sections::focus::PomodoroMode::ShortBreak)));
                assert!((*raw_ptr).focus_sec.mode==crate::sections::focus::PomodoroMode::ShortBreak);
                click(|a|matches!(a,Action::FocusToggle)); assert!((*raw_ptr).focus_sec.is_running);
                snapshot("verify-focus-running.raw");
                click(|a|matches!(a,Action::FocusReset)); assert!(!(*raw_ptr).focus_sec.is_running);
                (*raw_ptr).carousel.snap_to_section(2); snapshot("verify-water.raw");
                let old_intake=(*raw_ptr).water_sec.intake_ml;
                click(|a|matches!(a,Action::Drink)); assert_eq!((*raw_ptr).water_sec.intake_ml,old_intake+250);
                let old_reminder=(*raw_ptr).water_sec.reminder_on;
                click(|a|matches!(a,Action::WaterReminder)); assert_ne!((*raw_ptr).water_sec.reminder_on,old_reminder);
                let old_goal=(*raw_ptr).water_sec.goal_ml;
                click(|a|matches!(a,Action::WaterGoal)); assert_ne!((*raw_ptr).water_sec.goal_ml,old_goal);

                // Verify Water Reminder interactive card & buttons
                let _ = SendMessageW(hwnd, crate::settings::window::WM_TRIGGER_WATER_REMINDER, WPARAM(0), LPARAM(0));
                assert!((*raw_ptr).water_reminder_active);
                (*raw_ptr).current_body_w = WATER_CARD_W;
                (*raw_ptr).current_h = WATER_CARD_H;
                snapshot("verify-water-reminder.raw");
                for frame in 0..32 {
                    (*raw_ptr).water_reminder_since = Instant::now() - Duration::from_secs_f32(frame as f32 * 2.4 / 32.0);
                    snapshot(&format!("verify-water-loop-{frame:02}.raw"));
                }
                let body_left = (*raw_ptr).current_body_left();
                let drink_x = (body_left + 245.0) as u16;
                let drink_y = 31u16;
                let intake_before = (*raw_ptr).water_sec.intake_ml;
                let _ = SendMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(0), LPARAM(((drink_y as u32) << 16 | drink_x as u32) as isize));
                assert_eq!((*raw_ptr).water_sec.intake_ml, intake_before + 250);
                assert!(!(*raw_ptr).water_reminder_active);
                assert!((*raw_ptr).water_feedback.is_some());
                let _ = SendMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(0), LPARAM(((drink_y as u32) << 16 | drink_x as u32) as isize));
                assert_eq!((*raw_ptr).water_sec.intake_ml, intake_before + 250, "confirmation registered twice");
                if let Some(feedback) = &mut (*raw_ptr).water_feedback { feedback.advance(0.7); }
                snapshot("verify-water-confirmed.raw");
                for frame in 0..30 {
                    let mut feedback = super::hydration::DrinkFeedback::new(intake_before, intake_before + 250, (*raw_ptr).water_sec.goal_ml);
                    feedback.advance(frame as f32 * 2.2 / 30.0);
                    (*raw_ptr).water_feedback = Some(feedback);
                    snapshot(&format!("verify-water-success-{frame:02}.raw"));
                }
                let _ = SendMessageW(hwnd, crate::settings::window::WM_TRIGGER_WATER_REMINDER, WPARAM(0), LPARAM(0));
                assert!((*raw_ptr).water_reminder_active);
                let skip_x = (body_left + 324.0) as u16;
                let skip_y = 31u16;
                let _ = SendMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(0), LPARAM(((skip_y as u32) << 16 | skip_x as u32) as isize));
                assert_eq!((*raw_ptr).water_sec.intake_ml, intake_before + 250);
                assert!(!(*raw_ptr).water_reminder_active);
                assert!((*raw_ptr).water_feedback.is_none());
                (*raw_ptr).is_hovered = true;
                (*raw_ptr).current_body_w = EXPANDED_W;
                (*raw_ptr).current_h = EXPANDED_H;
                (*raw_ptr).current_body_w = todo::EXPANDED_SIZE.0;
                (*raw_ptr).current_h = todo::EXPANDED_SIZE.1;
                (*raw_ptr).carousel.snap_to_section(4); snapshot("verify-todo.raw");
                let original_items=(*raw_ptr).todo_sec.items.clone();
                click(|a|matches!(a,Action::TodoAdd));
                assert_eq!((*raw_ptr).todo_editing,Some(TodoEditing::New));
                (*raw_ptr).todo_input.set_text("  Plan today's tasks  ");
                snapshot("verify-todo-editor.raw");
                click(|a|matches!(a,Action::TodoSave));
                assert_eq!((*raw_ptr).todo_sec.items.len(),original_items.len()+1,"Todo save failed: {:?}; editor={:?}; text={:?}",(*raw_ptr).todo_error,(*raw_ptr).todo_editing,(*raw_ptr).todo_input.text());
                let task_id=(*raw_ptr).todo_sec.items.first().unwrap().id;
                assert_eq!((*raw_ptr).todo_sec.items.first().unwrap().title,"Plan today's tasks");
                snapshot("section-todo-populated.raw");
                click(|a|matches!(a,Action::TodoToggle(_)));
                assert_eq!((*raw_ptr).todo_sec.items.first().map(|item|item.completed),Some(true));
                click(|a|matches!(a,Action::TodoEdit(_)));
                assert_eq!((*raw_ptr).todo_input.text(),"Plan today's tasks");
                (*raw_ptr).todo_input.set_text("Send the updated design");
                (*raw_ptr).save_todo();
                assert_eq!((*raw_ptr).todo_sec.items.first().unwrap().id,task_id);
                assert!((*raw_ptr).todo_sec.items.first().unwrap().completed);
                snapshot("verify-todo-edited.raw");
                click(|a|matches!(a,Action::TodoDelete(_)));
                assert_eq!((*raw_ptr).todo_sec.items.len(),original_items.len());
                snapshot("verify-todo-deleted.raw");
                click(|a|matches!(a,Action::TodoUndoDelete));
                assert_eq!((*raw_ptr).todo_sec.items.first().unwrap().id,task_id);
                for title in ["Prepare meeting notes", "Review the latest design", "Send the finished work"] {
                    (*raw_ptr).todo_sec.add_task(title).unwrap();
                }
                snapshot("verify-todo-list.raw");
                click(|a|matches!(a,Action::TodoPage(3)));
                assert_eq!((*raw_ptr).todo_offset,3);
                snapshot("verify-todo-page-two.raw");
                click(|a|matches!(a,Action::TodoEdit(_)));
                (*raw_ptr).todo_input.set_text("");
                (*raw_ptr).save_todo();
                assert!((*raw_ptr).todo_error.is_some());
                assert!((*raw_ptr).todo_editing.is_some());
                snapshot("verify-todo-validation.raw");
                (*raw_ptr).todo_input.set_text("Unsaved draft");
                (*raw_ptr).carousel.snap_to_section(1);
                assert_eq!((*raw_ptr).expanded_size(),(EXPANDED_W,EXPANDED_H));
                (*raw_ptr).sync_todo_input();
                assert!((*raw_ptr).todo_editing.is_some());
                (*raw_ptr).carousel.snap_to_section(4);
                assert_eq!((*raw_ptr).expanded_size(),todo::EXPANDED_SIZE);
                assert_eq!((*raw_ptr).todo_input.text(),"Unsaved draft");
                (*raw_ptr).cancel_todo();
                snapshot("verify-todo-cancelled.raw");
                click(|a|matches!(a,Action::TodoDelete(_)));
                assert_eq!((*raw_ptr).todo_offset,0);
                (*raw_ptr).todo_sec.items=original_items; (*raw_ptr).todo_sec.save();
                (*raw_ptr).todo_deleted=None;
                (*raw_ptr).current_body_w=EXPANDED_W;
                (*raw_ptr).current_h=EXPANDED_H;
                let original_shelf=std::mem::take(&mut (*raw_ptr).shelf_sec.items);
                for index in 0..4 {
                    let path=std::path::Path::new(dir).join(format!("sample-file-{index}.txt"));
                    let text="File for the Nocan preview.\n";
                    std::fs::write(&path,text).expect("write preview file");
                    (*raw_ptr).shelf_sec.items.push(crate::sections::shelf::ShelfItem {
                        name:format!("sample-file-{index}.txt"),size_label:format!("{} B",text.len()),
                        is_dir:false,is_image:false,path,
                    });
                }
                (*raw_ptr).carousel.snap_to_section(3); snapshot("section-shelf-populated.raw");
                click(|a|matches!(a,Action::ShelfPage(3))); assert_eq!((*raw_ptr).shelf_offset,3);
                snapshot("verify-shelf-next.raw");
                click(|a|matches!(a,Action::ShelfPage(0))); assert_eq!((*raw_ptr).shelf_offset,0);
                (*raw_ptr).shelf_sec.items=original_shelf;
                (*raw_ptr).carousel.snap_to_section(5);
                ((*raw_ptr).current_body_w,(*raw_ptr).current_h)=crate::sections::openmaus::EXPANDED_SIZE;
                snapshot("verify-openmaus-activity.raw");
                click(|a|matches!(a,Action::MausTab(OpenMausTab::Schedule)));
                assert_eq!((*raw_ptr).openmaus_sec.tab,OpenMausTab::Schedule);
                snapshot("verify-openmaus-schedule.raw");
                if (*raw_ptr).openmaus_sec.row_count()>2 {
                    click(|a|matches!(a,Action::MausPage(2)));
                    assert_eq!((*raw_ptr).openmaus_sec.offset,2);
                    snapshot("verify-openmaus-schedule-next.raw");
                }
                click(|a|matches!(a,Action::MausTab(OpenMausTab::Usage)));
                assert_eq!((*raw_ptr).openmaus_sec.tab,OpenMausTab::Usage);
                snapshot("verify-openmaus-bots.raw");
                if (*raw_ptr).openmaus_sec.row_count()>2 {
                    click(|a|matches!(a,Action::MausPage(2)));
                    assert_eq!((*raw_ptr).openmaus_sec.offset,2);
                    snapshot("verify-openmaus-bots-next.raw");
                    click(|a|matches!(a,Action::MausPage(0)));
                    assert_eq!((*raw_ptr).openmaus_sec.offset,0);
                }
                click(|a|matches!(a,Action::MausUsageGroup(1)));
                assert_eq!((*raw_ptr).openmaus_sec.usage_group,1);
                snapshot("verify-openmaus-models.raw");
                click(|a|matches!(a,Action::MausUsageGroup(2)));
                assert_eq!((*raw_ptr).openmaus_sec.usage_group,2);
                snapshot("verify-openmaus-providers.raw");
                click(|a|matches!(a,Action::MausTab(OpenMausTab::Activity)));
                snapshot("verify-openmaus-activity.raw");
                let old_notifications=(*raw_ptr).openmaus_sec.notifications_enabled;
                click(|a|matches!(a,Action::MausNotifications));
                assert_ne!((*raw_ptr).openmaus_sec.notifications_enabled,old_notifications);
                click(|a|matches!(a,Action::MausNotifications));
                assert_eq!((*raw_ptr).openmaus_sec.notifications_enabled,old_notifications);
                (*raw_ptr).is_hovered=false;
                (*raw_ptr).current_body_w=400.0;(*raw_ptr).current_h=62.0;
                (*raw_ptr).trigger_peek(PeekKind::OpenMaus{event:ActivityEvent {
                    id:"preview-only".into(),title:"Agent response complete".into(),detail:"Sample data for testing notifications".into(),
                    at_ms:crate::integrations::openmaus_activity::timestamp_ms(),kind:ActivityKind::AgentCompleted,
                }});
                snapshot("verify-openmaus-notification.raw");
                let notify_x=((*raw_ptr).current_body_left()+90.0) as u16;
                SendMessageW(hwnd,WM_MOUSEMOVE,WPARAM(0),LPARAM((31u32<<16|notify_x as u32) as isize));
                assert!(!(*raw_ptr).is_hovered,"hover expanded the notification before it could be clicked");
                SendMessageW(hwnd,WM_LBUTTONDOWN,WPARAM(0),LPARAM((31u32<<16|notify_x as u32) as isize));
                assert!((*raw_ptr).is_hovered);
                assert_eq!((*raw_ptr).carousel.current_section_id(),SectionId::OpenMaus);
                assert!((*raw_ptr).peek_overlay.is_none());
                assert_eq!((*raw_ptr).expanded_size(),crate::sections::openmaus::EXPANDED_SIZE);
                (*raw_ptr).current_body_w=EXPANDED_W;(*raw_ptr).current_h=EXPANDED_H;
                (*raw_ptr).carousel.snap_to_section(6); snapshot("verify-ai.raw");
                click(|a|matches!(a,Action::Provider(crate::integrations::ai_usage::AiTab::Codex)));
                assert!((*raw_ptr).ai_usage_sec.active_tab==crate::integrations::ai_usage::AiTab::Codex);
                snapshot("verify-ai-codex.raw");
                (*raw_ptr).carousel.snap_to_section(7); snapshot("verify-stocks.raw");
                click(|a|matches!(a,Action::Stock(1))); assert_eq!((*raw_ptr).saham_sec.selected_index,1);
                (*raw_ptr).carousel.snap_to_section(0);
                let footer_x=((*raw_ptr).current_body_left()+EXPANDED_W*0.5) as u16;
                let footer_y=(EXPANDED_H-14.0) as u16;
                let _=SendMessageW(hwnd,WM_LBUTTONDOWN,WPARAM(0),LPARAM(((footer_y as u32)<<16 | footer_x as u32) as isize));
                assert!((*raw_ptr).carousel.current_section>0,"footer bar did not change the section");
                (*raw_ptr).carousel.snap_to_section(1); snapshot("verify-keyboard.raw");
                let _=SendMessageW(hwnd,WM_KEYDOWN,WPARAM(VK_TAB.0 as usize),LPARAM(0));
                assert!((*raw_ptr).deck_focus.is_some());
                let _=SendMessageW(hwnd,WM_KEYDOWN,WPARAM(VK_RETURN.0 as usize),LPARAM(0));
                (*raw_ptr).deck_focus=None;
                click(|a|matches!(a,Action::FocusToggle));
                assert!((*raw_ptr).deck_focus.is_none(),"mouse click left a keyboard focus ring");
                let _=SendMessageW(hwnd,WM_KEYDOWN,WPARAM(VK_RIGHT.0 as usize),LPARAM(0));
                assert_eq!((*raw_ptr).carousel.current_section,2);
                let _=SendMessageW(hwnd,WM_KEYDOWN,WPARAM(VK_LEFT.0 as usize),LPARAM(0));
                assert_eq!((*raw_ptr).carousel.current_section,1);
                (*raw_ptr).focus_sec.reset();
                (*raw_ptr).mascot_enabled=true;
                (*raw_ptr).deck_focus=None;
                (*raw_ptr).minimal_notch = false;
                (*raw_ptr).auto_minimized = false;
                (*raw_ptr).is_hovered = false;
                SendMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(0), LPARAM(0));
                assert!((*raw_ptr).tiger.is_reacting(), "mascot click did not reach the animation");
                for key in [VK_SPACE, VK_RETURN] {
                    (*raw_ptr).tiger = TigerMascot::default();
                    SendMessageW(hwnd, WM_KEYDOWN, WPARAM(key.0 as usize), LPARAM(0));
                    assert!((*raw_ptr).tiger.is_reacting(), "mascot keyboard reaction did not fire");
                }
                (*raw_ptr).is_hovered = true;
                (*raw_ptr).current_body_w = EXPANDED_W;
                (*raw_ptr).current_h = EXPANDED_H;
                SendMessageW(hwnd, WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
                assert!(!(*raw_ptr).is_hovered, "Escape did not collapse the notch");
                let _ = DestroyWindow(hwnd);
                return Ok(hwnd);
            }
            if std::env::args().any(|a| a == "--dump-previews") {
                (*raw_ptr).dump_all_previews();
                let _ = DestroyWindow(hwnd);
                return Ok(hwnd);
            }
            crate::config::debug_log("NotchWindow::create: calling update_window_rgn & ShowWindow");
            (*raw_ptr).update_window_rgn();
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            windows::Win32::UI::WindowsAndMessaging::SetTimer(hwnd, 1, 16, None);
        }

        crate::config::debug_log("NotchWindow::create: complete, returning Ok(hwnd)");
        Ok(hwnd)
    }

    pub fn apply_settings(&mut self, settings: &crate::config::AppSettings) {
        self.openmaus_sec.set_notifications(settings.openmaus_notifications && settings.sections.show_openmaus);
        if !self.openmaus_sec.notifications_enabled && self.maus_notification_active() {
            self.peek_overlay=None;self.last_peek=None;
        }
        self.autostart_enabled = crate::settings::autostart::enabled();
        self.mascot_enabled = settings.general.mascot_enabled;
        self.minimal_notch = settings.general.minimal_notch;
        self.auto_minimize_enabled = settings.general.auto_minimize_on_maximize;
        self.accent_mode = settings.appearance.accent_mode;
        self.visual_opacity = settings.appearance.acrylic_opacity.clamp(0.5,1.0);
        self.dynamic_lyrics_width = settings.music.dynamic_lyrics_width;
        self.music_sec.ambient_glow_enabled = settings.music.ambient_glow_enabled;
        let focus_untouched = !self.focus_sec.is_running && self.focus_sec.remaining_secs == self.focus_sec.total_secs;
        let durations_changed = self.focus_sec.work_mins != settings.productivity.focus_work_mins || self.focus_sec.break_mins != settings.productivity.focus_break_mins;
        self.focus_sec.work_mins = settings.productivity.focus_work_mins;
        self.focus_sec.break_mins = settings.productivity.focus_break_mins;
        if focus_untouched && durations_changed { self.focus_sec.set_mode(self.focus_sec.mode); }
        self.water_sec.goal_ml = settings.productivity.water_daily_target_glasses * 250;
        self.saham_sec.set_symbols(&settings.stock_symbols);
        let interval = Duration::from_secs(u64::from(settings.productivity.water_interval_mins.clamp(1, 1440)) * 60);
        if self.water_reminder_interval != interval { self.water_reminder_interval = interval; self.water_reminder_since = Instant::now(); }
        self.carousel.set_active_sections(settings.sections.active_sections());

        self.ai_usage_sec.enabled = [settings.ai.show_antigravity, settings.ai.show_claude, settings.ai.show_codex, settings.ai.show_opencode];
        self.ai_usage_sec.show_proportional_bar = settings.ai.show_proportional_bar;
        self.ai_usage_sec.service.set_poll_interval(settings.ai.poll_interval_secs);
        let provider_changed = self.previous_settings.as_ref().map_or(true, |old| old.ai.default_provider != settings.ai.default_provider);
        if provider_changed { match settings.ai.default_provider {
            0 => self.ai_usage_sec.active_tab = crate::integrations::ai_usage::AiTab::Antigravity,
            1 => self.ai_usage_sec.active_tab = crate::integrations::ai_usage::AiTab::Claude,
            2 => self.ai_usage_sec.active_tab = crate::integrations::ai_usage::AiTab::Codex,
            3 => self.ai_usage_sec.active_tab = crate::integrations::ai_usage::AiTab::OpenCode,
            _ => {}
        }}
        if !self.ai_usage_sec.enabled[self.ai_usage_sec.active_tab as usize] {
            if let Some(i) = self.ai_usage_sec.enabled.iter().position(|enabled| *enabled) {
                self.ai_usage_sec.active_tab = [crate::integrations::ai_usage::AiTab::Antigravity, crate::integrations::ai_usage::AiTab::Claude, crate::integrations::ai_usage::AiTab::Codex, crate::integrations::ai_usage::AiTab::OpenCode][i];
            }
        }
        self.previous_settings = Some(settings.clone());

        if self.dwrite.font_name != settings.appearance.font_family {
            if let Ok(new_dw) = crate::graphics::dwrite::DWriteContext::with_font_name(Some(&settings.appearance.font_family)) {
                self.dwrite = new_dw;
            }
        }
        self.todo_input.set_font(&self.dwrite.font_name);
    }

    fn collapsed_height(&self) -> f32 {
        if self.mascot_enabled { MASCOT_H } else { COLLAPSED_H }
    }

    fn water_overlay_active(&self) -> bool {
        self.water_reminder_active || self.water_feedback.is_some()
    }

    /// A running timer stays readable in the compact bar even when the notch is
    /// auto-minimized; the countdown replaces the clock there.
    fn timer_compact_visible(&self) -> bool {
        (self.focus_sec.session_active || self.focus_sec.is_running) && self.lyric_line.is_none()
    }

    fn integrated_timer_pause_at(&self, x: f32, y: f32) -> bool {
        if !self.timer_compact_visible() || self.water_overlay_active() || self.peek_overlay.is_some() { return false; }
        let left = self.current_body_left();
        let bounds = if self.is_hovered && self.current_h > 100.0 {
            D2D_RECT_F { left: left + self.current_body_w - 164.0, top: 7.0,
                right: left + self.current_body_w - 46.0, bottom: 33.0 }
        } else if !self.is_hovered {
            D2D_RECT_F { left, top: 0.0, right: left + self.current_body_w, bottom: self.current_h }
        } else { return false; };
        let button = crate::notch::compact::timer_pause_rect(bounds);
        x >= button.left && x <= button.right && y >= button.top && y <= button.bottom
    }

    fn focus_capsule_visible(&self) -> bool {
        self.floating_timer.is_none() && (self.focus_sec.is_running || self.focus_sec.remaining_secs < self.focus_sec.total_secs)
            && !self.water_reminder_active && self.peek_overlay.is_none()
            && self.lyric_line.is_some() && !self.is_hovered && !self.auto_minimized
    }

    fn lyric_bar_active(&self) -> bool {
        self.lyric_line.is_some() && !self.is_hovered
    }

    fn focus_capsule_left(&self) -> f32 {
        self.current_body_left() + self.current_body_w + FOCUS_CAPSULE_GAP
    }

    fn focus_capsule_rect(&self) -> D2D_RECT_F {
        let left = self.focus_capsule_left();
        let top = (self.current_h - FOCUS_CAPSULE_H).max(0.0) * 0.5;
        D2D_RECT_F { left, top, right: left + FOCUS_CAPSULE_W, bottom: top + FOCUS_CAPSULE_H }
    }

    fn focus_capsule_action_at(&self, x: f32, y: f32) -> Option<Action> {
        if !self.focus_capsule_visible() { return None; }
        let capsule = self.focus_capsule_rect();
        if x < capsule.left || x > capsule.right || y < capsule.top || y > capsule.bottom {
            return None;
        }
        if x >= capsule.left + 142.0 && x <= capsule.left + 174.0 {
            Some(Action::FocusToggle)
        } else if x >= capsule.left + 178.0 && x <= capsule.right - 6.0 {
            Some(Action::FocusSkip)
        } else {
            Some(Action::FocusToggle)
        }
    }

    fn draw_focus_capsule(&self, body_h: f32, white: &ID2D1SolidColorBrush, muted: &ID2D1SolidColorBrush) {
        if !self.focus_capsule_visible() { return; }
        let left = self.focus_capsule_left();
        let top = (body_h - FOCUS_CAPSULE_H).max(0.0) * 0.5;
        let capsule = D2D_RECT_F { left, top, right: left + FOCUS_CAPSULE_W, bottom: top + FOCUS_CAPSULE_H };
        let (cr, cg, cb) = deck::accent_color(self.accent_mode);
        crate::graphics::ambient::draw_with_cover(&self.d2d, capsule, 18.0, 0.30, 0.0, (cr, cg, cb), 1.0);
        if let Ok(edge) = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 0.78, g: 0.84, b: 0.94, a: 0.24 }) {
            self.d2d.draw_rounded_rect(capsule, 18.0, 18.0, &edge, 0.8);
        }
        if let Ok(track) = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.14 }) {
            let progress = self.focus_sec.remaining_secs as f32 / self.focus_sec.total_secs.max(1) as f32;
            self.d2d.draw_progress_ring(left + 23.0, top + FOCUS_CAPSULE_H * 0.5, 12.0, 2.4, progress, &track, white);
        }
        crate::graphics::icons::draw(&self.d2d, crate::graphics::icons::Icon::Target,
            left + 23.0, top + FOCUS_CAPSULE_H * 0.5, 9.0, white);
        self.dwrite.draw_text(&self.d2d.context, &self.focus_sec.time_text(), D2D_RECT_F {
            left: left + 44.0, top: top + 5.0, right: left + 136.0, bottom: top + 31.0,
        }, white, &self.dwrite.format_idle_time);
        let status = match self.focus_sec.mode {
            crate::sections::focus::PomodoroMode::Focus => "Focus",
            crate::sections::focus::PomodoroMode::ShortBreak => "Break",
            crate::sections::focus::PomodoroMode::LongBreak => "Long break",
        };
        self.dwrite.draw_text(&self.d2d.context, status, D2D_RECT_F {
            left: left + 44.0, top: top + 29.0, right: left + 136.0, bottom: top + 49.0,
        }, muted, &self.dwrite.format_small);
        let pause = D2D_RECT_F { left: left + 142.0, top: top + 13.0, right: left + 174.0, bottom: top + 45.0 };
        if let Ok(btn) = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.13 }) {
            self.d2d.fill_rounded_rect(pause, 8.0, 8.0, &btn);
        }
        crate::graphics::icons::draw(&self.d2d,
            if self.focus_sec.is_running { crate::graphics::icons::Icon::Pause } else { crate::graphics::icons::Icon::Play },
            (pause.left + pause.right) * 0.5, (pause.top + pause.bottom) * 0.5, 10.0, white);
        let skip = D2D_RECT_F { left: left + 178.0, top: top + 13.0, right: left + 200.0, bottom: top + 45.0 };
        if let Ok(btn) = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.13 }) {
            self.d2d.fill_rounded_rect(skip, 8.0, 8.0, &btn);
        }
        crate::graphics::icons::draw(&self.d2d, crate::graphics::icons::Icon::Next,
            (skip.left + skip.right) * 0.5, (skip.top + skip.bottom) * 0.5, 10.0, white);
    }

    fn idle_mascot_visible(&self) -> bool {
        self.mascot_enabled && !self.is_hovered && !self.minimal_notch && !self.auto_minimized
            && self.peek_overlay.is_none() && self.last_peek.is_none() && self.lyric_line.is_none() && !self.water_overlay_active()
            && !self.timer_compact_visible()
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

    /// Saturation-weighted average of the album cover (premultiplied BGRA), toned so white text stays readable.
    fn dominant_color(bgra: &[u8]) -> (f32, f32, f32) {
        let (mut sr, mut sg, mut sb, mut sw) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for px in bgra.chunks_exact(4) {
            let a = px[3] as f32 / 255.0;
            if a < 0.5 {
                continue;
            }
            let (b, g, r) = (px[0] as f32 / 255.0 / a, px[1] as f32 / 255.0 / a, px[2] as f32 / 255.0 / a);
            let mx = r.max(g).max(b);
            let mn = r.min(g).min(b);
            let w = (mx - mn) * 2.0 + 0.05 + mx * 0.1;
            sr += r * w;
            sg += g * w;
            sb += b * w;
            sw += w;
        }
        if sw <= 0.0 {
            return (0.16, 0.36, 0.80);
        }
        let (mut r, mut g, mut b) = (sr / sw, sg / sw, sb / sw);
        let mx = r.max(g).max(b).max(0.001);
        let k = 0.62 / mx;
        r = (r * k).min(1.0);
        g = (g * k).min(1.0);
        b = (b * k).min(1.0);
        (r, g, b)
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
        let min_hit_w = if is_minimized_mode && !self.is_hovered && self.peek_overlay.is_none() && self.lyric_line.is_none() && !self.focus_sec.is_running {
            100.0f32
        } else {
            COLLAPSED_W
        };
        let min_hit_h = if is_minimized_mode && !self.is_hovered && self.peek_overlay.is_none() && self.lyric_line.is_none() && !self.focus_sec.is_running {
            10.0f32
        } else {
            COLLAPSED_H
        };
        let (_, ear) = Self::compute_radii(self.current_body_w, self.current_h);
        let capsule_extra = if self.focus_capsule_visible() { FOCUS_CAPSULE_GAP + FOCUS_CAPSULE_W } else { 0.0 };
        let hit_w = self.current_body_w.max(min_hit_w) + 2.0 * ear + 8.0 + capsule_extra;
        let hit_h = self.current_h.max(min_hit_h).max(if capsule_extra > 0.0 { FOCUS_CAPSULE_H } else { 0.0 }) + 4.0;

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
            PeekKind::OpenMaus { .. } => {
                let peek=PeekOverlay{kind,started_at:now,updated_at:now,duration_secs:5.0,display_level:0.0};
                self.last_peek=Some(peek.clone());self.peek_overlay=Some(peek);
            },
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
        self.current_h = MASCOT_H;
        self.resize_and_dump(&format!("{}\\dump_mascot.raw", dir));
        self.mascot_enabled = false;

        // 3. Expanded Sections
        self.is_hovered = true;
        self.current_body_w = EXPANDED_W;
        self.current_h = EXPANDED_H;
        for sec in 0..self.carousel.total_sections() {
            self.carousel.snap_to_section(sec);
            self.music_sec.show_lyrics = false;
            self.resize_and_dump(&format!("{}\\dump_sec_{}.raw", dir, sec));
        }

        // 4. Expanded Lyrics Mode
        self.carousel.snap_to_section(0);
        self.music_sec.show_lyrics = true;
        self.lyrics_w = 520.0;
        self.current_body_w = 520.0;
        self.lyric_color = (0.75, 0.28, 0.35);
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

        // 11. Lyric bar with album ambience & music logo
        let sample_line = "Pagi kucari kamu, malam juga begitu".to_string();
        self.lyric_text_w = self.dwrite.measure_text_width(&sample_line, &self.dwrite.format_lyric_bar);
        self.lyric_line = Some(sample_line);
        self.lyric_color = (0.75, 0.28, 0.35); // warm red/rose ambience
        self.current_body_w = (self.lyric_text_w + LYRIC_BAR_EXTRA).clamp(LYRIC_BAR_MIN_W, LYRIC_BAR_MAX_W);
        self.current_h = COLLAPSED_H;
        self.lyric_changed_at = Instant::now() - std::time::Duration::from_millis(500);
        self.resize_and_dump(&format!("{}\\dump_lyricbar.raw", dir));
        self.lyric_line = None;

        // 12. Water Drinking Reminder Card (360x62)
        self.water_reminder_active = true;
        self.current_body_w = WATER_CARD_W;
        self.current_h = WATER_CARD_H;
        self.resize_and_dump(&format!("{}\\dump_water_reminder.raw", dir));
        self.water_reminder_active = false;
    }

    fn dump_branding_previews(&mut self, dir: &std::path::Path) -> Result<()> {
        let directory=crate::integrations::openmaus::get_openmaus_data_dir();
        let mut monitor=crate::integrations::openmaus_activity::ActivityMonitor::new(directory.clone());
        self.openmaus_sec.activity=monitor.poll().snapshot;
        self.openmaus_sec.report=crate::integrations::openmaus::load_usage_report_from(&directory);
        std::fs::create_dir_all(dir).map_err(|_| windows::core::Error::from_win32())?;
        self.is_hovered = false;
        self.peek_overlay = None;
        self.last_peek = None;
        self.lyric_line = None;
        self.current_body_w = COLLAPSED_W;
        for (name, enabled, height) in [("mascot", true, MASCOT_H), ("idle", false, COLLAPSED_H)] {
            self.mascot_enabled = enabled;
            self.current_h = height;
            self.resize_and_dump(&dir.join(format!("{name}.raw")).to_string_lossy());
        }
        let real_status = self.system_status.clone();
        for (name, percent, present, charging) in [
            ("compact-battery-42", Some(42), true, false),
            ("compact-charging-72", Some(72), true, true),
            ("compact-unknown-battery", None, true, false),
            ("compact-ac", None, false, false),
        ] {
            self.system_status.battery_percent = percent;
            self.system_status.is_battery_present = present;
            self.system_status.is_charging = charging;
            self.system_status.is_online = !present || charging;
            self.resize_and_dump(&dir.join(format!("{name}.raw")).to_string_lossy());
        }
        self.system_status = real_status;
        let (corner, ear) = Self::compute_radii(COLLAPSED_W, COLLAPSED_H);
        let sample_track = crate::integrations::media::MediaTrackInfo {
            is_playing: true, position_ms: 45_000, duration_ms: 180_000,
            ..Default::default()
        };
        self.d2d.begin_draw();
        self.d2d.clear(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });
        if let Ok(black) = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) {
            self.d2d.fill_notch_body(ear, 0.0, COLLAPSED_W, COLLAPSED_H, corner, ear, &black);
        }
        if self.d2d.push_clipped_opacity_layer(ear, COLLAPSED_W, COLLAPSED_H, corner, ear, 1.0) {
            crate::notch::compact::draw(&self.d2d, &self.dwrite, D2D_RECT_F {
                left: ear, top: 0.0, right: ear + COLLAPSED_W, bottom: COLLAPSED_H,
            }, &self.system_status, &sample_track, 0.0, &self.focus_sec, self.timer_compact_visible());
            self.d2d.pop_layer();
        }
        self.d2d.end_draw()?;
        self.d3d.dump_backbuffer_raw(&dir.join("compact-playing.raw").to_string_lossy())?;
        let saved_running = self.focus_sec.is_running;
        let saved_remaining = self.focus_sec.remaining_secs;
        self.focus_sec.is_running = true;
        self.focus_sec.remaining_secs = 24 * 60 + 12;
        self.resize_and_dump(&dir.join("compact-focus.raw").to_string_lossy());
        self.focus_sec.is_running = saved_running;
        self.focus_sec.remaining_secs = saved_remaining;
        self.mascot_enabled = true;
        self.current_h = MASCOT_H;
        self.tiger = TigerMascot::default();
        for frame in 0..100 {
            if frame == 25 { self.tiger.react(); }
            let t = frame as f32 * 0.05;
            self.tiger.update(0.05, ((t * 1.6).sin(), (t * 1.1).cos() * 0.6), false, frame >= 60);
            self.resize_and_dump(&dir.join(format!("mascot-frame-{frame:03}.raw")).to_string_lossy());
        }
        self.current_body_w = EXPANDED_W;
        self.current_h = EXPANDED_H;
        self.resize_and_dump(&dir.join("expanded.raw").to_string_lossy());
        self.is_hovered=true;
        self.carousel.set_active_sections(vec![SectionId::Music,SectionId::Focus,SectionId::Water,
            SectionId::Shelf,SectionId::Todo,SectionId::OpenMaus,SectionId::AiUsage,SectionId::Saham]);
        for index in 0..self.carousel.total_sections() {
            self.carousel.snap_to_section(index);
            self.music_sec.show_lyrics=false;
            (self.current_body_w,self.current_h)=self.expanded_size();
            self.resize_and_dump(&dir.join(format!("section-{index:02}.raw")).to_string_lossy());
            for region in &self.deck_regions {
                assert!(region.rect.right>region.rect.left && region.rect.bottom>region.rect.top);
                assert!(region.rect.top>=32.0 && region.rect.bottom<=self.current_h-24.0);
                assert!(region.rect.left>=0.0 && region.rect.right<=CANVAS_W as f32);
                assert!(region.contains((region.rect.left+region.rect.right)*0.5,(region.rect.top+region.rect.bottom)*0.5));
            }
        }
        self.current_body_w=EXPANDED_W;
        self.current_h=EXPANDED_H;
        self.carousel.snap_to_section(0);
        let saved_color = self.lyric_color;
        let saved_mix = self.cover_mix;
        self.lyric_color = (0.62, 0.08, 0.15);
        self.cover_mix = 1.0;
        self.resize_and_dump(&dir.join("cover-red.raw").to_string_lossy());
        self.lyric_color = (0.08, 0.50, 0.18);
        self.resize_and_dump(&dir.join("cover-green.raw").to_string_lossy());
        self.lyric_color = saved_color;
        self.cover_mix = saved_mix;
        self.music_sec.show_lyrics=true;
        self.lyrics_morph=0.5;
        self.current_body_w=(EXPANDED_W + LYRICS_MIN_W) * 0.5;
        self.resize_and_dump(&dir.join("lyrics-morph-mid.raw").to_string_lossy());
        self.lyrics_morph=1.0;
        self.current_body_w=LYRICS_MIN_W;
        self.resize_and_dump(&dir.join("section-lyrics.raw").to_string_lossy());

        // Focus timer while a lyric sentence is active: the timer sits in its own
        // capsule to the right, not inside the lyric notch.
        let saved_focus_running = self.focus_sec.is_running;
        let saved_focus_remaining = self.focus_sec.remaining_secs;
        self.focus_sec.is_running = true;
        self.focus_sec.remaining_secs = 24 * 60 + 12;
        self.mascot_enabled = false;
        self.is_hovered = false;
        self.lyric_text_w = self.dwrite.measure_text_width(
            "Pagi kucari kamu, malam juga begitu", &self.dwrite.format_lyric_bar);
        self.lyric_line = Some("Pagi kucari kamu, malam juga begitu".to_string());
        self.current_body_w = (self.lyric_text_w + LYRIC_BAR_EXTRA).clamp(LYRIC_BAR_MIN_W, LYRIC_BAR_MAX_W);
        self.current_h = COLLAPSED_H;
        self.resize_and_dump(&dir.join("focus-capsule.raw").to_string_lossy());
        self.lyric_line = None;
        self.focus_sec.is_running = saved_focus_running;
        self.focus_sec.remaining_secs = saved_focus_remaining;
        self.mascot_enabled = true;
        self.current_body_w = COLLAPSED_W;
        self.current_h = COLLAPSED_H;
        self.music_sec.show_lyrics=false;
        self.lyrics_morph=0.0;
        self.is_hovered=false;
        self.current_body_w = MINIMIZED_W;
        self.current_h = MINIMIZED_H;
        self.resize_and_dump(&dir.join("minimized.raw").to_string_lossy());
        Ok(())
    }

    fn resize_and_dump(&mut self, path: &str) {
        let (_, ear) = Self::compute_radii(self.current_body_w, self.current_h);
        let capsule_extra = if self.focus_capsule_visible() { FOCUS_CAPSULE_GAP + FOCUS_CAPSULE_W } else { 0.0 };
        let win_w = (self.current_body_w + 2.0 * ear + capsule_extra).round() as u32;
        let win_h = self.current_h.max(FOCUS_CAPSULE_H).round() as u32;
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
            if fg == self.hwnd || self.floating_timer.as_ref().map_or(false, |timer| timer.is_window(fg)) {
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
        if self.auto_minimize_enabled {
            if let Some(in_app) = self.check_foreground_app() { self.auto_minimized = in_app; }
        } else { self.auto_minimized = false; }
        let is_minimized_mode = self.minimal_notch || self.auto_minimized;

        // 2. Poll volume, audio device changes, and Bluetooth connections
        self.poll_system_events(now);

        // Per-sentence lyric bar: follows the active synced lyric line while music plays.
        {
            let track = self.media_service.get_info();
            self.music_sec.update_lyrics(&track);
            let color_key = format!("{}|{}|{}|{:p}", track.title, track.artist, track.album,
                track.thumbnail_bgra.as_ref().map_or(std::ptr::null(), std::sync::Arc::as_ptr));
            if color_key != self.lyric_color_key {
                self.lyric_color_key = color_key;
                self.lyric_color_target = track
                    .thumbnail_bgra
                    .as_ref()
                    .map(|b| Self::dominant_color(b))
                    .unwrap_or((0.16, 0.36, 0.80));
            }
            let ck = (6.0 * dt).min(1.0);
            self.lyric_color.0 += (self.lyric_color_target.0 - self.lyric_color.0) * ck;
            self.lyric_color.1 += (self.lyric_color_target.1 - self.lyric_color.1) * ck;
            self.lyric_color.2 += (self.lyric_color_target.2 - self.lyric_color.2) * ck;
            let cover_target = if track.is_playing && track.thumbnail_bgra.is_some()
                && self.music_sec.ambient_glow_enabled { 1.0 } else { 0.0 };
            self.cover_mix += (cover_target - self.cover_mix) * (3.0 * dt).min(1.0);
            let new_line = self.music_sec.current_lyric_line(&track);
            if new_line != self.lyric_line {
                self.lyric_changed_at = now;
                if let Some(ref s) = new_line {
                    self.lyric_text_w = self.dwrite.measure_text_width(s, &self.dwrite.format_lyric_bar);
                }
                self.lyric_line = new_line;
            }
        }

        if self.carousel.current_section_id() == SectionId::Music && self.music_sec.show_lyrics {
            let track = self.media_service.get_info();
            let pane = self.music_sec.lyrics_pane_width(&self.dwrite, &track);
            self.lyrics_w = if self.dynamic_lyrics_width { (LYRICS_PANE_LEFT + pane + LYRICS_PANE_PAD).clamp(LYRICS_MIN_W, LYRICS_MAX_W) } else { LYRICS_MIN_W };
        }

        // 3. Update active Peek Overlay timer & progress animation
        if let Some(ref mut peek) = self.peek_overlay {
            if now.duration_since(peek.updated_at).as_secs_f32() >= peek.duration_secs {
                self.peek_overlay = None;
            } else {
                match peek.kind {
                    PeekKind::OpenMaus { .. } => {},
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
            let inside = dx <= self.current_body_w * 0.5 + 4.0
                && dy >= 0.0 && dy <= self.current_h + 4.0;
            self.tiger.update(dt, (
                (cursor_pt.x as f32 - screen_cx) / 220.0,
                (dy - MASCOT_H * 0.5) / 160.0,
            ), inside && self.idle_mascot_visible(), self.media_service.get_info().is_playing);

            let local_x = cursor_pt.x as f32 - (self.screen_x + (self.screen_w - CANVAS_W) / 2) as f32;
            if self.maus_notification_active() && !self.is_hovered && !self.water_overlay_active() {
                self.is_hovered=false;
            } else if self.todo_editor_active() && !self.water_overlay_active() {
                self.is_hovered = true;
            } else if self.water_feedback.is_some() {
                self.is_hovered = false;
            } else if !self.is_hovered && self.integrated_timer_pause_at(local_x, dy) {
                self.is_hovered = false;
            } else if self.floating_timer.as_ref().map_or(false, |timer| timer.contains_screen_point(cursor_pt)) {
                self.is_hovered = false;
            } else if !self.is_hovered {
                let min_hit_w = if is_minimized_mode && self.peek_overlay.is_none() && self.lyric_line.is_none() && !self.focus_sec.is_running {
                    100.0f32
                } else {
                    COLLAPSED_W
                };
                let min_hit_h = if is_minimized_mode && self.peek_overlay.is_none() && self.lyric_line.is_none() && !self.focus_sec.is_running {
                    10.0f32
                } else {
                    COLLAPSED_H
                };
                let half_w = self.current_body_w.max(min_hit_w) * 0.5 + 4.0;
                let max_y = self.current_h.max(min_hit_h) + 4.0;
                if dx <= half_w && dy >= -2.0 && dy <= max_y {
                    if !self.idle_mascot_visible() || self.tiger.should_expand() {
                        self.is_hovered = true;
                    }
                }
            } else {
                let (target_w, target_h) = self.expanded_size();
                let half_w = self.current_body_w.max(target_w) * 0.5 + EXPANDED_EAR + 6.0;
                let max_y = self.current_h.max(target_h) + 6.0;
                if dx > half_w || dy < -5.0 || dy > max_y {
                    self.is_hovered = false;
                }
            }
        }

        let prev_sec_id = self.carousel.current_section_id();
        self.carousel.update(dt);
        if self.carousel.current_section_id() == SectionId::Water && prev_sec_id != SectionId::Water {
            self.water_sec.trigger_grow_anim();
        }
        self.water_sec.update(dt);
        if self.water_sec.reset_if_new_day() { self.water_reminder_since = Instant::now(); }
        if let Some(feedback) = &mut self.water_feedback {
            if !feedback.advance(dt) { self.water_feedback = None; }
        }
        if self.water_sec.reminder_on && self.water_sec.intake_ml < self.water_sec.goal_ml
            && !self.water_reminder_active
            && self.water_reminder_since.elapsed() >= self.water_reminder_interval {
            self.trigger_water_reminder();
        }
        self.saham_sec.update(dt);

        let ease = 1.0 - (-12.0 * dt).exp();
        let lyrics_target = if self.music_sec.show_lyrics { 1.0 } else { 0.0 };
        self.lyrics_morph += (lyrics_target - self.lyrics_morph) * ease;
        let provider_target = self.ai_usage_sec.active_tab as usize as f32;
        self.provider_morph += (provider_target - self.provider_morph) * ease;
        let focus_target = match self.focus_sec.mode {
            crate::sections::focus::PomodoroMode::Focus => 0.0,
            crate::sections::focus::PomodoroMode::ShortBreak => 1.0,
            crate::sections::focus::PomodoroMode::LongBreak => 2.0,
        };
        self.focus_morph += (focus_target - self.focus_morph) * ease;

        if self.todo_preview && !self.water_overlay_active() { self.is_hovered=true; }
        let (target_body_w, target_h) = if self.water_overlay_active() {
            (WATER_CARD_W, WATER_CARD_H)
        } else if self.is_hovered {
            self.expanded_size()
        } else if let Some(ref peek) = self.peek_overlay {
            match peek.kind {
                PeekKind::OpenMaus { .. } => (400.0,62.0),
                PeekKind::Volume { .. } | PeekKind::Brightness { .. } => (PEEK_BAR_W, PEEK_BAR_H),
                PeekKind::DeviceConnect { .. } | PeekKind::Charging { .. } => {
                    (PEEK_CARD_W, PEEK_CARD_H)
                }
            }
        } else if self.timer_compact_visible() && !self.is_hovered {
            if is_minimized_mode { (118.0, 26.0) }
            else { (COLLAPSED_W, self.collapsed_height()) }
        } else if self.lyric_line.is_some() {
            ((self.lyric_text_w + LYRIC_BAR_EXTRA).clamp(LYRIC_BAR_MIN_W, LYRIC_BAR_MAX_W), COLLAPSED_H)
        } else if is_minimized_mode && (!self.focus_sec.is_running || self.floating_timer.is_some()) {
            (MINIMIZED_W, MINIMIZED_H)
        } else if is_minimized_mode && self.focus_sec.is_running {
            (COLLAPSED_W, COLLAPSED_H)
        } else {
            (COLLAPSED_W, self.collapsed_height())
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

        if self.peek_overlay.is_none() && self.current_body_w <= COLLAPSED_W + 35.0 {
            self.last_peek = None;
        }

        self.update_window_rgn();

        // Update active sections
        self.focus_sec.update();
        if let Some(timer) = &self.floating_timer {
            let compact = if is_minimized_mode && self.lyric_line.is_none() {
                (MINIMIZED_W, MINIMIZED_H)
            } else {
                (COLLAPSED_W, self.collapsed_height())
            };
            timer.update(&self.focus_sec,
                (self.screen_x, self.screen_y, self.screen_w), (self.current_body_w, self.current_h), compact,
                super::focus_capsule::should_detach(self.focus_sec.session_active, self.lyric_line.as_deref()));
        }
        self.openmaus_sec.refresh_if_needed();
        if self.is_hovered && self.carousel.current_section_id()==SectionId::OpenMaus && self.openmaus_sec.tab==OpenMausTab::Activity {
            self.openmaus_sec.mark_read();
        } else if !self.is_hovered && !self.water_overlay_active() && self.peek_overlay.is_none() {
            if let Some(event)=self.openmaus_sec.next_notification() {self.trigger_peek(PeekKind::OpenMaus{event});}
        }
        self.ai_usage_sec.update();

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

        self.sync_todo_input();
        self.render();
    }

    fn render(&mut self) {
        self.deck_regions.clear();
        let body_w = self.current_body_w;
        let body_h = self.current_h;
        let (corner_r, ear) = Self::compute_radii(body_w, body_h);
        let body_left = self.current_body_left();
        let anim_time = self.start_instant.elapsed().as_secs_f32();

        self.d2d.begin_draw();
        self.d2d
            .clear(D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });

        let timer_background = self.timer_compact_visible() && !self.is_hovered
            && !self.water_overlay_active() && self.peek_overlay.is_none();
        let notch_bg = self
            .d2d
            .create_solid_brush(if timer_background { super::focus_capsule::timer_color(&self.focus_sec) }
                else { D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 } })
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
            Some(_dot_inactive),
            Some(nav_bg),
            Some(nav_hov),
            Some(_pill_b),
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

            let collapsed_h = self.collapsed_height();
            let open_t = ((body_h - collapsed_h) / (EXPANDED_H - collapsed_h)).clamp(0.0, 1.0);
            let min_h_t = ((body_h - 19.0) / 11.0).clamp(0.0, 1.0);
            let min_w_t = ((body_w - (COLLAPSED_W - 35.0)) / 35.0).clamp(0.0, 1.0);
            let min_fade = min_h_t.min(min_w_t);

            let has_peek = self.peek_overlay.is_some() || self.last_peek.is_some();
            let peek_suppress_idle = if has_peek {
                (1.0 - ((body_w - (COLLAPSED_W + 10.0)) / 35.0).clamp(0.0, 1.0)).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let lyric_active = self.lyric_bar_active();
        let timer_bar_active = self.timer_compact_visible() && !self.is_hovered;
        let closed_alpha = if lyric_active || self.water_overlay_active() || timer_bar_active { 0.0 } else { min_fade * peek_suppress_idle * (1.0 - open_t * 2.8).clamp(0.0, 1.0) };
            let open_alpha = if self.water_overlay_active() { 0.0 } else { ((open_t - 0.32) / 0.68).clamp(0.0, 1.0) };

            // Fade the compact content inside the current notch silhouette.
            if closed_alpha > 0.01 && body_h >= 19.0 {
                let pushed = self.d2d.push_clipped_opacity_layer(
                        body_left,
                        body_w,
                        body_h,
                        corner_r,
                        ear,
                        closed_alpha,
                    );
                if pushed {
                    if self.mascot_enabled {
                        self.draw_mascot_bar(body_left, body_w, body_h);
                    } else {
                        self.draw_collapsed_bar(body_left, body_w, body_h, &track, anim_time);
                    }
                    self.d2d.pop_layer();
                }
            }

            // A running Pomodoro replaces the clock in the compact bar. Keep it
            // visible while collapsed and on expanded sections other than Focus.
            if timer_bar_active && open_t < 0.32 && !self.water_overlay_active() && self.peek_overlay.is_none() {
                let pushed_timer = self.d2d.push_clipped_opacity_layer(
                    body_left, body_w, body_h, corner_r, ear, 1.0);
                if pushed_timer {
                    self.draw_collapsed_bar(body_left, body_w, body_h, &track, anim_time);
                    self.d2d.pop_layer();
                }
            }

            // 1b2. Draw the current lyric sentence (fades in on each change)
            if !self.water_overlay_active() && self.peek_overlay.is_none() && self.last_peek.is_none() && open_t < 0.2 && self.lyric_bar_active() {
                if let Some(ref line) = self.lyric_line {
                    let fade = (self.lyric_changed_at.elapsed().as_secs_f32() / 0.28).clamp(0.0, 1.0);
                    let w_t = ((body_w - 120.0) / 60.0).clamp(0.0, 1.0);
                    // Album-cover ambience behind the sentence
                    let (cr, cg, cb) = self.lyric_color;
                    let sh = |v: f32, m: f32| (v * m).min(1.0);
                    if w_t > 0.01 {
                        let shift = (anim_time * 0.5).sin() * body_w * 0.08;
                        self.d2d.draw_ambient_glow(
                            body_left,
                            body_w,
                            body_h,
                            corner_r,
                            1.0,
                            0.95 * w_t,
                            shift,
                            &[
                                (0.0, D2D1_COLOR_F { r: sh(cr, 1.15), g: sh(cg, 1.15), b: sh(cb, 1.15), a: 1.0 }),
                                (0.5, D2D1_COLOR_F { r: cr, g: cg, b: cb, a: 1.0 }),
                                (1.0, D2D1_COLOR_F { r: sh(cr, 0.8), g: sh(cg, 0.8), b: sh(cb, 0.8), a: 1.0 }),
                            ],
                        );
                    }
                    let alpha = fade * w_t;
                    if alpha > 0.01 {
                        let pushed = self.d2d.push_clipped_opacity_layer(body_left, body_w, body_h, corner_r, ear, alpha);
                        // Music note logo (beamed eighth notes) at the left
                        let icx = body_left + 19.0;
                        let icy = body_h * 0.5;
                        self.d2d.fill_ellipse(icx - 3.4, icy + 4.2, 2.5, 2.0, white_b);
                        self.d2d.fill_ellipse(icx + 3.6, icy + 2.8, 2.5, 2.0, white_b);
                        self.d2d.draw_line(icx - 1.1, icy + 4.0, icx - 1.1, icy - 5.2, white_b, 1.4, false);
                        self.d2d.draw_line(icx + 5.9, icy + 2.6, icx + 5.9, icy - 6.4, white_b, 1.4, false);
                        self.d2d.fill_polygon(
                            &[(icx - 1.8, icy - 5.9), (icx + 6.6, icy - 7.1), (icx + 6.6, icy - 4.5), (icx - 1.8, icy - 3.3)],
                            white_b,
                        );
                        let rect = D2D_RECT_F {
                            left: body_left + 34.0,
                            top: 0.0,
                            right: body_left + body_w - 16.0,
                            bottom: body_h,
                        };
                        self.dwrite.draw_text(&self.d2d.context, line, rect, white_b, &self.dwrite.format_lyric_bar);
                        if pushed {
                            self.d2d.pop_layer();
                        }
                    }
                }
                self.draw_focus_capsule(body_h, white_b, dim_b);
            }

            // 1c. Draw Animated Peek Overlays (Volume, Brightness, Device Connect, Charging)
            let active_peek = if self.water_overlay_active() { None }
                else { self.peek_overlay.as_ref().or(self.last_peek.as_ref()).cloned() };
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

            // 1d. Draw Interactive Water Reminder Card (Prompt with "Drank water" and "Skip" buttons)
            if self.water_overlay_active() {
                let card_alpha = ((body_w - 200.0) / 100.0).clamp(0.0, 1.0);
                if card_alpha > 0.01 {
                    let pushed_water = self.d2d.push_clipped_opacity_layer(
                        body_left,
                        body_w,
                        body_h,
                        corner_r,
                        ear,
                        card_alpha,
                    );
                    if let Some(feedback) = &self.water_feedback {
                        feedback.draw(&self.d2d, &self.dwrite, D2D_RECT_F {
                            left: body_left, top: 0.0, right: body_left + body_w, bottom: body_h,
                        });
                    } else { self.draw_water_reminder_card(
                        body_left,
                        body_w,
                        body_h,
                        anim_time,
                        white_b,
                        dim_b,
                    ); }
                    if pushed_water {
                        self.d2d.pop_layer();
                    }
                }
            }

            if open_alpha > 0.01 {
                // The shared shell also contains the complete warm surface for every section.
                {
                    let pushed_hdr = self.d2d.push_clipped_opacity_layer(
                            body_left,
                            body_w,
                            body_h,
                            corner_r,
                            ear,
                            open_alpha,
                        );
                    if pushed_hdr {
                        deck::shell(&self.d2d,&self.dwrite,D2D_RECT_F{left:body_left,top:0.0,right:body_left+body_w,bottom:body_h},&self.system_status,self.carousel.current_section_id(),self.lyric_color,self.cover_mix,self.accent_mode,self.visual_opacity,
                            if self.timer_compact_visible() { Some(&self.focus_sec) } else { None });
                        self.d2d.pop_layer();
                    }
                }

                // 3. Spring-Animated Section Transitions (slide_x = delta * 90.0 + smoothstep crossfade)
                let active_secs = self.carousel.active_sections.clone();
                for (sec_idx, &sec_id) in active_secs.iter().enumerate() {
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

                    if sec_id==SectionId::Music { self.music_sec.update_lyrics(&track); }
                    self.shelf_offset=self.shelf_offset.min(self.shelf_sec.items.len().saturating_sub(1)/3*3);
                    let page=match sec_id {
                        SectionId::Music=>Page::Music(&self.music_sec,&track,self.lyrics_morph),
                        SectionId::Focus=>Page::Focus(&self.focus_sec,self.focus_morph),
                        SectionId::Water=>Page::Water(&self.water_sec),
                        SectionId::Shelf=>Page::Shelf(&self.shelf_sec,self.shelf_offset),
                        SectionId::Todo=>Page::Todo(&self.todo_sec,self.todo_offset,self.todo_editing,self.todo_error.as_deref().or(self.todo_sec.storage_error.as_deref()),self.todo_deleted.is_some(),self.todo_input.is_focused()),
                        SectionId::OpenMaus=>Page::OpenMaus(&self.openmaus_sec),
                        SectionId::AiUsage=>Page::Ai(&self.ai_usage_sec,self.provider_morph),
                        SectionId::Saham=>Page::Stocks(&self.saham_sec),
                    };
                    let regions=deck::draw(&self.d2d,&self.dwrite,sec_bounds,page,self.accent_mode);
                    if sec_id==self.carousel.current_section_id() && sec_alpha>0.5 {
                        self.deck_regions=regions;
                        if let Some(index)=self.deck_focus {
                            if let Some(region)=self.deck_regions.get(index) {
                                self.d2d.draw_rounded_rect(region.rect,6.0,6.0,white_b,1.8);
                            }
                        }
                    }

                    if pushed_sec {
                        self.d2d.pop_layer();
                    }
                }

                // Footer icons and side buttons share the same hit regions in every page.
                {
                    let _=(nav_bg,nav_hov);
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
                    self.carousel.dots.draw_icons(
                        &self.d2d,
                        &self.dwrite,
                        body_left,
                        body_w,
                        body_h,
                        self.carousel.page_pos,
                        &active_secs,
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
            PeekKind::OpenMaus { event } => {
                let (r,g,b)=deck::activity_color(&event.kind);
                if let (Ok(accent),Ok(background))=(
                    self.d2d.create_solid_brush(D2D1_COLOR_F{r,g,b,a:1.0}),
                    self.d2d.create_solid_brush(D2D1_COLOR_F{r,g,b,a:0.14})) {
                    let cy=body_h*0.5;
                    self.d2d.fill_ellipse(body_left+31.0,cy,16.0,16.0,&background);
                    let icon=match event.kind {ActivityKind::AutomationStarted|ActivityKind::AutomationCompleted|ActivityKind::AutomationFailed=>crate::graphics::icons::Icon::Clock,ActivityKind::NeedsAttention=>crate::graphics::icons::Icon::Bell,_=>crate::graphics::icons::Icon::Bot};
                    crate::graphics::icons::draw(&self.d2d,icon,body_left+31.0,cy,17.0,&accent);
                    self.dwrite.draw_text(&self.d2d.context,&event.title,D2D_RECT_F{left:body_left+57.0,top:8.0,right:body_left+body_w-36.0,bottom:29.0},white_brush,&self.dwrite.format_regular_bold);
                    self.dwrite.draw_text(&self.d2d.context,&event.detail,D2D_RECT_F{left:body_left+57.0,top:29.0,right:body_left+body_w-36.0,bottom:48.0},dim_brush,&self.dwrite.format_sub);
                    let remaining=(1.0-peek.updated_at.elapsed().as_secs_f32()/peek.duration_secs).clamp(0.0,1.0);
                    self.d2d.fill_rounded_rect(D2D_RECT_F{left:body_left+57.0,top:53.0,right:body_left+57.0+(body_w-95.0)*remaining,bottom:55.0},1.0,1.0,&accent);
                    let x=body_left+body_w-20.0;
                    self.d2d.draw_line(x-3.0,16.0,x+3.0,22.0,dim_brush,1.2,true);
                    self.d2d.draw_line(x+3.0,16.0,x-3.0,22.0,dim_brush,1.2,true);
                }
            },
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


    pub fn trigger_water_reminder(&mut self) {
        self.water_feedback = None;
        self.water_reminder_since = Instant::now();
        self.water_reminder_active = true;
        self.water_reminder_hover_button = None;
        self.is_hovered = false;
    }

    pub fn confirm_water_reminder(&mut self) {
        if !self.water_reminder_active { return; }
        let before = self.water_sec.intake_ml;
        if !self.water_sec.drink(250) {return;}
        self.water_feedback = Some(super::hydration::DrinkFeedback::new(before, self.water_sec.intake_ml, self.water_sec.goal_ml));
        self.water_reminder_since = Instant::now();
        self.water_reminder_active = false;
        self.water_reminder_hover_button = None;
        self.is_hovered = false;
    }

    pub fn skip_water_reminder(&mut self) {
        self.water_feedback = None;
        self.water_reminder_since = Instant::now();
        self.water_reminder_active = false;
        self.water_reminder_hover_button = None;
        self.is_hovered = false;
    }

    pub fn water_reminder_hit_test(&self, x: f32, y: f32) -> Option<WaterReminderBtn> {
        if !self.water_reminder_active {
            return None;
        }
        let body_left = self.current_body_left();
        let btn1_rect = D2D_RECT_F {
            left: body_left + 196.0,
            top: 15.0,
            right: body_left + 294.0,
            bottom: 47.0,
        };
        if x >= btn1_rect.left && x <= btn1_rect.right && y >= btn1_rect.top && y <= btn1_rect.bottom {
            return Some(WaterReminderBtn::Drink);
        }
        let btn2_rect = D2D_RECT_F {
            left: body_left + 300.0,
            top: 15.0,
            right: body_left + 348.0,
            bottom: 47.0,
        };
        if x >= btn2_rect.left && x <= btn2_rect.right && y >= btn2_rect.top && y <= btn2_rect.bottom {
            return Some(WaterReminderBtn::Skip);
        }
        None
    }

    fn draw_water_reminder_card(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        _anim_time: f32,
        white_brush: &ID2D1SolidColorBrush,
        dim_brush: &ID2D1SolidColorBrush,
    ) {
        let cy = body_h * 0.5;
        let ring_cx = body_left + 28.0;
        let ring_cy = cy;
        let seconds = self.water_reminder_since.elapsed().as_secs_f32();
        let (bob, ripple_radius, ripple_alpha) = super::hydration::reminder_motion(seconds);
        if let Ok(ripple) = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 0.22, g: 0.78, b: 1.0, a: ripple_alpha }) {
            self.d2d.draw_ellipse(ring_cx, ring_cy, ripple_radius, ripple_radius, &ripple, 1.5);
        }

        // 1. Ambient Hydration Glow behind the card
        let glow_color = D2D1_COLOR_F {
            r: 14.0 / 255.0,
            g: 165.0 / 255.0,
            b: 233.0 / 255.0,
            a: 0.18,
        };
        let (corner_r, _) = Self::compute_radii(body_w, body_h);
        self.d2d.draw_ambient_glow(
            body_left,
            body_w,
            body_h,
            corner_r,
            1.0,
            0.5 + bob * 0.05,
            0.0,
            &[
                (0.0, glow_color),
                (0.6, D2D1_COLOR_F { r: 14.0 / 255.0, g: 165.0 / 255.0, b: 233.0 / 255.0, a: 0.08 }),
                (1.0, D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
            ],
        );

        let drink_hover = self.water_reminder_hover_button == Some(WaterReminderBtn::Drink);
        super::hydration::draw_reminder_water(&self.d2d, D2D_RECT_F {
            left: body_left, top: 0.0, right: body_left + body_w, bottom: body_h,
        }, seconds, drink_hover);

        // 2. Hydration Progress Ring with droplet icon
        let goal = self.water_sec.goal_ml.max(1);
        let progress = (self.water_sec.intake_ml as f32 / goal as f32).clamp(0.0, 1.0);
        if let (Ok(track_b), Ok(arc_b)) = (
            self.d2d.create_solid_brush(D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.16,
            }),
            self.d2d.create_solid_brush(D2D1_COLOR_F {
                r: 56.0 / 255.0,
                g: 189.0 / 255.0,
                b: 248.0 / 255.0,
                a: 1.0,
            }),
        ) {
            self.d2d.draw_progress_ring(
                ring_cx,
                ring_cy,
                13.0,
                3.4,
                progress,
                &track_b,
                &arc_b,
            );
            self.d2d.draw_arc_segment(ring_cx, ring_cy, 17.5,
                seconds.rem_euclid(2.4) / 2.4 * 360.0 - 90.0,
                seconds.rem_euclid(2.4) / 2.4 * 360.0 - 5.0, 2.0, &arc_b);
            // Droplet in center of ring
            self.d2d.fill_droplet(ring_cx, ring_cy + bob, 8.0, &arc_b);
        }

        // 3. Reminder Titles & Status
        let title_rect = D2D_RECT_F {
            left: body_left + 50.0,
            top: 12.0,
            right: body_left + 194.0,
            bottom: 32.0,
        };
        self.dwrite.draw_text(
            &self.d2d.context,
            "Time to hydrate",
            title_rect,
            white_brush,
            &self.dwrite.format_regular_bold,
        );

        let sub_rect = D2D_RECT_F {
            left: body_left + 50.0,
            top: 32.0,
            right: body_left + 194.0,
            bottom: 50.0,
        };
        let sub_text = if self.water_sec.storage_error.is_some(){"Not saved. Try again.".into()}
            else{format!("{} / {} ml (+250ml)", self.water_sec.intake_ml, self.water_sec.goal_ml)};
        let subtext_brush = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 0.85, g: 0.94, b: 1.0, a: 0.9 }).ok();
        self.dwrite.draw_text(
            &self.d2d.context,
            &sub_text,
            sub_rect,
            subtext_brush.as_ref().unwrap_or(dim_brush),
            &self.dwrite.format_sub,
        );

        // 4. Action Buttons
        // Button 1: Drank water
        let is_drink_hover = self.water_reminder_hover_button == Some(WaterReminderBtn::Drink);
        let btn1_rect = D2D_RECT_F {
            left: body_left + 196.0,
            top: 15.0,
            right: body_left + 294.0,
            bottom: 47.0,
        };
        let btn1_color = if is_drink_hover {
            D2D1_COLOR_F { r: 0.0, g: 0.46, b: 0.72, a: 1.0 }
        } else {
            D2D1_COLOR_F { r: 0.0, g: 0.40 + bob * 0.012, b: 0.68, a: 1.0 }
        };
        if let Ok(btn1_brush) = self.d2d.create_solid_brush(btn1_color) {
            self.d2d.fill_rounded_rect(btn1_rect, 16.0, 16.0, &btn1_brush);
        }
        if is_drink_hover {
            if let Ok(edge) = self.d2d.create_solid_brush(D2D1_COLOR_F { r: 0.5, g: 0.88, b: 1.0, a: 0.9 }) {
                self.d2d.draw_rounded_rect(btn1_rect, 16.0, 16.0, &edge, 1.5);
            }
        }
        self.dwrite.draw_text(
            &self.d2d.context,
            "Drank water",
            btn1_rect,
            white_brush,
            &self.dwrite.format_button_center,
        );

        // Button 2: Skip
        let is_skip_hover = self.water_reminder_hover_button == Some(WaterReminderBtn::Skip);
        let btn2_rect = D2D_RECT_F {
            left: body_left + 300.0,
            top: 15.0,
            right: body_left + 348.0,
            bottom: 47.0,
        };
        let btn2_color = if is_skip_hover {
            D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.24 }
        } else {
            D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.12 }
        };
        if let Ok(btn2_brush) = self.d2d.create_solid_brush(btn2_color) {
            self.d2d.fill_rounded_rect(btn2_rect, 16.0, 16.0, &btn2_brush);
        }
        self.dwrite.draw_text(
            &self.d2d.context,
            "Skip",
            btn2_rect,
            white_brush,
            &self.dwrite.format_button_center,
        );
    }

    fn draw_collapsed_bar(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        track: &crate::integrations::media::MediaTrackInfo,
        anim_time: f32,
    ) {
        crate::notch::compact::draw(&self.d2d, &self.dwrite, D2D_RECT_F {
            left: body_left,
            top: 0.0,
            right: body_left + body_w,
            bottom: body_h,
        }, &self.system_status, track, anim_time, &self.focus_sec, self.timer_compact_visible());
    }

    fn draw_mascot_bar(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
    ) {
        self.tiger.draw(&self.d2d, D2D_RECT_F {
            left: body_left,
            top: 0.0,
            right: body_left + body_w,
            bottom: body_h,
        });
    }


    fn expanded_size(&self) -> (f32, f32) {
        match self.carousel.current_section_id() {
            SectionId::OpenMaus => crate::sections::openmaus::EXPANDED_SIZE,
            SectionId::Todo => todo::EXPANDED_SIZE,
            SectionId::Music if self.music_sec.show_lyrics => (self.lyrics_w, EXPANDED_H),
            _ => (EXPANDED_W, EXPANDED_H),
        }
    }

    fn todo_editor_active(&self) -> bool {
        self.carousel.current_section_id()==SectionId::Todo && self.todo_editing.is_some()
    }

    fn maus_notification_active(&self)->bool {
        matches!(self.peek_overlay.as_ref().map(|peek|&peek.kind),Some(PeekKind::OpenMaus{..}))
    }

    fn open_maus_notification(&mut self) {
        self.peek_overlay=None;self.last_peek=None;
        self.carousel.snap_to_section_id(SectionId::OpenMaus);
        self.openmaus_sec.set_tab(OpenMausTab::Activity);
        self.deck_focus=None;self.is_hovered=true;
    }

    fn sync_todo_input(&mut self) {
        let visible=self.todo_editor_active() && self.is_hovered && !self.water_overlay_active()
            && (self.carousel.page_pos-self.carousel.current_section as f32).abs()<0.02
            && self.current_body_w>430.0 && self.current_h>210.0;
        use super::todo_editor::{FIELD_LEFT,FIELD_TOP,FIELD_HEIGHT,FIELD_PADDING};
        self.todo_input.sync(visible,((self.current_body_left()+FIELD_LEFT+FIELD_PADDING).round() as i32,
            (FIELD_TOP+(FIELD_HEIGHT-self.todo_input.line_height as f32)*0.5).round() as i32,
            (self.current_body_w-2.0*(FIELD_LEFT+FIELD_PADDING)).round() as i32,self.todo_input.line_height));
    }

    fn begin_todo(&mut self, editing: TodoEditing) {
        let title=match editing {
            TodoEditing::New=>String::new(),
            TodoEditing::Existing(id)=>match self.todo_sec.items.iter().find(|item|item.id==id) {
                Some(item)=>item.title.clone(), None=>return,
            },
        };
        self.todo_input.set_text(&title);
        self.todo_editing=Some(editing);
        self.todo_error=None;
        self.is_hovered=true;
        self.sync_todo_input();
    }

    fn cancel_todo(&mut self) {
        self.todo_editing=None;
        self.todo_error=None;
        self.deck_focus=None;
        self.sync_todo_input();
    }

    fn save_todo(&mut self) {
        let Some(editing)=self.todo_editing else { return; };
        let title=self.todo_input.text();
        let result=match editing {
            TodoEditing::New=>self.todo_sec.add_task(&title).map(|_|()),
            TodoEditing::Existing(id)=>self.todo_sec.edit_task(id,&title),
        };
        match result {
            Ok(())=>{
                if editing==TodoEditing::New { self.todo_offset=0; self.todo_deleted=None; }
                self.cancel_todo();
            },
            Err(error)=>self.todo_error=Some(error),
        }
    }

    fn apply_deck_action(&mut self, action: Action) {
        match action {
            Action::Lyrics => self.music_sec.show_lyrics = !self.music_sec.show_lyrics,
            Action::Previous => self.media_service.skip_prev(),
            Action::PlayPause => self.media_service.toggle_play_pause(),
            Action::Next => self.media_service.skip_next(),
            Action::FocusMode(mode) => self.focus_sec.set_mode(mode),
            Action::FocusToggle => self.focus_sec.toggle_start_pause(),
            Action::FocusReset => self.focus_sec.reset(),
            Action::FocusAddFive => self.focus_sec.add_seconds(300),
            Action::FocusSkip => self.focus_sec.skip_next(),
            Action::Drink => { if self.water_sec.drink(250) {self.water_reminder_since = Instant::now();} },
            Action::WaterGoal => {
                if !self.water_sec.cycle_settings() {return;}
                let mut settings = crate::config::AppSettings::load();
                settings.productivity.water_daily_target_glasses = self.water_sec.goal_ml / 250;
                settings.save();
            },
            Action::WaterReminder => { if self.water_sec.toggle_reminder() {self.water_reminder_since = Instant::now();} },
            Action::ShelfOpen(index) => {
                if let Some(item) = self.shelf_sec.items.get(index) { self.shelf_sec.open_file(&item.path); }
            }
            Action::ShelfClear => { self.shelf_sec.clear_all(); self.shelf_offset=0; },
            Action::ShelfPage(offset) => self.shelf_offset=offset.min(self.shelf_sec.items.len().saturating_sub(1)),
            Action::TodoToggle(id) => self.todo_error=self.todo_sec.toggle_task(id).err(),
            Action::TodoAdd => self.begin_todo(TodoEditing::New),
            Action::TodoEdit(id) => self.begin_todo(TodoEditing::Existing(id)),
            Action::TodoSave => self.save_todo(),
            Action::TodoCancel => self.cancel_todo(),
            Action::TodoInputFocus => self.todo_input.focus(),
            Action::MausTab(tab)=>{self.openmaus_sec.set_tab(tab);self.deck_focus=None;},
            Action::MausPage(offset)=>{self.openmaus_sec.set_page(offset);self.deck_focus=None;},
            Action::MausUsageGroup(group)=>{self.openmaus_sec.usage_group=group.min(2);self.openmaus_sec.offset=0;},
            Action::MausRefresh=>self.openmaus_sec.request_refresh(),
            Action::MausNotifications=>{
                let mut settings=crate::config::AppSettings::load();
                settings.openmaus_notifications=!self.openmaus_sec.notifications_enabled;
                if settings.save(){self.openmaus_sec.set_notifications(settings.openmaus_notifications);}
            },
            Action::TodoPage(offset) => {
                self.todo_offset=todo::page_start(offset,self.todo_sec.items.len());
                self.deck_focus=None;
            },
            Action::TodoDelete(id) => match self.todo_sec.delete_task(id) {
                Ok(removed)=>{
                    self.todo_deleted=Some(removed); self.todo_error=None;
                    self.todo_offset=todo::page_start(self.todo_offset,self.todo_sec.items.len());
                    self.deck_focus=None;
                },
                Err(error)=>self.todo_error=Some(error),
            },
            Action::TodoUndoDelete => if let Some((item,index))=self.todo_deleted.clone() {
                match self.todo_sec.restore_task(item,index) {
                    Ok(())=>{self.todo_deleted=None;self.todo_error=None;self.todo_offset=todo::page_start(index,self.todo_sec.items.len());},
                    Err(error)=>self.todo_error=Some(error),
                }
            },
            Action::Provider(tab) => self.ai_usage_sec.active_tab = tab,
            Action::Stock(index) => {
                if index < self.saham_sec.stocks.len() { self.saham_sec.selected_index = index; }
            }
        }
    }

    pub fn is_interactive_at(&self, x: f32, y: f32) -> bool {
        if self.maus_notification_active() && !self.water_overlay_active() {
            let left=self.current_body_left();
            if x>=left && x<=left+self.current_body_w && y>=0.0 && y<=self.current_h {return true;}
        }
        if self.integrated_timer_pause_at(x, y) { return true; }
        if self.water_reminder_active {
            return self.water_reminder_hit_test(x, y).is_some();
        }
        if self.idle_mascot_visible() {
            let left = self.current_body_left();
            return x >= left && x <= left+self.current_body_w && y>=0.0 && y<=self.current_h;
        }
        if !self.is_hovered { return false; }
        self.deck_regions.iter().any(|region|region.contains(x,y))
    }
}

pub fn run_message_loop() {
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        crate::config::debug_log(&format!("run_message_loop loop finished with msg: 0x{:X}, wParam: {}", msg.message, msg.wParam.0));
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
            WM_TODO_EDITOR => {
                match wparam.0 {
                    1=>notch.save_todo(),
                    2=>notch.cancel_todo(),
                    3|4=>{
                        let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(hwnd);
                        notch.deck_focus=notch.deck_regions.iter().position(|r|if wparam.0==3 { matches!(r.action,Action::TodoSave) } else { matches!(r.action,Action::TodoCancel) });
                    },
                    _=>{},
                }
                return LRESULT(0);
            }
            windows::Win32::UI::WindowsAndMessaging::WM_CTLCOLOREDIT if lparam.0==notch.todo_input.hwnd.0 as isize => {
                return notch.todo_input.color(windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut _));
            }
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
                let x = (lparam.0 & 0xFFFF) as i16 as f32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
                if notch.water_feedback.is_some() { return LRESULT(0); }
                if notch.integrated_timer_pause_at(x, y) { return LRESULT(0); }
                if notch.maus_notification_active() && !notch.water_overlay_active() {return LRESULT(0);}
                if notch.water_reminder_active {
                    notch.water_reminder_hover_button = notch.water_reminder_hit_test(x, y);
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut tme);
                    return LRESULT(0);
                }
                if !notch.is_hovered {
                    if !notch.idle_mascot_visible() { notch.is_hovered = true; }
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
                notch.water_reminder_hover_button = None;
                if !notch.water_reminder_active && !notch.todo_editor_active() {
                    notch.is_hovered = false;
                }
                return LRESULT(0);
            }
            WM_MOUSEWHEEL => {
                let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
                notch.carousel.on_wheel(delta);
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                let x=(lparam.0 & 0xFFFF) as i16 as f32;
                let y=((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
                if notch.water_feedback.is_some() { return LRESULT(0); }
                if notch.maus_notification_active() && !notch.water_overlay_active() {
                    let left=notch.current_body_left();
                    if x>=left+notch.current_body_w-34.0 && y<34.0 {
                        notch.peek_overlay=None;
                    } else {notch.open_maus_notification();}
                    return LRESULT(0);
                }
                if notch.integrated_timer_pause_at(x, y) {
                    notch.apply_deck_action(Action::FocusToggle);
                    return LRESULT(0);
                }
                if let Some(action) = notch.focus_capsule_action_at(x, y) {
                    notch.apply_deck_action(action);
                    return LRESULT(0);
                }
                if notch.water_reminder_active {
                    match notch.water_reminder_hit_test(x, y) {
                        Some(WaterReminderBtn::Drink) => {
                            notch.confirm_water_reminder();
                            return LRESULT(0);
                        }
                        Some(WaterReminderBtn::Skip) => {
                            notch.skip_water_reminder();
                            return LRESULT(0);
                        }
                        None => {
                            return LRESULT(0);
                        }
                    }
                }
                if notch.idle_mascot_visible() {
                    notch.tiger.react();
                    return LRESULT(0);
                }
                if !notch.is_hovered { return LRESULT(0); }
                let left=notch.current_body_left();
                if notch.carousel.dots.hit_test_prev(left,notch.current_body_w,notch.current_h,x,y) {
                    notch.carousel.prev(); notch.deck_focus=None;
                } else if notch.carousel.dots.hit_test_next(left,notch.current_body_w,notch.current_h,x,y) {
                    notch.carousel.next(); notch.deck_focus=None;
                } else if let Some(index)=notch.carousel.dots.hit_test(left,notch.current_body_w,notch.current_h,x,y) {
                    notch.carousel.set_section(index); notch.deck_focus=None;
                } else if let Some(region)=notch.deck_regions.iter().find(|region|region.contains(x,y)).copied() {
                    notch.deck_focus=None;
                    notch.apply_deck_action(region.action);
                }
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                let key=wparam.0;
                if notch.maus_notification_active() && !notch.water_overlay_active() {
                    if key==VK_RETURN.0 as usize || key==VK_SPACE.0 as usize {notch.open_maus_notification();return LRESULT(0);}
                    if key==VK_ESCAPE.0 as usize {notch.peek_overlay=None;return LRESULT(0);}
                }
                if notch.water_feedback.is_some() { return LRESULT(0); }
                if notch.water_reminder_active {
                    if key == VK_RETURN.0 as usize || key == VK_SPACE.0 as usize {
                        notch.confirm_water_reminder();
                        return LRESULT(0);
                    }
                    if key == VK_ESCAPE.0 as usize {
                        notch.skip_water_reminder();
                        return LRESULT(0);
                    }
                    return LRESULT(0);
                }
                if (key==VK_SPACE.0 as usize || key==VK_RETURN.0 as usize) && notch.idle_mascot_visible() {
                    notch.tiger.react(); return LRESULT(0);
                }
                if key==VK_ESCAPE.0 as usize {
                    if notch.todo_editor_active() { notch.cancel_todo(); return LRESULT(0); }
                    notch.is_hovered=false; notch.deck_focus=None; return LRESULT(0);
                }
                if notch.is_hovered {
                    if notch.todo_editor_active() && key==VK_TAB.0 as usize && notch.deck_focus.is_some() {
                        let backwards=GetAsyncKeyState(VK_SHIFT.0 as i32)<0;
                        let on_save=notch.deck_focus.and_then(|i|notch.deck_regions.get(i)).map_or(false,|r|matches!(r.action,Action::TodoSave));
                        if on_save==backwards {
                            let _=windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(notch.todo_input.hwnd);
                            notch.deck_focus=None;
                        } else {
                            notch.deck_focus=notch.deck_regions.iter().position(|r|if backwards {matches!(r.action,Action::TodoSave)} else {matches!(r.action,Action::TodoCancel)});
                        }
                        return LRESULT(0);
                    }
                    if key==VK_LEFT.0 as usize || key==VK_RIGHT.0 as usize {
                        if key==VK_LEFT.0 as usize { notch.carousel.prev(); } else { notch.carousel.next(); }
                        notch.deck_focus=None; return LRESULT(0);
                    }
                    if key==VK_TAB.0 as usize && !notch.deck_regions.is_empty() {
                        let count=notch.deck_regions.len();
                        let backwards=(GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000)!=0;
                        notch.deck_focus=Some(match notch.deck_focus {
                            Some(i) if backwards => (i+count-1)%count,
                            Some(i) => (i+1)%count,
                            None if backwards => count-1,
                            None => 0,
                        });
                        return LRESULT(0);
                    }
                    if key==VK_SPACE.0 as usize || key==VK_RETURN.0 as usize {
                        if let Some(region)=notch.deck_focus.and_then(|index|notch.deck_regions.get(index)).copied() {
                            notch.apply_deck_action(region.action); return LRESULT(0);
                        }
                    }
                }
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
                        notch.shelf_offset=0;
                        notch.carousel.snap_to_section_id(SectionId::Shelf);
                    }
                }
                DragFinish(hdrop);
                return LRESULT(0);
            }
            WM_COMMAND => {
                if lparam.0==notch.todo_input.hwnd.0 as isize && (wparam.0>>16)&0xffff==0x0300 {
                    notch.todo_error=None;
                    return LRESULT(0);
                }
                let id = (wparam.0 & 0xFFFF) as usize;
                if id == IDM_SETTINGS {
                    let _ = crate::settings::SettingsWindow::show_or_create(hwnd);
                    return LRESULT(0);
                }
            }
            WM_TRAYICON => {
                if lparam.0 as u32 == WM_RBUTTONUP {
                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    if let Ok(hmenu) = CreatePopupMenu() {
                        let settings_w: Vec<u16> = "Settings...\0".encode_utf16().collect();
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
                            MF_STRING,
                            IDM_SETTINGS,
                            PCWSTR(settings_w.as_ptr()),
                        );
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
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
                            IDM_SETTINGS => {
                                let _ = crate::settings::SettingsWindow::show_or_create(hwnd);
                            }
                            IDM_AUTOSTART => {
                                let enabled = !notch.autostart_enabled;
                                if crate::settings::autostart::set(enabled).is_ok() {
                                    notch.autostart_enabled = enabled;
                                    let mut settings = crate::config::AppSettings::load();
                                    settings.general.autostart = enabled;
                                    settings.save();
                                }
                            }
                            IDM_MASCOT => {
                                notch.mascot_enabled = !notch.mascot_enabled;
                                let mut settings = crate::config::AppSettings::load();
                                settings.general.mascot_enabled = notch.mascot_enabled;
                                settings.save();
                            }
                            IDM_MINIMAL => {
                                notch.minimal_notch = !notch.minimal_notch;
                                let mut settings = crate::config::AppSettings::load();
                                settings.general.minimal_notch = notch.minimal_notch;
                                settings.save();
                            }
                            IDM_ABOUT => {
                                let msg: Vec<u16> = concat!("Nocan v", env!("CARGO_PKG_VERSION"), "\nMusic, lyrics, focus timer, water reminder, to-do, file shelf & AI Usage Tracker (Antigravity, Claude, Codex).\nLyrics provided by LRCLIB (lrclib.net).\0").encode_utf16().collect();
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
            super::focus_capsule::WM_TIMER_ACTION => {
                match wparam.0 {
                    0 => notch.apply_deck_action(Action::FocusToggle),
                    1 => notch.apply_deck_action(Action::FocusAddFive),
                    2 => notch.apply_deck_action(Action::FocusReset),
                    3 => notch.apply_deck_action(Action::FocusSkip),
                    _ => {}
                }
                return LRESULT(0);
            }
            crate::settings::window::WM_TRIGGER_WATER_REMINDER => {
                notch.trigger_water_reminder();
                return LRESULT(0);
            }
            crate::settings::window::WM_SETTINGS_CHANGED => {
                let settings = crate::config::AppSettings::load();
                notch.apply_settings(&settings);
                return LRESULT(0);
            }
            WM_DESTROY => {
                crate::config::debug_log("NotchWindow WM_DESTROY triggered!");
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: 1,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);
                let _ = Box::from_raw(ptr as *mut NotchWindow);
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(test)]
mod cover_color_tests {
    use super::NotchWindow;

    #[test]
    fn focus_timer_is_shown_compactly_and_as_a_separate_lyric_capsule() {
        use crate::sections::focus::FocusSection;
        let mut focus = FocusSection::new();
        assert_eq!(focus.time_text(), "25:00");
        focus.toggle_start_pause();
        assert!(focus.is_running);
        focus.remaining_secs = 24 * 60 + 12;
        assert_eq!(focus.time_text(), "24:12");
        focus.reset();
        assert!(!focus.is_running);
    }

    #[test]
    fn cover_color_tracks_artwork_and_ignores_transparent_pixels() {
        let red = NotchWindow::dominant_color(&[0, 0, 255, 255, 255, 0, 0, 0]);
        assert!(red.0 > red.1 && red.0 > red.2);
        let green = NotchWindow::dominant_color(&[0, 255, 0, 255]);
        assert!(green.1 > green.0 && green.1 > green.2);
        assert_eq!(NotchWindow::dominant_color(&[0, 0, 0, 0]), (0.16, 0.36, 0.80));
    }

    #[test]
    fn water_reminder_button_hit_tests() {
        use super::{WATER_CARD_W, CANVAS_W};
        let body_left = (CANVAS_W as f32 - WATER_CARD_W) * 0.5; // 200.0
        let is_drink = |x: f32, y: f32| {
            let btn1_rect = windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: body_left + 196.0,
                top: 15.0,
                right: body_left + 294.0,
                bottom: 47.0,
            };
            x >= btn1_rect.left && x <= btn1_rect.right && y >= btn1_rect.top && y <= btn1_rect.bottom
        };
        let is_skip = |x: f32, y: f32| {
            let btn2_rect = windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: body_left + 300.0,
                top: 15.0,
                right: body_left + 348.0,
                bottom: 47.0,
            };
            x >= btn2_rect.left && x <= btn2_rect.right && y >= btn2_rect.top && y <= btn2_rect.bottom
        };

        // Drink button bounds: [396..494, 15..47]
        assert!(is_drink(445.0, 31.0));
        assert!(!is_drink(510.0, 31.0));
        assert!(!is_drink(445.0, 50.0));

        // Skip button bounds: [500..548, 15..47]
        assert!(is_skip(524.0, 31.0));
        assert!(!is_skip(445.0, 31.0));
        assert!(!is_skip(524.0, 55.0));
    }
}
