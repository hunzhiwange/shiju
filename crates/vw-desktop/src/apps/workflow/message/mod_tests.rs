//! 工作流消息分派的测试，覆盖顶层消息到各状态处理器的路由。
//! 用例确保无活动应用或编辑器时的操作保持安全空操作。
#![allow(unused_must_use)]
use super::*;
use crate::apps::workflow::model::create_node_from_type;
use crate::apps::workflow::state::WorkflowVariablePanelKind;
use iced::{Point, Vector};

fn app_with_workflow() -> App {
    let mut app = App::new().0;
    app.window_size = (1000.0, 800.0);
    let loaded = load_document_from_text(
        None,
        r#"
app:
  name: 消息入口测试
workflow:
  graph:
    nodes:
      - id: start
        position:
          x: 10
          y: 20
        data:
          title: Start
          type: start
      - id: answer
        position:
          x: 260
          y: 20
        data:
          title: Answer
          type: answer
    edges: []
"#
        .to_string(),
    )
    .expect("workflow should load");
    apply_loaded(&mut app, loaded);
    app
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("mod_tests"));
}

#[test]
fn update_dispatches_to_app_node_document_and_canvas_handlers() {
    let mut app = app_with_workflow();

    update(&mut app, WorkflowMessage::SavedAppSearchChanged("abc".to_string()));
    assert_eq!(app.workflow_state.saved_app_search_query, "abc");

    update(&mut app, WorkflowMessage::OpenSavedAppRunPrompt("workflow-1".to_string()));
    assert_eq!(
        app.workflow_state.saved_app_run_prompt.as_ref().map(|p| p.uuid.as_str()),
        Some("workflow-1")
    );

    update(&mut app, WorkflowMessage::ToggleQuickInsertPanel);
    assert!(app.workflow_state.quick_insert_panel_open);

    update(&mut app, WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::System));
    assert_eq!(app.workflow_state.variable_panel, Some(WorkflowVariablePanelKind::System));

    update(&mut app, WorkflowMessage::SelectNode("start".to_string()));
    assert_eq!(app.workflow_state.selected_node_id.as_deref(), Some("start"));
}

#[test]
fn update_dispatches_run_preview_retry_and_cancel_messages() {
    let mut app = app_with_workflow();

    app.workflow_state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    );
    update(&mut app, WorkflowMessage::CancelSavedAppRun);
    assert_eq!(app.workflow_state.running_saved_app_uuid, None);
    assert_eq!(
        app.workflow_state.saved_app_run_preview.as_ref().map(|preview| preview.status),
        Some(crate::apps::workflow::state::WorkflowRunStatus::Canceled)
    );

    app.workflow_state.saved_app_run_preview =
        Some(crate::apps::workflow::state::WorkflowRunPreview {
            uuid: "workflow-1".to_string(),
            run_id: Some("run-1".to_string()),
            status: crate::apps::workflow::state::WorkflowRunStatus::Failed,
            nodes: vec![],
            text_output: "boom".to_string(),
            error: Some(crate::apps::workflow::state::WorkflowRunErrorPreview {
                status: 500,
                code: "workflow_failed".to_string(),
                message: "boom".to_string(),
            }),
            is_finished: true,
        });
    app.workflow_state.running_saved_app_query = "hello".to_string();

    update(&mut app, WorkflowMessage::RetrySavedAppRunPreview);
    assert_eq!(app.workflow_state.running_saved_app_uuid.as_deref(), Some("workflow-1"));
    let preview = app.workflow_state.saved_app_run_preview.as_ref().expect("preview should reset");
    assert_eq!(preview.status, crate::apps::workflow::state::WorkflowRunStatus::Running);
    assert!(preview.nodes.is_empty());
    assert!(preview.error.is_none());
    assert!(preview.text_output.is_empty());
}

#[test]
fn apply_loaded_syncs_top_tab_and_suggested_position_accounts_for_viewport() {
    let mut app = App::new().0;
    app.window_size = (1200.0, 900.0);
    let loaded = load_document_from_text(
        None,
        "app:\n  name: Loaded\nworkflow:\n  graph:\n    nodes: []\n    edges: []\n".to_string(),
    )
    .expect("workflow should load");

    apply_loaded(&mut app, loaded);
    assert_eq!(app.screen, crate::app::Screen::WorkflowTool);
    assert_eq!(app.active_tab_id.as_deref(), Some(crate::apps::workflow::WORKFLOW_TOOL_TAB_ID));
    assert_eq!(app.workflow_state.title(), "Loaded");

    app.workflow_state.pan = Vector::new(20.0, 40.0);
    app.workflow_state.zoom = 2.0;
    app.workflow_state.document.nodes =
        vec![create_node_from_type("answer", "a".to_string(), Point::ORIGIN, 1.0).unwrap()];
    let position = suggested_new_node_position(&app);
    assert_eq!(position, Point::new(330.0, 233.0));
}

#[test]
fn helper_functions_cover_save_export_and_record_mapping() {
    let mut empty_app = App::new().0;
    let _ = load_saved_apps_task();
    let _ = open_saved_app_task("uuid".to_string());
    let _ = delete_saved_app_task("uuid".to_string());
    let _ = open_file();
    let _ = save_active_app(&mut empty_app, false);

    let mut app = app_with_workflow();
    assert_eq!(suggested_export_file_name(" Sales/Bot ", "svg"), "Sales_Bot.svg");
    assert_eq!(suggested_export_file_name("!!!", "png"), "workflow_app.png");

    let entry = app.workflow_state.active_entry_snapshot().expect("active entry");
    let _ = save_entry(entry.clone(), false);
    let _ = save_entry(entry, true);

    let _ = export_svg(&mut empty_app);
    let _ = export_svg(&mut app);
    let _ = export_png(&mut app);
    let _ = export_jpeg(&mut app);
    export_finished(&mut app, Ok(()));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已导出工作流图片"));
    export_finished(&mut app, Err("bad export".to_string().into()));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("bad export"));

}

#[test]
fn suggested_node_position_clamps_zero_zoom_and_offsets_by_node_count() {
    let mut app = App::new().0;
    app.window_size = (100.0, 80.0);
    app.workflow_state.zoom = 0.0;
    app.workflow_state.pan = Vector::new(0.0, 0.0);

    let position = suggested_new_node_position(&app);
    assert!(position.x.is_finite());
    assert!(position.y.is_finite());
    assert_eq!(position, Point::new(520_000.0, 400_000.0));

    app.workflow_state.zoom = 1.0;
    app.workflow_state.document.nodes = (0..7)
        .map(|index| {
            create_node_from_type("answer", format!("node-{index}"), Point::ORIGIN, index as f32)
                .expect("answer node should be available")
        })
        .collect();
    assert_eq!(suggested_new_node_position(&app), Point::new(80.0, 68.0));
}
