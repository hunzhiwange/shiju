//! Local workflow state transitions; external effects remain in the desktop adapter.
use super::*;
use desktop_workflow_state::state::WorkflowState;
pub fn handle(
    state: &mut WorkflowState,
    _window_size: (f32, f32),
    message: WorkflowMessage,
) -> bool {
    match message {
        WorkflowMessage::OpenVariablePanel(kind) => {
            state.open_variable_panel(kind);
        }
        WorkflowMessage::CloseVariablePanel => {
            state.close_variable_panel();
        }
        WorkflowMessage::OpenCreateEnvironmentVariableEditor => {
            state.open_create_environment_variable_editor();
        }
        WorkflowMessage::OpenEditEnvironmentVariableEditor(id) => {
            if let Err(error) = state.open_edit_environment_variable_editor(&id) {
                state.set_error(error);
            }
        }
        WorkflowMessage::OpenCreateConversationVariableEditor => {
            state.open_create_conversation_variable_editor();
        }
        WorkflowMessage::OpenEditConversationVariableEditor(id) => {
            if let Err(error) = state.open_edit_conversation_variable_editor(&id) {
                state.set_error(error);
            }
        }
        WorkflowMessage::CloseVariableEditor => {
            state.close_variable_editor();
        }
        WorkflowMessage::VariableEditorNameChanged(value) => {
            state.set_variable_editor_name(value);
        }
        WorkflowMessage::VariableEditorDescriptionChanged(value) => {
            state.set_variable_editor_description(value);
        }
        WorkflowMessage::VariableEditorTypeChanged(value) => {
            state.set_variable_editor_type(value);
        }
        WorkflowMessage::VariableEditorValueAction(action) => {
            state.variable_editor_action(action);
        }
        WorkflowMessage::SubmitVariableEditor => {
            if let Err(error) = state.submit_variable_editor() {
                state.set_error(error);
            }
        }
        WorkflowMessage::DeleteEnvironmentVariable(id) => {
            state.delete_environment_variable(&id);
        }
        WorkflowMessage::DeleteConversationVariable(id) => {
            state.delete_conversation_variable(&id);
        }
        WorkflowMessage::ToggleActionMenu => {
            state.toggle_action_menu();
        }
        WorkflowMessage::CloseFloatingPanels => {
            state.close_floating_panels();
        }
        _ => return false,
    }
    true
}
