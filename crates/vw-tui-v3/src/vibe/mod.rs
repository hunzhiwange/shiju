//! VibeWindow-owned boundary for the isolated v3 TUI.

pub mod action;
pub mod event;
pub mod surface;

#[cfg(test)]
#[path = "surface_tests.rs"]
mod surface_tests;
