use crate::app::{App, Message};
use desktop_navigation::components::notification as shared;
pub fn view(app: &App) -> iced::Element<'_, Message> {
    shared::view(shared::Input {
        desktop_locale: app.general_settings.desktop_locale, notifications: &app.notification_state.notifications,
        notification_editors: &app.notification_state.notification_editors, copied_notification_id: app.notification_state.copied_notification_id,
        notifications_scroll_id: app.notification_state.notifications_scroll_id.clone(),
    }).map(|event| { use shared::NotificationMessage as E; use crate::app::message::NotificationMessage as H;
        let shared::Message::Notification(event) = event;
        Message::Notification(match event {
            E::EditorAction(id, action) => H::EditorAction(id, action), E::Copy(id) => H::Copy(id),
            E::Remove(id) => H::Remove(id), E::ClearAll => H::ClearAll, E::ToggleExpanded => H::ToggleExpanded,
        })
    })
}
