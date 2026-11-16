//! Iced task/result assembly for native notifications.
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::{App, Message};
use config_ui::DesktopLocale;
pub(crate) use desktop_platform::desktop_notification::SUPPORTED;
use iced::Task;
pub(crate) fn failure_feedback(app: &mut App, error: String) -> Task<Message> {
    match desktop_ui_controls::toast::show_supplemental_error(
        &mut app.toast_state.active_toast,
        &mut app.toast_state.next_toast_id,
        error,
    ) {
        Some(id) => crate::app::message::after(
            std::time::Duration::from_millis(2200),
            Message::Notification(crate::app::message::NotificationMessage::HideToast(id)),
        ),
        None => Task::none(),
    }
}
pub(crate) fn completion_task(locale: DesktopLocale) -> Task<Message> {
    Task::perform(desktop_platform::desktop_notification::completion(locale), |result| {
        Message::Settings(SettingsMessage::General(
            GeneralSettingsMessage::TaskCompletionNotificationSent(result),
        ))
    })
}
pub(crate) fn permission_task(locale: DesktopLocale) -> Task<Message> {
    Task::perform(desktop_platform::desktop_notification::permission(locale), |result| {
        Message::Settings(SettingsMessage::General(
            GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(result),
        ))
    })
}
