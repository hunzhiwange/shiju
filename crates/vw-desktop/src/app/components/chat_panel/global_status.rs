//! 桌面状态栏的只读输入装配。
pub(crate) use desktop_chat_view::global_status::{COMPACT_ACTIVITY_SLOT_SIZE, COMPACT_ACTIVITY_TEXT_SIZE};
use crate::app::{App, Message};
use iced::Element;
pub(crate) fn chat_global_status_view(app: &App) -> Element<'_, Message> {
    let runtime = app.current_session_runtime_ref();
    desktop_chat_view::global_status::chat_global_status_view(
        &app.chat, runtime.is_some_and(|runtime| runtime.is_requesting), &app.usage,
        app.desktop_locale, app.status_animation_frame,
        runtime.map(|runtime| runtime.processing_timer.elapsed_ms_at(web_time::Instant::now())).unwrap_or(0),
    )
}
