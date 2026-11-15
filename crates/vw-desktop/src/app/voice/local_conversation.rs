//! Desktop chat projection and native speech facade.
pub use desktop_voice::local_conversation::{speak, spoken_prefix};
pub(crate) fn speech_text(raw: &str) -> String {
    if raw.len() > 65_536 {
        return String::new();
    }
    crate::app::components::chat_panel::message_view::assistant_speech_text(raw)
}

#[cfg(test)]
#[path = "local_conversation_tests.rs"]
mod tests;
