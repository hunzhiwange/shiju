use iced::{Element, Length};
use iced::widget::column;
pub mod bottom;
pub const WORKSPACE_BOTTOM_GAP: f32 = 6.0;
pub fn build_base_layout<'a, Message: 'a>(workspace: Element<'a, Message>, bottom: Element<'a, Message>, hide_bottom: bool) -> Element<'a, Message> {
    if hide_bottom {
        workspace
    } else {
        column![
            iced::widget::container(workspace)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding::default().bottom(WORKSPACE_BOTTOM_GAP)),
            bottom
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
