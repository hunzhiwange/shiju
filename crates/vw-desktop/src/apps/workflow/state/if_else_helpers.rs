//! # Workflow 条件分支辅助
//!
//! 该模块提供条件分支编辑器的默认草稿、焦点初始化和校验刷新辅助函数。

use super::*;

pub(super) fn default_if_else_case_draft() -> WorkflowIfElseCaseDraft {
    let case_id = generate_case_id();
    WorkflowIfElseCaseDraft {
        raw_case: yaml_map_for_state(vec![
            ("case_id", Value::String(case_id.clone())),
            ("conditions", Value::Sequence(vec![default_if_else_condition_value()])),
            ("id", Value::String(case_id.clone())),
            ("logical_operator", Value::String("and".to_string())),
        ]),
        case_id,
        logical_operator: "and".to_string(),
        conditions: vec![default_if_else_condition_draft()],
    }
}

pub(super) fn default_if_else_condition_draft() -> WorkflowIfElseConditionDraft {
    WorkflowIfElseConditionDraft {
        raw_condition: default_if_else_condition_value(),
        variable_selector_input: String::new(),
        comparison_operator: "contains".to_string(),
        compare_value: String::new(),
        var_type: "string".to_string(),
    }
}

pub(super) fn default_if_else_condition_value() -> Value {
    yaml_map_for_state(vec![
        ("comparison_operator", Value::String("contains".to_string())),
        ("id", Value::String(generate_condition_id())),
        ("value", Value::String(String::new())),
        ("varType", Value::String("string".to_string())),
        ("variable_selector", Value::Sequence(Vec::new())),
    ])
}

pub(super) fn yaml_map_for_state(entries: Vec<(&str, Value)>) -> Value {
    let mut map = Mapping::new();
    for (key, value) in entries {
        map.insert(yaml_key(key), value);
    }
    Value::Mapping(map)
}

pub(super) fn initial_start_variable_focus(
    visual_draft: Option<&WorkflowNodeVisualDraft>,
) -> Option<usize> {
    match visual_draft {
        Some(WorkflowNodeVisualDraft::Start { variables }) if !variables.is_empty() => Some(0),
        _ => None,
    }
}

pub(super) fn clamp_node_editor_start_variable_focus(editor: &mut WorkflowNodeEditorDraft) {
    match editor.visual_draft.as_ref() {
        Some(WorkflowNodeVisualDraft::Start { variables }) if variables.is_empty() => {
            editor.start_variable_focus_index = None;
        }
        Some(WorkflowNodeVisualDraft::Start { variables }) => {
            editor.start_variable_focus_index = match editor.start_variable_focus_index {
                Some(index) if index < variables.len() => Some(index),
                _ => Some(0),
            };
        }
        _ => {
            editor.start_variable_focus_index = None;
        }
    }
}

pub(super) fn refresh_node_editor_validation(editor: &mut WorkflowNodeEditorDraft) {
    editor.validation = validate_node_editor_draft(
        &editor.block_type,
        &editor.title,
        &editor.description,
        &editor.raw_data_editor.text(),
        editor.visual_draft.as_ref(),
    );
}

pub(super) fn clear_node_editor_validation(editor: &mut WorkflowNodeEditorDraft) {
    editor.validation = WorkflowNodeEditorValidation::default();
}
