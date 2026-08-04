//! Desktop state for SHIJU applications.

mod launcher;
mod state;

pub(crate) use launcher::*;
pub(crate) use state::*;

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod launcher_tests;
