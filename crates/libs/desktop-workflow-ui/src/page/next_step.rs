use super::*;
pub(super) fn build_node_next_step_existing_item<'a>(
    i18n: desktop_i18n::I18n,
    edge: &'a WorkflowEdge,
    node: &'a WorkflowNode,
) -> Element<'a, Message> {
    crate::view::next_step::build_node_next_step_existing_item(i18n, edge, node)
        .map(Message::WorkflowTool)
}
pub(super) fn build_node_next_step_section<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowState,
    editor: &'a super::state::WorkflowNodeEditorDraft,
) -> Element<'a, Message> {
    crate::view::next_step::build_node_next_step_section(i18n, state, editor)
        .map(Message::WorkflowTool)
}
pub(super) fn build_start_next_step_button(
    source_node_id: String,
    source_handle_id: Option<&str>,
    node_type: WorkflowNodeTypeDescriptor,
) -> Element<'static, Message> {
    crate::view::next_step::build_start_next_step_button(
        source_node_id,
        source_handle_id,
        node_type,
    )
    .map(Message::WorkflowTool)
}
