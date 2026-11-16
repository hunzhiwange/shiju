//! 工作流菜单视图模块，提供节点选择、上下文菜单按钮和错误提示的通用渲染入口。

use super::*;

pub(super) fn workflow_node_icon_badge(
    icon: WorkflowNodeIconDescriptor,
    accent: Color,
    icon_size: f32,
) -> Element<'static, Message> {
    crate::view::menu::workflow_node_icon_badge(icon, accent, icon_size).map(Message::WorkflowTool)
}

pub(super) fn context_menu_content_button(
    content: Element<'static, Message>,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    crate::view::menu::context_menu_content_button(content, Message::WorkflowTool(message))
}

pub(super) fn context_menu_button(
    label: impl Into<String>,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    crate::view::menu::context_menu_button(label, message).map(Message::WorkflowTool)
}

pub(super) fn build_context_node_picker_menu(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
    title: &'static str,
    description: &'static str,
    exclude_start: bool,
) -> Element<'static, Message> {
    crate::view::menu::build_context_node_picker_menu(
        i18n,
        state,
        title,
        description,
        exclude_start,
    )
    .map(Message::WorkflowTool)
}

pub(super) fn context_node_picker_button(
    i18n: desktop_i18n::I18n,
    node_type: WorkflowNodeTypeDescriptor,
) -> Element<'static, Message> {
    crate::view::menu::context_node_picker_button(i18n, node_type).map(Message::WorkflowTool)
}

pub(super) fn error_banner(
    i18n: desktop_i18n::I18n,
    error: impl Into<String>,
) -> Element<'static, Message> {
    crate::view::menu::error_banner(i18n, error).map(Message::WorkflowTool)
}
