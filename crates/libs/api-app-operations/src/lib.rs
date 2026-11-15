//! Application lifecycle, data management and prompt contracts.

pub mod actions;
pub mod data_management;
pub mod prompts;

pub use actions::*;
pub use data_management::*;
pub use prompts::*;

#[cfg(test)]
mod actions_tests;
#[cfg(test)]
mod data_management_tests;
