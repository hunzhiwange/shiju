//! 应用组合兼容路径测试。

#[test]
fn agent_compat_namespace_exposes_selected_runtime_modules() {
    let config = super::agent::config::AutonomyConfig::default();
    let manager = super::agent::approval::ApprovalManager::from_config(&config);

    assert_eq!(config.level, super::agent::security::AutonomyLevel::Supervised);
    assert!(manager.needs_approval("file_write"));
}
