//! Application contracts exposed through the compatibility facade.

pub use vw_api_app_identity::*;

mod actions;
pub use actions::*;
#[cfg(test)]
mod actions_tests;

pub mod aggregation;
#[cfg(test)]
mod aggregation_tests;

mod prompts;
pub use prompts::*;

mod data_management;
pub use data_management::*;
#[cfg(test)]
mod data_management_tests;
