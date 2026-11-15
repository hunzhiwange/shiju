use super::{App, Message, message};
use iced::Element;
pub(crate) fn with_permission_modal<'a>(app: &App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_chat_view::modals::permission_modal::with_permission_modal(app.permission_modal_request.as_ref(), &app.permission_modal_requests, app.desktop_locale, app.active_toast.as_ref().map(|_| crate::app::components::toast::view(app)), |event| {
        use desktop_chat_view::modals::permission_modal::Event;
        Message::Chat(match event {
            Event::PermissionReject => message::ChatMessage::PermissionReject,
            Event::PermissionSelectRequest(id) => message::ChatMessage::PermissionSelectRequest(id),
            Event::PermissionApproveAlways => message::ChatMessage::PermissionApproveAlways,
            Event::PermissionApproveOnce => message::ChatMessage::PermissionApproveOnce,
            Event::PermissionApproveAllAlways => message::ChatMessage::PermissionApproveAllAlways,
        })
    }, root)
}

#[cfg(test)]
#[path = "permission_modal_tests.rs"]
mod permission_modal_tests;
