//! 会话状态与历史分页。
pub mod active;
pub mod chat_timeline;
#[cfg(test)]
mod chat_timeline_tests;
use session_ui_types::{ChatMessage, ChatSessionStep};
use crate::message::chat::stream::assign_message_model_to_step;
#[allow(dead_code)] // Used only by the retained full-session compatibility loader.
pub fn backfill_message_models_from_steps(
    messages: &mut [ChatMessage],
    steps: &[ChatSessionStep],
) {
    for step in steps {
        if let Some(model) = step.model.as_deref() {
            assign_message_model_to_step(messages, step.index, model, false);
        }
    }
}


#[cfg(test)]
mod active_tests;
