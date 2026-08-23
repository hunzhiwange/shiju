//! 工作流应用级消息的测试，覆盖新建、打开、关闭和编辑器状态转换。
//! 历史与脏状态 fixture 用于验证切换应用时的保存提示和选择保持。
use super::*;
use crate::apps::workflow::model::{
    WorkflowDocument, WorkflowEdge, WorkflowHandleSide, WorkflowNode, WorkflowViewport,
};
use crate::apps::workflow::state::{WorkflowAppEntry, WorkflowHistorySnapshot};
use iced::Size;
use serde_yaml::Value;

fn new_app() -> App {
    App::new().0
}

fn workflow_node(id: &str, block_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: id.to_string(),
        description: String::new(),
        position: Point::new(0.0, 0.0),
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn workflow_edge(source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("{source}-{target}"),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "start".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn workflow_document(name: &str) -> WorkflowDocument {
    WorkflowDocument {
        name: name.to_string(),
        nodes: vec![workflow_node("start", "start"), workflow_node("answer", "answer")],
        edges: vec![workflow_edge("start", "answer")],
        viewport: WorkflowViewport { x: 0.0, y: 0.0, zoom: 1.0 },
    }
}

fn workflow_snapshot(document: &WorkflowDocument) -> WorkflowHistorySnapshot {
    WorkflowHistorySnapshot {
        meta: WorkflowAppMeta::default(),
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    }
}

fn app_entry(id: &str, local_uuid: Option<&str>, name: &str) -> WorkflowAppEntry {
    let document = workflow_document(name);
    WorkflowAppEntry {
        id: id.to_string(),
        local_uuid: local_uuid.map(str::to_string),
        meta: WorkflowAppMeta { name: name.to_string(), ..Default::default() },
        source_path: None,
        raw_root: Value::Null,
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
        connection_draft: None,
        is_dirty: false,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        saved_snapshot: workflow_snapshot(&document),
    }
}

fn loaded_workflow(name: &str, local_uuid: Option<&str>) -> LoadedWorkflow {
    LoadedWorkflow {
        local_uuid: local_uuid.map(str::to_string),
        source_path: Some(format!("/tmp/{name}.yml")),
        source_name: name.to_string(),
        app_meta: WorkflowAppMeta { name: name.to_string(), ..Default::default() },
        document: workflow_document(name),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        had_viewport: true,
        raw_root: Value::Null,
    }
}

#[test]
fn saved_app_list_messages_update_loading_search_menu_and_delete_state() {
    let mut app = new_app();

    assert!(app::handle(&mut app, WorkflowMessage::LoadSavedApps).is_some());
    assert!(app.workflow_state.saved_apps_loading);

    app::handle(
        &mut app,
        WorkflowMessage::LoadSavedAppsFinished(Ok(vec![WorkflowSavedAppSummary {
            uuid: "uuid-1".to_string(),
            name: "应用".to_string(),
            description: "描述".to_string(),
            created_at_ms: 1,
            updated_at_ms: 2,
        }])),
    );
    assert!(!app.workflow_state.saved_apps_loading);
    assert!(app.workflow_state.saved_apps_loaded);
    assert_eq!(app.workflow_state.saved_apps.len(), 1);

    app::handle(&mut app, WorkflowMessage::SavedAppSearchChanged("query".to_string()));
    assert_eq!(app.workflow_state.saved_app_search_query, "query");

    app::handle(&mut app, WorkflowMessage::ToggleSavedAppActions("uuid-1".to_string()));
    assert_eq!(app.workflow_state.saved_app_actions_menu_uuid.as_deref(), Some("uuid-1"));
    app::handle(&mut app, WorkflowMessage::CloseSavedAppActions);
    assert!(app.workflow_state.saved_app_actions_menu_uuid.is_none());

    app::handle(&mut app, WorkflowMessage::RequestDeleteSavedApp("uuid-1".to_string()));
    assert_eq!(app.workflow_state.confirm_delete_saved_app_uuid.as_deref(), Some("uuid-1"));
    app::handle(&mut app, WorkflowMessage::CancelDeleteSavedApp);
    assert!(app.workflow_state.confirm_delete_saved_app_uuid.is_none());

    app::handle(&mut app, WorkflowMessage::DeleteSavedApp("uuid-1".to_string()));
    assert_eq!(app.workflow_state.deleting_saved_app_uuid.as_deref(), Some("uuid-1"));
    app::handle(&mut app, WorkflowMessage::DeleteSavedAppFinished(Ok("uuid-1".to_string())));
    assert!(app.workflow_state.deleting_saved_app_uuid.is_none());
    assert!(app.workflow_state.saved_apps.is_empty());
}

#[test]
fn saved_app_errors_and_copy_state_are_recorded() {
    let mut app = new_app();

    app::handle(&mut app, WorkflowMessage::LoadSavedAppsFinished(Err("load failed".to_string())));
    assert_eq!(app.workflow_state.saved_apps_error.as_deref(), Some("load failed"));

    app::handle(&mut app, WorkflowMessage::OpenSavedAppFinished(Err("open failed".to_string())));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("open failed"));

    app::handle(
        &mut app,
        WorkflowMessage::DeleteSavedAppFinished(Err("delete failed".to_string())),
    );
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("delete failed"));

    app::handle(&mut app, WorkflowMessage::CopySavedAppUuid("uuid-2".to_string()));
    assert_eq!(app.workflow_state.copied_saved_app_uuid.as_deref(), Some("uuid-2"));
    app::handle(&mut app, WorkflowMessage::ClearCopiedSavedAppUuid("uuid-2".to_string()));
    assert!(app.workflow_state.copied_saved_app_uuid.is_none());
}

