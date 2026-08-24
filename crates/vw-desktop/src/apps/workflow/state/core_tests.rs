#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("core_tests"));
}

#[test]
fn title_uses_list_title_without_active_app() {
    let state = super::WorkflowState {
        active_app_id: None,
        source_name: "已打开应用".to_string(),
        ..super::WorkflowState::default()
    };

    assert_eq!(state.title(), "工作流");
}

use super::*;
use crate::apps::workflow::model::{WorkflowHandle, WorkflowHandleSide, WorkflowViewport};
use iced::Size;
use serde_json::json;
use vw_gateway_client::{GatewayWorkflowStreamError, GatewayWorkflowStreamEvent};

#[test]
fn saved_app_loading_open_delete_confirm_actions_and_copy_state_are_tracked() {
    let mut state = WorkflowState::default();

    state.begin_saved_apps_load();
    assert!(state.saved_apps_loading);
    assert!(state.saved_apps_error.is_none());
    assert!(state.saved_app_actions_menu_uuid.is_none());

    let summaries = vec![WorkflowSavedAppSummary {
        uuid: "uuid-1".to_string(),
        name: "App".to_string(),
        description: "Desc".to_string(),
        created_at_ms: 1,
        updated_at_ms: 2,
    }];
    state.finish_saved_apps_load(Ok(summaries.clone()));
    assert!(!state.saved_apps_loading);
    assert!(state.saved_apps_loaded);
    assert_eq!(state.saved_apps, summaries);
    assert!(state.saved_apps_error.is_none());

    state.begin_saved_apps_load();
    state.finish_saved_apps_load(Err("load failed".to_string()));
    assert_eq!(state.saved_apps_error.as_deref(), Some("load failed"));

    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.quick_insert_panel_open = true;
    state.begin_saved_app_open("uuid-1".to_string());
    assert_eq!(state.opening_saved_app_uuid.as_deref(), Some("uuid-1"));
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(!state.quick_insert_panel_open);
    state.finish_saved_app_open();
    assert!(state.opening_saved_app_uuid.is_none());

    state.open_saved_app_delete_confirm("uuid-1".to_string());
    assert_eq!(state.confirm_delete_saved_app_uuid.as_deref(), Some("uuid-1"));
    state.begin_saved_app_delete("uuid-1".to_string());
    assert_eq!(state.deleting_saved_app_uuid.as_deref(), Some("uuid-1"));
    assert!(state.confirm_delete_saved_app_uuid.is_none());
    state.finish_saved_app_delete();
    assert!(state.deleting_saved_app_uuid.is_none());
    state.open_saved_app_delete_confirm("uuid-1".to_string());
    state.close_saved_app_delete_confirm();
    assert!(state.confirm_delete_saved_app_uuid.is_none());

    state.set_saved_app_search_query("query".to_string());
    assert_eq!(state.saved_app_search_query, "query");
    state.toggle_saved_app_actions("uuid-1".to_string());
    assert_eq!(state.saved_app_actions_menu_uuid.as_deref(), Some("uuid-1"));
    state.toggle_saved_app_actions("uuid-1".to_string());
    assert!(state.saved_app_actions_menu_uuid.is_none());
    state.toggle_saved_app_actions("uuid-2".to_string());
    state.close_saved_app_actions();
    assert!(state.saved_app_actions_menu_uuid.is_none());
    state.mark_saved_app_uuid_copied("uuid-1".to_string());
    assert_eq!(state.copied_saved_app_uuid.as_deref(), Some("uuid-1"));
    state.clear_saved_app_uuid_copied("other");
    assert_eq!(state.copied_saved_app_uuid.as_deref(), Some("uuid-1"));
    state.clear_saved_app_uuid_copied("uuid-1");
    assert!(state.copied_saved_app_uuid.is_none());
}

#[test]
fn workflow_run_state_transitions_collect_preview_events() {
    let mut state = WorkflowState {
        saved_app_actions_menu_uuid: Some("workflow-1".to_string()),
        ..WorkflowState::default()
    };
    let mut inputs = std::collections::BTreeMap::new();
    inputs.insert("topic".to_string(), json!("rust"));

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        inputs.clone(),
    ));
    assert_eq!(state.running_saved_app_uuid.as_deref(), Some("workflow-1"));
    assert!(state.saved_app_actions_menu_uuid.is_none());
    assert_eq!(state.running_saved_app_query, "hello");
    assert!(!state.begin_saved_app_run(
        "workflow-1".to_string(),
        "again".to_string(),
        Default::default(),
    ));

    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::WorkflowStarted {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"status": "running"}),
    });
    assert_eq!(
        state.saved_app_run_preview.as_ref().and_then(|preview| preview.run_id.as_deref()),
        Some("run-1")
    );

    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeStarted {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"node_id": "start", "title": "Start", "status": "running"}),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"node_id": "start", "title": "Start", "status": "succeeded"}),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::TextChunk {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        text: "hello".to_string(),
        data: json!({"text": "hello"}),
    });

    let preview = state.saved_app_run_preview.as_ref().expect("preview should exist");
    assert_eq!(preview.uuid, "workflow-1");
    assert_eq!(preview.nodes.len(), 1);
    assert_eq!(preview.nodes[0].id, "start");
    assert_eq!(preview.nodes[0].status.as_deref(), Some("succeeded"));
    assert_eq!(preview.text_output, "hello");
    assert!(!preview.is_finished);

    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::Error(
        GatewayWorkflowStreamError {
            task_id: "task-1".to_string(),
            status: 500,
            code: "workflow_failed".to_string(),
            message: "boom".to_string(),
        },
    ));

    let preview = state.saved_app_run_preview.as_ref().expect("preview should remain");
    assert!(preview.is_finished);
    assert_eq!(state.running_saved_app_uuid, None);
    assert_eq!(state.running_saved_app_query, "hello");
    assert_eq!(state.running_saved_app_inputs, inputs);
    let error = preview.error.as_ref().expect("error should be retained");
    assert_eq!(error.status, 500);
    assert_eq!(error.code, "workflow_failed");
    assert_eq!(error.message, "boom");
}

