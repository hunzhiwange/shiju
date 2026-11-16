//! 通知领域与桌面任务映射。
use crate::app::{App, Message};
use iced::Task;
pub use desktop_navigation::message::notification::NotificationMessage;

pub fn update(app: &mut App, message: NotificationMessage) -> Task<Message> {
    let copy = desktop_navigation::message::notification::update(
        desktop_navigation::message::notification::State {
            notifications: &mut app.notification_state.notifications,
            notification_editors: &mut app.notification_state.notification_editors,
            next_notification_id: &mut app.notification_state.next_notification_id,
            copied_notification_id: &mut app.notification_state.copied_notification_id,
            notifications_expanded: &mut app.notification_state.notifications_expanded,
            active_toast: &mut app.toast_state.active_toast,
            desktop_locale: app.general_settings.desktop_locale,
        }, message);
    let Some((id, text)) = copy else { return Task::none(); };
    Task::batch([
        iced::clipboard::write(text).map(|_: ()| Message::CopyDone),
        super::after(std::time::Duration::from_secs(2),
            Message::Notification(NotificationMessage::ResetCopied(id))),
    ])
}