#[test]
fn open_saved_app_selects_cached_entry_or_marks_pending_open() {
    let mut app = new_app();
    app.workflow_state.apps.push(app_entry("cached", Some("uuid-cached"), "Cached"));
    app.workflow_state.action_menu_open = true;

    app::handle(&mut app, WorkflowMessage::OpenSavedApp("uuid-cached".to_string()));
    assert_eq!(app.workflow_state.active_app_id.as_deref(), Some("cached"));
    assert_eq!(app.workflow_state.source_name, "Cached");
    assert!(!app.workflow_state.action_menu_open);
    assert!(app.workflow_state.opening_saved_app_uuid.is_none());

    app.workflow_state.action_menu_open = true;
    app.workflow_state.saved_app_actions_menu_uuid = Some("uuid-missing".to_string());
    app::handle(&mut app, WorkflowMessage::OpenSavedApp("uuid-missing".to_string()));
    assert_eq!(app.workflow_state.opening_saved_app_uuid.as_deref(), Some("uuid-missing"));
    assert!(app.workflow_state.saved_app_actions_menu_uuid.is_none());
    assert!(!app.workflow_state.action_menu_open);
}

#[test]
fn open_saved_app_finished_applies_loaded_workflow_and_clears_pending_state() {
    let mut app = new_app();
    app.workflow_state.opening_saved_app_uuid = Some("uuid-loaded".to_string());

    app::handle(
        &mut app,
        WorkflowMessage::OpenSavedAppFinished(Ok(loaded_workflow("Loaded", Some("uuid-loaded")))),
    );

    assert!(app.workflow_state.opening_saved_app_uuid.is_none());
    assert_eq!(app.workflow_state.local_uuid.as_deref(), Some("uuid-loaded"));
    assert_eq!(app.workflow_state.source_name, "Loaded");
    assert_eq!(
        app.workflow_state.active_app().map(|entry| entry.meta.name.as_str()),
        Some("Loaded")
    );
}

#[test]
fn open_file_finished_handles_loaded_cancel_and_error_results() {
    let mut app = new_app();

    app::handle(
        &mut app,
        WorkflowMessage::OpenFileFinished(Ok(Some(loaded_workflow("FileApp", None)))),
    );
    assert_eq!(app.workflow_state.source_name, "FileApp");

    app.workflow_state.status_message = Some("unchanged".to_string());
    app::handle(&mut app, WorkflowMessage::OpenFileFinished(Ok(None)));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("unchanged"));

    assert!(
        app::handle(&mut app, WorkflowMessage::OpenFileFinished(Err("open failed".to_string())))
            .is_some()
    );
}

#[test]
fn show_saved_apps_loads_once_and_closes_active_panels() {
    let mut app = new_app();
    app.workflow_state.saved_apps_loaded = false;
    app.workflow_state.saved_apps_loading = false;
    app.workflow_state.action_menu_open = true;

    assert!(app::handle(&mut app, WorkflowMessage::ShowSavedApps).is_some());
    assert!(app.workflow_state.saved_apps_loading);
    assert!(!app.workflow_state.action_menu_open);

    assert!(app::handle(&mut app, WorkflowMessage::ShowSavedApps).is_some());
}

#[test]
fn inline_yaml_opens_valid_preview_and_reports_invalid_yaml() {
    let mut app = new_app();
    let yaml = r#"
app:
  name: Inline
workflow:
  graph:
    nodes: []
    edges: []
"#;

    app::handle(
        &mut app,
        WorkflowMessage::OpenInlineYaml {
            workflow_yaml: yaml.to_string(),
            focus_node_id: Some("".to_string()),
        },
    );
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已打开临时工作流预览"));

    app::handle(
        &mut app,
        WorkflowMessage::OpenInlineYaml {
            workflow_yaml: "not: [valid".to_string(),
            focus_node_id: None,
        },
    );
    assert!(app.workflow_state.error_message.is_some());
}

#[test]
fn inline_yaml_reports_missing_nonempty_focus_node_after_loading_preview() {
    let mut app = new_app();
    let yaml = r#"
app:
  name: Focused
workflow:
  graph:
    nodes: []
    edges: []
"#;

    app::handle(
        &mut app,
        WorkflowMessage::OpenInlineYaml {
            workflow_yaml: yaml.to_string(),
            focus_node_id: Some("missing-node".to_string()),
        },
    );

    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已打开临时工作流预览"));
    assert!(app.workflow_state.error_message.is_some());
}

