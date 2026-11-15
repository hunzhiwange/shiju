use super::*;
use iced::widget::{column, row};
const CONTEXT_NODE_PICKER_SCROLLBAR_GUTTER: f32 = 12.0;
const CONTEXT_NODE_PICKER_WIDTH: f32 = 260.0;

pub fn workflow_node_icon_badge(
    icon: WorkflowNodeIconDescriptor,
    accent: Color,
    icon_size: f32,
) -> Element<'static, Message> {
    let icon_element: Element<'static, Message> =
        if let Some(handle) = assets::get_named_icon_image(icon.family, icon.name, accent) {
            image(handle).width(Length::Fixed(icon_size)).height(Length::Fixed(icon_size)).into()
        } else {
            text("•").size(icon_size - 1.0).color(accent).into()
        };

    container(icon_element)
        .width(Length::Fixed(icon_size + 14.0))
        .height(Length::Fixed(icon_size + 14.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(accent.scale_alpha(0.12))),
            border: Border { color: accent.scale_alpha(0.18), width: 1.0, radius: 12.0.into() },
            ..Default::default()
        })
        .into()
}

pub fn context_menu_content_button<Message: Clone + 'static>(
    content: Element<'static, Message>,
    message: Message,
) -> Element<'static, Message> {
    button(content)
        .style(rounded_action_btn_style)
        .padding([8, 10])
        .width(Length::Fill)
        .on_press(message)
        .into()
}

pub fn context_menu_button(
    label: impl Into<String>,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    let content: Element<'static, Message> = text(label.into()).size(12).into();
    context_menu_content_button(content, message)
}

pub fn build_context_node_picker_menu(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
    title: &'static str,
    description: &'static str,
    exclude_start: bool,
) -> Element<'static, Message> {
    let mut items = column![].spacing(6);

    for node_type in available_node_types(state, exclude_start) {
        items = items.push(context_node_picker_button(i18n, node_type));
    }

    container(
        column![
            column![
                text(title).size(13),
                text(description).size(10).style(settings_muted_text_style),
            ]
            .spacing(2),
            scrollable(container(items).width(Length::Fill).padding(iced::Padding {
                top: 0.0,
                right: CONTEXT_NODE_PICKER_SCROLLBAR_GUTTER,
                bottom: 0.0,
                left: 0.0,
            }))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fixed(276.0))
            .width(Length::Fill),
        ]
        .spacing(10),
    )
    .width(Length::Fixed(CONTEXT_NODE_PICKER_WIDTH))
    .padding([10, 10])
    .style(floating_panel_style)
    .into()
}

pub fn context_node_picker_button(
    i18n: desktop_i18n::I18n,
    node_type: WorkflowNodeTypeDescriptor,
) -> Element<'static, Message> {
    let accent = workflow_node_accent_color(node_type.block_type);

    button(
        row![
            workflow_node_icon_badge(node_type.icon, accent, 14.0),
            column![
                text(i18n.__(node_type.label)).size(12),
                text(i18n.__(node_type.summary)).size(10).style(settings_muted_text_style),
            ]
            .spacing(2)
            .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(rounded_action_btn_style)
    .padding([8, 10])
    .width(Length::Fill)
    .on_press(WorkflowMessage::CreateContextNode(
        node_type.block_type.to_string(),
    ))
    .into()
}
