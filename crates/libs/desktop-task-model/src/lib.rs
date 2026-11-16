//! 任务模型、生命周期状态机与活动投影。
pub mod models;
pub mod workflow;
pub mod agent_activity;
pub use models::*;
pub use workflow::*;

#[cfg(test)]
mod desktop_split_01_tests;

#[cfg(test)]
mod desktop_split_20_tests;

pub mod i18n;
#[cfg(test)]
mod host_module_39_tests;
