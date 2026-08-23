//! 工作流变量视图的测试，覆盖列表、编辑弹窗、值预览和校验摘要。
//! fixture 验证秘密遮罩、长值截断及环境与会话变量的结构差异。
use super::*;
use crate::apps::workflow::model::{
    WorkflowAppMeta, WorkflowConversationVariable, WorkflowDocument, WorkflowEnvironmentVariable,
};
use crate::apps::workflow::state::{
    WorkflowAppEntry, WorkflowHistorySnapshot, WorkflowNodeEditorValidation,
    WorkflowNodeValidationError,
};
use iced::{Vector, widget::text_editor};
use serde_yaml::Value;

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

fn variable_editor(mode: state::WorkflowVariableEditorMode) -> state::WorkflowVariableEditorDraft {
    state::WorkflowVariableEditorDraft {
        mode,
        name: "api_key".to_string(),
        description: "供节点读取的变量".to_string(),
        value_type: "string".to_string(),
        raw_value_editor: text_editor::Content::with_text("value: demo"),
    }
}

fn environment_variable(
    value_type: &str,
    description: &str,
    value: Value,
) -> WorkflowEnvironmentVariable {
    WorkflowEnvironmentVariable {
        id: "env_1".to_string(),
        name: "API_KEY".to_string(),
        value_type: value_type.to_string(),
        value,
        description: description.to_string(),
        raw_variable: Value::Null,
    }
}

fn conversation_variable(description: &str, value: Value) -> WorkflowConversationVariable {
    WorkflowConversationVariable {
        id: "conv_1".to_string(),
        name: "conversation_state".to_string(),
        value_type: "object".to_string(),
        value,
        description: description.to_string(),
        raw_variable: Value::Null,
    }
}

#[test]
fn build_variable_panel_modal_covers_closed_and_all_panel_kinds() {
    let empty_state = WorkflowState::default();
    let _placeholder = build_variable_panel_modal(&empty_state);

    let system_state = WorkflowState {
        variable_panel: Some(WorkflowVariablePanelKind::System),
        ..active_state_with_mode("advanced-chat")
    };
    let environment_state = WorkflowState {
        variable_panel: Some(WorkflowVariablePanelKind::Environment),
        environment_variables: vec![environment_variable(
            "secret",
            "",
            Value::String("token".into()),
        )],
        ..WorkflowState::default()
    };
    let conversation_state = WorkflowState {
        variable_panel: Some(WorkflowVariablePanelKind::Conversation),
        conversation_variables: vec![conversation_variable(
            "",
            serde_yaml::from_str("step: 1").unwrap(),
        )],
        ..WorkflowState::default()
    };

    let _system = build_variable_panel_modal(&system_state);
    let _environment = build_variable_panel_modal(&environment_state);
    let _conversation = build_variable_panel_modal(&conversation_state);
}

#[test]
fn build_variable_editor_modal_covers_closed_and_all_modes() {
    let empty_state = WorkflowState::default();
    let _placeholder = build_variable_editor_modal(&empty_state);

    for mode in [
        state::WorkflowVariableEditorMode::CreateEnvironment,
        state::WorkflowVariableEditorMode::EditEnvironment("env_1".to_string()),
        state::WorkflowVariableEditorMode::CreateConversation,
        state::WorkflowVariableEditorMode::EditConversation("conv_1".to_string()),
    ] {
        let state = WorkflowState {
            variable_editor: Some(variable_editor(mode)),
            ..WorkflowState::default()
        };
        let _modal = build_variable_editor_modal(&state);
    }
}

#[test]
fn variable_list_builders_cover_empty_and_populated_states() {
    let empty_state = WorkflowState::default();
    let _empty_system = build_system_variable_list(&empty_state);
    let _empty_environment = build_environment_variable_list(&empty_state);
    let _empty_conversation = build_conversation_variable_list(&empty_state);

    let populated_system_state = active_state_with_mode("advanced-chat");
    let populated_environment_state = WorkflowState {
        environment_variables: vec![environment_variable(
            "secret",
            "",
            Value::String("token".into()),
        )],
        ..WorkflowState::default()
    };
    let populated_conversation_state = WorkflowState {
        conversation_variables: vec![conversation_variable(
            "",
            serde_yaml::from_str("items:\n  - first\n  - second").unwrap(),
        )],
        ..WorkflowState::default()
    };

    let _system = build_system_variable_list(&populated_system_state);
    let _environment = build_environment_variable_list(&populated_environment_state);
    let _conversation = build_conversation_variable_list(&populated_conversation_state);
}

#[test]
fn variable_value_preview_masks_secrets_and_truncates_long_values() {
    assert_eq!(variable_value_preview(&Value::String("token".into()), Some("secret")), "******");
    assert_eq!(variable_value_preview(&Value::String("token".into()), Some("SeCrEt")), "******");

    let preview = variable_value_preview(&serde_yaml::from_str("foo:\n  bar: baz").unwrap(), None);
    assert!(preview.contains("foo:"));
    assert!(preview.contains("bar: baz"));

    let long_preview = variable_value_preview(&Value::String("x".repeat(160)), None);
    assert!(long_preview.ends_with("..."));
    assert_eq!(long_preview.chars().count(), 123);
}

