#![windows_subsystem = "windows"]

use windows::core::{PCWSTR, Result};
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::CreateMutexW;

pub mod graphics;
pub mod integrations;
pub mod notch;
pub mod sections;

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
            log("Exiting: ERROR_ALREADY_EXISTS");
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
    match NotchWindow::create(media_service.clone()) {
        Ok(hwnd) => {
            log(&format!("NotchWindow created successfully, HWND: {:?}", hwnd));
        }
        Err(e) => {
            log(&format!("Failed to create NotchWindow: {:?}", e));
            return Err(e);
        }
    };

    // 5. Message Loop
    log("Entering run_message_loop...");
    notch::window::run_message_loop();
    log("run_message_loop exited");

    Ok(())
}
