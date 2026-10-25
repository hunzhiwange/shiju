//! 验证通道配置保存后重新初始化桌面状态。
#[test]
fn channel_default_project_reload_restores_bindings() {
    let mut config = vw_config_types::channels::ChannelsConfig::default();
    config.instance_default_projects.insert("qq:default".into(), "project-one".into());
    config.instance_default_projects.insert("feishu:team".into(), "project-two".into());
    let loaded = serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
    let settings = super::build_channels_settings(&loaded);
    assert_eq!(settings.instance_default_projects, config.instance_default_projects);
}