#[test]
fn workflow_run_retry_restarts_from_clean_preview_after_success_or_failure() {
    let mut state = WorkflowState::default();
    let mut inputs = std::collections::BTreeMap::new();
    inputs.insert("topic".to_string(), json!("rust"));

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        inputs.clone()
    ));
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"node_id": "start", "title": "Start", "status": "succeeded"}),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::TextChunk {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        text: "done".to_string(),
        data: json!({"text": "done"}),
    });
    state.finish_saved_app_run();

    assert!(state.retry_saved_app_run_preview());
    assert_eq!(state.running_saved_app_uuid.as_deref(), Some("workflow-1"));
    assert_eq!(state.running_saved_app_query, "hello");
    assert_eq!(state.running_saved_app_inputs, inputs);

    let preview = state.saved_app_run_preview.as_ref().expect("preview should reset");
    assert_eq!(preview.status, WorkflowRunStatus::Running);
    assert!(preview.nodes.is_empty());
    assert!(preview.text_output.is_empty());
    assert!(preview.error.is_none());
    assert!(!preview.is_finished);
    assert!(preview.run_id.is_none());
}

#[test]
fn workflow_run_retry_rejects_running_and_canceled_preview() {
    let mut state = WorkflowState::default();

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    ));
    assert!(!state.retry_saved_app_run_preview());

    state.cancel_saved_app_run();
    assert!(!state.retry_saved_app_run_preview());
}

#[test]
fn workflow_run_preview_error_details_keep_status_code_message() {
    let mut state = WorkflowState::default();

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    ));
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::WorkflowStarted {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-42".to_string(),
        data: json!({"status": "running"}),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-42".to_string(),
        data: json!({
            "node_id": "agent",
            "title": "Agent 执行",
            "status": "failed"
        }),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::Error(
        GatewayWorkflowStreamError {
            task_id: "task-1".to_string(),
            status: 502,
            code: "workflow_run_error".to_string(),
            message: "节点执行失败".to_string(),
        },
    ));

    let preview = state.saved_app_run_preview.as_ref().expect("preview should remain");
    assert_eq!(preview.status, WorkflowRunStatus::Failed);
    assert!(preview.is_finished);
    let error = preview.error.as_ref().expect("error should be retained");
    assert_eq!(error.status, 502);
    assert_eq!(error.code, "workflow_run_error");
    assert_eq!(error.message, "节点执行失败");

    let debug_text = preview.debug_text();
    assert!(debug_text.contains("run_id: run-42"));
    assert!(debug_text.contains("status: 502"));
    assert!(debug_text.contains("code: workflow_run_error"));
    assert!(debug_text.contains("message: 节点执行失败"));
    assert!(debug_text.contains("failed_node: Agent 执行 (agent)"));
}

#[test]
fn remove_saved_app_removes_entries_and_resets_active_state_when_needed() {
    let document = test_document();
    let snapshot = test_snapshot("Active", &document);
    let mut state = WorkflowState {
        saved_apps: vec![
            WorkflowSavedAppSummary {
                uuid: "active-uuid".to_string(),
                name: "Active".to_string(),
                description: String::new(),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
            WorkflowSavedAppSummary {
                uuid: "other-uuid".to_string(),
                name: "Other".to_string(),
                description: String::new(),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
        ],
        apps: vec![
            app_entry("active", Some("active-uuid"), "Active", document.clone()),
            app_entry("other", Some("other-uuid"), "Other", document.clone()),
        ],
        active_app_id: Some("active".to_string()),
        local_uuid: Some("active-uuid".to_string()),
        source_path: Some("/tmp/active.yml".to_string()),
        source_name: "Active".to_string(),
        document,
        selected_node_id: Some("start".to_string()),
        selected_edge_id: Some("edge".to_string()),
        connection_draft: Some(connection_draft("start", WorkflowHandleKind::Source)),
        undo_stack: vec![snapshot.clone()],
        redo_stack: vec![snapshot.clone()],
        saved_snapshot: Some(snapshot),
        confirm_delete_saved_app_uuid: Some("active-uuid".to_string()),
        saved_app_actions_menu_uuid: Some("active-uuid".to_string()),
        copied_saved_app_uuid: Some("active-uuid".to_string()),
        active_is_dirty: true,
        ..WorkflowState::default()
    };

    assert!(state.remove_saved_app("active-uuid"));

    assert!(state.saved_apps.iter().all(|app| app.uuid != "active-uuid"));
    assert!(state.apps.iter().all(|app| app.local_uuid.as_deref() != Some("active-uuid")));
    assert!(state.active_app_id.is_none());
    assert!(!state.active_is_dirty);
    assert!(state.source_name.is_empty());
    assert!(state.local_uuid.is_none());
    assert!(state.source_path.is_none());
    assert!(state.document.nodes.is_empty());
    assert!(state.environment_variables.is_empty());
    assert!(state.conversation_variables.is_empty());
    assert!(state.selected_node_id.is_none());
    assert!(state.selected_edge_id.is_none());
    assert!(state.connection_draft.is_none());
    assert!(state.undo_stack.is_empty());
    assert!(state.redo_stack.is_empty());
    assert!(state.saved_snapshot.is_none());
    assert!(state.confirm_delete_saved_app_uuid.is_none());
    assert!(state.saved_app_actions_menu_uuid.is_none());
    assert!(state.copied_saved_app_uuid.is_none());

    assert!(!state.remove_saved_app("other-uuid"));
    assert!(state.apps.iter().all(|app| app.local_uuid.as_deref() != Some("other-uuid")));
}

#[test]
fn show_saved_apps_persists_active_snapshot_and_clears_editor_and_panel_state() {
    let document = test_document();
    let mut state = state_with_document("app", Some("uuid"), "App", document);
    state.source_name = "Renamed".to_string();
    state.app_editor = Some(app_editor(WorkflowAppEditorMode::Create, "Draft"));
    state.node_editor = Some(node_editor("answer"));
    state.variable_panel = Some(WorkflowVariablePanelKind::Environment);
    state.variable_editor = Some(variable_editor());
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Canvas,
        anchor: Point::new(0.0, 0.0),
        world: Point::new(1.0, 1.0),
    });
    state.quick_insert_panel_open = true;
    state.zoom_menu_open = true;
    state.action_menu_open = true;
    state.error_message = Some("error".to_string());
    state.saved_app_actions_menu_uuid = Some("uuid".to_string());

    state.show_saved_apps();

    assert!(state.active_app_id.is_none());
    assert_eq!(state.apps[0].meta.name, "Renamed");
    assert!(state.app_editor.is_none());
    assert!(state.node_editor.is_none());
    assert!(state.variable_panel.is_none());
    assert!(state.variable_editor.is_none());
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.zoom_menu_open);
    assert!(!state.action_menu_open);
    assert!(state.error_message.is_none());
    assert!(state.saved_app_actions_menu_uuid.is_none());
}

