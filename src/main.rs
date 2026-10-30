#![windows_subsystem = "windows"]

use windows::core::{PCWSTR, Result};
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
    INFINITE,
};

pub mod config;
pub mod graphics;
pub mod integrations;
pub mod notch;
pub mod sections;
pub mod settings;
pub mod storage;

use integrations::discord::DiscordRpcService;
use integrations::media::MediaService;
use notch::window::NotchWindow;

fn main() -> Result<()> {
    if std::env::var_os("NOCAN_DATA_DIR").is_none() && std::env::args().any(|arg|arg.starts_with("--preview-") || arg.starts_with("--dump-")) {
        std::env::set_var("NOCAN_DATA_DIR",std::env::temp_dir().join(format!("nocan-preview-{}",std::process::id())));
    }
    if std::env::args().any(|arg|arg=="--inspect-openmaus") {
        let directory=integrations::openmaus::get_openmaus_data_dir();
        let mut monitor=integrations::openmaus_activity::ActivityMonitor::new(directory.clone());
        let poll=monitor.poll();
        let report=integrations::openmaus::load_usage_report_from(&directory);
        println!("source_found={} server_online={} events={} schedules={} agents_running={} automations_running={} startup_notifications={} error={:?}",
            poll.snapshot.source_found,poll.snapshot.server_online,poll.snapshot.events.len(),poll.snapshot.routines.len(),
            poll.snapshot.running_agents,poll.snapshot.running_automations,poll.notifications.len(),poll.snapshot.sync_error);
        println!("usage_turns={} today_turns={} bots={} models={} providers={}",report.total_turns,report.today_turns,report.bots.len(),report.models.len(),report.providers.len());
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--preview-focus" || arg == "--preview-todo") {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?; }
        NotchWindow::create(MediaService::new())?;
        notch::window::run_message_loop();
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--preview-settings") {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?; }
        settings::SettingsWindow::show_or_create(HWND::default())?;
        notch::window::run_message_loop();
        return Ok(());
    }

    if std::env::args().any(|arg| arg == "--dump-settings-preview") {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?; }
        let dir = std::env::args().nth(2).unwrap_or_else(|| "target/settings-preview".to_string());
        settings::ui::dump_settings_previews(std::path::Path::new(&dir))?;
        println!("Dumped settings previews to {}", dir);
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--preview-branding") {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?; }
        NotchWindow::create(MediaService::new())?;
        return Ok(());
    }
    let log_dir = storage::data_dir();
    let _ = std::fs::create_dir_all(&log_dir);

    let panic_log_path = log_dir.join("nocan_panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::fs::write(&panic_log_path, format!("{:?}", info));
    }));

    let log = |msg: &str| {
        crate::config::debug_log(msg);
    };

    log("Starting nocan...");

    // 1. Single Instance Check
    let mutex_name: Vec<u16> = "NocanNotch.SingleInstance\0".encode_utf16().collect();
    unsafe {
        let _handle = CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr()))?;
        let last_err = windows::Win32::Foundation::GetLastError();
        if last_err == ERROR_ALREADY_EXISTS {
            log("Exiting: ERROR_ALREADY_EXISTS - signaling running instance to open Settings");
            let event_name: Vec<u16> = "NocanNotch.OpenSettingsEvent\0".encode_utf16().collect();
            if let Ok(ev) = OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(event_name.as_ptr())) {
                let _ = SetEvent(ev);
                let _ = CloseHandle(ev);
                log("Signaled NocanNotch.OpenSettingsEvent to running instance");
            }
            return Ok(());
        }
    }
    log("Single instance acquired");
    if settings::autostart::enabled() {
        // Versioned executable names must not leave Windows launching an older build.
        if settings::autostart::set(true).is_err() {
            log("Could not refresh the existing startup entry for this build");
        }
    }

    // 2. Initialize COM
    unsafe {
        let res = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        log(&format!("CoInitializeEx: {:?}", res));
    }

    // 3. Start Media & Discord RPC Services
    log("Starting MediaService...");
    let media_service = MediaService::new();
    media_service.start();

    log("Starting DiscordRpcService...");
    let discord_service = DiscordRpcService::new();
    discord_service.start(media_service.clone());

    // 4. Create Living Notch Window
    log("Creating NotchWindow...");
    let notch_hwnd = match NotchWindow::create(media_service.clone()) {
        Ok(hwnd) => {
            log(&format!("NotchWindow created successfully, HWND: {:?}", hwnd));
            hwnd
        }
        Err(e) => {
            log(&format!("Failed to create NotchWindow: {:?}", e));
            return Err(e);
        }
    };

    if std::env::args().any(|a| a == "--settings") {
        let _ = settings::SettingsWindow::show_or_create(notch_hwnd);
    }

    // Named event listener for --settings invocation from second instances
    let event_name: Vec<u16> = "NocanNotch.OpenSettingsEvent\0".encode_utf16().collect();
    let settings_event = unsafe {
        CreateEventW(None, false, false, PCWSTR(event_name.as_ptr()))?
    };

    let notch_hwnd_raw = notch_hwnd.0 as isize;
    let event_raw = settings_event.0 as isize;
    std::thread::spawn(move || {
        let ev = HANDLE(event_raw as *mut _);
        let hwnd = HWND(notch_hwnd_raw as *mut _);
        loop {
            let wait_res = unsafe { WaitForSingleObject(ev, INFINITE) };
            if wait_res.0 == 0 {
                crate::config::debug_log("OpenSettingsEvent signaled! Posting WM_COMMAND 1000 to notch");
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_COMMAND,
                        WPARAM(1000),
                        LPARAM(0),
                    );
                }
            } else {
                break;
            }
        }
    });

    // 5. Message Loop
    log("Entering run_message_loop...");
    notch::window::run_message_loop();
    log("run_message_loop exited");

    Ok(())
}
