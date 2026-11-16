//! Rendering fixture used by the explicit tray lifecycle acceptance.
pub fn tray_view<Message: 'static>() -> iced::Element<'static, Message> {
    iced::widget::container(
        iced::widget::text("关闭后恢复窗口验收：重建 / 菜单栏 / 最小化").size(20),
    )
    .center(iced::Length::Fill)
    .into()
}
