use crate::app::App;
pub(super) fn reset_new_session_picker_state(app: &mut App) {
    desktop_workspace::project_message::session_common::reset_picker_selection(
        &mut app.workspace_state.new_session_picker_project, &mut app.workspace_state.new_session_picker_options,
        &mut app.workspace_state.new_session_worktree_name,
    );
    clear_new_session_picker_messages(app);
}
pub(super) fn clear_new_session_picker_messages(app: &mut App) {
    desktop_workspace::project_message::session_common::clear_picker_messages(
        &mut app.workspace_state.new_session_confirm_delete_directory, &mut app.workspace_state.new_session_force_delete_directory,
        &mut app.workspace_state.new_session_delete_error, &mut app.workspace_state.new_session_confirm_reset_directory,
        &mut app.workspace_state.new_session_reset_error,
    );
    app.input_feedback.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeDelete);
    app.input_feedback.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeReset);
}

#[cfg(test)]
pub(super) use desktop_workspace::project_message::session_common::parse_clamped_u32;
#[cfg(test)]
pub(super) use desktop_workspace::project_message::session_common::parse_clamped_u64;
#[cfg(test)]
pub(super) use desktop_workspace::project_message::session_common::trim_to_option;
