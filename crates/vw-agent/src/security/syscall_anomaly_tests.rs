//! 验证系统调用异常解析、告警去重限流与审计日志写入。

use super::*;
use crate::app::agent::config::{AuditConfig, SyscallAnomalyConfig};
use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::{TempDir, tempdir};

fn detector_with(mut config: SyscallAnomalyConfig) -> (TempDir, SyscallAnomalyDetector) {
    let tmp = tempdir().expect("tempdir");
    config.log_path = tmp.path().join("syscall-anomalies.jsonl").to_string_lossy().to_string();
    let audit = AuditConfig { enabled: false, ..AuditConfig::default() };
    let detector = SyscallAnomalyDetector::new(config, tmp.path(), audit);
    (tmp, detector)
}

fn enabled_config() -> SyscallAnomalyConfig {
    SyscallAnomalyConfig {
        enabled: true,
        alert_on_unknown_syscall: true,
        ..SyscallAnomalyConfig::default()
    }
}

fn alert(kind: SyscallAnomalyKind, syscall: Option<&str>, command: &str) -> SyscallAnomalyAlert {
    SyscallAnomalyAlert {
        timestamp: Utc::now(),
        kind,
        command: command.to_string(),
        syscall: syscall.map(str::to_string),
        denied_events_last_minute: 0,
        total_events_last_minute: 0,
        sample: "sample".to_string(),
    }
}

#[test]
fn new_normalizes_baseline_and_resolves_relative_log_path() {
    let tmp = tempdir().expect("tempdir");
    let config = SyscallAnomalyConfig {
        baseline_syscalls: vec![" Read ".into(), "".into(), "WRITE".into()],
        log_path: "logs/anomalies.jsonl".into(),
        ..SyscallAnomalyConfig::default()
    };

    let detector = SyscallAnomalyDetector::new(
        config,
        tmp.path(),
        AuditConfig { enabled: false, ..AuditConfig::default() },
    );

    assert!(detector.baseline.contains("read"));
    assert!(detector.baseline.contains("write"));
    assert!(!detector.baseline.contains(""));
    assert_eq!(detector.anomaly_log_path, tmp.path().join("logs/anomalies.jsonl"));
    assert!(detector.audit_logger.is_some());
}

#[test]
fn disabled_detector_and_empty_output_emit_no_alerts() {
    let (_tmp, disabled) =
        detector_with(SyscallAnomalyConfig { enabled: false, ..SyscallAnomalyConfig::default() });
    assert!(
        disabled
            .inspect_command_output("ls -la", "", "seccomp denied syscall=openat", Some(1))
            .is_empty()
    );

    let (_tmp, enabled) = detector_with(enabled_config());
    assert!(enabled.inspect_command_output("ls -la", "plain output", "", Some(0)).is_empty());
}

#[test]
fn inspect_command_output_emits_unknown_and_denied_alerts_and_writes_jsonl() {
    let (tmp, detector) = detector_with(SyscallAnomalyConfig {
        strict_mode: true,
        baseline_syscalls: vec!["read".into(), "write".into()],
        max_alerts_per_minute: 20,
        alert_cooldown_secs: 1,
        ..enabled_config()
    });

    let alerts = detector.inspect_command_output(
        "Echo HI",
        "",
        "seccomp denied syscall=openat\nseccomp denied syscall=__NR_clone3",
        Some(126),
    );

    assert!(alerts.iter().any(|alert| alert.kind == SyscallAnomalyKind::UnknownSyscall));
    assert!(alerts.iter().any(|alert| alert.kind == SyscallAnomalyKind::DeniedSyscall));
    assert!(alerts.iter().all(|alert| alert.command == "Echo HI"));

    let log = std::fs::read_to_string(tmp.path().join("syscall-anomalies.jsonl")).unwrap();
    assert!(log.contains("\"kind\":\"unknown_syscall\""));
    assert!(log.contains("\"kind\":\"denied_syscall\""));
    assert!(log.contains("\"command\":\"Echo HI\""));
}

