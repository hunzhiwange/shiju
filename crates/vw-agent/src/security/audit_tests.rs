//! 验证安全审计事件构造、JSONL 写入、轮转与文件错误传播。

use super::*;
use crate::app::agent::config::AuditConfig;

#[test]
fn audit_event_builder_preserves_actor_action_and_result() {
    let event = AuditEvent::new(AuditEventType::CommandExecution)
        .with_actor("agent".into(), Some("user-1".into()), Some("Ada".into()))
        .with_action("ls".into(), "low".into(), true, true)
        .with_result(true, Some(0), 7, None);

    let actor = event.actor.as_ref().unwrap();
    assert_eq!(actor.channel, "agent");
    assert_eq!(actor.user_id.as_deref(), Some("user-1"));
    assert_eq!(actor.username.as_deref(), Some("Ada"));

    let action = event.action.as_ref().unwrap();
    assert_eq!(action.command.as_deref(), Some("ls"));
    assert_eq!(action.risk_level.as_deref(), Some("low"));
    assert!(action.approved);
    assert!(action.allowed);

    let result = event.result.as_ref().unwrap();
    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.duration_ms, Some(7));
    assert_eq!(result.error, None);
}

#[test]
fn audit_event_defaults_include_safe_security_context() {
    let event = AuditEvent::new(AuditEventType::SecurityEvent);

    assert!(!event.event_id.is_empty());
    assert!(event.actor.is_none());
    assert!(event.action.is_none());
    assert!(event.result.is_none());
    assert!(!event.security.policy_violation);
    assert_eq!(event.security.rate_limit_remaining, None);
    assert_eq!(event.security.sandbox_backend, None);
}

#[test]
fn audit_event_builder_sets_security_backend_and_failure_result() {
    let event = AuditEvent::new(AuditEventType::PolicyViolation)
        .with_security(Some("docker".into()))
        .with_result(false, Some(126), 12, Some("blocked".into()));

    assert_eq!(event.security.sandbox_backend.as_deref(), Some("docker"));
    let result = event.result.unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, Some(126));
    assert_eq!(result.duration_ms, Some(12));
    assert_eq!(result.error.as_deref(), Some("blocked"));
}

#[test]
fn audit_event_types_serialize_to_stable_snake_case_values() {
    let cases = [
        (AuditEventType::CommandExecution, "command_execution"),
        (AuditEventType::FileAccess, "file_access"),
        (AuditEventType::ConfigChange, "config_change"),
        (AuditEventType::AuthSuccess, "auth_success"),
        (AuditEventType::AuthFailure, "auth_failure"),
        (AuditEventType::PolicyViolation, "policy_violation"),
        (AuditEventType::SecurityEvent, "security_event"),
    ];

    for (event_type, expected) in cases {
        let encoded = serde_json::to_string(&event_type).unwrap();
        assert_eq!(encoded, format!("\"{}\"", expected));

        let decoded: AuditEventType = serde_json::from_str(&encoded).unwrap();
        assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
    }
}

#[test]
fn disabled_logger_does_not_create_or_write_log_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: false, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();

    logger.log(&AuditEvent::new(AuditEventType::AuthSuccess)).unwrap();

    assert!(!dir.path().join("audit.log").exists());
}

#[test]
fn enabled_logger_initializes_nested_log_path_and_writes_jsonl() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig {
        enabled: true,
        log_path: "logs/audit.jsonl".into(),
        max_size_mb: 10,
        ..AuditConfig::default()
    };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();
    let event = AuditEvent::new(AuditEventType::ConfigChange).with_actor(
        "cli".into(),
        None,
        Some("operator".into()),
    );

    logger.log(&event).unwrap();

    let log_path = dir.path().join("logs/audit.jsonl");
    let raw = std::fs::read_to_string(log_path).unwrap();
    let parsed: AuditEvent = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(parsed.actor.unwrap().username.as_deref(), Some("operator"));
}

#[test]
fn enabled_logger_appends_events_as_separate_json_lines_without_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: true, max_size_mb: 10, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();

    logger.log(&AuditEvent::new(AuditEventType::AuthSuccess)).unwrap();
    logger
        .log(&AuditEvent::new(AuditEventType::AuthFailure).with_result(
            false,
            None,
            0,
            Some("credentials rejected".into()),
        ))
        .unwrap();

    let log_path = dir.path().join("audit.log");
    let raw = std::fs::read_to_string(&log_path).unwrap();
    let events: Vec<AuditEvent> =
        raw.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0].event_type, AuditEventType::AuthSuccess));
    assert!(matches!(&events[1].event_type, AuditEventType::AuthFailure));
    assert_eq!(events[1].result.as_ref().unwrap().error.as_deref(), Some("credentials rejected"));
    assert!(!std::path::PathBuf::from(format!("{}.1.log", log_path.display())).exists());
}

