use crate::app::App;
pub(super) use desktop_workspace::project_message::session_common::{parse_clamped_u32, parse_clamped_u64, trim_to_option};
#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;
pub(super) fn reset_new_session_picker_state(app: &mut App) {
    desktop_workspace::project_message::session_common::reset_picker_selection(
        &mut app.new_session_picker_project, &mut app.new_session_picker_options,
        &mut app.new_session_worktree_name,
    );
    clear_new_session_picker_messages(app);
}
pub(super) fn clear_new_session_picker_messages(app: &mut App) {
    desktop_workspace::project_message::session_common::clear_picker_messages(
        &mut app.new_session_confirm_delete_directory, &mut app.new_session_force_delete_directory,
        &mut app.new_session_delete_error, &mut app.new_session_confirm_reset_directory,
        &mut app.new_session_reset_error,
    );
    app.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeDelete);
    app.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeReset);
}
