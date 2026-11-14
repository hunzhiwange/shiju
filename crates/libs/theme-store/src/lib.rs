//! 主题图片资源、本地 JSON 磁盘存储与主题包导入导出。

mod assets;
mod pack;
mod store;

pub use assets::{
    ThemeAssetKind, ThemeAssetSources, copy_theme_asset, relative_asset_name,
    supported_image_extension, validate_relative_asset_name,
};
pub use pack::{
    export_theme_definition_to_pack, export_theme_dir_to_pack, import_theme_pack_into_store,
};
pub use store::ThemeStore;

#[cfg(test)]
mod pack_tests;

#[cfg(test)]
mod assets_tests;

#[cfg(test)]
mod store_tests;
