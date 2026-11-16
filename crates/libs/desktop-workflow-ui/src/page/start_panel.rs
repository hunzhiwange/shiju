//! Desktop workflow message mapping.

use super::*;

pub(super) fn build_start_visual_section<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowState,
    variables: &'a [super::state::WorkflowStartVariableDraft],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    crate::view::start_panel::build_start_visual_section(i18n, state, variables, validation)
        .map(Message::WorkflowTool)
}

pub(super) fn build_start_variable_editor_modal(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::start_panel::build_start_variable_editor_modal(
        i18n,
        state,
        Message::WorkflowTool,
        Message::None,
    )
}

pub(super) fn build_start_variable_section<'a>(
    i18n: desktop_i18n::I18n,
    variables: &'a [super::state::WorkflowStartVariableDraft],
    hovered_index: Option<usize>,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    crate::view::start_panel::build_start_variable_section(
        i18n,
        variables,
        hovered_index,
        validation,
    )
    .map(Message::WorkflowTool)
}

pub(super) fn build_start_variable_summary_item<'a>(
    i18n: desktop_i18n::I18n,
    index: usize,
    _hovered_index: Option<usize>,
    variable: &'a super::state::WorkflowStartVariableDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    crate::view::start_panel::build_start_variable_summary_item(
        i18n,
        index,
        _hovered_index,
        variable,
        validation,
    )
    .map(Message::WorkflowTool)
}

pub(super) fn build_start_builtin_variables_section(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::start_panel::build_start_builtin_variables_section(i18n, state)
        .map(Message::WorkflowTool)
}

pub(super) fn build_start_builtin_variable_item(
    i18n: desktop_i18n::I18n,
    item: StartBuiltinVariableItem,
) -> Element<'static, Message> {
    crate::view::start_panel::build_start_builtin_variable_item(i18n, item)
        .map(Message::WorkflowTool)
}
