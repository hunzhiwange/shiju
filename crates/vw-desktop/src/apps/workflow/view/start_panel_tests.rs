use super::*;
use crate::apps::workflow::model::{WorkflowAppMeta, WorkflowDocument};
use crate::apps::workflow::state::{
    WorkflowAppEntry, WorkflowHistorySnapshot, WorkflowNodeEditorDraft, WorkflowNodeEditorMode,
    WorkflowNodeEditorTab, WorkflowNodeEditorValidation, WorkflowNodeValidationError,
    WorkflowNodeVisualDraft, WorkflowStartVariableDraft, WorkflowStartVariableEditorDraft,
    WorkflowStartVariableEditorMode, WorkflowState,
};
use iced::widget::text_editor;
use iced::{Point, Vector};
use serde_yaml::Value;

fn start_variable(variable: &str, label: &str, input_type: &str) -> WorkflowStartVariableDraft {
    WorkflowStartVariableDraft {
        raw_variable: Value::Null,
        label: label.to_string(),
        variable: variable.to_string(),
        input_type: input_type.to_string(),
        required: false,
        hidden: false,
        options: Vec::new(),
        allowed_file_types: Vec::new(),
        allowed_file_extensions: Vec::new(),
        allowed_file_extensions_input: String::new(),
        allowed_file_upload_methods: Vec::new(),
        default_value: String::new(),
        default_file_values: Vec::new(),
        placeholder: String::new(),
        hint: String::new(),
        max_length_input: String::new(),
    }
}

fn start_variable_editor(
    mode: WorkflowStartVariableEditorMode,
    variable: WorkflowStartVariableDraft,
) -> WorkflowStartVariableEditorDraft {
    WorkflowStartVariableEditorDraft {
        mode,
        default_value_editor: text_editor::Content::with_text(&variable.default_value),
        default_file_url_input: variable.default_value.clone(),
        show_default_file_url_input: false,
        variable,
    }
}

fn node_editor_with_start_editor(
    hovered_start_variable_index: Option<usize>,
    validation: WorkflowNodeEditorValidation,
    start_variable_editor: Option<WorkflowStartVariableEditorDraft>,
) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Create,
        active_tab: WorkflowNodeEditorTab::Visual,
        block_type: "start".to_string(),
        title: "开始".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::new(),
        position: Point::ORIGIN,
        visual_draft: Some(WorkflowNodeVisualDraft::Start { variables: Vec::new() }),
        validation,
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::new(),
        hovered_start_variable_index,
        start_variable_focus_index: None,
        start_variable_editor,
    }
}

fn active_state_with_mode(mode: &str) -> WorkflowState {
    let document = WorkflowDocument::default();
    let meta = WorkflowAppMeta { mode: mode.to_string(), ..WorkflowAppMeta::default() };
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
            local_uuid: None,
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
            saved_snapshot,
        }],
        active_app_id: Some("app_1".to_string()),
        document,
        zoom: 1.0,
        ..WorkflowState::default()
    }
}

#[test]
fn build_start_visual_section_handles_empty_and_hovered_states() {
    let empty_state = WorkflowState::default();
    let empty_validation = WorkflowNodeEditorValidation::default();
    let empty_section = build_start_visual_section(&empty_state, &[], &empty_validation);

    let variable = start_variable("query", "问题", "text-input");
    let validation = WorkflowNodeEditorValidation::default();
    let state = WorkflowState {
        node_editor: Some(node_editor_with_start_editor(Some(0), validation, None)),
        ..WorkflowState::default()
    };
    let variables = [variable];
    let populated_validation =
        &state.node_editor.as_ref().expect("node editor should exist").validation;
    let populated_section = build_start_visual_section(&state, &variables, populated_validation);

    std::hint::black_box(empty_section);
    std::hint::black_box(populated_section);
}