#[test]
fn logger_recreates_log_removed_after_initialization() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: true, max_size_mb: 10, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();
    let log_path = dir.path().join("audit.log");
    std::fs::remove_file(&log_path).unwrap();

    logger.log(&AuditEvent::new(AuditEventType::FileAccess)).unwrap();

    let raw = std::fs::read_to_string(log_path).unwrap();
    let parsed: AuditEvent = serde_json::from_str(raw.trim()).unwrap();
    assert!(matches!(parsed.event_type, AuditEventType::FileAccess));
}

#[test]
fn rotate_if_needed_ignores_missing_log_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: false, max_size_mb: 0, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();

    assert!(!dir.path().join("audit.log").exists());
    logger.rotate_if_needed().unwrap();
    assert!(!dir.path().join("audit.log.1.log").exists());
}

#[test]
fn enabled_logger_reports_error_when_parent_component_is_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("occupied"), "not a directory").unwrap();
    let config = AuditConfig {
        enabled: true,
        log_path: "occupied/audit.log".into(),
        ..AuditConfig::default()
    };

    assert!(AuditLogger::new(config, dir.path().to_path_buf()).is_err());
}

#[test]
fn logger_propagates_write_error_when_log_path_becomes_directory() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: true, max_size_mb: 10, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();
    let log_path = dir.path().join("audit.log");
    std::fs::remove_file(&log_path).unwrap();
    std::fs::create_dir(&log_path).unwrap();

    let result = logger.log(&AuditEvent::new(AuditEventType::SecurityEvent));

    assert!(result.is_err());
}

#[test]
fn log_command_event_writes_complete_command_context() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: true, max_size_mb: 10, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();

    logger
        .log_command_event(CommandExecutionLog {
            channel: "api",
            command: "inspect --dry-run",
            risk_level: "high",
            approved: false,
            allowed: false,
            success: false,
            duration_ms: 0,
        })
        .unwrap();

    let raw = std::fs::read_to_string(dir.path().join("audit.log")).unwrap();
    let parsed: AuditEvent = serde_json::from_str(raw.trim()).unwrap();
    assert!(matches!(parsed.event_type, AuditEventType::CommandExecution));

    let actor = parsed.actor.unwrap();
    assert_eq!(actor.channel, "api");
    assert_eq!(actor.user_id, None);
    assert_eq!(actor.username, None);

    let action = parsed.action.unwrap();
    assert_eq!(action.command.as_deref(), Some("inspect --dry-run"));
    assert_eq!(action.risk_level.as_deref(), Some("high"));
    assert!(!action.approved);
    assert!(!action.allowed);

    let result = parsed.result.unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, None);
    assert_eq!(result.duration_ms, Some(0));
    assert_eq!(result.error, None);
}

#[test]
fn log_command_compat_method_writes_command_execution_event() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: true, max_size_mb: 10, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();

    logger.log_command("discord", "echo ok", "low", true, true, true, 33).unwrap();

    let raw = std::fs::read_to_string(dir.path().join("audit.log")).unwrap();
    let parsed: AuditEvent = serde_json::from_str(raw.trim()).unwrap();
    assert!(matches!(parsed.event_type, AuditEventType::CommandExecution));
    assert_eq!(parsed.actor.unwrap().channel, "discord");
    assert_eq!(parsed.action.unwrap().command.as_deref(), Some("echo ok"));
    assert_eq!(parsed.result.unwrap().duration_ms, Some(33));
}

#[test]
fn logger_rotates_when_existing_file_reaches_limit() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: true, max_size_mb: 0, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();
    let log_path = dir.path().join("audit.log");
    std::fs::write(&log_path, "old\n").unwrap();

    logger.log(&AuditEvent::new(AuditEventType::SecurityEvent)).unwrap();

    assert!(std::path::PathBuf::from(format!("{}.1.log", log_path.display())).exists());
    let new_log = std::fs::read_to_string(log_path).unwrap();
    assert!(new_log.contains("security_event"));
}

#[test]
fn logger_rotation_shifts_existing_numbered_history() {
    let dir = tempfile::tempdir().unwrap();
    let config = AuditConfig { enabled: true, max_size_mb: 0, ..AuditConfig::default() };
    let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();
    let log_path = dir.path().join("audit.log");
    let first_backup = std::path::PathBuf::from(format!("{}.1.log", log_path.display()));
    let ninth_backup = std::path::PathBuf::from(format!("{}.9.log", log_path.display()));
    std::fs::write(&log_path, "current\n").unwrap();
    std::fs::write(&first_backup, "first backup\n").unwrap();
    std::fs::write(&ninth_backup, "ninth backup\n").unwrap();

    logger.log(&AuditEvent::new(AuditEventType::SecurityEvent)).unwrap();

    assert_eq!(std::fs::read_to_string(first_backup).unwrap(), "current\n");
    assert_eq!(
        std::fs::read_to_string(format!("{}.2.log", log_path.display())).unwrap(),
        "first backup\n"
    );
    assert_eq!(
        std::fs::read_to_string(format!("{}.10.log", log_path.display())).unwrap(),
        "ninth backup\n"
    );
    let new_log = std::fs::read_to_string(log_path).unwrap();
    let parsed: AuditEvent = serde_json::from_str(new_log.trim()).unwrap();
    assert!(matches!(parsed.event_type, AuditEventType::SecurityEvent));
}
