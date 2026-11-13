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

#[test]
fn wecom_delivery_modal_loads_independent_webhook_without_prefilled_draft() {
    let cfg = vw_config_types::channels::ChannelsConfig {
        wecom_webhook: Some(vw_config_types::channels::WeComWebhookConfig {
            webhook_url: "***MASKED***".into(),
        }),
        ..Default::default()
    };
    let settings = super::build_channels_settings(&cfg);
    assert!(settings.wecom.is_none());
    assert_eq!(settings.wecom_webhook.unwrap().webhook_url, "***MASKED***");
    assert!(settings.wecom_delivery.is_none());
}
