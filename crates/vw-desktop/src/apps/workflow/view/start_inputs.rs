//! Desktop workflow message mapping.

use super::*;

pub(super) fn start_variable_file_type_options(
    i18n: crate::app::i18n::I18n,
) -> [(&'static str, &'static str, &'static str); 5] {
    desktop_workflow_ui::view::start_inputs::start_variable_file_type_options(i18n)
}

pub(super) fn build_start_variable_option_editor<'a>(
    i18n: crate::app::i18n::I18n,
    variable: &'a super::state::WorkflowStartVariableDraft,
) -> Element<'a, Message> {
    desktop_workflow_ui::view::start_inputs::build_start_variable_option_editor(i18n, variable).map(Message::WorkflowTool)
}

pub(super) fn build_start_variable_select_default_field<'a>(
    i18n: crate::app::i18n::I18n,
    variable: &'a super::state::WorkflowStartVariableDraft,
) -> Element<'a, Message> {
    desktop_workflow_ui::view::start_inputs::build_start_variable_select_default_field(i18n, variable).map(Message::WorkflowTool)
}

pub(super) fn build_start_variable_file_type_button(
    value: &'static str,
    label: &'static str,
    description: &'static str,
    selected: bool,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::start_inputs::build_start_variable_file_type_button(value, label, description, selected).map(Message::WorkflowTool)
}

pub(super) fn build_start_variable_upload_method_button(
    value: &'static str,
    label: &'static str,
    selected: bool,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::start_inputs::build_start_variable_upload_method_button(value, label, selected).map(Message::WorkflowTool)
}

pub(super) fn build_start_variable_file_settings<'a>(
    i18n: crate::app::i18n::I18n,
    start_variable_editor: &'a super::state::WorkflowStartVariableEditorDraft,
) -> Element<'a, Message> {
    desktop_workflow_ui::view::start_inputs::build_start_variable_file_settings(i18n, start_variable_editor).map(Message::WorkflowTool)
}

pub(super) fn build_start_variable_card<'a>(
    i18n: crate::app::i18n::I18n,
    start_variable_editor: &'a super::state::WorkflowStartVariableEditorDraft,
) -> Element<'a, Message> {
    desktop_workflow_ui::view::start_inputs::build_start_variable_card(i18n, start_variable_editor).map(Message::WorkflowTool)
}
