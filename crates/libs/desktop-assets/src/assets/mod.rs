//! 资源管理模块
//!
//! 本模块负责管理和提供应用程序中使用的所有静态资源，包括：
//! - SVG 图标（通过 iced 的 svg::Handle）
//! - PNG 图片（通过 iced 的 image::Handle）
//! - Provider 提供商图标
//!
//! 所有资源在首次访问时通过 `Lazy` 静态变量延迟加载，
//! 并缓存在 `HashMap` 中以避免重复加载。

mod icon;
mod icon_actions;
mod icon_apps;
mod icon_canvas;
mod icon_files;
mod icon_ui;
mod images;
pub(crate) mod named_icon_generated {
    include!(concat!(env!("OUT_DIR"), "/named_icon_generated.rs"));
}
mod named_icons;
mod provider_icons;
mod svg_icons;

pub use icon::Icon;
pub use images::get_image;
pub use named_icons::{
    NamedIconFamily, canonical_named_icon_family, get_named_icon_image,
    get_named_icon_image_with_weight, named_icon_catalog, named_icon_family_json,
    named_icon_family_label,
};
pub use provider_icons::get_provider_icon;
pub use svg_icons::get_icon;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
