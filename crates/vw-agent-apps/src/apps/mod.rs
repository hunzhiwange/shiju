//! Application catalog rules, independent of execution and installation state.
pub mod acp_projection;
#[cfg(test)]
mod acp_projection_tests;
pub mod catalog;
#[cfg(test)]
mod catalog_tests;
pub mod discovery;

pub mod lifecycle;
pub mod uninstall;
pub mod versions;

pub mod preferences;
#[cfg(test)]
mod preferences_tests;

pub mod launch;
