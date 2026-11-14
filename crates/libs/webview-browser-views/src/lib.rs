//! Browser view construction and native layout boundaries.

#![cfg(not(any(target_arch = "wasm32", coverage)))]

mod bounds;
mod content_view;
mod toolbar_view;

pub use bounds::to_wry_rect;
pub use content_view::build_content_view;
pub use toolbar_view::build_toolbar_view;

#[cfg(test)]
mod bounds_tests;