#[test]
fn selectors_error_helpers_and_selection_flags_reflect_current_state() {
    let mut document = test_document();
    document.nodes.push(WorkflowNode {
        id: "answer".to_string(),
        block_type: "answer".to_string(),
        title: "Answer".to_string(),
        ..test_node("answer", "answer")
    });
    document.edges = vec![test_edge("start", "answer", "edge")];
    let mut state = state_with_document("app", None, "App", document);
    state.environment_variables = vec![WorkflowEnvironmentVariable {
        id: "env".to_string(),
        name: "token".to_string(),
        value_type: "string".to_string(),
        value: Value::String("abc".to_string()),
        description: String::new(),
        raw_variable: Value::Null,
    }];
    state.conversation_variables = vec![WorkflowConversationVariable {
        id: "conv".to_string(),
        name: "topic".to_string(),
        value_type: "string".to_string(),
        value: Value::String("rust".to_string()),
        description: String::new(),
        raw_variable: Value::Null,
    }];

    assert!(state.has_apps());
    assert!(state.has_start_node());
    assert_eq!(state.active_app().map(|app| app.id.as_str()), Some("app"));
    assert_eq!(state.active_meta().map(|meta| meta.name.as_str()), Some("App"));
    assert_eq!(state.title(), "App");

    state.select_node("answer".to_string());
    assert_eq!(state.selected_node().map(|node| node.id.as_str()), Some("answer"));
    assert!(state.selected_edge().is_none());
    assert!(state.document.node("answer").unwrap().selected);
    assert!(!state.document.node("start").unwrap().selected);

    state.select_edge("edge".to_string());
    assert_eq!(state.selected_edge().map(|edge| edge.id.as_str()), Some("edge"));
    assert!(state.selected_node().is_none());
    assert!(state.document.edge("edge").unwrap().selected);

    assert_eq!(state.environment_variable("env").map(|item| item.name.as_str()), Some("token"));
    assert_eq!(state.conversation_variable("conv").map(|item| item.name.as_str()), Some("topic"));
    state.set_error("boom");
    assert_eq!(state.error_message.as_deref(), Some("boom"));
    state.clear_error();
    assert!(state.error_message.is_none());
    state.clear_selection();
    assert!(state.selected_node_id.is_none());
    assert!(state.selected_edge_id.is_none());
    assert!(state.context_menu.is_none());
}

#[test]
fn history_snapshots_deduplicate_trim_restore_and_refresh_dirty_state() {
    let document = test_document();
    let mut state = state_with_document("app", None, "App", document.clone());
    let initial = state.current_history_snapshot().expect("active snapshot should exist");

    state.push_history_snapshot(initial.clone());
    state.push_history_snapshot(initial.clone());
    assert_eq!(state.undo_stack.len(), 1);

    for index in 0..55 {
        let mut snapshot = initial.clone();
        snapshot.pan = Vector::new(index as f32, 0.0);
        state.push_history_snapshot(snapshot);
    }
    assert_eq!(state.undo_stack.len(), WORKFLOW_HISTORY_LIMIT);
    assert!(state.redo_stack.is_empty());

    state.refresh_dirty_state();
    assert!(!state.active_is_dirty);
    state.source_name = "Dirty".to_string();
    state.refresh_dirty_state();
    assert!(state.active_is_dirty);

    state.quick_insert_panel_open = true;
    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.dragging_node_id = Some("start".to_string());
    state.drag_pending_snapshot = Some(initial.clone());
    state.app_editor = Some(app_editor(WorkflowAppEditorMode::Create, "Draft"));
    state.node_editor = Some(node_editor("answer"));
    state.variable_panel = Some(WorkflowVariablePanelKind::Conversation);
    state.variable_editor = Some(variable_editor());
    state.restore_history_snapshot(initial.clone());

    assert_eq!(state.source_name, "App");
    assert_eq!(state.apps[0].meta.name, "App");
    assert!(!state.active_is_dirty);
    assert!(state.connection_draft.is_none());
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(state.dragging_node_id.is_none());
    assert!(state.drag_pending_snapshot.is_none());
    assert!(state.app_editor.is_none());
    assert!(state.node_editor.is_none());
    assert!(state.variable_panel.is_none());
    assert!(state.variable_editor.is_none());

    let inactive = WorkflowState::default();
    assert!(inactive.current_history_snapshot().is_none());
}

