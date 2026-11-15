use crate::app::{App, Message, message};
#[cfg(test)] pub(super) use desktop_navigation::components::top_bar::workspace_mode::{copy, mode_icon, mode_label_font};
pub(super) fn selector(app: &App) -> iced::Element<'_, Message> {
 desktop_navigation::components::top_bar::workspace_mode::selector(crate::app::i18n::I18n::new(app.desktop_locale), app.workspace_mode, app.show_workspace_mode_popover).map(|event| {
 use desktop_navigation::components::top_bar::workspace_mode::Message as Event;
 Message::View(match event {
 Event::WorkspaceModeSelected(mode) => message::ViewMessage::WorkspaceModeSelected(mode),
 Event::ToggleWorkspaceModePopover => message::ViewMessage::ToggleWorkspaceModePopover,
 Event::CloseWorkspaceModePopover => message::ViewMessage::CloseWorkspaceModePopover,
 })
 })
}
