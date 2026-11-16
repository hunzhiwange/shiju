//! 真实聊天及输入组件的移动适配入口。
use crate::app::{App, Message, components::{chat_panel, input_panel}};
use iced::{Element, Length};
pub fn view(app: &App) -> Element<'_, Message> {
    iced::widget::column![chat_panel::view(app), input_panel::view(app)]
        .width(Length::Fill).height(Length::Fill).into()
}
