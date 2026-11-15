//! # Workflow 条件分支辅助
//!
//! 该模块提供条件分支编辑器的默认草稿、焦点初始化和校验刷新辅助函数。

use super::*;

pub(super) use desktop_workflow_state::default_if_else_case_draft;

pub(super) use desktop_workflow_state::default_if_else_condition_draft;

pub(super) use desktop_workflow_state::default_if_else_condition_value;

pub(super) use desktop_workflow_state::yaml_map_for_state;

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
