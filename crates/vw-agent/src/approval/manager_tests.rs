//! 测试审批管理器从配置创建时的空运行态。

use crate::app::agent::config::AutonomyConfig;

use super::ApprovalManager;

#[test]
fn manager_from_config_starts_with_empty_runtime_state() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    assert!(manager.audit_log().is_empty());
    assert!(manager.session_allowlist().is_empty());
    assert!(manager.non_cli_session_allowlist().is_empty());
    assert_eq!(manager.non_cli_allow_all_once_remaining(), 0);
}
