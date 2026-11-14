//! 主题图片资源与本地 JSON 磁盘存储。

mod assets;
mod store;

pub use assets::{
    ThemeAssetKind, ThemeAssetSources, copy_theme_asset, relative_asset_name,
    supported_image_extension, validate_relative_asset_name,
};
pub use store::ThemeStore;

#[cfg(test)]
mod assets_tests;

#[cfg(test)]
mod store_tests;