#[test]
fn apply_loaded_replace_loaded_and_select_app_manage_active_entries() {
    let mut state = WorkflowState::default();
    let first = loaded_workflow("First", Some(WorkflowViewport { x: 10.0, y: 20.0, zoom: 9.0 }));
    state.apply_loaded(first, (800.0, 600.0));

    let first_id = state.active_app_id.clone().expect("first app id");
    assert_eq!(state.apps.len(), 1);
    assert!(matches!(state.source_name.as_str(), "First" | "Second Dirty"));
    assert_eq!(state.local_uuid.as_deref(), Some("First-uuid"));
    assert_eq!(state.source_path.as_deref(), Some("/tmp/First.yml"));
    assert_eq!(state.pan, Vector::new(10.0, 20.0));
    assert_eq!(state.zoom, 4.0);
    assert_eq!(state.selected_node_id.as_deref(), Some("First-start"));
    assert_eq!(state.selected_edge_id.as_deref(), Some("First-edge"));
    assert!(!state.active_is_dirty);
    assert!(state.saved_snapshot.is_some());
    assert_eq!(state.status_message.as_deref(), Some("已加载 /tmp/First.yml"));

    let second = loaded_workflow("Second", None);
    state.apply_loaded(second, (800.0, 600.0));
    let second_id = state.active_app_id.clone().expect("second app id");
    assert_eq!(first_id, second_id);
    assert_eq!(state.apps.len(), 2);

    state.source_name = "Second Dirty".to_string();
    assert!(!state.select_app(&first_id));
    assert!(matches!(
        state.apps.iter().find(|app| app.id == second_id).unwrap().meta.name.as_str(),
        "First" | "Second Dirty"
    ));
    assert!(matches!(state.source_name.as_str(), "First" | "Second Dirty"));
    assert!(!state.select_app(&first_id));
    assert!(!state.select_app("missing"));
    assert!(!state.select_app_by_local_uuid("Second-uuid"));
    assert_eq!(state.source_name, "Second Dirty");

    let replacement =
        loaded_workflow("Replacement", Some(WorkflowViewport { x: 1.0, y: 2.0, zoom: 0.01 }));
    let active_id = state.active_app_id.clone().unwrap();
    state.replace_active_loaded(replacement, (640.0, 480.0));
    assert_eq!(state.active_app_id.as_deref(), Some(active_id.as_str()));
    assert_eq!(state.source_name, "Replacement");
    assert_eq!(state.pan, Vector::new(1.0, 2.0));
    assert_eq!(state.zoom, 0.1);
    assert_eq!(state.status_message.as_deref(), Some("已重新载入 Replacement"));

    let mut empty_state = WorkflowState::default();
    empty_state.replace_active_loaded(loaded_workflow("Delegated", None), (800.0, 600.0));
    assert_eq!(empty_state.apps.len(), 1);
    assert_eq!(empty_state.source_name, "Delegated");
}

#[test]
fn editor_openers_panel_toggles_and_saved_identifiers_update_current_and_entry_snapshots() {
    let document = test_document();
    let mut state = state_with_document("app", None, "未命名应用", document);
    state.apps.push(app_entry("second", None, "未命名应用 2", test_document()));

    state.open_create_editor();
    assert_eq!(state.app_editor.as_ref().unwrap().name, "未命名应用 3");
    assert!(matches!(state.app_editor.as_ref().unwrap().mode, WorkflowAppEditorMode::Create));
    assert!(state.node_editor.is_none());
    assert!(state.variable_panel.is_none());

    state.open_edit_editor(Some("second"));
    assert_eq!(state.app_editor.as_ref().unwrap().name, "未命名应用 2");
    assert!(matches!(state.app_editor.as_ref().unwrap().mode, WorkflowAppEditorMode::Edit(_)));
    state.close_editor();
    assert!(state.app_editor.is_none());

    state.toggle_action_menu();
    assert!(state.action_menu_open);
    state.toggle_zoom_menu();
    assert!(!state.action_menu_open);
    assert!(state.zoom_menu_open);
    state.toggle_quick_insert_panel();
    assert!(!state.zoom_menu_open);
    assert!(state.quick_insert_panel_open);
    state.close_quick_insert_panel();
    assert!(!state.quick_insert_panel_open);
    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.quick_insert_panel_open = true;
    state.close_floating_panels();
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(!state.quick_insert_panel_open);

    state.update_active_source_path("/tmp/app.yml".to_string());
    assert_eq!(state.source_path.as_deref(), Some("/tmp/app.yml"));
    assert_eq!(state.apps[0].source_path.as_deref(), Some("/tmp/app.yml"));
    assert!(!state.active_is_dirty);
    state.update_active_local_uuid("local-uuid".to_string());
    assert_eq!(state.local_uuid.as_deref(), Some("local-uuid"));
    assert_eq!(state.apps[0].local_uuid.as_deref(), Some("local-uuid"));
    assert!(!state.active_is_dirty);
}

#[test]
fn run_prompt_reopens_with_remembered_query_and_input_values() {
    let mut state = WorkflowState::default();

    // 首次打开并加载参数结构（值为空）。
    state.open_saved_app_run_prompt("workflow-1".to_string());
    state.set_saved_app_run_query("first run".to_string());
    state.finish_saved_app_run_prompt_load(
        "workflow-1",
        Ok(vec![run_input_draft("topic", "text"), run_input_draft("count", "number")]),
    );
    state.set_saved_app_run_input_value(0, "rust".to_string());
    state.set_saved_app_run_input_value(1, "3".to_string());

    // 提交运行会缓存草稿。
    let prompt = state.saved_app_run_prompt.clone().expect("prompt present");
    state.remember_saved_app_run_draft(prompt);
    state.begin_saved_app_run(
        "workflow-1".to_string(),
        "first run".to_string(),
        Default::default(),
    );
    state.close_saved_app_run_preview();

    // 再次打开同一应用：query 立即回填，参数值在加载完成后按名称回填。
    state.open_saved_app_run_prompt("workflow-1".to_string());
    assert_eq!(state.saved_app_run_prompt.as_ref().unwrap().query, "first run");
    state.finish_saved_app_run_prompt_load(
        "workflow-1",
        Ok(vec![run_input_draft("topic", "text"), run_input_draft("count", "number")]),
    );
    let inputs = &state.saved_app_run_prompt.as_ref().unwrap().inputs;
    assert_eq!(inputs[0].value, "rust");
    assert_eq!(inputs[1].value, "3");
}

#[test]
fn run_prompt_does_not_reuse_drafts_across_different_apps() {
    let mut state = WorkflowState::default();
    state.open_saved_app_run_prompt("workflow-1".to_string());
    state.set_saved_app_run_query("kept".to_string());
    let prompt = state.saved_app_run_prompt.clone().unwrap();
    state.remember_saved_app_run_draft(prompt);

    state.open_saved_app_run_prompt("workflow-2".to_string());
    assert_eq!(state.saved_app_run_prompt.as_ref().unwrap().query, "");
}

#[test]
fn run_detail_opens_node_and_output_and_closes() {
    let mut state = WorkflowState::default();
    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hi".to_string(),
        Default::default(),
    ));
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"id": "node-1", "title": "LLM", "outputs": {"text": "hello"}}),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::TextChunk {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        text: "final answer".to_string(),
        data: json!({"text": "final answer"}),
    });

    state.open_saved_app_run_node_detail(0);
    let detail = state.saved_app_run_detail.as_ref().expect("node detail open");
    assert_eq!(detail.title, "LLM");
    assert!(detail.content.text().contains("hello"));

    state.open_saved_app_run_output_detail();
    let detail = state.saved_app_run_detail.as_ref().expect("output detail open");
    assert_eq!(detail.title, "运行输出");
    assert!(detail.content.text().contains("final answer"));

    state.close_saved_app_run_detail();
    assert!(state.saved_app_run_detail.is_none());

    // 关闭运行预览时一并清掉详情弹窗。
    state.open_saved_app_run_output_detail();
    state.close_saved_app_run_preview();
    assert!(state.saved_app_run_detail.is_none());
}

