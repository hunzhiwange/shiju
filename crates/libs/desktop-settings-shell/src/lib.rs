//! Settings navigation and layout, independent of desktop application state.
mod tabs;
pub use tabs::*;
mod field_scroll;
pub use field_scroll::reveal_field;
pub mod general_fields;
pub mod navigation;
pub const SETTINGS_CONTENT_SCROLL: &str = "system-settings-content-scroll";
pub const SETTINGS_SCROLL_GUTTER: u16 = 7;
/// 常规配置页内的一级标签。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GeneralSettingsTab {
    #[default]
    Interface,
    Terminal,
    Editor,
    Themes,
    Companion,
}


mod layout;
pub use layout::*;
