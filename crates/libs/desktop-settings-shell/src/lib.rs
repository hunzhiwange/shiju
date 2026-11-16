//! Settings navigation and layout, independent of desktop application state.
mod tabs;
pub use tabs::*;
mod field_scroll;
pub use desktop_settings_appearance::general_fields;
pub use field_scroll::reveal_field;
pub mod navigation;
pub const SETTINGS_CONTENT_SCROLL: &str = "system-settings-content-scroll";
pub const SETTINGS_SCROLL_GUTTER: u16 = 7;

mod layout;
pub use layout::*;

#[cfg(test)]
mod host_module_30_tests;

pub mod messages;

pub mod types;

pub mod search;

#[cfg(test)]
mod host_module_36_tests;


#[cfg(test)]
mod cron_render_test_support;
