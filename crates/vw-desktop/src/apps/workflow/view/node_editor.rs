use super::*;
pub(super) use desktop_workflow_ui::view::node_editor::{StartBuiltinVariableItem, StartVariableTypeOption};
pub(super) fn build_node_editor_modal(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    desktop_workflow_ui::view::node_editor::build_node_editor_modal(i18n, state, state.node_editor.as_ref().filter(|editor| editor.active_tab == WorkflowNodeEditorTab::Visual).and_then(|editor| build_node_visual_section(i18n, state, editor)), Message::WorkflowTool)
}
pub(super) fn build_embedded_text_editor<'a, F>(
    content: &'a text_editor::Content,
    placeholder: &'static str,
    map_message: F,
    height: f32,
) -> Element<'a, Message>
where
    F: Fn(text_editor::Action) -> WorkflowMessage + 'static + Copy, {
    desktop_workflow_ui::view::node_editor::build_embedded_text_editor(content, placeholder, map_message, height).map(Message::WorkflowTool)
}
pub(super) fn build_node_editor_header(
    i18n: crate::app::i18n::I18n,
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Element<'_, Message> {
    desktop_workflow_ui::view::node_editor::build_node_editor_header(i18n, editor).map(Message::WorkflowTool)
}
pub(super) fn build_node_editor_tabs(
    i18n: crate::app::i18n::I18n,
    has_visual_tab: bool,
    active_tab: WorkflowNodeEditorTab,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::node_editor::build_node_editor_tabs(i18n, has_visual_tab, active_tab).map(Message::WorkflowTool)
}
pub(super) fn build_node_editor_tab_button(
    label: &'static str,
    tab: WorkflowNodeEditorTab,
    active_tab: WorkflowNodeEditorTab,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::node_editor::build_node_editor_tab_button(label, tab, active_tab).map(Message::WorkflowTool)
}
pub(super) fn build_node_advanced_dsl_section<'a>(
    i18n: crate::app::i18n::I18n,
    state: &'a WorkflowState,
    editor: &'a super::state::WorkflowNodeEditorDraft,
) -> Element<'a, Message> {
    desktop_workflow_ui::view::node_editor::build_node_advanced_dsl_section(i18n, state, editor).map(Message::WorkflowTool)
}
pub(super) fn build_node_visual_placeholder(
    i18n: crate::app::i18n::I18n,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::node_editor::build_node_visual_placeholder(i18n).map(Message::WorkflowTool)
}
