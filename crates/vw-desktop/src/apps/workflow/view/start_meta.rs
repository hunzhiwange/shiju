//! Desktop workflow message mapping.

use super::*;

pub(super) fn start_builtin_variables(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Vec<StartBuiltinVariableItem> {
    desktop_workflow_ui::view::start_meta::start_builtin_variables(i18n, state)
}

pub(super) fn start_variable_badge(label: &'static str) -> Element<'static, Message> {
    desktop_workflow_ui::view::start_meta::start_variable_badge(label).map(Message::WorkflowTool)
}

pub(super) fn start_variable_summary_error(
    i18n: crate::app::i18n::I18n,
    index: usize,
    validation: &super::state::WorkflowNodeEditorValidation,
) -> Option<String> {
    desktop_workflow_ui::view::start_meta::start_variable_summary_error(i18n, index, validation)
}

pub(super) fn start_variable_default_error(
    i18n: crate::app::i18n::I18n,
    variable: &super::state::WorkflowStartVariableDraft,
) -> Option<&'static str> {
    desktop_workflow_ui::view::start_meta::start_variable_default_error(i18n, variable)
}

pub(super) fn start_variable_advanced_hint(input_type: &str) -> Option<&'static str> {
    desktop_workflow_ui::view::start_meta::start_variable_advanced_hint(input_type)
}

pub(super) fn start_variable_type_options(
    i18n: crate::app::i18n::I18n,
) -> Vec<StartVariableTypeOption> {
    desktop_workflow_ui::view::start_meta::start_variable_type_options(i18n)
}

pub(super) fn selected_start_variable_type_option(
    i18n: crate::app::i18n::I18n,
    input_type: &str,
) -> Option<StartVariableTypeOption> {
    desktop_workflow_ui::view::start_meta::selected_start_variable_type_option(i18n, input_type)
}

pub(super) fn build_start_variable_type_selector(
    i18n: crate::app::i18n::I18n,
    input_type: &str,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::start_meta::build_start_variable_type_selector(i18n, input_type).map(Message::WorkflowTool)
}

pub(super) fn start_variable_file_type_options(
    i18n: crate::app::i18n::I18n,
) -> [(&'static str, &'static str, &'static str); 5] {
    desktop_workflow_ui::view::start_meta::start_variable_file_type_options(i18n)
}

pub(super) use desktop_workflow_ui::view::start_meta::start_variable_value_type_label;
