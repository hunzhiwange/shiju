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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tools_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_21_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_24_tests;
