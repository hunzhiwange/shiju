//! 复用当前会话的消息与草稿；仅改变布局，不持有聊天业务状态。
use crate::app::{App, Message, components::{chat_panel, input_panel}};
use iced::{Element, Length};
use iced::widget::{column, container};

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        container(chat_panel::view(app)).width(Length::Fill).height(Length::Fill),
        container(input_panel::view(app)).width(Length::Fill).height(Length::Shrink),
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
