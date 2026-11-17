//! Desktop chat projection and native speech facade.
#[cfg(test)]
pub(crate) fn speech_text(raw: &str) -> String {
    if raw.len() > 65_536 {
        return String::new();
    }
    crate::app::components::chat_panel::message_view::assistant_speech_text(raw)
}

#[cfg(test)]
#[path = "local_conversation_tests.rs"]
mod tests;

#[cfg(test)]
pub use desktop_voice::local_conversation::speak;
#[cfg(test)]
pub use desktop_voice::local_conversation::spoken_prefix;
