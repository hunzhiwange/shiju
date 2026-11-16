//! Shared LLM execution mechanics, with host scope supplied explicitly.
pub mod acp;
#[cfg(not(target_arch = "wasm32"))]
pub mod aisdk;
pub mod logging;
pub mod messages;
mod error;
pub use error::Error;
mod events;
pub use events::StreamEvent;