#[test]
// 同一批输出中的重复信号只生成一次告警，避免噪声放大。
fn inspect_command_output_deduplicates_identical_alerts_from_same_batch() {
    let (_tmp, detector) = detector_with(SyscallAnomalyConfig {
        strict_mode: true,
        baseline_syscalls: vec!["read".into()],
        max_alerts_per_minute: 20,
        alert_cooldown_secs: 1,
        ..enabled_config()
    });

    let alerts = detector.inspect_command_output(
        "cat file",
        "",
        "seccomp denied syscall=openat\nseccomp denied syscall=openat",
        Some(1),
    );

    assert_eq!(
        alerts.iter().filter(|alert| alert.kind == SyscallAnomalyKind::UnknownSyscall).count(),
        1
    );
    assert_eq!(
        alerts.iter().filter(|alert| alert.kind == SyscallAnomalyKind::DeniedSyscall).count(),
        1
    );
}

#[test]
fn inspect_command_output_can_suppress_unknown_syscalls() {
    let (_tmp, detector) = detector_with(SyscallAnomalyConfig {
        alert_on_unknown_syscall: false,
        strict_mode: false,
        baseline_syscalls: vec!["read".into()],
        ..enabled_config()
    });

    let alerts =
        detector.inspect_command_output("curl", "", "audit: type=1326 syscall=connect", Some(0));

    assert!(alerts.is_empty());
}

#[test]
fn inspect_command_output_ignores_known_allowed_syscall() {
    let (_tmp, detector) = detector_with(SyscallAnomalyConfig {
        strict_mode: false,
        baseline_syscalls: vec!["openat".into()],
        ..enabled_config()
    });

    let alerts = detector.inspect_command_output("reader", "audit: syscall=OPENAT", "", Some(0));

    assert!(alerts.is_empty());
    assert_eq!(detector.state.lock().events.len(), 1);
}

#[test]
fn inspect_command_output_reports_denied_signal_without_syscall_name() {
    let (_tmp, detector) = detector_with(SyscallAnomalyConfig {
        strict_mode: true,
        alert_on_unknown_syscall: false,
        max_alerts_per_minute: 10,
        ..enabled_config()
    });

    let alerts = detector.inspect_command_output("worker", "", "SIGSYS killed process", None);

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].kind, SyscallAnomalyKind::DeniedSyscall);
    assert_eq!(alerts[0].syscall, None);
    assert_eq!(alerts[0].denied_events_last_minute, 1);
    assert_eq!(alerts[0].total_events_last_minute, 1);
    assert_eq!(alerts[0].sample, "SIGSYS killed process");
}

#[test]
fn inspect_command_output_reports_denied_and_total_rate_spikes() {
    let (_tmp, detector) = detector_with(SyscallAnomalyConfig {
        strict_mode: false,
        baseline_syscalls: vec!["openat".into()],
        max_denied_events_per_minute: 1,
        max_total_events_per_minute: 1,
        max_alerts_per_minute: 20,
        alert_cooldown_secs: 1,
        ..enabled_config()
    });

    let alerts = detector.inspect_command_output(
        "worker",
        "",
        "seccomp denied syscall=openat\nseccomp denied syscall=openat",
        Some(1),
    );

    assert!(alerts.iter().any(|alert| alert.kind == SyscallAnomalyKind::DeniedRateExceeded));
    assert!(alerts.iter().any(|alert| alert.kind == SyscallAnomalyKind::EventRateExceeded));
    assert!(alerts.iter().all(|alert| alert.denied_events_last_minute == 2));
    assert!(alerts.iter().all(|alert| alert.total_events_last_minute == 2));
}

#[test]
fn inspect_command_output_reports_total_rate_for_allowed_events() {
    let (_tmp, detector) = detector_with(SyscallAnomalyConfig {
        strict_mode: false,
        baseline_syscalls: vec!["read".into(), "write".into()],
        max_denied_events_per_minute: u32::MAX,
        max_total_events_per_minute: 1,
        max_alerts_per_minute: 10,
        ..enabled_config()
    });

    let alerts = detector.inspect_command_output(
        "worker",
        "audit: syscall=write",
        "audit: syscall=read",
        Some(0),
    );

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].kind, SyscallAnomalyKind::EventRateExceeded);
    assert_eq!(alerts[0].denied_events_last_minute, 0);
    assert_eq!(alerts[0].total_events_last_minute, 2);
    assert_eq!(alerts[0].sample, "audit: syscall=read");
}