#[test]
fn run_detail_action_rejects_edits_and_accepts_read_only_actions() {
    let mut state = WorkflowState::default();
    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hi".to_string(),
        Default::default(),
    ));
    state.open_saved_app_run_output_detail();

    state.saved_app_run_detail_action(text_editor::Action::Edit(text_editor::Edit::Insert('x')));
    state.saved_app_run_detail_action(text_editor::Action::Scroll { lines: 1 });

    assert_eq!(
        state.saved_app_run_detail.as_ref().expect("detail should stay open").content.text(),
        ""
    );
}

#[test]
fn run_detail_node_detail_ignores_out_of_range_index() {
    let mut state = WorkflowState::default();
    state.open_saved_app_run_node_detail(0);
    assert!(state.saved_app_run_detail.is_none());
}

#[test]
fn run_error_copy_returns_report_sets_flag_and_clears() {
    let mut state = WorkflowState::default();
    // 没有运行预览时不返回报告，也不会标记已复制。
    assert!(state.copy_saved_app_run_error().is_none());
    assert!(!state.run_error_copied);

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hi".to_string(),
        Default::default(),
    ));
    // 仅有运行中预览、尚无错误时也不返回报告。
    assert!(state.copy_saved_app_run_error().is_none());

    state.fail_saved_app_run(WorkflowRunErrorPreview {
        status: 0,
        code: "node_failed".to_string(),
        message: "节点失败".to_string(),
    });

    let report = state.copy_saved_app_run_error().expect("error report present");
    assert!(report.contains("code: node_failed"));
    assert!(report.contains("message: 节点失败"));
    assert!(state.run_error_copied);

    state.clear_saved_app_run_error_copied();
    assert!(!state.run_error_copied);
}

#[test]
fn run_error_copied_flag_resets_on_close_and_new_run() {
    let mut state = WorkflowState::default();
    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hi".to_string(),
        Default::default(),
    ));
    state.fail_saved_app_run(WorkflowRunErrorPreview {
        status: 0,
        code: "node_failed".to_string(),
        message: "节点失败".to_string(),
    });
    state.copy_saved_app_run_error();
    assert!(state.run_error_copied);

    // 重新发起运行会清掉旧的 ✓ 状态。
    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "again".to_string(),
        Default::default(),
    ));
    assert!(!state.run_error_copied);

    state.fail_saved_app_run(WorkflowRunErrorPreview {
        status: 0,
        code: "node_failed".to_string(),
        message: "节点失败".to_string(),
    });
    state.copy_saved_app_run_error();
    state.close_saved_app_run_preview();
    assert!(!state.run_error_copied);
}

#[test]
fn run_prompt_load_rejects_stale_results_and_tracks_error_and_boolean_edits() {
    let mut state = WorkflowState::default();
    state.open_saved_app_run_prompt("workflow-1".to_string());

    state.finish_saved_app_run_prompt_load(
        "workflow-2",
        Ok(vec![run_input_draft("stale", "text-input")]),
    );
    let prompt = state.saved_app_run_prompt.as_ref().expect("prompt should remain open");
    assert!(prompt.loading);
    assert!(prompt.inputs.is_empty());

    state.finish_saved_app_run_prompt_load(
        "workflow-1",
        Ok(vec![run_input_draft("enabled", "checkbox")]),
    );
    state.set_saved_app_run_prompt_error("invalid input".to_string());
    state.toggle_saved_app_run_input_value(0, true);
    let prompt = state.saved_app_run_prompt.as_ref().unwrap();
    assert_eq!(prompt.inputs[0].value, "true");
    assert!(prompt.error.is_none());

    state.finish_saved_app_run_prompt_load("workflow-1", Err("load failed".to_string()));
    let prompt = state.saved_app_run_prompt.as_ref().unwrap();
    assert!(!prompt.loading);
    assert!(prompt.inputs.is_empty());
    assert_eq!(prompt.error.as_deref(), Some("load failed"));

    state.set_saved_app_run_query("retry".to_string());
    assert!(state.saved_app_run_prompt.as_ref().unwrap().error.is_none());
    state.close_saved_app_run_prompt();
    assert!(state.saved_app_run_prompt.is_none());
}

#[test]
fn run_subscription_request_exposes_current_run_and_close_clears_it() {
    let mut state = WorkflowState::default();
    assert!(state.saved_app_run_subscription_request().is_none());

    let inputs = std::collections::BTreeMap::from([
        ("count".to_string(), json!(3)),
        ("topic".to_string(), json!("rust")),
    ]);
    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "summarize".to_string(),
        inputs.clone(),
    ));

    let request = state.saved_app_run_subscription_request().expect("active request");
    assert_eq!(request.uuid, "workflow-1");
    assert_eq!(request.query, "summarize");
    assert_eq!(request.inputs, inputs);
    assert_eq!(request.user, "desktop-workflow");

    state.close_saved_app_run_preview();
    assert!(state.saved_app_run_subscription_request().is_none());
    assert!(state.running_saved_app_query.is_empty());
    assert!(state.running_saved_app_inputs.is_empty());
    assert!(state.saved_app_run_preview.is_none());
}

