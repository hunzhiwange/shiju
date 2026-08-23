//! Gateway-backed terminal UI composition and runtime boundary.

use vw_tui::V3Surface;

pub(crate) mod app;
pub(crate) mod bridge;
pub(crate) mod controller;
pub(crate) mod editor;
pub(crate) mod model;
pub(crate) mod paste;
pub(crate) mod runtime;
pub(crate) mod state;
pub(crate) mod vim;

#[cfg(not(test))]
pub(crate) use app::run_tui;

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;

#[cfg(test)]
#[path = "parity_tests.rs"]
mod parity_tests;

/// Constructs the independent surface without selecting it as the default TUI.
pub(crate) fn build_surface() -> V3Surface {
    V3Surface::new()
}
