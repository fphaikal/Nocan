use std::collections::HashSet;
use windows::core::GUID;
use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Win32::Devices::FunctionDiscovery::PKEY_DeviceInterface_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// Windows Power Setting GUID for display brightness changes (`{aded5e82-b909-4619-9949-f5d71dac0bcb}`).
/// Matches `nocan.original.exe` (`0x140094e74`).
pub const GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS: GUID = GUID::from_u128(
    0xaded5e82_b909_4619_9949_f5d71dac0bcb,
);

#[derive(Clone, Debug)]
pub struct SystemStatus {
    pub time_str: String,
    pub date_str: String,
    pub clock_str: String,
    pub battery_percent: Option<u8>,
    pub is_charging: bool,
    pub is_battery_present: bool,
    pub is_online: bool,
}

impl Default for SystemStatus {
    fn default() -> Self {
        Self {
            time_str: String::new(),
            date_str: String::new(),
            clock_str: String::new(),
            battery_percent: None,
            is_charging: false,
            is_battery_present: false,
            is_online: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AudioState {
    pub volume: f32,
    pub muted: bool,
    pub device_id: String,
    pub device_name: String,
    pub active_count: u32,
}

/// Queries the current system master audio volume, mute state, and default audio output device.
/// Matches `nocan.original.exe` (`0x14003b131..0x14003b550`).
pub fn query_audio_state() -> Option<AudioState> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;

        let active_count = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .ok()
            .and_then(|col| col.GetCount().ok())
            .unwrap_or(0);

        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;

        let mut device_id = String::new();
        if let Ok(pwstr) = device.GetId() {
            if !pwstr.is_null() {
                device_id = pwstr.to_string().unwrap_or_default();
            }
            CoTaskMemFree(Some(pwstr.0 as *const _));
        }

        let mut device_name = String::new();
        if let Ok(store) = device.OpenPropertyStore(STGM_READ) {
            if let Ok(prop) = store.GetValue(&PKEY_DeviceInterface_FriendlyName) {
                let s = prop.to_string();
                if !s.trim().is_empty() {
                    device_name = clean_audio_device_name(&s);
                }
            }
        }
        if device_name.is_empty() {
            device_name = "Audio Device".to_string();
        }

        let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let volume = ep_vol.GetMasterVolumeLevelScalar().unwrap_or(0.0).clamp(0.0, 1.0);
        let muted = ep_vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);

        Some(AudioState {
            volume,
            muted,
            device_id,
            device_name,
            active_count,
        })
    }
}

fn clean_audio_device_name(raw: &str) -> String {
    let trimmed = raw.trim();
    // Often Windows formats endpoints as "Headphones (AirBuds Pro)" -> extract inner if helpful or keep clean name
    if let (Some(open), Some(close)) = (trimmed.find('('), trimmed.rfind(')')) {
        if open < close {
            let prefix = trimmed[..open].trim();
            let inside = trimmed[open + 1..close].trim();
            if prefix.eq_ignore_ascii_case("Headphones")
                || prefix.eq_ignore_ascii_case("Headset")
                || prefix.eq_ignore_ascii_case("Speakers")
            {
                if !inside.is_empty()
                    && !inside.contains("Realtek")
                    && !inside.contains("High Definition Audio")
                {
                    return inside.to_string();
                }
            }
        }
    }
    trimmed.to_string()
}

/// Queries currently connected Bluetooth devices using WinRT `BluetoothDevice` + `DeviceInformation`.
/// Matches `nocan.original.exe` (`0x14002df00..0x14002e150`).
pub fn query_connected_bluetooth_devices() -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(selector) =
        BluetoothDevice::GetDeviceSelectorFromConnectionStatus(BluetoothConnectionStatus::Connected)
    {
        if let Ok(op) = DeviceInformation::FindAllAsyncAqsFilter(&selector) {
            if let Ok(list) = op.get() {
                let count = list.Size().unwrap_or(0);
                for i in 0..count {
                    if let Ok(info) = list.GetAt(i) {
                        if let Ok(name) = info.Name() {
                            let s = name.to_string_lossy();
                            let trimmed = s.trim();
                            if !trimmed.is_empty() {
                                set.insert(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    set
}

pub fn get_system_status() -> SystemStatus {
    let (date_str, clock_str, time_str) = format_current_time();

    let mut battery_percent = None;
    let mut is_charging = false;
    let mut is_battery_present = false;

    unsafe {
        let mut sps = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut sps).is_ok() {
            if sps.BatteryLifePercent != 255 {
                battery_percent = Some(sps.BatteryLifePercent);
                is_battery_present = true;
            }
            if sps.ACLineStatus == 1 || (sps.BatteryFlag & 8) != 0 {
                is_charging = true;
            }
        }
    }

    SystemStatus {
        time_str,
        date_str,
        clock_str,
        battery_percent,
        is_charging,
        is_battery_present,
        is_online: true,
    }
}

fn format_current_time() -> (String, String, String) {
    unsafe {
        let st = windows::Win32::System::SystemInformation::GetLocalTime();

        let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];

        let day_name = days.get(st.wDayOfWeek as usize).unwrap_or(&"");
        let month_name = months
            .get(st.wMonth.saturating_sub(1) as usize)
            .unwrap_or(&"");

        let date_str = format!("{} {} {} {}", day_name, st.wDay, month_name, st.wYear);
        let clock_str = format!("{:02}:{:02}", st.wHour, st.wMinute);
        let time_str = format!("{} {}", date_str, clock_str);
        (date_str, clock_str, time_str)
    }
}