#[test]
fn build_start_variable_editor_modal_handles_hidden_create_and_edit_modes() {
    let hidden_state = WorkflowState::default();
    let hidden_modal = build_start_variable_editor_modal(&hidden_state);

    let create_state = WorkflowState {
        node_editor: Some(node_editor_with_start_editor(
            None,
            WorkflowNodeEditorValidation::default(),
            Some(start_variable_editor(
                WorkflowStartVariableEditorMode::Create,
                start_variable("query", "问题", "text-input"),
            )),
        )),
        ..WorkflowState::default()
    };
    let create_modal = build_start_variable_editor_modal(&create_state);

    let edit_state = WorkflowState {
        node_editor: Some(node_editor_with_start_editor(
            None,
            WorkflowNodeEditorValidation::default(),
            Some(start_variable_editor(
                WorkflowStartVariableEditorMode::Edit(1),
                start_variable("files", "附件", "file-list"),
            )),
        )),
        ..WorkflowState::default()
    };
    let edit_modal = build_start_variable_editor_modal(&edit_state);

    std::hint::black_box(hidden_modal);
    std::hint::black_box(create_modal);
    std::hint::black_box(edit_modal);
}

#[test]
fn build_start_variable_editor_modal_stays_hidden_without_nested_editor() {
    let state = WorkflowState {
        node_editor: Some(node_editor_with_start_editor(
            None,
            WorkflowNodeEditorValidation::default(),
            None,
        )),
        ..WorkflowState::default()
    };

    let modal = build_start_variable_editor_modal(&state);

    std::hint::black_box(modal);
}

#[test]
fn build_start_variable_section_handles_empty_and_populated_inputs() {
    let empty_validation = WorkflowNodeEditorValidation::default();
    let empty_section = build_start_variable_section(&[], None, &empty_validation);
    let variables = [start_variable("query", "问题", "text-input")];
    let populated_validation = WorkflowNodeEditorValidation::default();
    let populated_section =
        build_start_variable_section(&variables, Some(0), &populated_validation);

    std::hint::black_box(empty_section);
    std::hint::black_box(populated_section);
}

#[test]
fn build_start_variable_summary_item_handles_blank_names_labels_and_errors() {
    let fallback_variable = start_variable("", "", "text-input");
    let fallback_validation = WorkflowNodeEditorValidation::default();
    let fallback_summary =
        build_start_variable_summary_item(0, None, &fallback_variable, &fallback_validation);

    let validation = WorkflowNodeEditorValidation {
        field_errors: vec![WorkflowNodeValidationError {
            path: "start.variables[1].label".to_string(),
            message: "显示名称不能为空".to_string(),
        }],
    };
    let error_variable = start_variable("files", "附件", "file-list");
    let error_summary = build_start_variable_summary_item(1, Some(1), &error_variable, &validation);

    std::hint::black_box(fallback_summary);
    std::hint::black_box(error_summary);
}

#[test]
fn build_start_variable_action_button_handles_normal_and_danger_variants() {
    let edit_button = build_start_variable_action_button(
        Icon::Pencil,
        false,
        Message::WorkflowTool(WorkflowMessage::NodeEditorStartSelectVariable(0)),
    );
    let remove_button = build_start_variable_action_button(
        Icon::Trash,
        true,
        Message::WorkflowTool(WorkflowMessage::NodeEditorStartRemoveVariable(0)),
    );

    std::hint::black_box(edit_button);
    std::hint::black_box(remove_button);
}

#[test]
fn build_start_builtin_variable_views_handle_chat_and_legacy_items() {
    let chat_state = WorkflowState::default();
    let legacy_state = active_state_with_mode("workflow");
    let chat_section = build_start_builtin_variables_section(&chat_state);
    let legacy_section = build_start_builtin_variables_section(&legacy_state);
    let modern_item = build_start_builtin_variable_item(StartBuiltinVariableItem {
        name: "userinput.query",
        value_type: "String",
        description: "用户当前这一轮输入的问题文本。",
        legacy: false,
    });
    let legacy_item = build_start_builtin_variable_item(StartBuiltinVariableItem {
        name: "userinput.files",
        value_type: "Array[File]",
        description: "用户上传的文件列表，可作为文件型输入或后续节点的附件来源。",
        legacy: true,
    });

    std::hint::black_box(chat_section);
    std::hint::black_box(legacy_section);
    std::hint::black_box(modern_item);
    std::hint::black_box(legacy_item);
}