#[test]
fn inspect_command_output_uses_empty_sample_for_prior_denied_rate_event() {
    let (_tmp, detector) = detector_with(SyscallAnomalyConfig {
        strict_mode: false,
        baseline_syscalls: vec!["read".into()],
        max_denied_events_per_minute: 0,
        max_total_events_per_minute: u32::MAX,
        max_alerts_per_minute: 10,
        ..enabled_config()
    });
    detector.state.lock().events.push_back(ObservedEvent { at: Instant::now(), denied: true });

    let alerts = detector.inspect_command_output("worker", "", "audit: syscall=read", Some(0));

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].kind, SyscallAnomalyKind::DeniedRateExceeded);
    assert_eq!(alerts[0].denied_events_last_minute, 1);
    assert_eq!(alerts[0].total_events_last_minute, 2);
    assert!(alerts[0].sample.is_empty());
}

#[test]
fn parse_syscall_signal_handles_supported_formats_and_ignores_noise() {
    let numeric = parse_syscall_signal("audit: type=1326 audit(1.234:66): syscall=59").unwrap();
    assert_eq!(numeric.syscall.as_deref(), Some("execve"));
    assert!(!numeric.denied);

    let hex = parse_syscall_signal("audit: type=1326 syscall=0x3b seccomp denied").unwrap();
    assert_eq!(hex.syscall.as_deref(), Some("execve"));
    assert!(hex.denied);

    let symbolic = parse_syscall_signal("seccomp blocked syscall=SYS_openat").unwrap();
    assert_eq!(symbolic.syscall.as_deref(), Some("openat"));
    assert!(symbolic.denied);

    let named = parse_syscall_signal("audit: syscall_name=clone3").unwrap();
    assert_eq!(named.syscall.as_deref(), Some("clone3"));
    assert!(!named.denied);

    let unknown_number = parse_syscall_signal("audit: syscall=999999").unwrap();
    assert_eq!(unknown_number.syscall.as_deref(), Some("syscall#999999"));

    let seccomp_without_name = parse_syscall_signal("seccomp profile loaded").unwrap();
    assert_eq!(seccomp_without_name.syscall, None);
    assert!(!seccomp_without_name.denied);

    let denied_without_name = parse_syscall_signal("Bad system call").unwrap();
    assert_eq!(denied_without_name.syscall, None);
    assert!(denied_without_name.denied);

    assert!(parse_syscall_signal("").is_none());
    assert!(parse_syscall_signal("audit(1.2:3): cwd=\"/tmp\"").is_none());
    assert!(parse_syscall_signal("ordinary stderr").is_none());
}

#[test]
fn parse_syscall_signal_recognizes_denial_markers_without_names() {
    for line in [
        "seccomp forbidden by profile",
        "SIGSYS received",
        "seccomp operation not permitted",
        "seccomp EPERM",
        "seccomp killed child",
    ] {
        let signal = parse_syscall_signal(line).expect("denial marker should parse");
        assert!(signal.denied, "line should be denied: {line}");
        assert_eq!(signal.syscall, None);
        assert_eq!(signal.raw_line, line);
    }
}

#[test]
fn parse_syscall_signal_supports_number_keyword_and_system_call_name() {
    let numbered = parse_syscall_signal("audit: syscall number=257").unwrap();
    assert_eq!(numbered.syscall.as_deref(), Some("openat"));
    assert!(!numbered.denied);

    let named = parse_syscall_signal("seccomp denied system call: SYS_clone3").unwrap();
    assert_eq!(named.syscall.as_deref(), Some("clone3"));
    assert!(named.denied);
}

#[test]
// 系统调用跟踪通常写入标准错误，解析顺序需优先保留该来源。
fn extract_signals_reads_stderr_before_stdout() {
    let signals = extract_signals("seccomp denied syscall=openat", "audit: type=1326 syscall=read");

    assert_eq!(signals.len(), 2);
    assert_eq!(signals[0].syscall.as_deref(), Some("openat"));
    assert!(signals[0].denied);
    assert_eq!(signals[1].syscall.as_deref(), Some("read"));
    assert!(!signals[1].denied);
}

