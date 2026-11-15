//! 应用提示的状态投影与重试消息映射。
use crate::app::{App, Message};
use iced::{Element, widget::Container};
use desktop_ui_controls::toast as shared;
pub use shared::confirm_dialog;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cache_sync_retry_message(app: &App) -> Option<Message> {
    app.expert_hub.cache_sync.pending_retry_skill_id()
        .map(|_| Message::SkillHub(crate::app::message::SkillHubMessage::RetrySkillCacheSync))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn cache_sync_retry_message(_app: &App) -> Option<Message> {
    None
}

pub fn view(app: &App) -> Element<'static, Message> {
    shared::view(app.active_toast.as_ref(), app.desktop_locale, cache_sync_retry_message(app))
}

pub(crate) fn modal_with_feedback<'a>(
    app: &App,
    card: Container<'a, Message>,
) -> Container<'a, Message> {
    shared::modal_with_feedback(card, app.active_toast.as_ref().map(|_| view(app)))
}

#[cfg(test)]
#[path = "toast_ui_refresh_33_tests.rs"]
mod toast_ui_refresh_33_tests;
