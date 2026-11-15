//! Local workflow state transitions; external effects remain in the desktop adapter.
use super::*;
use desktop_workflow_state::state::WorkflowState;
pub fn handle(
    state: &mut WorkflowState,
    window_size: (f32, f32),
    message: WorkflowMessage,
) -> bool {
    match message {
        WorkflowMessage::LoadSavedAppsFinished(result) => {
            state.finish_saved_apps_load(result);
        }
        WorkflowMessage::SavedAppSearchChanged(query) => {
            state.set_saved_app_search_query(query);
        }
        WorkflowMessage::ToggleSavedAppActions(uuid) => {
            state.toggle_saved_app_actions(uuid);
        }
        WorkflowMessage::CloseSavedAppActions => {
            state.close_saved_app_actions();
        }
        WorkflowMessage::ClearCopiedSavedAppUuid(uuid) => {
            state.clear_saved_app_uuid_copied(&uuid);
        }
        WorkflowMessage::CancelDeleteSavedApp => {
            state.close_saved_app_delete_confirm();
        }
        WorkflowMessage::OpenCreateAppEditor => {
            state.open_create_editor();
        }
        WorkflowMessage::OpenEditAppEditor(id) => {
            state.open_edit_editor(id.as_deref());
        }
        WorkflowMessage::CloseAppEditor => {
            state.close_editor();
        }
        WorkflowMessage::AppEditorNameChanged(value) => {
            state.set_editor_name(value);
        }
        WorkflowMessage::AppEditorDescriptionChanged(value) => {
            state.set_editor_description(value);
        }
        WorkflowMessage::AppEditorIconChanged(value) => {
            state.set_editor_icon(value);
        }
        WorkflowMessage::AppEditorUseIconAsAnswerIconChanged(value) => {
            state.set_editor_use_icon_as_answer_icon(value);
        }
        WorkflowMessage::AppEditorMaxActiveRequestsChanged(value) => {
            state.set_editor_max_active_requests_input(value);
        }
        WorkflowMessage::OrganizeActiveApp => {
            if let Err(error) = state.organize_active_app(window_size) {
                state.set_error(error);
            }
        }
        _ => return false,
    }
    true
}

/// Returns whether a submitted editor requires the host tab to synchronize.
pub fn submit_editor(
    state: &mut WorkflowState,
    window_size: (f32, f32),
) -> Result<bool, desktop_i18n::workflow::Feedback> {
    let Some(mode) = state.app_editor.as_ref().map(|draft| draft.mode.clone()) else {
        return Ok(false);
    };
    let template = match mode {
        WorkflowAppEditorMode::Create | WorkflowAppEditorMode::Edit(_) => {
            create_blank_workflow(WorkflowAppMeta::default())?
        }
    };
    state.submit_editor(window_size, template)?;
    Ok(true)
}
