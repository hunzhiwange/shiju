pub mod delta_scheduler;
pub mod draft;
pub mod permissions;
pub mod questions;
pub mod steps;
#[cfg(test)]
mod delta_scheduler_tests;
mod provider {
    pub use provider_resolution::{default_model, get_model, list};
    pub use model_types::{parse_model, ParsedModelRef};
}
use session_ui_types::{ChatMessage, ChatRole};
pub fn bind_message_model_to_step(
    messages: &mut [ChatMessage],
    step_index: u32,
    model: &str,
) {
    assign_message_model_to_step(messages, step_index, model, true);
}
pub fn assign_message_model_to_step(
    messages: &mut [ChatMessage],
    step_index: u32,
    model: &str,
    overwrite: bool,
) {
    let model = model.trim();
    if model.is_empty() || step_index == 0 {
        return;
    }

    let mut assistant_count = 0u32;
    for message in messages {
        let message_step = match message.role {
            ChatRole::User => assistant_count.saturating_add(1),
            ChatRole::Assistant => {
                assistant_count = assistant_count.saturating_add(1);
                assistant_count
            }
            ChatRole::System | ChatRole::Developer | ChatRole::Tool => continue,
        };
        if message_step == step_index && (overwrite || message.model.is_none()) {
            message.model = Some(model.to_string());
        }
    }
}
