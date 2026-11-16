use super::{App, Message, message};
use iced::Element;
pub(crate) fn with_permission_modal<'a>(app: &App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_chat_view::modals::permission_modal::with_permission_modal(app.session_state.permission_modal_request.as_ref(), &app.session_state.permission_modal_requests, app.general_settings.desktop_locale, app.toast_state.active_toast.as_ref().map(|_| crate::app::components::toast::view(app)), |event| {
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