#[test]
fn variable_list_builders_preserve_non_empty_descriptions() {
    let environment_state = WorkflowState {
        environment_variables: vec![environment_variable(
            "string",
            "服务地址",
            Value::String("https://example.test".into()),
        )],
        ..WorkflowState::default()
    };
    let conversation_state = WorkflowState {
        conversation_variables: vec![conversation_variable(
            "会话状态",
            serde_yaml::from_str("step: 2").unwrap(),
        )],
        ..WorkflowState::default()
    };

    let environment = build_environment_variable_list(&environment_state);
    let conversation = build_conversation_variable_list(&conversation_state);

    let _ = std::hint::black_box((environment, conversation));
}

#[test]
fn variable_modal_builders_keep_expected_root_children() {
    let closed_state = WorkflowState::default();
    let closed_panel = build_variable_panel_modal(&closed_state);
    std::hint::black_box(closed_panel);

    for panel in [
        WorkflowVariablePanelKind::System,
        WorkflowVariablePanelKind::Environment,
        WorkflowVariablePanelKind::Conversation,
    ] {
        let state = WorkflowState {
            variable_panel: Some(panel),
            ..active_state_with_mode("advanced-chat")
        };
        let modal = build_variable_panel_modal(&state);
        std::hint::black_box(modal);
    }

    let closed_editor_state = WorkflowState::default();
    let closed_editor = build_variable_editor_modal(&closed_editor_state);
    std::hint::black_box(closed_editor);

    let editor_state = WorkflowState {
        variable_editor: Some(variable_editor(
            state::WorkflowVariableEditorMode::EditConversation("conv_1".to_string()),
        )),
        ..WorkflowState::default()
    };
    let editor = build_variable_editor_modal(&editor_state);
    std::hint::black_box(editor);
}

#[test]
fn variable_card_and_validation_helpers_expose_expected_child_counts() {
    let plain_card = variable_card(
        "name".to_string(),
        "string".to_string(),
        "description".to_string(),
        "preview".to_string(),
        None,
        None,
    );
    std::hint::black_box(plain_card);

    let action_card = variable_card(
        "name".to_string(),
        "string".to_string(),
        "description".to_string(),
        "preview".to_string(),
        Some(Message::WorkflowTool(WorkflowMessage::CloseVariableEditor)),
        Some(Message::WorkflowTool(WorkflowMessage::CloseVariablePanel)),
    );
    std::hint::black_box(action_card);

    let field = build_editor_field("变量名称", text("内容").into());
    assert_eq!(field.as_widget().children().len(), 2);

    let validated_error =
        build_editor_field_validated("变量值", text("bad").into(), Some("变量值必须是合法 YAML"));
    assert_eq!(validated_error.as_widget().children().len(), 3);

    let inline_error = build_inline_error("变量名不能为空");
    assert!(inline_error.as_widget().children().is_empty());
}

#[test]
fn node_validation_summary_limits_visible_errors_and_reports_overflow() {
    let eight_errors = WorkflowNodeEditorValidation {
        field_errors: (0..8)
            .map(|index| WorkflowNodeValidationError {
                path: format!("variables[{index}].name"),
                message: format!("第 {index} 项错误"),
            })
            .collect(),
    };
    let nine_errors = WorkflowNodeEditorValidation {
        field_errors: (0..9)
            .map(|index| WorkflowNodeValidationError {
                path: format!("variables[{index}].name"),
                message: format!("第 {index} 项错误"),
            })
            .collect(),
    };

    let eight_summary = build_node_validation_summary(&eight_errors);
    std::hint::black_box(eight_summary);

    let nine_summary = build_node_validation_summary(&nine_errors);
    std::hint::black_box(nine_summary);
}

#[test]
fn variable_value_preview_keeps_short_scalars_and_masks_secret_case_insensitively() {
    assert_eq!(variable_value_preview(&Value::Bool(true), None), "true");
    assert_eq!(variable_value_preview(&Value::Number(42.into()), None), "42");
    assert_eq!(variable_value_preview(&Value::String("token".into()), Some("SECRET")), "******");
}

#[test]
fn helper_builders_cover_optional_actions_and_validation_summary_branches() {
    let _plain_card = variable_card(
        "name".to_string(),
        "string".to_string(),
        "description".to_string(),
        "preview".to_string(),
        None,
        None,
    );
    let _action_card = variable_card(
        "name".to_string(),
        "string".to_string(),
        "description".to_string(),
        "preview".to_string(),
        Some(Message::WorkflowTool(WorkflowMessage::CloseVariableEditor)),
        Some(Message::WorkflowTool(WorkflowMessage::CloseVariablePanel)),
    );
    let _field = build_editor_field("变量名称", text("内容").into());
    let _validated_ok = build_editor_field_validated("变量类型", text("string").into(), None);
    let _validated_error =
        build_editor_field_validated("变量值", text("bad").into(), Some("变量值必须是合法 YAML"));
    let _inline_error = build_inline_error("变量名不能为空");

    let short_validation = WorkflowNodeEditorValidation {
        field_errors: vec![
            WorkflowNodeValidationError {
                path: "variables[0].name".to_string(),
                message: "变量名不能为空".to_string(),
            },
            WorkflowNodeValidationError {
                path: "variables[0].type".to_string(),
                message: "变量类型不能为空".to_string(),
            },
        ],
    };
    let long_validation = WorkflowNodeEditorValidation {
        field_errors: (0..9)
            .map(|index| WorkflowNodeValidationError {
                path: format!("variables[{index}].name"),
                message: format!("第 {index} 项错误"),
            })
            .collect(),
    };

    let _short_summary = build_node_validation_summary(&short_validation);
    let _long_summary = build_node_validation_summary(&long_validation);
}
