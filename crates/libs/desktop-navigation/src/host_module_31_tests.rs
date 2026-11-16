//! Host initialization contracts exercised through the real navigation constructors.
use crate::{
    app_init,
    components::top_bar::types::{ExternalOpenApp, WorkspaceMode},
    ui_types::{Screen, SettingsTab},
};
use serde_json::json;

#[test]
fn navigation_initial_state_preserves_home_tab_and_panel_geometry() {
    let cfg = json!({});
    let workspace = app_init::work_workspace();
    assert_eq!(workspace.work_split_ratio, 0.6);
    assert!(!workspace.work_resources_visible);
    assert!(!workspace.dragging_work_split);
    assert!(workspace.work_expanded_spaces.is_empty());
    let drag = app_init::layout_drag(&cfg);
    assert_eq!(drag.file_manager_width, 260.0);
    assert_eq!(drag.file_manager_start_width, 260.0);
    assert_eq!(drag.settings_panel_width, 476.0);
    assert_eq!(drag.work_sidebar_width, 292.0);
    assert_eq!(drag.split_ratio, 0.6);
    assert_eq!(drag.work_split_drag_start_ratio, 0.6);
    let panels = app_init::panels(&cfg);
    assert!(panels.show_settings);
    assert!(panels.show_diff);
    assert_eq!(panels.settings_tab, SettingsTab::Sessions);
    assert!(!panels.show_system_settings);
    assert!(!panels.show_about_modal);
    let tabs = app_init::tabs();
    assert_eq!(tabs.screen, Screen::Home);
    assert_eq!(tabs.active_tab_id.as_deref(), Some("home"));
    assert_eq!(tabs.open_tabs.len(), 1);
    assert_eq!(tabs.open_tabs[0].title, "首页");
    assert_eq!(tabs.open_tabs[0].screen, Screen::Home);
    assert!(tabs.preview_tabs.is_empty());
    assert!(tabs.project_preview_tabs.is_empty());
    let sidebar = app_init::sidebar(&cfg);
    assert_eq!(
        sidebar.work_sidebar_visible_tasks,
        crate::presentation::WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS
    );
    assert_eq!(
        sidebar.work_sidebar_visible_spaces,
        crate::presentation::WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS
    );
    assert!(sidebar.work_sidebar_space_task_visible.is_empty());
}

#[test]
fn navigation_configuration_preserves_width_bounds_and_explicit_flags() {
    for (width, expected) in [(10.0, 180.0), (320.0, 320.0), (900.0, 600.0)] {
        let cfg = json!({"file_manager_width": width});
        let drag = app_init::layout_drag(&cfg);
        assert_eq!(drag.file_manager_width, expected);
        assert_eq!(drag.file_manager_start_width, expected);
    }
    let cfg =
        json!({"show_settings": false, "auto_max_mode": true, "work_sidebar_collapsed": true});
    assert!(!app_init::panels(&cfg).show_settings);
    assert!(app_init::panels(&cfg).auto_max_mode);
    assert!(app_init::sidebar(&cfg).work_sidebar_collapsed);
}

#[test]
fn navigation_backfill_preserves_omitted_values_and_active_drag() {
    let cfg = json!({});
    let mut panels = app_init::panels(&cfg);
    let mut mode = app_init::workspace_mode(&cfg);
    let mut sidebar = app_init::sidebar(&cfg);
    let mut external = crate::panel::external::State {
        open_external_app: ExternalOpenApp::Finder,
        open_external_exists: Default::default(),
    };
    let mut drag = app_init::layout_drag(&cfg);
    panels.show_settings = false;
    drag.dragging_file_manager = true;
    drag.file_manager_width = 333.0;
    drag.file_manager_start_width = 330.0;
    let cfg = json!({"auto_max_mode": true, "work_sidebar_collapsed": true,
        "open_external_app": "vscode", "file_manager_width": 500.0});
    app_init::apply_preferences(
        &mut panels,
        &mut mode,
        &mut sidebar,
        &mut external,
        &mut drag,
        &cfg,
    );
    assert!(!panels.show_settings);
    assert!(panels.auto_max_mode);
    assert!(sidebar.work_sidebar_collapsed);
    assert_eq!(external.open_external_app, ExternalOpenApp::VSCode);
    assert_eq!(drag.file_manager_width, 333.0);
    assert_eq!(drag.file_manager_start_width, 330.0);
    let previous_mode = mode.workspace_mode;
    drag.dragging_file_manager = false;
    app_init::apply_preferences(
        &mut panels,
        &mut mode,
        &mut sidebar,
        &mut external,
        &mut drag,
        &json!({"workspace_mode": "invalid", "open_external_app": "invalid", "file_manager_width": 900.0}),
    );
    assert_eq!(mode.workspace_mode, previous_mode);
    assert_eq!(external.open_external_app, ExternalOpenApp::VSCode);
    assert_eq!(drag.file_manager_width, 600.0);
    assert_eq!(drag.file_manager_start_width, 600.0);
    let work = WorkspaceMode::Work;
    app_init::apply_preferences(
        &mut panels,
        &mut mode,
        &mut sidebar,
        &mut external,
        &mut drag,
        &json!({"workspace_mode": work.as_str()}),
    );
    assert_eq!(mode.workspace_mode, work);
}