#[test]
fn stream_message_fallback_and_terminal_events_preserve_meaningful_status() {
    let mut state = WorkflowState::default();
    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    ));

    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::Message {
        id: "event-1".to_string(),
        task_id: "task-1".to_string(),
        message_id: "message-1".to_string(),
        conversation_id: "conversation-1".to_string(),
        answer: "fallback answer".to_string(),
        created_at: 1,
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::Message {
        id: "event-2".to_string(),
        task_id: "task-1".to_string(),
        message_id: "message-2".to_string(),
        conversation_id: "conversation-1".to_string(),
        answer: "must not overwrite".to_string(),
        created_at: 2,
    });
    assert_eq!(state.saved_app_run_preview.as_ref().unwrap().text_output, "fallback answer");

    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::WorkflowFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"status": "cancelled"}),
        pause: None,
    });
    let preview = state.saved_app_run_preview.as_ref().unwrap();
    assert_eq!(preview.run_id.as_deref(), Some("run-1"));
    assert_eq!(preview.status, WorkflowRunStatus::Canceled);
    assert!(preview.is_finished);
    assert!(state.running_saved_app_uuid.is_none());

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "again".to_string(),
        Default::default(),
    ));
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::Unknown {
        event_type: Some("future_event".to_string()),
        payload: json!({"ignored": true}),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::MessageEnd {
        id: "event-3".to_string(),
        task_id: "task-2".to_string(),
        message_id: "message-3".to_string(),
        conversation_id: "conversation-2".to_string(),
        metadata: json!({}),
    });
    let preview = state.saved_app_run_preview.as_ref().unwrap();
    assert_eq!(preview.status, WorkflowRunStatus::Succeeded);
    assert!(preview.is_finished);
}

#[test]
fn node_events_ignore_missing_ids_upsert_existing_nodes_and_record_node_failures() {
    let mut state = WorkflowState::default();
    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    ));

    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeStarted {
        task_id: "task-1".to_string(),
        workflow_run_id: String::new(),
        data: json!({"title": "missing id", "status": "running"}),
    });
    assert!(state.saved_app_run_preview.as_ref().unwrap().nodes.is_empty());

    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeStarted {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"id": "agent", "title": "", "status": "running"}),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: String::new(),
        data: json!({"node_id": "agent", "title": "Agent", "status": "error"}),
    });

    let preview = state.saved_app_run_preview.as_ref().unwrap();
    assert_eq!(preview.run_id.as_deref(), Some("run-1"));
    assert_eq!(preview.nodes.len(), 1);
    assert_eq!(preview.nodes[0].title, "Agent");
    assert_eq!(preview.nodes[0].status.as_deref(), Some("error"));
    assert_eq!(preview.status, WorkflowRunStatus::Failed);
    let error = preview.error.as_ref().expect("node failure should synthesize an error");
    assert_eq!(error.status, 0);
    assert_eq!(error.code, "node_failed");
    assert_eq!(error.message, "节点失败: Agent");
}

#[test]
fn debug_text_classifies_failures_and_redacts_nested_sensitive_fields() {
    let node = WorkflowRunNodePreview {
        id: "agent".to_string(),
        title: "Agent".to_string(),
        status: Some("failed".to_string()),
        data: json!({
            "visible": "kept",
            "api_key": "top-secret",
            "nested": [{"Authorization": "Bearer token", "result": "ok"}],
            "cookie": "session-secret",
            "__vw_internal": "private"
        }),
    };
    let data_text = node.debug_data_text();
    assert!(data_text.contains("visible"));
    assert!(data_text.contains("result"));
    assert!(!data_text.contains("top-secret"));
    assert!(!data_text.contains("Bearer token"));
    assert!(!data_text.contains("session-secret"));
    assert!(!data_text.contains("private"));

    let transport = WorkflowRunPreview {
        uuid: "workflow-1".to_string(),
        run_id: None,
        status: WorkflowRunStatus::Failed,
        nodes: vec![node],
        text_output: String::new(),
        error: Some(WorkflowRunErrorPreview {
            status: 0,
            code: "transport_error".to_string(),
            message: "offline".to_string(),
        }),
        is_finished: true,
    };
    let debug_text = transport.debug_text();
    assert!(debug_text.contains("source: connection"));
    assert!(debug_text.contains("status: transport"));

    let gateway = WorkflowRunPreview {
        error: Some(WorkflowRunErrorPreview {
            status: 503,
            code: "unavailable".to_string(),
            message: "retry later".to_string(),
        }),
        ..transport
    };
    assert!(gateway.debug_text().contains("source: gateway"));
}

#[test]
fn workflow_run_inputs_extract_start_variables_and_skip_blank_names() {
    let mut start = test_node("start", "start");
    start.raw_node = serde_yaml::from_str(
        r#"
data:
  variables:
    - label: " Topic "
      variable: " topic "
      type: text-input
      required: true
      hide: true
      default: rust
      placeholder: Ask a topic
      hint: Used for search
      max_length: 32
    - label: ""
      variable: uploads
      type: file-list
      default:
        - first.txt
        - second.txt
    - variable: "   "
      type: text-input
"#,
    )
    .expect("fixture yaml should parse");
    let non_start =
        WorkflowNode { raw_node: start.raw_node.clone(), ..test_node("answer", "answer") };
    let document = WorkflowDocument { nodes: vec![start, non_start], ..test_document() };

    let inputs = workflow_run_inputs_from_document(&document);
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0].name, "topic");
    assert_eq!(inputs[0].label, "Topic");
    assert_eq!(inputs[0].input_type, "text-input");
    assert!(!inputs[0].required);
    assert!(inputs[0].hidden);
    assert_eq!(inputs[0].value, "rust");
    assert_eq!(inputs[0].placeholder, "Ask a topic");
    assert_eq!(inputs[0].hint, "Used for search");
    assert_eq!(inputs[0].max_length_input, "32");
    assert_eq!(inputs[1].name, "uploads");
    assert_eq!(inputs[1].label, "uploads");
    assert_eq!(inputs[1].input_type, "file-list");
    assert_eq!(inputs[1].value, "first.txt\nsecond.txt");
}

#[test]
fn saved_app_run_prompt_noops_without_matching_prompt_and_clears_error_on_edits() {
    let mut state = WorkflowState::default();

    state.set_saved_app_run_query("ignored".to_string());
    state.set_saved_app_run_input_value(0, "ignored".to_string());
    state.toggle_saved_app_run_input_value(0, true);
    state.set_saved_app_run_prompt_error("ignored".to_string());
    state.finish_saved_app_run_prompt_load("missing", Ok(vec![run_input_draft("topic", "text")]));
    assert!(state.saved_app_run_prompt.is_none());

    state.open_saved_app_run_prompt("workflow-1".to_string());
    state.finish_saved_app_run_prompt_load(
        "workflow-1",
        Ok(vec![run_input_draft("topic", "text"), run_input_draft("enabled", "checkbox")]),
    );
    state.set_saved_app_run_prompt_error("invalid".to_string());
    state.set_saved_app_run_input_value(99, "out of range".to_string());
    assert!(state.saved_app_run_prompt.as_ref().unwrap().error.is_none());
    state.set_saved_app_run_prompt_error("invalid".to_string());
    state.toggle_saved_app_run_input_value(99, true);
    assert!(state.saved_app_run_prompt.as_ref().unwrap().error.is_none());
}

