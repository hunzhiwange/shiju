//! 验证代理身份字段、头像选择与编辑消息的映射。
//! 空身份、预设头像和自定义值用于约束展示回退及保存负载。

use super::identity_view as view;

#[test]
fn identity_view_builds_system_prompt_panel() {
    let app = crate::agents_state::AgentsSettingsState::default();
    let entry = app
        .entries
        .iter()
        .find(|entry| entry.key == app.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}

#[test]
fn identity_view_builds_workspace_file_panel_with_metadata_and_custom_root() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    let file_name = {
        let file_state =
            app.workspace_identity_files.first_mut().expect("default identity file should exist");
        file_state.size_bytes = Some(2048);
        file_state.modified_at_ms = Some(0);
        file_state.file_name.clone()
    };
    app.active_prompt_tab = file_name;
    app.workspace_identity_root_path = Some("profiles/main".to_string());
    let entry = app
        .entries
        .iter()
        .find(|entry| entry.key == app.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}

#[test]
fn identity_view_builds_workspace_file_without_metadata_using_default_root() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    let file_name = app
        .workspace_identity_files
        .first()
        .expect("default identity file should exist")
        .file_name
        .clone();
    app.active_prompt_tab = file_name;
    app.workspace_identity_root_path = None;
    let entry = app
        .entries
        .iter()
        .find(|entry| entry.key == app.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}

#[test]
fn identity_view_builds_missing_workspace_file_fallback() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    app.active_prompt_tab = "MISSING.md".to_string();
    let entry = app
        .entries
        .iter()
        .find(|entry| entry.key == app.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}
