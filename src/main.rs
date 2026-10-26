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

use integrations::discord::DiscordRpcService;
use integrations::media::MediaService;
use notch::window::NotchWindow;

fn main() -> Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let _ = std::fs::write("C:\\Users\\abufa\\AppData\\Local\\Nocan\\nocan_panic.log", format!("{:?}", info));
    }));

    let log = |msg: &str| {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("C:\\Users\\abufa\\AppData\\Local\\Nocan\\nocan_debug.log") {
            let _ = writeln!(f, "[{}] {}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(), msg);
        }
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
