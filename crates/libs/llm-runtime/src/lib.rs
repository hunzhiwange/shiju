//! Shared LLM execution mechanics, with host scope supplied explicitly.
pub mod acp;
mod error;
pub use error::Error;
mod events;
pub use events::StreamEvent;
