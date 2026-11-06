//! Whether the computer runs on battery, so background writing can wait for the charger:
//! local inference keeps the processor busy for a minute per entry and drains a laptop.
//!
//! macOS answers through `pmset -g batt`, Windows through `GetSystemPowerStatus` (unverified on
//! real hardware), Linux through `/sys/class/power_supply`. Anywhere else, or when the answer
//! cannot be read, the computer counts as plugged in, so writing is never blocked by a guess.

/// Whether `pmset -g batt` says the Mac draws from its battery.
// Called on macOS only; tested everywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn pmset_on_battery(output: &str) -> bool {
    output
        .lines()
        .next()
        .is_some_and(|first| first.contains("'Battery Power'"))
}

/// Whether Linux power supplies say no charger is connected while a battery exists.
// Called on Linux only; tested everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn linux_on_battery(supplies: &[(String, String)]) -> bool {
    let has_battery = supplies.iter().any(|(kind, _)| kind == "Battery");
    let mains: Vec<&String> = supplies
        .iter()
        .filter(|(kind, _)| kind == "Mains" || kind == "USB")
        .map(|(_, online)| online)
        .collect();
    has_battery && !mains.is_empty() && mains.iter().all(|online| online.trim() == "0")
}

#[cfg(target_os = "macos")]
pub fn on_battery() -> bool {
    std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|out| pmset_on_battery(&out))
}

#[cfg(target_os = "windows")]
pub fn on_battery() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // SAFETY: the call only writes into the struct it is given.
    let ok = unsafe { GetSystemPowerStatus(&mut status) } != 0;
    // ACLineStatus: 0 offline, 1 online, 255 unknown.
    ok && status.ACLineStatus == 0
}

#[cfg(target_os = "linux")]
pub fn on_battery() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") else {
        return false;
    };
    let supplies: Vec<(String, String)> = entries
        .flatten()
        .map(|e| {
            let read =
                |name: &str| std::fs::read_to_string(e.path().join(name)).unwrap_or_default();
            (read("type").trim().to_string(), read("online"))
        })
        .collect();
    linux_on_battery(&supplies)
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn on_battery() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pmset() {
        assert!(pmset_on_battery(
            "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t84%; discharging"
        ));
        assert!(!pmset_on_battery(
            "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t100%; charged"
        ));
        // A desktop Mac has no battery line at all.
        assert!(!pmset_on_battery("Now drawing from 'AC Power'\n"));
        assert!(!pmset_on_battery(""));
    }

    #[test]
    fn reads_linux_power_supplies() {
        let s = |k: &str, o: &str| (k.to_string(), o.to_string());
        assert!(linux_on_battery(&[s("Mains", "0\n"), s("Battery", "")]));
        assert!(!linux_on_battery(&[s("Mains", "1\n"), s("Battery", "")]));
        // A desktop: no battery, never "on battery".
        assert!(!linux_on_battery(&[s("Mains", "0\n")]));
        // A battery with no charger reported: unknown, so not blocked.
        assert!(!linux_on_battery(&[s("Battery", "")]));
    }
}