#[test]
fn open_file_closes_floating_panels_before_returning_task() {
    let mut app = new_app();
    app.workflow_state.action_menu_open = true;
    app.workflow_state.quick_insert_panel_open = true;
    app.workflow_state.zoom_menu_open = true;

    assert!(app::handle(&mut app, WorkflowMessage::OpenFile).is_some());

    assert!(!app.workflow_state.action_menu_open);
    assert!(!app.workflow_state.quick_insert_panel_open);
    assert!(!app.workflow_state.zoom_menu_open);
}

#[test]
fn app_editor_messages_edit_and_submit_create_flow() {
    let mut app = new_app();

    app::handle(&mut app, WorkflowMessage::OpenCreateAppEditor);
    assert!(app.workflow_state.app_editor.is_some());
    app::handle(&mut app, WorkflowMessage::AppEditorNameChanged("Demo".to_string()));
    app::handle(&mut app, WorkflowMessage::AppEditorDescriptionChanged("Desc".to_string()));
    app::handle(&mut app, WorkflowMessage::AppEditorIconChanged("D".to_string()));
    app::handle(&mut app, WorkflowMessage::AppEditorUseIconAsAnswerIconChanged(true));
    app::handle(&mut app, WorkflowMessage::AppEditorMaxActiveRequestsChanged("3".to_string()));

    let editor = app.workflow_state.app_editor.as_ref().unwrap();
    assert_eq!(editor.name, "Demo");
    assert_eq!(editor.description, "Desc");
    assert_eq!(editor.icon, "D");
    assert!(editor.use_icon_as_answer_icon);
    assert_eq!(editor.max_active_requests_input, "3");

    app::handle(&mut app, WorkflowMessage::SubmitAppEditor);
    assert!(app.workflow_state.app_editor.is_none());
    assert_eq!(app.workflow_state.source_name, "Demo");

    app::handle(&mut app, WorkflowMessage::OpenEditAppEditor(None));
    assert!(app.workflow_state.app_editor.is_some());
    app::handle(&mut app, WorkflowMessage::CloseAppEditor);
    assert!(app.workflow_state.app_editor.is_none());
}

#[test]
fn submit_app_editor_without_editor_is_noop_and_invalid_input_sets_error() {
    let mut app = new_app();

    app::handle(&mut app, WorkflowMessage::SubmitAppEditor);
    assert!(app.workflow_state.app_editor.is_none());
    assert!(app.workflow_state.error_message.is_none());

    app::handle(&mut app, WorkflowMessage::OpenCreateAppEditor);
    app::handle(&mut app, WorkflowMessage::AppEditorMaxActiveRequestsChanged("many".to_string()));
    app::handle(&mut app, WorkflowMessage::SubmitAppEditor);

    assert_eq!(app.workflow_state.error_message.as_deref(), Some("最大活跃请求数必须是非负整数"));
    assert!(app.workflow_state.app_editor.is_some());
}

#[test]
fn delete_saved_app_finished_clears_active_app_when_removed_entry_was_active() {
    let mut app = new_app();
    app.workflow_state.apps.push(app_entry("active", Some("uuid-active"), "Active"));
    app.workflow_state.saved_apps.push(WorkflowSavedAppSummary {
        uuid: "uuid-active".to_string(),
        name: "Active".to_string(),
        description: String::new(),
        created_at_ms: 1,
        updated_at_ms: 2,
    });
    app::handle(&mut app, WorkflowMessage::OpenSavedApp("uuid-active".to_string()));

    app::handle(&mut app, WorkflowMessage::DeleteSavedAppFinished(Ok("uuid-active".to_string())));

    assert!(app.workflow_state.active_app_id.is_none());
    assert!(app.workflow_state.local_uuid.is_none());
    assert!(app.workflow_state.apps.is_empty());
    assert!(app.workflow_state.saved_apps.is_empty());
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已删除应用"));
}

#[test]
fn organize_active_app_reports_missing_active_and_updates_loaded_document() {
    let mut app = new_app();

    app::handle(&mut app, WorkflowMessage::OrganizeActiveApp);
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("请先打开一个应用"));

    app::handle(
        &mut app,
        WorkflowMessage::OpenFileFinished(Ok(Some(loaded_workflow("Organized", None)))),
    );
    let before = app.workflow_state.document.node("answer").unwrap().position;
    app::handle(&mut app, WorkflowMessage::OrganizeActiveApp);
    let after = app.workflow_state.document.node("answer").unwrap().position;

    assert_ne!(after, before);
    assert!(app.workflow_state.active_is_dirty);
}

#[test]
fn app_handle_returns_none_for_non_app_messages() {
    let mut app = new_app();

    assert!(app::handle(&mut app, WorkflowMessage::ToggleZoomMenu).is_none());
}
