//! 会话执行边界共享的协议契约。

mod events;
pub mod llm_raw;
mod prompt;
pub mod ui_types;

pub use events::*;
pub use prompt::{PromptEvent, PromptTermination};

#[cfg(test)]
mod llm_raw_tests;
