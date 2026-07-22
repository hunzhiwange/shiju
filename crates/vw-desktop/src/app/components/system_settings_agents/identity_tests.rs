use super::identity::{
    default_workspace_identity_root, format_file_size, view, workspace_identity_hint,
    workspace_identity_meta_line,
};

#[test]
fn default_workspace_identity_root_uses_agent_key() {
    let config_root = format!("~/{}", vw_config_types::paths::HOME_CONFIG_DIR_NAME);

    assert_eq!(default_workspace_identity_root("main"), format!("{config_root}/workspace"));
    assert_eq!(default_workspace_identity_root("coder"), format!("{config_root}/workspace-coder"));
}

#[test]
fn workspace_identity_hint_prefers_custom_root() {
    assert_eq!(
        workspace_identity_hint(Some("profiles/reviewer"), "reviewer", "AGENTS.md"),
        "编辑身份文件 `AGENTS.md`，保存会直接写回 `profiles/reviewer/AGENTS.md`。"
    );
}

#[test]
fn workspace_identity_hint_uses_default_root_for_main_and_workers() {
    assert_eq!(
        workspace_identity_hint(None, "main", "AGENTS.md"),
        format!(
            "编辑身份文件 `AGENTS.md`，保存会直接写回 `~/{}/workspace/AGENTS.md`。",
            vw_config_types::paths::HOME_CONFIG_DIR_NAME
        )
    );
    assert_eq!(
        workspace_identity_hint(None, "reviewer", "AGENTS.md"),
        format!(
            "编辑身份文件 `AGENTS.md`，保存会直接写回 `~/{}/workspace-reviewer/AGENTS.md`。",
            vw_config_types::paths::HOME_CONFIG_DIR_NAME
        )
    );
}

#[test]
fn format_file_size_uses_human_readable_units() {
    assert_eq!(format_file_size(None), None);
    assert_eq!(format_file_size(Some(0)), Some("0 B".to_string()));
    assert_eq!(format_file_size(Some(512)), Some("512 B".to_string()));
    assert_eq!(format_file_size(Some(1023)), Some("1023 B".to_string()));
    assert_eq!(format_file_size(Some(1024)), Some("1.0 KB".to_string()));
    assert_eq!(format_file_size(Some(2048)), Some("2.0 KB".to_string()));
    assert_eq!(format_file_size(Some(1024 * 1024)), Some("1.0 MB".to_string()));
    assert_eq!(format_file_size(Some(2 * 1024 * 1024 + 512 * 1024)), Some("2.5 MB".to_string()));
}

#[test]
fn workspace_identity_meta_line_combines_size_and_modified_time() {
    assert_eq!(
        workspace_identity_meta_line(Some(1536), Some(0)),
        Some("1.5 KB · 刚刚".to_string())
    );
}

#[test]
fn workspace_identity_meta_line_handles_single_and_missing_values() {
    assert_eq!(workspace_identity_meta_line(Some(512), None), Some("512 B".to_string()));
    assert_eq!(workspace_identity_meta_line(None, Some(0)), Some("刚刚".to_string()));
    assert_eq!(workspace_identity_meta_line(None, None), None);
}

#[test]
fn identity_view_builds_system_prompt_panel() {
    let (app, _) = crate::app::App::new();
    let entry = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == app.agents_settings.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}

#[test]
fn identity_view_builds_workspace_file_panel_with_metadata_and_custom_root() {
    let (mut app, _) = crate::app::App::new();
    let file_name = {
        let file_state = app
            .agents_settings
            .workspace_identity_files
            .first_mut()
            .expect("default identity file should exist");
        file_state.size_bytes = Some(2048);
        file_state.modified_at_ms = Some(0);
        file_state.file_name.clone()
    };
    app.agents_settings.active_prompt_tab = file_name;
    app.agents_settings.workspace_identity_root_path = Some("profiles/main".to_string());
    let entry = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == app.agents_settings.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}

#[test]
fn identity_view_builds_workspace_file_without_metadata_using_default_root() {
    let (mut app, _) = crate::app::App::new();
    let file_name = app
        .agents_settings
        .workspace_identity_files
        .first()
        .expect("default identity file should exist")
        .file_name
        .clone();
    app.agents_settings.active_prompt_tab = file_name;
    app.agents_settings.workspace_identity_root_path = None;
    let entry = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == app.agents_settings.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}

#[test]
fn identity_view_builds_missing_workspace_file_fallback() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.active_prompt_tab = "MISSING.md".to_string();
    let entry = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == app.agents_settings.selected_agent)
        .expect("selected agent should exist");

    let element = view(&app, entry);

    assert_eq!(element.as_widget().children().len(), 3);
}
