//! 工作流应用编辑器视图测试模块，覆盖创建、编辑和 UUID 展示分支。

use super::*;
use crate::apps::workflow::model::{WorkflowAppMeta, WorkflowDocument};
use crate::apps::workflow::state::{
    WorkflowAppEditorDraft, WorkflowAppEditorMode, WorkflowAppEntry, WorkflowHistorySnapshot,
};
use iced::Vector;
use serde_yaml::Value;

#[test]
fn app_editor_modal_renders_empty_placeholder_when_editor_is_closed() {
    let state = WorkflowState::default();

    let _element = build_app_editor_modal(&state);
}

#[test]
fn app_editor_modal_renders_create_form() {
    let state = WorkflowState {
        app_editor: Some(editor_draft(WorkflowAppEditorMode::Create)),
        ..WorkflowState::default()
    };

    let _element = build_app_editor_modal(&state);
}

#[test]
fn app_editor_modal_renders_edit_form_without_active_organize_action() {
    let state = workflow_state_with_editor(
        WorkflowAppEditorMode::Edit("app_1".to_string()),
        Some("uuid-1"),
        None,
        None,
    );

    let _element = build_app_editor_modal(&state);
}

#[test]
fn app_editor_modal_renders_active_edit_form_with_organize_action() {
    let state = workflow_state_with_editor(
        WorkflowAppEditorMode::Edit("app_1".to_string()),
        Some("uuid-1"),
        Some("app_1"),
        None,
    );

    let _element = build_app_editor_modal(&state);
}

#[test]
fn app_editor_modal_renders_copied_uuid_state() {
    let state = workflow_state_with_editor(
        WorkflowAppEditorMode::Edit("app_1".to_string()),
        Some("uuid-1"),
        Some("app_1"),
        Some("uuid-1"),
    );

    let _element = build_app_editor_modal(&state);
}

#[test]
fn app_editor_modal_renders_uuid_as_not_copied_when_copy_marker_differs() {
    let state = workflow_state_with_editor(
        WorkflowAppEditorMode::Edit("app_1".to_string()),
        Some("uuid-1"),
        Some("app_1"),
        Some("uuid-2"),
    );

    let _element = build_app_editor_modal(&state);
}

#[test]
fn app_editor_modal_renders_inactive_app_with_matching_copy_marker() {
    let state = workflow_state_with_editor(
        WorkflowAppEditorMode::Edit("app_1".to_string()),
        Some("uuid-1"),
        Some("another_app"),
        Some("uuid-1"),
    );

    let element = build_app_editor_modal(&state);

    assert_eq!(element.as_widget().size(), iced::Size::new(Length::Fill, Length::Fill));
    assert_eq!(element.as_widget().children().len(), 3);
}

#[test]
fn app_editor_modal_renders_empty_fields_zero_limit_and_disabled_icon_replacement() {
    let mut draft = editor_draft(WorkflowAppEditorMode::Create);
    draft.name.clear();
    draft.description.clear();
    draft.icon.clear();
    draft.use_icon_as_answer_icon = false;
    draft.max_active_requests_input = "0".to_string();
    let state = WorkflowState { app_editor: Some(draft), ..WorkflowState::default() };

    let _element = build_app_editor_modal(&state);
}

#[test]
fn app_uuid_field_renders_missing_uuid_state() {
    let state = workflow_state_with_editor(
        WorkflowAppEditorMode::Edit("app_1".to_string()),
        None,
        Some("app_1"),
        None,
    );

    let _element = build_app_uuid_field(&state, "app_1");
}

#[test]
fn app_uuid_field_renders_missing_target_state() {
    let state = workflow_state_with_editor(
        WorkflowAppEditorMode::Edit("app_1".to_string()),
        Some("uuid-1"),
        Some("app_1"),
        None,
    );

    let _element = build_app_uuid_field(&state, "missing_app");
}

#[test]
fn app_organize_field_renders_action_button() {
    let _element = build_app_organize_field();
}

fn workflow_state_with_editor(
    mode: WorkflowAppEditorMode,
    local_uuid: Option<&str>,
    active_app_id: Option<&str>,
    copied_uuid: Option<&str>,
) -> WorkflowState {
    let document = WorkflowDocument::default();
    let meta = WorkflowAppMeta {
        name: "客服分流工作流".to_string(),
        description: "按问题类型分配客服。".to_string(),
        icon: "K".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests: 8,
        ..WorkflowAppMeta::default()
    };
    let saved_snapshot = WorkflowHistorySnapshot {
        meta: meta.clone(),
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    };

    WorkflowState {
        apps: vec![WorkflowAppEntry {
            id: "app_1".to_string(),
            local_uuid: local_uuid.map(str::to_string),
            meta,
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
            saved_snapshot: saved_snapshot.clone(),
        }],
        active_app_id: active_app_id.map(str::to_string),
        copied_saved_app_uuid: copied_uuid.map(str::to_string),
        app_editor: Some(editor_draft(mode)),
        document,
        saved_snapshot: Some(saved_snapshot),
        ..WorkflowState::default()
    }
}

fn editor_draft(mode: WorkflowAppEditorMode) -> WorkflowAppEditorDraft {
    WorkflowAppEditorDraft {
        mode,
        name: "客服分流工作流".to_string(),
        description: "按问题类型分配客服。".to_string(),
        icon: "K".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests_input: "8".to_string(),
    }
}