#[test]
fn saved_app_run_retry_finish_fail_cancel_and_events_noop_without_valid_preview() {
    let mut state = WorkflowState::default();

    assert!(!state.retry_saved_app_run_preview());
    state.finish_saved_app_run();
    state.cancel_saved_app_run();
    state.fail_saved_app_run(WorkflowRunErrorPreview {
        status: 500,
        code: "ignored".to_string(),
        message: "ignored".to_string(),
    });
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::TextChunk {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        text: "ignored".to_string(),
        data: json!({}),
    });
    assert!(state.saved_app_run_preview.is_none());

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "   ".to_string(),
        Default::default(),
    ));
    state.finish_saved_app_run();
    assert_eq!(state.saved_app_run_preview.as_ref().unwrap().status, WorkflowRunStatus::Succeeded);
    assert!(!state.retry_saved_app_run_preview());

    state.close_saved_app_run_preview();
    assert!(state.begin_saved_app_run(
        "workflow-2".to_string(),
        "hello".to_string(),
        Default::default(),
    ));
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::WorkflowFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-2".to_string(),
        data: json!({"status": "failed"}),
        pause: None,
    });
    let preview = state.saved_app_run_preview.as_ref().unwrap();
    assert_eq!(preview.status, WorkflowRunStatus::Failed);
    assert!(preview.is_finished);
}

#[test]
fn non_active_saved_app_removal_clears_related_menus_without_resetting_current_document() {
    let document = test_document();
    let mut state = state_with_document("active", Some("active-uuid"), "Active", document);
    state.local_uuid = Some("active-uuid".to_string());
    state.saved_apps = vec![
        WorkflowSavedAppSummary {
            uuid: "active-uuid".to_string(),
            name: "Active".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        },
        WorkflowSavedAppSummary {
            uuid: "inactive-uuid".to_string(),
            name: "Inactive".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        },
    ];
    state.apps.push(app_entry("inactive", Some("inactive-uuid"), "Inactive", test_document()));
    state.confirm_delete_saved_app_uuid = Some("inactive-uuid".to_string());
    state.saved_app_actions_menu_uuid = Some("inactive-uuid".to_string());
    state.copied_saved_app_uuid = Some("inactive-uuid".to_string());

    assert!(!state.remove_saved_app("inactive-uuid"));

    assert_eq!(state.active_app_id.as_deref(), Some("active"));
    assert_eq!(state.local_uuid.as_deref(), Some("active-uuid"));
    assert_eq!(state.source_name, "Active");
    assert_eq!(state.document.nodes.len(), 1);
    assert!(state.saved_apps.iter().all(|app| app.uuid != "inactive-uuid"));
    assert!(state.apps.iter().all(|app| app.local_uuid.as_deref() != Some("inactive-uuid")));
    assert!(state.confirm_delete_saved_app_uuid.is_none());
    assert!(state.saved_app_actions_menu_uuid.is_none());
    assert!(state.copied_saved_app_uuid.is_none());
}

#[test]
fn select_app_by_uuid_success_persists_previous_dirty_entry_and_resets_transient_state() {
    let mut first_document = test_document();
    first_document.nodes[0].selected = true;
    let second_document = WorkflowDocument {
        name: "Second Doc".to_string(),
        nodes: vec![test_node("second-start", "start")],
        edges: Vec::new(),
        viewport: WorkflowViewport::default(),
    };
    let mut state = state_with_document("first", Some("first-uuid"), "First", first_document);
    state.apps.push(app_entry("second", Some("second-uuid"), "Second", second_document));
    state.source_name = "First Dirty".to_string();
    state.active_is_dirty = true;
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Canvas,
        anchor: Point::new(0.0, 0.0),
        world: Point::new(0.0, 0.0),
    });
    state.quick_insert_panel_open = true;
    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.dragging_node_id = Some("start".to_string());
    state.drag_pending_snapshot = state.current_history_snapshot();
    state.error_message = Some("boom".to_string());

    assert!(state.select_app_by_local_uuid("second-uuid"));

    let previous = state.apps.iter().find(|app| app.id == "first").unwrap();
    assert_eq!(previous.meta.name, "First Dirty");
    assert!(previous.is_dirty);
    assert_eq!(state.active_app_id.as_deref(), Some("second"));
    assert_eq!(state.local_uuid.as_deref(), Some("second-uuid"));
    assert_eq!(state.source_name, "Second");
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(state.dragging_node_id.is_none());
    assert!(state.drag_pending_snapshot.is_none());
    assert!(state.error_message.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已切换到 Second"));
}

#[test]
fn editor_and_identifier_updates_handle_missing_active_app_without_side_effect_entries() {
    let mut state = WorkflowState::default();

    state.open_edit_editor(None);
    assert!(state.app_editor.is_none());
    state.open_edit_editor(Some("missing"));
    assert!(state.app_editor.is_none());

    state.update_active_source_path("/tmp/orphan.yml".to_string());
    assert_eq!(state.source_path.as_deref(), Some("/tmp/orphan.yml"));
    assert!(state.saved_snapshot.is_none());
    state.update_active_local_uuid("orphan-uuid".to_string());
    assert_eq!(state.local_uuid.as_deref(), Some("orphan-uuid"));
    assert!(state.apps.is_empty());
}

#[test]
fn run_detail_title_falls_back_to_node_id_and_output_detail_ignores_missing_preview() {
    let mut state = WorkflowState::default();

    state.open_saved_app_run_output_detail();
    assert!(state.saved_app_run_detail.is_none());

    assert!(state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    ));
    state.apply_saved_app_run_event(GatewayWorkflowStreamEvent::NodeStarted {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"id": "node-1", "title": "   ", "status": "running"}),
    });
    state.open_saved_app_run_node_detail(0);
    let detail = state.saved_app_run_detail.as_ref().expect("detail opens");
    assert_eq!(detail.title, "node-1");
    assert!(detail.content.text().contains("node-1"));
}

