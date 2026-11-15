//! 验证自治等级、预算、身份格式与显式权限限制。

use std::str::FromStr;

// 自治级别接受历史拼写，但未知值必须显式失败；身份格式保持兼容默认。
#[test]
fn parses_autonomy_levels_and_identity_defaults() {
    assert_eq!(super::AutonomyLevel::from_str("readonly").unwrap(), super::AutonomyLevel::ReadOnly);
    assert_eq!(
        super::AutonomyLevel::from_str("read_only").unwrap(),
        super::AutonomyLevel::ReadOnly
    );
    assert_eq!(
        super::AutonomyLevel::from_str("supervised").unwrap(),
        super::AutonomyLevel::Supervised
    );
    assert!(super::AutonomyLevel::from_str("invalid").is_err());

    let identity = super::IdentityConfig::default();
    assert_eq!(identity.format, "openclaw");
    assert!(identity.aieos_path.is_none());
}

// 默认预算与自动批准列表属于运行策略契约，反序列化空对象应与 Default 一致。
#[test]
fn autonomy_defaults_allow_full_access_with_expanded_budgets() {
    let defaulted = super::AutonomyConfig::default();
    let deserialized: super::AutonomyConfig = serde_json::from_str("{}").unwrap();

    for autonomy in [&defaulted, &deserialized] {
        assert_eq!(autonomy.level, super::AutonomyLevel::Full);
        assert!(!autonomy.workspace_only);
        assert!(!autonomy.require_approval_for_medium_risk);
        assert!(!autonomy.block_high_risk_commands);
        assert_eq!(autonomy.shell_redirect_policy, super::ShellRedirectPolicy::Allow);
        assert_eq!(autonomy.max_actions_per_hour, 2_000);
        assert_eq!(autonomy.max_cost_per_day_cents, 50_000);
        assert!(autonomy.auto_approve.contains(&"file_read".to_string()));
        assert!(autonomy.non_cli_excluded_tools.contains(&"bash".to_string()));
    }

    let serialized = serde_json::to_value(&defaulted).unwrap();
    assert_eq!(serialized["shell_redirect_policy"], "allow");
    let round_tripped: super::AutonomyConfig = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped.shell_redirect_policy, super::ShellRedirectPolicy::Allow);
}

// 所有限制项均允许用户独立收紧，不能因其它字段缺省而恢复宽松值。
#[test]
fn autonomy_explicit_restrictive_values_remain_supported() {
    let autonomy: super::AutonomyConfig = serde_json::from_str(
        r#"{
            "level": "supervised",
            "workspace_only": true,
            "require_approval_for_medium_risk": true,
            "block_high_risk_commands": true,
            "shell_redirect_policy": "block",
            "max_actions_per_hour": 20,
            "max_cost_per_day_cents": 500
        }"#,
    )
    .unwrap();

    assert_eq!(autonomy.level, super::AutonomyLevel::Supervised);
    assert!(autonomy.workspace_only);
    assert!(autonomy.require_approval_for_medium_risk);
    assert!(autonomy.block_high_risk_commands);
    assert_eq!(autonomy.shell_redirect_policy, super::ShellRedirectPolicy::Block);
    assert_eq!(autonomy.max_actions_per_hour, 20);
    assert_eq!(autonomy.max_cost_per_day_cents, 500);
}
