//! Start-node input editing and native default-file selection.
use super::*;
pub(super) fn handle(
    state: &mut WorkflowState,
    message: WorkflowMessage,
) -> Option<Task<WorkflowMessage>> {
    Some(match message {
        WorkflowMessage::NodeEditorStartAddVariable => {
            state.open_node_editor_start_variable_create();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableHovered(index) => {
            state.set_node_editor_start_variable_hovered(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartRemoveVariable(index) => {
            state.remove_node_editor_start_variable(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartSelectVariable(index) => {
            state.open_node_editor_start_variable_edit(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartCloseVariableEditor => {
            state.close_node_editor_start_variable_editor();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartSubmitVariableEditor => {
            if let Err(error) = state.submit_node_editor_start_variable_editor() {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorLabelChanged(value) => {
            state.set_node_editor_start_variable_editor_label(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorNameChanged(value) => {
            state.set_node_editor_start_variable_editor_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorTypeChanged(value) => {
            state.set_node_editor_start_variable_editor_type(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorRequiredChanged(value) => {
            state.set_node_editor_start_variable_editor_required(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorHiddenChanged(value) => {
            state.set_node_editor_start_variable_editor_hidden(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged(value) => {
            state.set_node_editor_start_variable_editor_default(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorDefaultAction(action) => {
            state.node_editor_start_variable_editor_default_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorMaxLengthChanged(value) => {
            state.set_node_editor_start_variable_editor_max_length(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorAddOption => {
            state.add_node_editor_start_variable_editor_option();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorRemoveOption(index) => {
            state.remove_node_editor_start_variable_editor_option(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorOptionChanged(index, value) => {
            state.set_node_editor_start_variable_editor_option(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorToggleFileType(value) => {
            state.toggle_node_editor_start_variable_editor_file_type(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorExtensionsChanged(value) => {
            state.set_node_editor_start_variable_editor_extensions(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorUploadMethodChanged(value) => {
            state.set_node_editor_start_variable_editor_upload_method(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFile => {
            #[cfg(target_arch = "wasm32")]
            {
                Task::perform(
                    async { Err("Web 平台暂不支持选择默认文件".to_string()) },
                    |res| {
                        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(res)
                    },
                )
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new().pick_file().await;
                        Ok(file.map(|handle| handle.path().to_string_lossy().to_string()))
                    },
                    |res| {
                        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(res)
                    },
                )
            }
        }
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(result) => {
            match result {
                Ok(Some(path)) => {
                    if let Err(error) =
                        state.set_node_editor_start_variable_editor_default_file_path(path)
                    {
                        state.set_error(error);
                    }
                }
                Ok(None) => {}
                Err(error) => state.set_error(error),
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorRemoveDefaultFile(index) => {
            state.remove_node_editor_start_variable_editor_default_file(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput => {
            state.open_node_editor_start_variable_editor_default_file_url_input();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorCloseDefaultFileUrlInput => {
            state.close_node_editor_start_variable_editor_default_file_url_input();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorDefaultFileUrlChanged(value) => {
            state.set_node_editor_start_variable_editor_default_file_url_input(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorSubmitDefaultFileUrl => {
            if let Err(error) = state.submit_node_editor_start_variable_editor_default_file_url() {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableLabelChanged(index, value) => {
            state.set_node_editor_start_variable_label(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableNameChanged(index, value) => {
            state.set_node_editor_start_variable_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableTypeChanged(index, value) => {
            state.set_node_editor_start_variable_type(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableRequiredChanged(index, value) => {
            state.set_node_editor_start_variable_required(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableDefaultChanged(index, value) => {
            state.set_node_editor_start_variable_default(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariablePlaceholderChanged(index, value) => {
            state.set_node_editor_start_variable_placeholder(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableHintChanged(index, value) => {
            state.set_node_editor_start_variable_hint(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableMaxLengthChanged(index, value) => {
            state.set_node_editor_start_variable_max_length(index, value);
            Task::none()
        }
        _ => return None,
    })
}
