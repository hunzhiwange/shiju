use crate::app::{App, Message, message::ViewMessage};
use desktop_workspace::views::project::layout::work::{self,WorkLayout};
pub fn build_work_layout(app:&App, _width:f32, _rail:f32, _scale:f32, radius:f32, spacing:f32, chat_pad:f32) -> iced::Element<'_,Message> {
    if app.terminal.is_fullscreen { return super::bottom::fullscreen_terminal(app); }
    work::build_work_layout(WorkLayout {
work_resources_visible: app.work_resources_visible,
work_chat_fullscreen: app.work_chat_fullscreen,
work_chat_half_fullscreen: app.work_chat_half_fullscreen,
work_resource_fullscreen: app.work_resource_fullscreen,
work_resource_half_fullscreen: app.work_resource_half_fullscreen,
work_split_ratio: app.work_split_ratio,
}, super::work_sidebar::view(app), super::chat::chat_area(app,spacing,radius,chat_pad), super::work_resources::view(app,radius), crate::app::components::chat_panel::tool_dialog_overlay(app).unwrap_or_else(|| iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into()), if app.terminal.is_visible {Some(super::bottom::bottom_panel(app))} else {None}, Message::View(ViewMessage::WorkSplitDragStarted), Message::View(ViewMessage::WorkSidebarDragStarted),radius,spacing)
}
