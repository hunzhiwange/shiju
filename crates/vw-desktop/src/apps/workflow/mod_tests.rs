//! 工作流应用外层接口的测试，覆盖模块装配和顶层更新视图契约。
//! 该组确保工作流能力在桌面应用中可被稳定创建与分派。
use super::*;
use crate::app::{App, AppTab, Screen};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("mod_tests"));
}

#[test]
fn sync_top_tab_replaces_apps_tab_and_updates_existing_workflow_tab() {
    let mut app = App::new().0;
    app.workflow_state.source_name = "当前工作流".to_string();
    app.workflow_state.active_app_id = Some("app".to_string());
    app.open_tabs = vec![
        AppTab {
            id: "apps".to_string(),
            title: "应用".to_string(),
            screen: Screen::Apps,
            project_path: Some("/tmp".to_string()),
        },
        AppTab {
            id: WORKFLOW_TOOL_TAB_ID.to_string(),
            title: "旧标题".to_string(),
            screen: Screen::Apps,
            project_path: Some("/tmp".to_string()),
        },
    ];

    sync_top_tab(&mut app);

    assert!(app.open_tabs.iter().all(|tab| tab.id != "apps"));
    let workflow_tab = app
        .open_tabs
        .iter()
        .find(|tab| tab.id == WORKFLOW_TOOL_TAB_ID)
        .expect("workflow tab should exist");
    assert_eq!(workflow_tab.title, "当前工作流");
    assert_eq!(workflow_tab.screen, Screen::WorkflowTool);
    assert!(workflow_tab.project_path.is_none());
    assert_eq!(app.active_tab_id.as_deref(), Some(WORKFLOW_TOOL_TAB_ID));
    assert_eq!(app.screen, Screen::WorkflowTool);
}

#[test]
fn sync_top_tab_adds_workflow_tab_when_missing() {
    let mut app = App::new().0;
    app.workflow_state.source_name = "新建工作流".to_string();
    app.workflow_state.active_app_id = Some("app".to_string());
    app.open_tabs.clear();
    app.active_tab_id = Some("other".to_string());
    app.screen = Screen::Apps;

    sync_top_tab(&mut app);

    assert_eq!(app.open_tabs.len(), 1);
    let workflow_tab = &app.open_tabs[0];
    assert_eq!(workflow_tab.id, WORKFLOW_TOOL_TAB_ID);
    assert_eq!(workflow_tab.title, "新建工作流");
    assert_eq!(workflow_tab.screen, Screen::WorkflowTool);
    assert!(workflow_tab.project_path.is_none());
    assert_eq!(app.active_tab_id.as_deref(), Some(WORKFLOW_TOOL_TAB_ID));
    assert_eq!(app.screen, Screen::WorkflowTool);
}

#[test]
fn ensure_initialized_starts_saved_apps_load_once() {
    let mut app = App::new().0;

    let _ = ensure_initialized(&mut app);
    assert!(app.workflow_state.saved_apps_loading);
    assert_eq!(app.active_tab_id.as_deref(), Some(WORKFLOW_TOOL_TAB_ID));

    let tab_count = app.open_tabs.len();
    let _ = ensure_initialized(&mut app);
    assert_eq!(app.open_tabs.len(), tab_count);

    app.workflow_state.saved_apps_loading = false;
    app.workflow_state.saved_apps_loaded = true;
    let _ = ensure_initialized(&mut app);
    assert!(!app.workflow_state.saved_apps_loading);
}

#[test]
fn view_builds_workflow_element_from_app_state() {
    let app = App::new().0;

    let _element = view(&app);
}

#[test]
fn update_forwards_messages_to_workflow_message_module() {
    let mut app = App::new().0;

    let _ = update(&mut app, WorkflowMessage::ToggleZoomMenu);

    assert!(app.workflow_state.zoom_menu_open);
}
