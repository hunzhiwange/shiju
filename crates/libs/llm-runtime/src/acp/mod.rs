pub mod lifecycle;
pub mod fork_replay;
pub mod approval;
pub mod cache;
pub mod options;
pub mod config;
pub mod content;
pub mod controls;
fn acp_option_error(message: impl Into<String>) -> crate::Error {
    crate::Error::Api(message_types::AssistantError::Unknown { message: format!("acp option error: {}", message.into()) })
}

pub mod model_source;
pub mod replay;
pub mod recovery;
