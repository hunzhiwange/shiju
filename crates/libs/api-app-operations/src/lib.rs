//! Application lifecycle, data management and prompt contracts.

pub mod actions;
pub mod prompts;

pub use actions::*;
pub use prompts::*;

#[cfg(test)]
mod actions_tests;
