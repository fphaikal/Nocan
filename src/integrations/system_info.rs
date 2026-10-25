use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

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
