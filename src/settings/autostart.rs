use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ};

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0";
const NAME: &str = "Nocan\0";
fn wide(s:&str) -> Vec<u16> { s.encode_utf16().collect() }

pub fn enabled() -> bool {
    let key=wide(RUN); let name=wide(NAME);
    unsafe { RegGetValueW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr()), RRF_RT_REG_SZ, None, None, None) == ERROR_SUCCESS }
}

pub fn set(enabled:bool) -> Result<()> {
    let subkey=wide(RUN); let name=wide(NAME); let mut key=HKEY::default();
    unsafe {
        RegCreateKeyExW(HKEY_CURRENT_USER,PCWSTR(subkey.as_ptr()),0,PCWSTR::null(),REG_OPTION_NON_VOLATILE,KEY_SET_VALUE,None,&mut key,None).ok()?;
        let result = if enabled {
            let command = format!("\"{}\"\0", std::env::current_exe().map_err(|_| windows::core::Error::from_win32())?.display());
            let chars:Vec<u16> = command.encode_utf16().collect();
            let bytes = std::slice::from_raw_parts(chars.as_ptr() as *const u8, chars.len()*2);
            RegSetValueExW(key,PCWSTR(name.as_ptr()),0,REG_SZ,Some(bytes))
        } else { RegDeleteValueW(key,PCWSTR(name.as_ptr())) };
        let _=RegCloseKey(key);
        if result == ERROR_FILE_NOT_FOUND && !enabled { return Ok(()); }
        result.ok()
    }
}
