//! Construction and runtime boundary for the gateway-backed TUI v3.

use vw_tui_v3::V3Surface;

pub(crate) mod app;
pub(crate) mod bridge;

#[cfg(not(test))]
pub(crate) use app::run_tui_v3;

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;

#[cfg(test)]
#[path = "parity_tests.rs"]
mod parity_tests;

/// Constructs the independent v3 surface without selecting it as the default TUI.
pub(crate) fn build_surface() -> V3Surface {
    V3Surface::new()
}
