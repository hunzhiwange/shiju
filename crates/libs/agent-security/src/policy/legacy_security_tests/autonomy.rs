use super::*;

// 测试默认自治级别应为 Supervised
#[test]
fn autonomy_default_is_supervised() {
    assert_eq!(AutonomyLevel::default(), AutonomyLevel::Supervised);
}

// 测试自治级别的 JSON 序列化/反序列化往返正确性
#[test]
fn autonomy_serde_roundtrip() {
    let json = serde_json::to_string(&AutonomyLevel::Full).unwrap();
    assert_eq!(json, "\"full\"");
    let parsed: AutonomyLevel = serde_json::from_str("\"readonly\"").unwrap();
    assert_eq!(parsed, AutonomyLevel::ReadOnly);
    let parsed2: AutonomyLevel = serde_json::from_str("\"supervised\"").unwrap();
    assert_eq!(parsed2, AutonomyLevel::Supervised);
}

// 测试只读模式下 can_act 返回 false
#[test]
fn can_act_readonly_false() {
    assert!(!readonly_policy().can_act());
}

// 测试监督模式下 can_act 返回 true
#[test]
fn can_act_supervised_true() {
    assert!(default_policy().can_act());
}

// 测试完全自治模式下 can_act 返回 true
#[test]
fn can_act_full_true() {
    assert!(full_policy().can_act());
}

// 测试只读模式下读取操作被允许
#[test]
fn enforce_tool_operation_read_allowed_in_readonly_mode() {
    let p = readonly_policy();
    assert!(p.enforce_tool_operation(ToolOperation::Read, "memory_recall").is_ok());
}

// 测试只读模式下执行操作被阻止
#[test]
fn enforce_tool_operation_act_blocked_in_readonly_mode() {
    let p = readonly_policy();
    let err = p.enforce_tool_operation(ToolOperation::Act, "memory_store").unwrap_err();
    assert!(err.contains("read-only mode"));
}

// 测试执行操作受速率限制约束
#[test]
fn enforce_tool_operation_act_uses_rate_budget() {
    let p = SecurityPolicy { max_actions_per_hour: 0, ..default_policy() };
    let err = p.enforce_tool_operation(ToolOperation::Act, "memory_store").unwrap_err();
    assert!(err.contains("Rate limit exceeded"));
}

// 测试只读模式即使在 allowlist 中也只允许只读命令
#[test]
fn readonly_only_allows_safe_readonly_commands() {
    let p = SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        allowed_commands: vec!["ls".into(), "cat".into(), "touch".into()],
        ..SecurityPolicy::default()
    };
    assert!(p.is_command_allowed("ls"));
    assert!(p.is_command_allowed("cat"));
    assert!(!p.is_command_allowed("touch file.txt"));
    assert!(!p.can_act());
}

// 测试监督模式允许白名单中的命令
#[test]
fn supervised_allows_listed_commands() {
    let p = SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        allowed_commands: vec!["git".into()],
        ..SecurityPolicy::default()
    };
    assert!(p.is_command_allowed("git status"));
    assert!(!p.is_command_allowed("docker ps"));
}
