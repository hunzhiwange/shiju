//! Desktop message and selectable-help mapping for the workflow application form.
use super::*;
pub(super) fn build_app_editor_modal(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::app_editor::build_app_editor_modal(
        i18n,
        state,
        Message::WorkflowTool,
        |i18n, title, label, description| {
            desktop_ui_controls::system_settings_common::settings_hover_help_label(
                label,
                desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
                    title,
                    description,
                    None,
                    i18n,
                ),
            )
        },
    )
}
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "app_editor_ui_refresh_38_tests.rs"]
mod ui_refresh_38_tests;
