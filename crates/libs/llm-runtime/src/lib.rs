//! Shared LLM execution mechanics, with host scope supplied explicitly.
pub mod acp;
#[cfg(not(target_arch = "wasm32"))]
pub mod aisdk;
pub mod logging;
pub mod messages;
pub mod types;
pub mod host;
pub mod stream;
pub mod catalog_error;
pub mod replay_safety;
pub mod retry_delay;
pub mod prompt;
pub mod options;
pub mod recovery_test_support;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_tools;
mod error;
pub use error::Error;
mod events;
pub use events::StreamEvent;
