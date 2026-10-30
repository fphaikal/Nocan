use windows::core::{Result, PCWSTR};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{LoadImageW, HICON, IMAGE_ICON, LR_SHARED};

pub fn app_icon(size: i32) -> Result<HICON> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        // Resource 1 is linked into the executable; LR_SHARED gives Windows ownership.
        let icon = LoadImageW(instance, PCWSTR(1usize as *const u16), IMAGE_ICON, size, size, LR_SHARED)?;
        Ok(HICON(icon.0))
    }
}
