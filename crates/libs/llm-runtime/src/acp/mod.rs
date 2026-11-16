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
pub mod host;
pub mod session;
pub mod request;
pub mod updates;
use crate::Error;
fn to_api_error(err: impl std::fmt::Display) -> Error {
    Error::Api(message_types::AssistantError::Unknown { message: format!("acp: {err}") })
}
pub fn is_acp_session_changed_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("acp session changed:")
}
pub fn is_acp_session_changed_error(err: &Error) -> bool {
    match err {
        Error::Api(message_types::AssistantError::Unknown { message }) => {
            is_acp_session_changed_message(message)
        }
        Error::Api(message_types::AssistantError::APIError { message, .. }) => {
            is_acp_session_changed_message(message)
        }
        _ => false,
    }
}
