//! 活动自定义主题的运行时皮肤快照（背景路径与 chrome）。

use std::path::PathBuf;

use anyhow::{Context, Result};

use super::model::{ThemeChrome, ThemeDefinition};
use super::store::ThemeStore;

/// 当前活动自定义主题的运行时皮肤。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveThemeSkin {
    pub theme_id: String,
    /// 背景图绝对路径；定义有资源但文件缺失时为 `None`。
    pub background_path: Option<PathBuf>,
    /// 封面绝对路径；缺失时为 `None`。
    pub cover_path: Option<PathBuf>,
    /// 聊天顶栏分区背景绝对路径；缺失时为 `None`。
    pub chat_header_path: Option<PathBuf>,
    /// 消息区分区背景绝对路径；缺失时为 `None`。
    pub chat_body_path: Option<PathBuf>,
    /// 输入区分区背景绝对路径；缺失时为 `None`。
    pub chat_input_path: Option<PathBuf>,
    pub chrome: ThemeChrome,
}

impl ActiveThemeSkin {
    /// 从主题定义与仓库解析绝对资源路径。
    pub fn from_definition(store: &ThemeStore, definition: &ThemeDefinition) -> Self {
        let background_path = absolute_asset_if_present(
            store,
            &definition.id,
            definition.assets.background.as_deref(),
        );
        let cover_path =
            absolute_asset_if_present(store, &definition.id, definition.assets.cover.as_deref());
        let chat_header_path = absolute_asset_if_present(
            store,
            &definition.id,
            definition.assets.chat_header.as_deref(),
        );
        let chat_body_path = absolute_asset_if_present(
            store,
            &definition.id,
            definition.assets.chat_body.as_deref(),
        );
        let chat_input_path = absolute_asset_if_present(
            store,
            &definition.id,
            definition.assets.chat_input.as_deref(),
        );

        if definition.assets.background.is_some() && background_path.is_none() {
            tracing::warn!(
                target: "vw_desktop",
                theme_id = %definition.id,
                "theme background asset is configured but file is missing; degrading to no wallpaper"
            );
        }
        if definition.assets.chat_header.is_some() && chat_header_path.is_none() {
            tracing::warn!(
                target: "vw_desktop",
                theme_id = %definition.id,
                "theme chat_header asset is configured but file is missing; degrading to no panel image"
            );
        }
        if definition.assets.chat_body.is_some() && chat_body_path.is_none() {
            tracing::warn!(
                target: "vw_desktop",
                theme_id = %definition.id,
                "theme chat_body asset is configured but file is missing; degrading to no panel image"
            );
        }
        if definition.assets.chat_input.is_some() && chat_input_path.is_none() {
            tracing::warn!(
                target: "vw_desktop",
                theme_id = %definition.id,
                "theme chat_input asset is configured but file is missing; degrading to no panel image"
            );
        }

        Self {
            theme_id: definition.id.clone(),
            background_path,
            cover_path,
            chat_header_path,
            chat_body_path,
            chat_input_path,
            chrome: definition.chrome.clone(),
        }
    }
}

fn absolute_asset_if_present(
    store: &ThemeStore,
    theme_id: &str,
    relative: Option<&str>,
) -> Option<PathBuf> {
    let relative = relative?;
    let path = store.theme_asset_path(theme_id, relative);
    path.is_file().then_some(path)
}

/// 加载指定 id 的活动皮肤快照。
pub fn resolve_active_theme_skin(store: &ThemeStore, theme_id: &str) -> Result<ActiveThemeSkin> {
    let definition = store
        .load(theme_id)
        .with_context(|| format!("failed to load theme `{theme_id}` for active skin"))?;
    Ok(ActiveThemeSkin::from_definition(store, &definition))
}

/// 按当前活动自定义主题 id 刷新或清空 App 上的皮肤缓存。
pub fn sync_active_theme_skin(app: &mut crate::app::App) {
    let Some(theme_id) = app.active_custom_theme_id.clone() else {
        app.active_theme_skin = None;
        return;
    };
    let Some(store) = super::theme_store_from_home() else {
        app.active_theme_skin = None;
        return;
    };
    match resolve_active_theme_skin(&store, &theme_id) {
        Ok(skin) => {
            app.active_theme_skin = Some(skin);
        }
        Err(err) => {
            tracing::warn!(
                target: "vw_desktop",
                theme_id = %theme_id,
                error = %err,
                "failed to resolve active theme skin; clearing wallpaper cache"
            );
            app.active_theme_skin = None;
        }
    }
}

/// 清空活动皮肤缓存（切换到内置主题时）。
pub fn clear_active_theme_skin(app: &mut crate::app::App) {
    app.active_theme_skin = None;
}

#[cfg(test)]
#[path = "active_tests.rs"]
mod active_tests;
