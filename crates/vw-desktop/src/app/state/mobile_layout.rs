//! Compatibility export for the navigation-owned responsive layout state.
pub use desktop_navigation::mobile_layout::{LayoutMode, LayoutPreference, MobileLayoutState};
#[cfg(test)]
#[path = "mobile_layout_tests.rs"]
mod mobile_layout_tests;
