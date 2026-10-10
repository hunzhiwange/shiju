//! 测试审批策略的自治级别、白名单、审计和非命令行授权。
//! 优先级组合明确验证始终询问覆盖自动批准，临时授权不会越过会话边界。

use std::collections::HashMap;

use crate::app::agent::config::{AutonomyConfig, NonCliNaturalLanguageApprovalMode};
use crate::app::agent::security::AutonomyLevel;

use super::{ApprovalManager, ApprovalResponse};

#[test]
fn policy_applies_auto_approve_always_ask_and_session_allowlist() {
    let config = AutonomyConfig {
        level: AutonomyLevel::Supervised,
        auto_approve: vec!["file_read".to_string()],
        always_ask: vec!["shell".to_string()],
        ..AutonomyConfig::default()
    };
    let manager = ApprovalManager::from_config(&config);

    assert!(!manager.needs_approval("file_read"));
    assert!(manager.needs_approval("file_write"));

    manager.record_decision(
        "file_write",
        &serde_json::json!({"path": "a"}),
        ApprovalResponse::Always,
        "cli",
    );
    assert!(!manager.needs_approval("file_write"));
    assert!(manager.needs_approval("shell"));
}

#[test]
fn policy_normalizes_approvers_and_channel_modes() {
    let mut modes = HashMap::new();
    modes.insert(" Discord ".to_string(), NonCliNaturalLanguageApprovalMode::Disabled);
    let config = AutonomyConfig {
        non_cli_approval_approvers: vec![" alice ".to_string(), "".to_string()],
        non_cli_natural_language_approval_mode_by_channel: modes,
        ..AutonomyConfig::default()
    };
    let manager = ApprovalManager::from_config(&config);

    assert!(manager.non_cli_approval_approvers().contains("alice"));
    assert_eq!(
        manager.non_cli_natural_language_approval_mode_for_channel("DISCORD"),
        NonCliNaturalLanguageApprovalMode::Disabled
    );
}

#[test]
fn policy_autonomy_levels_bypass_interactive_approval() {
    let full = ApprovalManager::from_config(&AutonomyConfig {
        level: AutonomyLevel::Full,
        always_ask: vec!["shell".to_string()],
        ..AutonomyConfig::default()
    });
    let readonly = ApprovalManager::from_config(&AutonomyConfig {
        level: AutonomyLevel::ReadOnly,
        always_ask: vec!["shell".to_string()],
        ..AutonomyConfig::default()
    });

    assert!(!full.needs_approval("shell"));
    assert!(!readonly.needs_approval("shell"));
}

#[test]
fn policy_always_ask_takes_precedence_over_auto_and_session_allowlist() {
    let manager = ApprovalManager::from_config(&AutonomyConfig {
        auto_approve: vec!["shell".to_string()],
        always_ask: vec!["shell".to_string()],
        ..AutonomyConfig::default()
    });

    manager.record_decision(
        "shell",
        &serde_json::json!({"command": "pwd"}),
        ApprovalResponse::Always,
        "cli",
    );

    assert!(manager.needs_approval("shell"));
    assert!(manager.session_allowlist().contains("shell"));
}

#[test]
fn policy_records_audit_entries_and_yes_does_not_grant_session() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    manager.record_decision(
        "file_write",
        &serde_json::json!({"path": "README.md"}),
        ApprovalResponse::Yes,
        "telegram",
    );

    let audit = manager.audit_log();
    assert_eq!(audit.len(), 1);
    assert!(chrono::DateTime::parse_from_rfc3339(&audit[0].timestamp).is_ok());
    assert_eq!(audit[0].tool_name, "file_write");
    assert_eq!(audit[0].decision, ApprovalResponse::Yes);
    assert_eq!(audit[0].channel, "telegram");
    assert!(audit[0].arguments_summary.contains("path: README.md"));
    assert!(!manager.session_allowlist().contains("file_write"));
}

