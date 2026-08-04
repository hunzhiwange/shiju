use tempfile::tempdir;
use vibe_agent::app::agent::config::AuditConfig;
use vibe_agent::app::agent::security::syscall_anomaly::{
    SyscallAnomalyAlert, SyscallAnomalyConfig, SyscallAnomalyDetector, SyscallAnomalyKind,
    map_linux_x86_64_syscall, normalize_baseline, parse_syscall_signal,
};

fn detector_with(config: SyscallAnomalyConfig) -> SyscallAnomalyDetector {
    let tmp = tempdir().expect("tempdir");
    let audit = AuditConfig { enabled: false, ..AuditConfig::default() };
    SyscallAnomalyDetector::new(config, tmp.path(), audit)
}

fn enabled_config() -> SyscallAnomalyConfig {
    SyscallAnomalyConfig {
        enabled: true,
        alert_on_unknown_syscall: true,
        ..SyscallAnomalyConfig::default()
    }
}

// 测试从audit日志中提取数字形式的系统调用号并转换为名称
#[test]
fn parse_syscall_signal_extracts_numeric_audit_syscall() {
    let line =
        r#"audit: type=1326 audit(1.234:66): auid=0 uid=0 gid=0 arch=c000003e syscall=59 compat=0"#;
    let signal = parse_syscall_signal(line).expect("signal should parse");
    assert_eq!(signal.syscall.as_deref(), Some("execve"));
    assert!(!signal.denied);
}

// 测试从seccomp日志中识别被拒绝的系统调用
#[test]
fn parse_syscall_signal_marks_denied_from_seccomp_line() {
    let line = "seccomp: denied syscall=openat by profile strict";
    let signal = parse_syscall_signal(line).expect("signal should parse");
    assert_eq!(signal.syscall.as_deref(), Some("openat"));
    assert!(signal.denied);
}

// 测试提取符号名称形式(如__NR_openat)的系统调用
#[test]
fn parse_syscall_signal_extracts_symbolic_name() {
    let line = "seccomp denied syscall=__NR_openat profile=default";
    let signal = parse_syscall_signal(line).expect("signal should parse");
    assert_eq!(signal.syscall.as_deref(), Some("openat"));
    assert!(signal.denied);
}

// 测试提取空格分隔的数字形式系统调用号
#[test]
fn parse_syscall_signal_extracts_space_separated_number() {
    let line = "seccomp blocked system call nr 59 from child";
    let signal = parse_syscall_signal(line).expect("signal should parse");
    assert_eq!(signal.syscall.as_deref(), Some("execve"));
    assert!(signal.denied);
}

// 测试提取十六进制形式的系统调用号
#[test]
fn parse_syscall_signal_extracts_hex_syscall_number() {
    let line = "audit: type=1326 syscall=0x3b seccomp denied";
    let signal = parse_syscall_signal(line).expect("signal should parse");
    assert_eq!(signal.syscall.as_deref(), Some("execve"));
    assert!(signal.denied);
}

// 测试检测器对不在基线中的未知系统调用发出告警
#[test]
fn detector_alerts_on_unknown_syscall() {
    let config = SyscallAnomalyConfig {
        baseline_syscalls: vec!["read".into(), "write".into()],
        ..enabled_config()
    };
    let detector = detector_with(config);
    let alerts = detector.inspect_command_output(
        "echo hi",
        "",
        "audit: type=1326 syscall=openat denied",
        Some(1),
    );
    assert!(alerts.iter().any(|alert| alert.kind == SyscallAnomalyKind::UnknownSyscall));
}

// 测试检测器对系统调用拒绝率突增发出告警
#[test]
fn detector_alerts_on_denied_rate_spike() {
    let config = SyscallAnomalyConfig {
        strict_mode: false,
        max_denied_events_per_minute: 1,
        baseline_syscalls: vec!["openat".into()],
        ..enabled_config()
    };
    let detector = detector_with(config);
    let alerts = detector.inspect_command_output(
        "echo hi",
        "",
        "seccomp denied syscall=openat\nseccomp denied syscall=openat",
        Some(1),
    );
    assert!(alerts.iter().any(|alert| alert.kind == SyscallAnomalyKind::DeniedRateExceeded));
}

// 测试禁用模式下检测器不产生任何告警
#[test]
fn detector_respects_disabled_mode() {
    let config = SyscallAnomalyConfig { enabled: false, ..SyscallAnomalyConfig::default() };
    let detector = detector_with(config);
    let alerts =
        detector.inspect_command_output("echo hi", "", "seccomp denied syscall=openat", Some(1));
    assert!(alerts.is_empty());
}

// 测试告警冷却时间机制,防止重复告警
#[test]
fn detector_applies_alert_cooldown() {
    let config = SyscallAnomalyConfig {
        max_denied_events_per_minute: 1,
        max_alerts_per_minute: 100,
        alert_cooldown_secs: 120,
        baseline_syscalls: vec!["openat".into()],
        ..enabled_config()
    };
    let detector = detector_with(config);

    let first = detector.inspect_command_output(
        "echo hi",
        "",
        "seccomp denied syscall=openat\nseccomp denied syscall=openat",
        Some(1),
    );
    assert!(first.iter().any(|alert| alert.kind == SyscallAnomalyKind::DeniedRateExceeded));

    let second = detector.inspect_command_output(
        "echo hi",
        "",
        "seccomp denied syscall=openat\nseccomp denied syscall=openat",
        Some(1),
    );
    assert!(
        !second.iter().any(|alert| alert.kind == SyscallAnomalyKind::DeniedRateExceeded),
        "cooldown should suppress repeated identical rate alerts"
    );
}

// 测试每分钟告警数量限制机制
#[test]
fn detector_limits_alerts_per_minute() {
    let config = SyscallAnomalyConfig {
        max_alerts_per_minute: 1,
        alert_cooldown_secs: 1,
        baseline_syscalls: vec!["read".into(), "write".into()],
        ..enabled_config()
    };
    let detector = detector_with(config);
    let alerts = detector.inspect_command_output(
        "echo hi",
        "",
        "seccomp denied syscall=openat\nseccomp denied syscall=clone3",
        Some(1),
    );
    assert_eq!(alerts.len(), 1, "alert budget should cap emitted alerts");
}

// 测试默认基线包含常见的已映射系统调用
#[test]
fn default_baseline_covers_common_mapped_syscalls() {
    let baseline = normalize_baseline(&SyscallAnomalyConfig::default().baseline_syscalls);
    let mapped_common = [43_i64, 50, 57, 72, 218, 273];
    for syscall_nr in mapped_common {
        let name = map_linux_x86_64_syscall(syscall_nr).expect("mapping should exist");
        assert!(baseline.contains(name), "default baseline should include mapped syscall {name}");
    }
}
