//! Application contracts exposed through the compatibility facade.

pub use vw_api_app_identity::*;

mod actions;
pub use actions::*;

pub mod aggregation;

mod prompts;
pub use prompts::*;

pub mod data_management;
pub use data_management::*;
