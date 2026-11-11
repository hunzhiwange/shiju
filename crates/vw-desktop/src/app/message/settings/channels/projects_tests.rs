//! 通道候选直接使用项目管理列表，保留名称与顺序。
use super::project_choices;
use crate::app::App;

#[test]
fn project_choices_uses_managed_projects_names_and_order() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/data/two".into(), "/data/one".into()];
    app.recent_projects_edits = vec!["项目二".into(), "项目一".into()];
    assert_eq!(
        project_choices(&app),
        vec![("/data/two".into(), "项目二".into()), ("/data/one".into(), "项目一".into()),]
    );
}

#[test]
fn project_choices_uses_directory_name_when_managed_name_is_blank() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/data/one".into()];
    app.recent_projects_edits = vec![" ".into()];
    app.recent_projects_meta.clear();
    assert_eq!(project_choices(&app), vec![("/data/one".into(), "one".into())]);
}

#[test]
fn channel_execution_binding_is_removed_with_channel_instance() {
    let mut app = App::new().0;
    app.channels_settings.instance_execution.insert(
        "wechat:default".into(),
        vw_config_types::channels::ChannelExecutionConfig {
            agent: Some("acp-codex".into()),
            model: None,
        },
    );
    app.channels_settings.wechat = None;
    super::retain_configured_bindings(&mut app);
    assert!(!app.channels_settings.instance_execution.contains_key("wechat:default"));
}
