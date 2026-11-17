//! Desktop workflow message mapping.

#[cfg(test)]
pub(super) fn start_builtin_variables(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Vec<StartBuiltinVariableItem> {
    crate::view::start_meta::start_builtin_variables(i18n, state)
}

#[cfg(test)]
pub(super) fn start_variable_badge(label: &'static str) -> Element<'static, Message> {
    crate::view::start_meta::start_variable_badge(label).map(Message::WorkflowTool)
}

#[cfg(test)]
pub(super) fn start_variable_summary_error(
    i18n: desktop_i18n::I18n,
    index: usize,
    validation: &super::state::WorkflowNodeEditorValidation,
) -> Option<String> {
    crate::view::start_meta::start_variable_summary_error(i18n, index, validation)
}

#[cfg(test)]
pub(super) fn start_variable_default_error(
    i18n: desktop_i18n::I18n,
    variable: &super::state::WorkflowStartVariableDraft,
) -> Option<&'static str> {
    crate::view::start_meta::start_variable_default_error(i18n, variable)
}

#[cfg(test)]
pub(super) fn start_variable_advanced_hint(input_type: &str) -> Option<&'static str> {
    crate::view::start_meta::start_variable_advanced_hint(input_type)
}

#[cfg(test)]
pub(super) fn start_variable_type_options(
    i18n: desktop_i18n::I18n,
) -> Vec<StartVariableTypeOption> {
    crate::view::start_meta::start_variable_type_options(i18n)
}

#[cfg(test)]
pub(super) fn selected_start_variable_type_option(
    i18n: desktop_i18n::I18n,
    input_type: &str,
) -> Option<StartVariableTypeOption> {
    crate::view::start_meta::selected_start_variable_type_option(i18n, input_type)
}

#[cfg(test)]
pub(super) fn build_start_variable_type_selector(
    i18n: desktop_i18n::I18n,
    input_type: &str,
) -> Element<'static, Message> {
    crate::view::start_meta::build_start_variable_type_selector(i18n, input_type)
        .map(Message::WorkflowTool)
}

#[cfg(test)]
pub(super) fn start_variable_file_type_options(
    i18n: desktop_i18n::I18n,
) -> [(&'static str, &'static str, &'static str); 5] {
    crate::view::start_meta::start_variable_file_type_options(i18n)
}

#[cfg(test)]
use super::*;
#[cfg(test)]
pub(super) use crate::view::node_editor::{StartBuiltinVariableItem, StartVariableTypeOption};
