#[cfg(test)]
pub(super) fn build_if_else_visual_section<'a>(
    i18n: desktop_i18n::I18n,
    cases: &'a [super::state::WorkflowIfElseCaseDraft],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    crate::view::if_else::build_if_else_visual_section(i18n, cases, validation)
        .map(Message::WorkflowTool)
}
#[cfg(test)]
pub(super) fn build_if_else_case_card<'a>(
    i18n: desktop_i18n::I18n,
    index: usize,
    case: &'a super::state::WorkflowIfElseCaseDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    crate::view::if_else::build_if_else_case_card(i18n, index, case, validation)
        .map(Message::WorkflowTool)
}
#[cfg(test)]
pub(super) fn build_if_else_condition_card<'a>(
    i18n: desktop_i18n::I18n,
    case_index: usize,
    condition_index: usize,
    condition: &'a super::state::WorkflowIfElseConditionDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    crate::view::if_else::build_if_else_condition_card(
        i18n,
        case_index,
        condition_index,
        condition,
        validation,
    )
    .map(Message::WorkflowTool)
}

#[cfg(test)]
use super::*;
