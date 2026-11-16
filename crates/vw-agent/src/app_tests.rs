//! 应用组合兼容路径测试。

#[test]
fn config_bridge_sets_and_loads_unique_field() {
    let key = format!("app_tests_bridge_{}", std::process::id());

    super::config::set_config_field(&key, serde_json::json!({"enabled": true}));
    let config = super::config::load_app_config();

    assert_eq!(config[&key]["enabled"], true);
}

#[test]
fn agent_compat_namespace_exposes_selected_runtime_modules() {
    let config = super::agent::config::AutonomyConfig::default();
    let manager = super::agent::approval::ApprovalManager::from_config(&config);

    assert_eq!(config.level, super::agent::security::AutonomyLevel::Supervised);
    assert!(manager.needs_approval("file_write"));
}
