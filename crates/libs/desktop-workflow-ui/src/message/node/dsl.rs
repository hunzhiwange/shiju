//! Raw DSL editor actions and clipboard/focus effects.
use super::*;
use desktop_ui_controls::text_editor_context_menu::{
    focus_editor_task, paste_action, paste_task, selection_copy_task, selection_cut_task,
    selection_delete_task,
};
pub(super) fn handle(
    state: &mut WorkflowState,
    message: WorkflowMessage,
) -> Option<Task<WorkflowMessage>> {
    Some(match message {
        WorkflowMessage::NodeEditorDataAction(action) => {
            close_node_editor_dsl_context_menu(state);
            state.node_editor_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorDslOpenContextMenu { x, y } => {
            state.node_editor_dsl_context_menu_open = true;
            state.node_editor_dsl_context_menu_pos = Some((x, y));
            Task::none()
        }
        WorkflowMessage::NodeEditorDslCloseContextMenu => {
            close_node_editor_dsl_context_menu(state);
            focus_editor_task(&node_editor_dsl_editor_id())
        }
        WorkflowMessage::NodeEditorDslUndo => {
            close_node_editor_dsl_context_menu(state);
            state.undo_node_editor_dsl();
            focus_editor_task(&node_editor_dsl_editor_id())
        }
        WorkflowMessage::NodeEditorDslRedo => {
            close_node_editor_dsl_context_menu(state);
            state.redo_node_editor_dsl();
            focus_editor_task(&node_editor_dsl_editor_id())
        }
        WorkflowMessage::NodeEditorDslContextMenuCopy => {
            close_node_editor_dsl_context_menu(state);
            state
                .node_editor
                .as_ref()
                .map(|editor| {
                    selection_copy_task(&editor.raw_data_editor, &node_editor_dsl_editor_id()).1
                })
                .unwrap_or_else(Task::none)
        }
        WorkflowMessage::NodeEditorDslContextMenuCut => {
            close_node_editor_dsl_context_menu(state);
            let previous = state
                .node_editor
                .as_ref()
                .map(|editor| editor.raw_data_editor.text())
                .unwrap_or_default();
            let task = state
                .node_editor
                .as_mut()
                .map(|editor| {
                    selection_cut_task(&mut editor.raw_data_editor, &node_editor_dsl_editor_id()).1
                })
                .unwrap_or_else(Task::none);
            state.finish_node_editor_raw_edit(previous);
            task
        }
        WorkflowMessage::NodeEditorDslContextMenuPaste => {
            close_node_editor_dsl_context_menu(state);
            paste_task(&node_editor_dsl_editor_id(), |content| {
                WorkflowMessage::NodeEditorDataAction(paste_action(content))
            })
        }
        WorkflowMessage::NodeEditorDslContextMenuDelete => {
            close_node_editor_dsl_context_menu(state);
            let previous = state
                .node_editor
                .as_ref()
                .map(|editor| editor.raw_data_editor.text())
                .unwrap_or_default();
            let task = state
                .node_editor
                .as_mut()
                .map(|editor| {
                    selection_delete_task(&mut editor.raw_data_editor, &node_editor_dsl_editor_id())
                        .1
                })
                .unwrap_or_else(Task::none);
            state.finish_node_editor_raw_edit(previous);
            task
        }
        _ => return None,
    })
}
fn node_editor_dsl_editor_id() -> iced::widget::Id {
    iced::widget::Id::new("workflow-node-advanced-dsl-editor")
}

fn close_node_editor_dsl_context_menu(state: &mut WorkflowState) {
    state.node_editor_dsl_context_menu_open = false;
    state.node_editor_dsl_context_menu_pos = None;
}
