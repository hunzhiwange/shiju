//! 自定义主题定义、本地磁盘仓库与 Iced Theme 转换。
//!
//! 仅覆盖 Iced Palette 六色与本地 JSON 持久化，不包含市场、支付或布局特效。

mod active;
mod assets;
mod chrome_style;
mod convert;
mod model;
mod overlay;
mod pack;
mod panel_surface;
mod presets;
mod resolve;
mod store;

pub use active::{
    ActiveThemeSkin, clear_active_theme_skin, resolve_active_theme_skin, sync_active_theme_skin,
};
pub use assets::ThemeAssetSources;
pub use chrome_style::{
    ChromeStyleOverrides, DEFAULT_WALLPAPER_PANEL_ALPHA, input_surface_color, panel_surface_color,
    resolve_chrome_style, resolve_chrome_style_from_skin, resolve_chrome_style_from_theme,
};
pub use convert::to_iced_theme;
pub use model::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex};
pub use overlay::{
    builtin_overlay_exists, builtin_overlay_slug, delete_builtin_overlay, find_builtin_theme,
    load_builtin_overlay, overlay_theme_store, resolve_builtin_with_overlay, save_builtin_overlay,
    save_builtin_overlay_with_sources,
};
pub use pack::{
    export_theme_definition_to_pack, export_theme_dir_to_pack, import_theme_pack_into_store,
};
pub use panel_surface::{PanelImageFit, with_panel_background};
pub use presets::{
    EnsurePresetsReport, bundled_preset_is_modified, ensure_bundled_presets, is_bundled_preset_id,
    list_bundled_preset_ids, restore_bundled_preset,
};
pub use resolve::{
    CUSTOM_THEME_PREFIX, config_value_for_theme, current_themes_root, custom_theme_id_from_config,
    find_custom_theme_id, resolve_app_theme_name, theme_store_from_home, themes_root,
};
pub use store::ThemeStore;

#[cfg(test)]
pub use resolve::set_test_themes_root;

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;

#[cfg(test)]
#[path = "assets_tests.rs"]
mod assets_tests;

#[cfg(test)]
#[path = "convert_tests.rs"]
mod convert_tests;

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod resolve_tests;

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod overlay_tests;

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;

#[cfg(test)]
#[path = "presets_assets_tests.rs"]
mod presets_assets_tests;

#[cfg(test)]
#[path = "presets_tests.rs"]
mod presets_tests;
