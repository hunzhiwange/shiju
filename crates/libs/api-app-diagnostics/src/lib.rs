//! Application diagnostic wire contracts; no probe execution.

pub mod agent_apps_diagnostics;
pub use agent_apps_diagnostics::*;

#[cfg(test)]
mod agent_apps_diagnostics_tests;
