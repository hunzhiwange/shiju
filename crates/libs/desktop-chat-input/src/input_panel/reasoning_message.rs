#[derive(Debug, Clone)]
pub enum Message {
    Selected(Option<vw_api_chat_identity::ReasoningEffortDto>),
    Toggle,
    Close,
}
pub fn dark_tooltip_content<'a>(label: impl Into<String>) -> iced::Element<'a, Message> {
    iced::widget::container(iced::widget::text(label.into()).size(12).color(iced::Color::WHITE))
        .style(super::styles::tooltip_dark_style)
        .padding([6, 8])
        .into()
}