#[test]
fn policy_manages_non_cli_session_and_allow_all_once_tokens() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    assert!(!manager.is_non_cli_session_granted("shell"));
    manager.grant_non_cli_session("shell");
    assert!(manager.is_non_cli_session_granted("shell"));
    assert!(manager.non_cli_session_allowlist().contains("shell"));
    assert!(manager.revoke_non_cli_session("shell"));
    assert!(!manager.revoke_non_cli_session("shell"));

    assert_eq!(manager.non_cli_allow_all_once_remaining(), 0);
    assert_eq!(manager.grant_non_cli_allow_all_once(), 1);
    assert_eq!(manager.grant_non_cli_allow_all_once(), 2);
    assert!(manager.consume_non_cli_allow_all_once());
    assert_eq!(manager.non_cli_allow_all_once_remaining(), 1);
    assert!(manager.consume_non_cli_allow_all_once());
    assert!(!manager.consume_non_cli_allow_all_once());
}

#[test]
fn policy_allow_all_once_grant_saturates_at_u32_max() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    *manager.non_cli_allow_all_once_remaining.lock() = u32::MAX;

    assert_eq!(manager.grant_non_cli_allow_all_once(), u32::MAX);
    assert_eq!(manager.non_cli_allow_all_once_remaining(), u32::MAX);
}

#[test]
fn policy_channel_mode_lookup_trims_channel_before_matching() {
    let mut modes = HashMap::new();
    modes.insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::RequestConfirm);
    let manager = ApprovalManager::from_config(&AutonomyConfig {
        non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode::Direct,
        non_cli_natural_language_approval_mode_by_channel: modes,
        ..AutonomyConfig::default()
    });

    assert_eq!(
        manager.non_cli_natural_language_approval_mode_for_channel(" Telegram "),
        NonCliNaturalLanguageApprovalMode::RequestConfirm
    );
}

#[test]
fn policy_applies_and_replaces_runtime_persistent_grants() {
    let manager = ApprovalManager::from_config(&AutonomyConfig {
        always_ask: vec!["shell".to_string()],
        ..AutonomyConfig::default()
    });

    manager.apply_persistent_runtime_grant("shell");
    assert!(manager.auto_approve_tools().contains("shell"));
    assert!(!manager.always_ask_tools().contains("shell"));
    assert!(!manager.needs_approval("shell"));
    assert!(manager.apply_persistent_runtime_revoke("shell"));
    assert!(!manager.apply_persistent_runtime_revoke("shell"));

    let mut modes = HashMap::new();
    modes.insert(" Telegram ".to_string(), NonCliNaturalLanguageApprovalMode::RequestConfirm);
    modes.insert(" ".to_string(), NonCliNaturalLanguageApprovalMode::Disabled);
    manager.replace_runtime_non_cli_policy(
        &["file_read".to_string()],
        &["file_write".to_string()],
        &[" bob ".to_string(), "".to_string()],
        NonCliNaturalLanguageApprovalMode::Disabled,
        &modes,
    );

    assert_eq!(manager.auto_approve_tools(), ["file_read".to_string()].into_iter().collect());
    assert_eq!(manager.always_ask_tools(), ["file_write".to_string()].into_iter().collect());
    assert_eq!(manager.non_cli_approval_approvers(), ["bob".to_string()].into_iter().collect());
    assert_eq!(
        manager.non_cli_natural_language_approval_mode(),
        NonCliNaturalLanguageApprovalMode::Disabled
    );
    assert_eq!(manager.non_cli_natural_language_approval_mode_by_channel().len(), 1);
    assert_eq!(
        manager.non_cli_natural_language_approval_mode_for_channel("telegram"),
        NonCliNaturalLanguageApprovalMode::RequestConfirm
    );
    assert_eq!(
        manager.non_cli_natural_language_approval_mode_for_channel("unknown"),
        NonCliNaturalLanguageApprovalMode::Disabled
    );
}
