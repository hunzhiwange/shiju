//! Desktop message and selectable-help mapping for the workflow application form.
use super::*;
pub(super) fn build_app_editor_modal(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    desktop_workflow_ui::app_editor::build_app_editor_modal(
        i18n,
        state,
        Message::WorkflowTool,
        |i18n, title, label, description| {
            crate::app::components::system_settings_common::settings_hover_help_label(
                i18n,
                None,
                title,
                label,
                description,
            )
        },
    )
}
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "app_editor_ui_refresh_38_tests.rs"]
mod ui_refresh_38_tests;
