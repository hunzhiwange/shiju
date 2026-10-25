use super::*;

fn configured() -> ChannelsConfig {
    let mut config = ChannelsConfig {
        dingtalk: Some(DingTalkConfig {
            client_id: "client".into(),
            client_secret: "secret".into(),
            allowed_users: vec![],
        }),
        ..ChannelsConfig::default()
    };
    config.dingtalk_instances.push(DingTalkInstanceConfig {
        id: "team".into(),
        config: config.dingtalk.clone().unwrap(),
    });
    config
}

#[test]
fn channel_project_binding_round_trips_registered_project_ids() {
    let mut config = configured();
    config.set_instance_default_project("dingtalk:default", "project-1").unwrap();
    let restored: ChannelsConfig =
        serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
    assert_eq!(
        restored.instance_default_projects.get("dingtalk:default").map(String::as_str),
        Some("project-1")
    );
    restored.validate_instance_default_projects().unwrap();
}

#[test]
fn channel_project_binding_rejects_invalid_updates_without_mutation() {
    let mut config = configured();
    config.set_instance_default_project("dingtalk:default", "project-1").unwrap();
    let before = config.instance_default_projects.clone();
    for (key, project) in [
        ("dingtalk:unknown", "project-2"),
        ("dingtalk:default", " "),
        ("dingtalk:default", "/tmp/project"),
        ("dingtalk:default:extra", "project-2"),
        ("qq:default", "project-2"),
    ] {
        assert!(config.set_instance_default_project(key, project).is_err(), "{key}: {project}");
        assert_eq!(config.instance_default_projects, before);
    }
}

#[test]
fn channel_project_binding_isolates_instances_and_rejects_stale_bindings() {
    let mut config = configured();
    config.set_instance_default_project("dingtalk:default", "project-1").unwrap();
    config.set_instance_default_project("dingtalk:team", "project-2").unwrap();
    assert_eq!(config.instance_default_projects["dingtalk:default"], "project-1");
    assert_eq!(config.instance_default_projects["dingtalk:team"], "project-2");
    config.qq = Some(QQConfig {
        app_id: "app".into(),
        app_secret: "secret".into(),
        allowed_users: vec![],
        receive_mode: QQReceiveMode::Webhook,
    });
    config.set_instance_default_project("qq:default", "project-3").unwrap();
    assert_eq!(config.instance_default_projects["dingtalk:default"], "project-1");
    assert_eq!(config.instance_default_projects["qq:default"], "project-3");
    config.dingtalk_instances.clear();
    assert!(config.validate_instance_default_projects().is_err());
}

#[test]
fn channel_project_binding_legacy_configuration_defaults_to_empty_bindings() {
    let mut value = serde_json::to_value(configured()).unwrap();
    value.as_object_mut().unwrap().remove("instance_default_projects");
    let config: ChannelsConfig = serde_json::from_value(value).unwrap();
    assert!(config.instance_default_projects.is_empty());
    config.validate_instance_default_projects().unwrap();
}
