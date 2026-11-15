//! Application failover policy and wire contracts; no runtime execution.

pub mod agent_apps_high_availability;
pub use agent_apps_high_availability::*;

#[cfg(test)]
mod agent_apps_high_availability_tests;
