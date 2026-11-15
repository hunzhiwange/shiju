//! 工作流菜单视图模块，提供节点选择、上下文菜单按钮和错误提示的通用渲染入口。

use super::*;

pub(super) fn workflow_node_icon_badge(
    icon: WorkflowNodeIconDescriptor,
    accent: Color,
    icon_size: f32,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::menu::workflow_node_icon_badge(icon, accent, icon_size).map(Message::WorkflowTool)
}

pub(super) fn context_menu_content_button(
    content: Element<'static, Message>,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::menu::context_menu_content_button(content, Message::WorkflowTool(message))
}

pub(super) fn context_menu_button(
    label: impl Into<String>,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::menu::context_menu_button(label, message).map(Message::WorkflowTool)
}

pub(super) fn build_context_node_picker_menu(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
    title: &'static str,
    description: &'static str,
    exclude_start: bool,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::menu::build_context_node_picker_menu(i18n, state, title, description, exclude_start).map(Message::WorkflowTool)
}

pub(super) fn context_node_picker_button(
    i18n: crate::app::i18n::I18n,
    node_type: WorkflowNodeTypeDescriptor,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::menu::context_node_picker_button(i18n, node_type).map(Message::WorkflowTool)
}

pub(super) fn error_banner(
    i18n: crate::app::i18n::I18n,
    error: impl Into<String>,
) -> Element<'static, Message> {
    container(
        row![
            text(error.into()).size(13).color(Color::from_rgb8(0x7F, 0x1D, 0x1D)),
            Space::new().width(Length::Fill),
            button(text(i18n.__("关闭")))
                .style(rounded_action_btn_style)
                .padding([4, 10])
                .on_press(Message::WorkflowTool(WorkflowMessage::DismissError)),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
    )
    .padding([10, 12])
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(Background::Color(palette.danger.weak.color.scale_alpha(0.22))),
            border: Border {
                color: palette.danger.base.color.scale_alpha(0.40),
                width: 1.0,
                radius: 14.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod menu_tests;
