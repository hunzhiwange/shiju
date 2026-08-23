//! 验证初始化系统与服务命令的解析、序列化和延迟配置。

use super::{InitSystem, ServiceCommands, internal_command_delay};
use std::time::Duration;

#[test]
fn init_system_from_str_accepts_supported_values_case_insensitively() {
    assert_eq!("auto".parse::<InitSystem>().unwrap(), InitSystem::Auto);
    assert_eq!("AUTO".parse::<InitSystem>().unwrap(), InitSystem::Auto);
    assert_eq!("systemd".parse::<InitSystem>().unwrap(), InitSystem::Systemd);
    assert_eq!("SyStEmD".parse::<InitSystem>().unwrap(), InitSystem::Systemd);
    assert_eq!("openrc".parse::<InitSystem>().unwrap(), InitSystem::Openrc);
    assert_eq!("OPENRC".parse::<InitSystem>().unwrap(), InitSystem::Openrc);
}

#[test]
fn init_system_from_str_rejects_unknown_values_with_supported_hint() {
    let err = "launchd".parse::<InitSystem>().expect_err("unknown init system should fail");
    let text = err.to_string();

    assert!(text.contains("Unknown init system: 'launchd'"));
    assert!(text.contains("Supported: auto, systemd, openrc"));
}

#[test]
fn init_system_default_and_debug_are_stable() {
    assert_eq!(InitSystem::default(), InitSystem::Auto);
    assert_eq!(format!("{:?}", InitSystem::Openrc), "Openrc");
}

#[test]
fn concrete_init_systems_resolve_to_themselves_without_detection() {
    assert_eq!(InitSystem::Systemd.resolve().unwrap(), InitSystem::Systemd);
    assert_eq!(InitSystem::Openrc.resolve().unwrap(), InitSystem::Openrc);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn auto_resolve_uses_systemd_placeholder_off_linux() {
    assert_eq!(InitSystem::Auto.resolve().unwrap(), InitSystem::Systemd);
}

#[test]
fn service_commands_serde_round_trips_all_variants() {
    let variants = [
        ServiceCommands::Install,
        ServiceCommands::Start,
        ServiceCommands::Stop,
        ServiceCommands::Restart,
        ServiceCommands::Status,
        ServiceCommands::Uninstall,
    ];

    for command in variants {
        let encoded = serde_json::to_string(&command).expect("service command should serialize");
        let decoded: ServiceCommands =
            serde_json::from_str(&encoded).expect("service command should deserialize");
        assert_eq!(decoded, command);
    }
}

#[test]
fn service_commands_clone_and_debug_include_variant_name() {
    let command = ServiceCommands::Restart;
    let cloned = command.clone();

    assert_eq!(cloned, ServiceCommands::Restart);
    assert_eq!(format!("{:?}", cloned), "Restart");
}

#[test]
fn internal_command_delay_accepts_trimmed_values_within_safe_range() {
    assert_eq!(internal_command_delay(Some(" 1 ")), Some(Duration::from_millis(1)));
    assert_eq!(internal_command_delay(Some("10000")), Some(Duration::from_millis(10_000)));
}

#[test]
fn internal_command_delay_rejects_missing_invalid_zero_and_excessive_values() {
    for raw in [None, Some(""), Some("abc"), Some("-1"), Some("0"), Some("10001")] {
        assert_eq!(internal_command_delay(raw), None, "raw delay: {raw:?}");
    }
}
