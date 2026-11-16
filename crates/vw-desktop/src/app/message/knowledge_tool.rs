//! 知识库领域与桌面反馈、拖拽状态的适配。
use crate::app::{App, Message};
use desktop_expert_hub::{Event, KnowledgeUpdate};
pub use desktop_expert_hub::message::knowledge_tool::KnowledgeToolMessage;

fn gateway() -> Result<gateway_http::GatewayTransport, String> {
    Ok(crate::app::gateway_client()?.transport().clone())
}
fn map_event(event: Event) -> Message {
    match event { Event::Knowledge(message) => Message::Knowledge(message) }
}
fn apply_feedback(app: &mut App, feedback: Option<(bool, String, desktop_i18n::format::Message)>) -> iced::Task<Message> {
    let Some((is_error, fallback, message)) = feedback else { return iced::Task::none() };
    let kind = if is_error { crate::app::state::ToastKind::Error } else { crate::app::state::ToastKind::Success };
    let task = app.show_toast(kind, fallback);
    app.toast_state.active_toast.as_mut().expect("inserted knowledge toast").localized_message = Some(message);
    task
}
pub fn update(app: &mut App, message: KnowledgeToolMessage) -> iced::Task<Message> {
    let dropped_paths = if matches!(message, KnowledgeToolMessage::ImportDroppedFiles) {
        let buffers = &mut app.file_tree_interaction;
        let dragging = std::mem::take(&mut buffers.dragging_file_paths);
        let pending = std::mem::take(&mut buffers.pending_drop_file_paths);
        buffers.dragging_file_position = None;
        buffers.pending_drop_file_position = None;
        if dragging.is_empty() { pending } else { dragging }
    } else { Vec::new() };
    let mut update = KnowledgeUpdate { knowledge: &mut app.knowledge, gateway, dropped_paths, feedback: None };
    let task = desktop_expert_hub::message::knowledge_tool::update(&mut update, message).map(map_event);
    let feedback = update.feedback.take();
    iced::Task::batch([apply_feedback(app, feedback), task])
}
pub(crate) fn ensure_loaded(app: &mut App) -> iced::Task<Message> {
    let mut update = KnowledgeUpdate { knowledge: &mut app.knowledge, gateway, dropped_paths: Vec::new(), feedback: None };
    desktop_expert_hub::message::knowledge_tool::ensure_loaded(&mut update).map(map_event)
}
