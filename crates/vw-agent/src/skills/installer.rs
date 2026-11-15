//! Compatibility export for the skill domain.

pub use agent_skills::installer::*;

#[cfg(test)]
#[path = "installer_tests.rs"]
mod installer_tests;