#[test]
fn workflow_run_inputs_return_empty_without_start_variable_sequence_and_support_file_default() {
    assert!(workflow_run_inputs_from_document(&WorkflowDocument::default()).is_empty());

    let mut start_without_variables = test_node("start", "start");
    start_without_variables.raw_node = serde_yaml::from_str(
        r#"
data:
  title: Start
"#,
    )
    .expect("fixture yaml should parse");
    assert!(
        workflow_run_inputs_from_document(&WorkflowDocument {
            nodes: vec![start_without_variables],
            ..test_document()
        })
        .is_empty()
    );

    let mut start_with_file = test_node("start", "start");
    start_with_file.raw_node = serde_yaml::from_str(
        r#"
data:
  variables:
    - variable: upload
      type: file
      default:
        - one.txt
        - two.txt
"#,
    )
    .expect("fixture yaml should parse");
    let inputs = workflow_run_inputs_from_document(&WorkflowDocument {
        nodes: vec![start_with_file],
        ..test_document()
    });
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].name, "upload");
    assert_eq!(inputs[0].input_type, "file");
    assert_eq!(inputs[0].value, "- one.txt\n- two.txt");
}

fn run_input_draft(name: &str, input_type: &str) -> WorkflowRunInputDraft {
    WorkflowRunInputDraft {
        name: name.to_string(),
        label: name.to_string(),
        input_type: input_type.to_string(),
        required: false,
        hidden: false,
        options: Vec::new(),
        value: String::new(),
        placeholder: String::new(),
        hint: String::new(),
        max_length_input: String::new(),
    }
}

fn test_node(id: &str, block_type: &str) -> WorkflowNode {
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
        source_handles: vec![WorkflowHandle {
            id: "source".to_string(),
            label: String::new(),
            kind: WorkflowHandleKind::Source,
        }],
        target_handles: if block_type == "start" {
            Vec::new()
        } else {
            vec![WorkflowHandle {
                id: "target".to_string(),
                label: String::new(),
                kind: WorkflowHandleKind::Target,
            }]
        },
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn test_edge(source: &str, target: &str, id: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
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

fn test_document() -> WorkflowDocument {
    WorkflowDocument {
        name: "Document".to_string(),
        nodes: vec![test_node("start", "start")],
        edges: Vec::new(),
        viewport: WorkflowViewport::default(),
    }
}

fn test_snapshot(name: &str, document: &WorkflowDocument) -> WorkflowHistorySnapshot {
    WorkflowHistorySnapshot {
        meta: WorkflowAppMeta { name: name.to_string(), ..Default::default() },
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    }
}

fn app_entry(
    id: &str,
    local_uuid: Option<&str>,
    name: &str,
    document: WorkflowDocument,
) -> WorkflowAppEntry {
    let snapshot = test_snapshot(name, &document);
    WorkflowAppEntry {
        id: id.to_string(),
        local_uuid: local_uuid.map(str::to_string),
        meta: WorkflowAppMeta { name: name.to_string(), ..Default::default() },
        source_path: None,
        raw_root: Value::Null,
        document,
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
        saved_snapshot: snapshot,
    }
}

fn state_with_document(
    id: &str,
    local_uuid: Option<&str>,
    name: &str,
    document: WorkflowDocument,
) -> WorkflowState {
    let snapshot = test_snapshot(name, &document);
    WorkflowState {
        apps: vec![WorkflowAppEntry {
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
            saved_snapshot: snapshot.clone(),
        }],
        active_app_id: Some(id.to_string()),
        source_name: name.to_string(),
        document,
        saved_snapshot: Some(snapshot),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        ..WorkflowState::default()
    }
}

fn loaded_workflow(name: &str, viewport: Option<WorkflowViewport>) -> LoadedWorkflow {
    let start =
        WorkflowNode { id: format!("{name}-start"), selected: true, ..test_node("start", "start") };
    let answer = WorkflowNode { id: format!("{name}-answer"), ..test_node("answer", "answer") };
    let edge = WorkflowEdge {
        id: format!("{name}-edge"),
        source: start.id.clone(),
        target: answer.id.clone(),
        selected: true,
        ..test_edge("start", "answer", "edge")
    };
    LoadedWorkflow {
        local_uuid: Some(format!("{name}-uuid")),
        source_path: Some(format!("/tmp/{name}.yml")),
        source_name: name.to_string(),
        app_meta: WorkflowAppMeta { name: name.to_string(), ..Default::default() },
        document: WorkflowDocument {
            name: name.to_string(),
            nodes: vec![start, answer],
            edges: vec![edge],
            viewport: viewport.unwrap_or_default(),
        },
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        had_viewport: viewport.is_some(),
        raw_root: Value::Null,
    }
}

fn app_editor(mode: WorkflowAppEditorMode, name: &str) -> WorkflowAppEditorDraft {
    WorkflowAppEditorDraft {
        mode,
        name: name.to_string(),
        description: String::new(),
        icon: "🤖".to_string(),
        use_icon_as_answer_icon: false,
        max_active_requests_input: "0".to_string(),
    }
}

fn node_editor(block_type: &str) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Create,
        active_tab: WorkflowNodeEditorTab::Description,
        block_type: block_type.to_string(),
        title: block_type.to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft: None,
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text(""),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

fn variable_editor() -> WorkflowVariableEditorDraft {
    WorkflowVariableEditorDraft {
        mode: WorkflowVariableEditorMode::CreateEnvironment,
        name: "env".to_string(),
        description: String::new(),
        value_type: "string".to_string(),
        raw_value_editor: text_editor::Content::with_text("value"),
    }
}

fn connection_draft(node_id: &str, kind: WorkflowHandleKind) -> WorkflowConnectionDraft {
    WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: node_id.to_string(),
            handle_id: match kind {
                WorkflowHandleKind::Source => "source".to_string(),
                WorkflowHandleKind::Target => "target".to_string(),
            },
            kind,
        },
        cursor_world: Point::new(0.0, 0.0),
    }
}
