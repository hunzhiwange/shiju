//! 统一智能体选择下，提供商切换和页面预加载的真实消息行为。
use super::*;
use vw_config_types::channels::{ChannelExecutionConfig, WeChatConfig};

fn configured_app() -> (App, String, String) {
    let mut app = App::new().0;
    app.channels_settings = Default::default();
    app.channels_settings.active_tab = crate::app::state::ChannelsSettingsTab::WeChat;
    app.channels_settings.wechat = Some(WeChatConfig {
        bot_token: "fixture-token".into(),
        base_url: "https://example.test".into(),
    });
    (app, "wechat:default".into(), tenant().expect("gateway tenant"))
}

#[test]
fn channel_execution_loading_provider_changes_clear_models_and_keep_unified_agent() {
    let (mut app, key, scope) = configured_app();
    let a = AgentModelReference { provider: "a".into(), model: "model-a".into() };
    let b = AgentModelReference { provider: "b".into(), model: "model-b".into() };
    let generation = app.channels_settings.begin_execution_choices(&scope, &key);
    app.channels_settings.finish_execution_choices(
        &scope,
        &key,
        generation,
        Ok(ChannelExecutionChoices {
            agents: vec![("acp-codex".into(), "Codex".into())],
            models: vec![a.clone(), b.clone()],
        }),
    );
    app.channels_settings.instance_execution.insert(
        key.clone(),
        ChannelExecutionConfig { agent: Some("acp-codex".into()), model: Some(a.clone()) },
    );
    let _ = select_provider(&mut app, key.clone(), Some("a".into()));
    assert_eq!(app.channels_settings.instance_execution[&key].model, Some(a.clone()));
    let _ = select_provider(&mut app, key.clone(), Some("missing".into()));
    assert_eq!(app.channels_settings.instance_execution[&key].model, Some(a));
    let _ = select_provider(&mut app, key.clone(), Some("b".into()));
    assert_eq!(app.channels_settings.execution_provider[&key], "b");
    assert!(app.channels_settings.instance_execution[&key].model.is_none());
    assert_eq!(app.channels_settings.instance_execution[&key].agent.as_deref(), Some("acp-codex"));
    let _ = select_model(&mut app, key.clone(), Some(b.clone()));
    assert_eq!(app.channels_settings.instance_execution[&key].model, Some(b));
    let _ = select_provider(&mut app, key.clone(), None);
    assert!(!app.channels_settings.execution_provider.contains_key(&key));
    assert!(app.channels_settings.instance_execution[&key].model.is_none());
}

#[test]
fn channel_execution_loading_starts_on_entry_and_restarts_after_project_change() {
    let (mut app, key, scope) = configured_app();
    let _ = load_active(&mut app);
    let generation = app.channels_settings.execution_request[&key].1;
    assert!(!app.channels_settings.execution_choices.contains_key(&key));
    app.channels_settings.finish_execution_choices(
        &scope,
        &key,
        generation,
        Ok(ChannelExecutionChoices::default()),
    );
    let projects = app.channels_settings.begin_project_choices(&scope, &key);
    app.channels_settings.finish_project_choices(
        &scope,
        &key,
        projects,
        Ok(vec![("project-two".into(), "项目二".into())]),
    );
    let _ = super::super::projects::select(&mut app, key.clone(), Some("project-two".into()));
    assert_eq!(app.channels_settings.instance_default_projects[&key], "project-two");
    assert!(app.channels_settings.execution_request[&key].1 > generation);
    assert!(!app.channels_settings.execution_choices.contains_key(&key));
}
