//! Desktop workflow message mapping.

use super::*;

pub(super) fn build_variable_panel_modal(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::variables::build_variable_panel_modal(i18n, state).map(Message::WorkflowTool)
}

pub(super) fn build_variable_editor_modal(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::variables::build_variable_editor_modal(i18n, state).map(Message::WorkflowTool)
}

pub(super) fn build_system_variable_list(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::variables::build_system_variable_list(i18n, state).map(Message::WorkflowTool)
}

pub(super) fn build_environment_variable_list(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::variables::build_environment_variable_list(i18n, state).map(Message::WorkflowTool)
}

pub(super) fn build_conversation_variable_list(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::variables::build_conversation_variable_list(i18n, state).map(Message::WorkflowTool)
}

pub(super) fn variable_value_preview(
    value: &serde_yaml::Value,
    value_type: Option<&str>,
) -> String {
    crate::view::variables::variable_value_preview(value, value_type)
}

pub(super) fn build_node_validation_summary(
    i18n: desktop_i18n::I18n,
    validation: &super::state::WorkflowNodeEditorValidation,
) -> Element<'_, Message> {
    crate::view::node_editor::build_node_validation_summary(i18n, validation)
        .map(Message::WorkflowTool)
}

pub(super) fn build_inline_error(message: impl Into<String>) -> Element<'static, Message> {
    crate::view::variables::build_inline_error(message).map(Message::WorkflowTool)
}

pub(super) use crate::view::variables::build_editor_field;

pub(super) use crate::view::variables::build_editor_field_validated;