#[test]
fn normalization_parsing_and_mapping_helpers_cover_boundaries() {
    let baseline = normalize_baseline(&[
        " Read ".to_string(),
        "".to_string(),
        "WRITE".to_string(),
        "  ".to_string(),
    ]);
    assert_eq!(baseline.len(), 2);
    assert!(baseline.contains("read"));
    assert!(baseline.contains("write"));

    assert_eq!(normalize_syscall_name(" OpenAt "), "openat");
    assert_eq!(normalize_symbolic_syscall("__NR_openat").as_deref(), Some("openat"));
    assert_eq!(normalize_symbolic_syscall("__nr_clone3").as_deref(), Some("clone3"));
    assert_eq!(normalize_symbolic_syscall("SYS_execve").as_deref(), Some("execve"));
    assert_eq!(normalize_symbolic_syscall("openat"), None);

    assert_eq!(parse_syscall_number("59"), Some(59));
    assert_eq!(parse_syscall_number("0x3b"), Some(59));
    assert_eq!(parse_syscall_number("0xnot_hex"), None);
    assert_eq!(parse_syscall_number("execve"), None);

    assert_eq!(map_linux_x86_64_syscall(0), Some("read"));
    assert_eq!(map_linux_x86_64_syscall(435), Some("clone3"));
    assert_eq!(map_linux_x86_64_syscall(-1), None);
}

#[test]
fn linux_x86_64_syscall_mapping_covers_supported_table() {
    let supported = [
        (0, "read"),
        (1, "write"),
        (2, "open"),
        (3, "close"),
        (9, "mmap"),
        (10, "mprotect"),
        (11, "munmap"),
        (12, "brk"),
        (16, "ioctl"),
        (32, "dup"),
        (33, "dup2"),
        (39, "getpid"),
        (41, "socket"),
        (42, "connect"),
        (43, "accept"),
        (44, "sendto"),
        (45, "recvfrom"),
        (47, "recvmsg"),
        (50, "listen"),
        (51, "getsockname"),
        (52, "getpeername"),
        (54, "setsockopt"),
        (55, "getsockopt"),
        (56, "clone"),
        (57, "fork"),
        (59, "execve"),
        (60, "exit"),
        (61, "wait4"),
        (72, "fcntl"),
        (202, "futex"),
        (218, "set_tid_address"),
        (231, "exit_group"),
        (257, "openat"),
        (262, "newfstatat"),
        (273, "set_robust_list"),
        (291, "epoll_create1"),
        (318, "getrandom"),
        (332, "statx"),
        (435, "clone3"),
    ];

    for (number, name) in supported {
        assert_eq!(map_linux_x86_64_syscall(number), Some(name), "syscall number {number}");
    }
    assert_eq!(map_linux_x86_64_syscall(436), None);
}

#[test]
fn path_sample_and_command_identity_helpers_handle_edges() {
    let base = Path::new("/tmp/vibewindow");
    assert_eq!(resolve_log_path(base, " logs/anomalies.log "), base.join("logs/anomalies.log"));

    let absolute = std::env::temp_dir().join("absolute-syscall.log");
    assert_eq!(resolve_log_path(base, absolute.to_string_lossy().as_ref()), absolute);

    let raw = "界".repeat(100);
    let truncated = truncate_sample(&raw);
    assert!(truncated.ends_with("..."));
    assert!(truncated.len() <= MAX_ALERT_SAMPLE_CHARS + 3);
    assert!(truncated.is_char_boundary(truncated.len()));
    assert_eq!(truncate_sample("short sample"), "short sample");

    assert_eq!(command_identity("  "), "-");
    assert_eq!(command_identity("Echo hello"), "echo");
    assert_eq!(command_identity(&format!("{} rest", "A".repeat(80))).len(), 64);
    assert_eq!(command_identity(&format!("{} rest", "界".repeat(30))), "界".repeat(21));
}

#[test]
fn append_log_line_creates_parent_and_appends_json_lines() {
    let tmp = tempdir().expect("tempdir");
    let detector = SyscallAnomalyDetector::new(
        SyscallAnomalyConfig {
            log_path: "nested/security/anomalies.jsonl".into(),
            ..SyscallAnomalyConfig::default()
        },
        tmp.path(),
        AuditConfig { enabled: false, ..AuditConfig::default() },
    );
    let first = alert(SyscallAnomalyKind::UnknownSyscall, Some("clone3"), "worker one");
    let second = alert(SyscallAnomalyKind::DeniedSyscall, None, "worker two");

    detector.append_log_line(&first).expect("first alert should append");
    detector.append_log_line(&second).expect("second alert should append");

    let contents = std::fs::read_to_string(&detector.anomaly_log_path).expect("alert log");
    let parsed = contents
        .lines()
        .map(|line| serde_json::from_str::<SyscallAnomalyAlert>(line).expect("valid alert JSON"))
        .collect::<Vec<_>>();
    assert_eq!(parsed, vec![first, second]);
}

