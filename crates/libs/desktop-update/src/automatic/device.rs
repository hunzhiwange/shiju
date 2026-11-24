use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn command(program: &str, args: &[&str]) -> Option<String> {
    // Keep native probes bounded even when a system service stops responding.
    let mut output = tempfile::tempfile().ok()?;
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(output.try_clone().ok()?)
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                use std::io::{Read, Seek};
                output.rewind().ok()?;
                let mut value = String::new();
                output.take(1024 * 1024).read_to_string(&mut value).ok()?;
                return Some(value);
            }
            Ok(Some(_)) => return None,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

const IDLE_MS: u64 = 10 * 60 * 1000;

#[cfg(target_os = "macos")]
pub(super) fn idle() -> bool {
    command("/usr/sbin/ioreg", &["-c", "IOHIDSystem", "-d", "1"]).is_some_and(|output| {
        output.lines().any(|line| {
            line.split_once("\"HIDIdleTime\" = ")
                .and_then(|(_, value)| value.trim().parse::<u64>().ok())
                .is_some_and(|nanoseconds| nanoseconds / 1_000_000 >= IDLE_MS)
        })
    })
}

#[cfg(target_os = "macos")]
pub(super) fn wifi() -> bool {
    let Some(route) = command("/sbin/route", &["-n", "get", "default"]) else {
        return false;
    };
    let Some(interface) = route.lines().find_map(|line| line.trim().strip_prefix("interface: "))
    else {
        return false;
    };
    command("/usr/sbin/networksetup", &["-listallhardwareports"]).is_some_and(|output| {
        output.split("\n\n").any(|port| {
            port.lines().any(|line| {
                matches!(line.trim(), "Hardware Port: Wi-Fi" | "Hardware Port: AirPort")
            }) && port.lines().any(|line| line.trim().strip_prefix("Device: ") == Some(interface))
        })
    })
}

#[cfg(target_os = "macos")]
pub(super) fn plugged_in() -> bool {
    command("/usr/bin/pmset", &["-g", "ps"])
        .is_some_and(|output| output.lines().next() == Some("Now drawing from 'AC Power'"))
}

#[cfg(target_os = "linux")]
pub(super) fn idle() -> bool {
    if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("x11") {
        return command("xprintidle", &[])
            .and_then(|value| value.trim().parse::<u64>().ok())
            .is_some_and(|milliseconds| milliseconds >= IDLE_MS);
    }
    command(
        "gdbus",
        &[
            "call",
            "--session",
            "--dest",
            "org.gnome.Mutter.IdleMonitor",
            "--object-path",
            "/org/gnome/Mutter/IdleMonitor/Core",
            "--method",
            "org.gnome.Mutter.IdleMonitor.GetIdletime",
        ],
    )
    .and_then(|value| {
        value.trim().strip_prefix("(uint64 ")?.strip_suffix(",)")?.parse::<u64>().ok()
    })
    .is_some_and(|milliseconds| milliseconds >= IDLE_MS)
}

#[cfg(target_os = "linux")]
pub(super) fn wifi() -> bool {
    command("nmcli", &["-t", "-f", "TYPE,STATE", "device", "status"])
        .is_some_and(|output| output.lines().any(|line| line == "wifi:connected"))
}

#[cfg(target_os = "linux")]
pub(super) fn plugged_in() -> bool {
    std::fs::read_dir("/sys/class/power_supply").is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let path = entry.path();
            std::fs::read_to_string(path.join("type")).is_ok_and(|kind| {
                matches!(kind.trim(), "Mains" | "USB" | "USB_C" | "USB_PD" | "USB_PD_DRP")
            }) && std::fs::read_to_string(path.join("online"))
                .is_ok_and(|value| value.trim() == "1")
        })
    })
}

#[cfg(target_os = "windows")]
pub(super) fn idle() -> bool {
    #[repr(C)]
    struct LastInputInfo {
        size: u32,
        time: u32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetLastInputInfo(info: *mut LastInputInfo) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetTickCount() -> u32;
    }
    let mut info = LastInputInfo { size: std::mem::size_of::<LastInputInfo>() as u32, time: 0 };
    unsafe {
        GetLastInputInfo(&mut info) != 0
            && u64::from(GetTickCount().wrapping_sub(info.time)) >= IDLE_MS
    }
}

#[cfg(target_os = "windows")]
pub(super) fn wifi() -> bool {
    command("powershell.exe", &[
        "-NoProfile", "-NonInteractive", "-Command",
        "if (Get-CimInstance -Namespace root/StandardCimv2 -ClassName MSFT_NetAdapter | Where-Object { $_.MediaConnectState -eq 1 -and $_.NdisPhysicalMedium -in 1,9 }) { 'connected' }",
    ]).is_some_and(|output| output.trim() == "connected")
}

#[cfg(target_os = "windows")]
pub(super) fn plugged_in() -> bool {
    #[repr(C)]
    struct PowerStatus {
        ac: u8,
        battery_flag: u8,
        battery_percent: u8,
        system_status: u8,
        battery_life: u32,
        full_life: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemPowerStatus(status: *mut PowerStatus) -> i32;
    }
    let mut status = std::mem::MaybeUninit::<PowerStatus>::uninit();
    unsafe { GetSystemPowerStatus(status.as_mut_ptr()) != 0 && status.assume_init().ac == 1 }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) fn idle() -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) fn wifi() -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) fn plugged_in() -> bool {
    false
}
