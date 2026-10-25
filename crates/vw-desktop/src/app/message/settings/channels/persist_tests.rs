#![allow(unused_imports, dead_code)]

// 验证启用频道汇总、配置构建与持久化前置校验。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::message::settings::channels::toggles;
use vw_config_types::channels::{
    DingTalkConfig, DingTalkInstanceConfig, FeishuConfig, FeishuInstanceConfig,
};

fn app() -> App {
    App::new().0
}

mod channel_instance_settings {
    use super::*;
}

#[test]
fn channel_default_project_saved_config_preserves_independent_bindings_and_secrets() {
    let mut settings = ChannelsSettingsState::default();
    settings.qq = Some(vw_config_types::channels::QQConfig {
        app_id: "app".into(),
        app_secret: "secret".into(),
        allowed_users: vec![],
        receive_mode: vw_config_types::channels::QQReceiveMode::Webhook,
    });
    let feishu: FeishuConfig =
        serde_json::from_value(serde_json::json!({"app_id":"app", "app_secret":"secret"})).unwrap();
    settings.feishu_instances.push(FeishuInstanceConfig { id: "team".into(), config: feishu });
    settings.instance_default_projects.insert("feishu:team".into(), "project-one".into());
    settings.instance_default_projects.insert("qq:default".into(), "project-two".into());
    let config = build_channels_config_from_settings(&settings).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("channels.json");
    std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    let loaded: ChannelsConfig = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    loaded.validate_instance_default_projects().unwrap();
    assert_eq!(loaded.instance_default_projects["feishu:team"], "project-one");
    assert_eq!(loaded.instance_default_projects["qq:default"], "project-two");
    assert_eq!(loaded.qq.unwrap().app_secret, "secret");
}