#[test]
fn inspect_command_output_returns_alert_when_log_path_is_directory() {
    let tmp = tempdir().expect("tempdir");
    let detector = SyscallAnomalyDetector::new(
        SyscallAnomalyConfig {
            baseline_syscalls: vec!["read".into()],
            log_path: tmp.path().to_string_lossy().to_string(),
            ..enabled_config()
        },
        tmp.path(),
        AuditConfig { enabled: false, ..AuditConfig::default() },
    );

    let alerts = detector.inspect_command_output("worker", "", "audit: syscall=clone3", Some(0));

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].kind, SyscallAnomalyKind::UnknownSyscall);
    assert_eq!(alerts[0].syscall.as_deref(), Some("clone3"));
}

#[test]
fn event_and_alert_window_helpers_prune_old_entries() {
    let now = Instant::now();
    let mut events = VecDeque::from([
        ObservedEvent { at: now - RATE_WINDOW - Duration::from_secs(1), denied: true },
        ObservedEvent { at: now - RATE_WINDOW, denied: false },
        ObservedEvent { at: now, denied: true },
    ]);

    prune_old_events(&mut events, now);

    assert_eq!(events.len(), 2);
    assert_eq!(count_denied(&events), 1);

    let mut timestamps =
        VecDeque::from([now - RATE_WINDOW - Duration::from_millis(1), now - RATE_WINDOW, now]);
    prune_old_alert_timestamps(&mut timestamps, now);
    assert_eq!(timestamps, VecDeque::from([now - RATE_WINDOW, now]));
}

#[test]
// 告警预算和冷却窗口共同限流，过期记录随后释放容量。
fn should_emit_alert_enforces_budget_cooldown_and_expiry() {
    let now = Instant::now();
    let mut state = DetectorState::default();
    let budget_one =
        SyscallAnomalyConfig { max_alerts_per_minute: 1, ..SyscallAnomalyConfig::default() };

    assert!(should_emit_alert(
        &mut state,
        &budget_one,
        &alert(SyscallAnomalyKind::UnknownSyscall, Some("openat"), "echo hi"),
        now
    ));
    assert!(!should_emit_alert(
        &mut state,
        &budget_one,
        &alert(SyscallAnomalyKind::DeniedSyscall, Some("clone3"), "echo hi"),
        now
    ));

    let mut state = DetectorState::default();
    let cooldown = SyscallAnomalyConfig {
        max_alerts_per_minute: 10,
        alert_cooldown_secs: 60,
        ..SyscallAnomalyConfig::default()
    };
    let repeated = alert(SyscallAnomalyKind::UnknownSyscall, Some("openat"), "Echo hi");

    assert!(should_emit_alert(&mut state, &cooldown, &repeated, now));
    assert!(!should_emit_alert(&mut state, &cooldown, &repeated, now + Duration::from_secs(1)));
    assert!(should_emit_alert(
        &mut state,
        &cooldown,
        &alert(SyscallAnomalyKind::UnknownSyscall, Some("openat"), "other"),
        now + Duration::from_secs(1)
    ));
    assert!(should_emit_alert(
        &mut state,
        &cooldown,
        &repeated,
        now + RATE_WINDOW + Duration::from_secs(1)
    ));

    let mut state = DetectorState::default();
    assert!(should_emit_alert(&mut state, &cooldown, &repeated, now));
    assert!(should_emit_alert(
        &mut state,
        &cooldown,
        &repeated,
        now + Duration::from_secs(cooldown.alert_cooldown_secs)
    ));

    let mut state = DetectorState::default();
    let zero_budget =
        SyscallAnomalyConfig { max_alerts_per_minute: 0, ..SyscallAnomalyConfig::default() };
    assert!(!should_emit_alert(&mut state, &zero_budget, &repeated, now));
    assert!(state.alert_timestamps.is_empty());
    assert!(state.last_alert_by_key.is_empty());
}
