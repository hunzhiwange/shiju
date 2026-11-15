use anyhow::Result;
use tempfile::TempDir;
use vw_config_types::security::AuditConfig;
use security_audit::{
    AuditEvent, AuditEventType, AuditLogger, CommandExecutionLog,
};

// 测试创建的审计事件是否具有唯一的事件ID
#[test]
fn audit_event_new_creates_unique_id() {
    let event1 = AuditEvent::new(AuditEventType::CommandExecution);
    let event2 = AuditEvent::new(AuditEventType::CommandExecution);
    assert_ne!(event1.event_id, event2.event_id);
}

// 测试为审计事件设置执行者（actor）信息
#[test]
fn audit_event_with_actor() {
    let event = AuditEvent::new(AuditEventType::CommandExecution).with_actor(
        "telegram".to_string(),
        Some("123".to_string()),
        Some("@alice".to_string()),
    );

    assert!(event.actor.is_some());
    let actor = event.actor.as_ref().unwrap();
    assert_eq!(actor.channel, "telegram");
    assert_eq!(actor.user_id, Some("123".to_string()));
    assert_eq!(actor.username, Some("@alice".to_string()));
}

// 测试为审计事件设置动作（action）信息，包括命令和风险等级
#[test]
fn audit_event_with_action() {
    let event = AuditEvent::new(AuditEventType::CommandExecution).with_action(
        "ls -la".to_string(),
        "low".to_string(),
        false,
        true,
    );

    assert!(event.action.is_some());
    let action = event.action.as_ref().unwrap();
    assert_eq!(action.command, Some("ls -la".to_string()));
    assert_eq!(action.risk_level, Some("low".to_string()));
}

// 测试审计事件能够正确序列化为JSON格式并反序列化还原
#[test]
fn audit_event_serializes_to_json() {
    let event = AuditEvent::new(AuditEventType::CommandExecution)
        .with_actor("telegram".to_string(), None, None)
        .with_action("ls".to_string(), "low".to_string(), false, true)
        .with_result(true, Some(0), 15, None);

    let json = serde_json::to_string(&event);
    assert!(json.is_ok());
    let json = json.expect("serialize");
    let parsed: AuditEvent = serde_json::from_str(json.as_str()).expect("parse");
    assert!(parsed.actor.is_some());
    assert!(parsed.action.is_some());
    assert!(parsed.result.is_some());
}

// 测试当审计日志功能禁用时，不会创建审计日志文件
#[test]
fn audit_logger_disabled_does_not_create_file() -> Result<()> {
    let tmp = TempDir::new()?;
    let config = AuditConfig { enabled: false, ..Default::default() };
    let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;
    let event = AuditEvent::new(AuditEventType::CommandExecution);

    logger.log(&event)?;

    assert!(!tmp.path().join("audit.log").exists());
    Ok(())
}

// 测试当审计日志功能启用时，初始化时会自动创建日志文件
#[test]
fn audit_logger_enabled_creates_file_on_init() -> Result<()> {
    let tmp = TempDir::new()?;
    let config = AuditConfig { enabled: true, ..Default::default() };

    let _logger = AuditLogger::new(config, tmp.path().to_path_buf())?;
    assert!(
        tmp.path().join("audit.log").exists(),
        "audit log file should be created when audit logging is enabled"
    );
    Ok(())
}

// 测试当配置的日志路径包含嵌套目录时，会自动创建父目录
#[test]
fn audit_logger_enabled_creates_parent_directories() -> Result<()> {
    let tmp = TempDir::new()?;
    let config = AuditConfig {
        enabled: true,
        log_path: "logs/security/audit.log".to_string(),
        ..Default::default()
    };

    let _logger = AuditLogger::new(config, tmp.path().to_path_buf())?;
    assert!(
        tmp.path().join("logs/security/audit.log").exists(),
        "audit logger should create nested directories for configured log path"
    );
    Ok(())
}

// 测试当审计日志启用时，能够正确将审计事件写入日志文件
#[tokio::test]
async fn audit_logger_writes_event_when_enabled() -> Result<()> {
    let tmp = TempDir::new()?;
    let config = AuditConfig { enabled: true, max_size_mb: 10, ..Default::default() };
    let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;
    let event = AuditEvent::new(AuditEventType::CommandExecution)
        .with_actor("cli".to_string(), None, None)
        .with_action("ls".to_string(), "low".to_string(), false, true);

    logger.log(&event)?;

    let log_path = tmp.path().join("audit.log");
    assert!(log_path.exists(), "audit log file must be created");

    let content = tokio::fs::read_to_string(&log_path).await?;
    assert!(!content.is_empty(), "audit log must not be empty");

    let parsed: AuditEvent = serde_json::from_str(content.trim())?;
    assert!(parsed.action.is_some());
    Ok(())
}

// 测试使用log_command_event方法记录命令执行事件，验证写入的结构化条目格式正确
#[tokio::test]
async fn audit_log_command_event_writes_structured_entry() -> Result<()> {
    let tmp = TempDir::new()?;
    let config = AuditConfig { enabled: true, max_size_mb: 10, ..Default::default() };
    let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;

    logger.log_command_event(CommandExecutionLog {
        channel: "telegram",
        command: "echo test",
        risk_level: "low",
        approved: false,
        allowed: true,
        success: true,
        duration_ms: 42,
    })?;

    let log_path = tmp.path().join("audit.log");
    let content = tokio::fs::read_to_string(&log_path).await?;
    let parsed: AuditEvent = serde_json::from_str(content.trim())?;

    let action = parsed.action.unwrap();
    assert_eq!(action.command, Some("echo test".to_string()));
    assert_eq!(action.risk_level, Some("low".to_string()));
    assert!(action.allowed);

    let result = parsed.result.unwrap();
    assert!(result.success);
    assert_eq!(result.duration_ms, Some(42));
    Ok(())
}

// 测试当日志文件超过最大限制时，会自动进行轮转并创建带编号的备份文件
#[test]
fn audit_rotation_creates_numbered_backup() -> Result<()> {
    let tmp = TempDir::new()?;
    let config = AuditConfig {
        enabled: true,
        max_size_mb: 0,
        ..Default::default()
    };
    let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;

    let log_path = tmp.path().join("audit.log");
    std::fs::write(&log_path, "initial content\n")?;

    let event = AuditEvent::new(AuditEventType::CommandExecution);
    logger.log(&event)?;

    let rotated = format!("{}.1.log", log_path.display());
    assert!(std::path::Path::new(&rotated).exists(), "rotation must create .1.log backup");
    Ok(())
}
