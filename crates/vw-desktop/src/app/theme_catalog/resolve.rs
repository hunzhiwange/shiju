//! 将配置中的主题名解析为 Iced Theme，并生成可持久化的配置值。
//!
//! 内置主题使用 `Theme::to_string()`；自定义主题使用 `custom:<id>`。

use std::path::{Path, PathBuf};

use iced::Theme;

use super::convert::to_iced_theme;
use super::overlay::resolve_builtin_with_overlay;
use super::store::ThemeStore;

/// 自定义主题配置前缀。
pub const CUSTOM_THEME_PREFIX: &str = "custom:";

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static TEST_THEMES_ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// 返回 `home_config_dir/themes` 路径。
pub fn themes_root(home: impl AsRef<Path>) -> PathBuf {
    vw_config_types::paths::home_config_dir(home).join("themes")
}

/// 测试用：覆盖默认 themes 根目录；传入 `None` 恢复。
#[cfg(test)]
pub fn set_test_themes_root(root: Option<PathBuf>) {
    TEST_THEMES_ROOT.with(|slot| {
        *slot.borrow_mut() = root;
    });
}

/// 构造当前用户下的主题仓库；无法解析 home 时返回 `None`。
pub fn theme_store_from_home() -> Option<ThemeStore> {
    current_themes_root().map(ThemeStore::new)
}

/// 当前 themes 根目录（测试时可被 `set_test_themes_root` 覆盖）。
pub fn current_themes_root() -> Option<PathBuf> {
    #[cfg(test)]
    {
        if let Some(root) = TEST_THEMES_ROOT.with(|slot| slot.borrow().clone()) {
            return Some(root);
        }
    }

    let home = directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))?;
    Some(themes_root(home))
}

/// 解析配置中的主题名。
///
/// 顺序：先匹配 `Theme::ALL`（并合并本地内置覆盖）；再解析 `custom:<id>` 并从仓库加载；否则返回 `None`。
pub fn resolve_app_theme_name(name: &str, store: Option<&ThemeStore>) -> Option<Theme> {
    if let Some(theme) = Theme::ALL.iter().find(|theme| theme.to_string() == name).cloned() {
        let root = current_themes_root();
        return match resolve_builtin_with_overlay(&theme, root.as_deref()) {
            Ok(resolved) => Some(resolved),
            Err(err) => {
                tracing::warn!(
                    target: "vw_desktop",
                    theme = %name,
                    error = %err,
                    "failed to apply builtin theme overlay; falling back to iced builtin"
                );
                Some(theme)
            }
        };
    }

    let Some(id) = name.strip_prefix(CUSTOM_THEME_PREFIX) else {
        return None;
    };
    if id.is_empty() {
        return None;
    }
    let Some(store) = store else {
        tracing::warn!(
            target: "vw_desktop",
            theme = %name,
            "custom theme requested but theme store is unavailable"
        );
        return None;
    };

    match store.load(id) {
        Ok(definition) => match to_iced_theme(&definition) {
            Ok(theme) => Some(theme),
            Err(err) => {
                tracing::warn!(
                    target: "vw_desktop",
                    theme_id = %id,
                    error = %err,
                    "failed to convert custom theme"
                );
                None
            }
        },
        Err(err) => {
            tracing::warn!(
                target: "vw_desktop",
                theme_id = %id,
                error = %err,
                "failed to load custom theme"
            );
            None
        }
    }
}

/// 生成写入系统设置的主题配置值。
///
/// 自定义主题必须带上已知 id，写成 `custom:<id>`；内置主题仍用显示名。
pub fn config_value_for_theme(theme: &Theme, custom_id: Option<&str>) -> String {
    if matches!(theme, Theme::Custom(_)) {
        if let Some(id) = custom_id.filter(|id| !id.is_empty()) {
            return format!("{CUSTOM_THEME_PREFIX}{id}");
        }
    }
    theme.to_string()
}

/// 在仓库中按名称与调色板匹配自定义主题 id。
pub fn find_custom_theme_id(theme: &Theme, store: &ThemeStore) -> Option<String> {
    if !matches!(theme, Theme::Custom(_)) {
        return None;
    }
    let definitions = match store.list() {
        Ok(definitions) => definitions,
        Err(err) => {
            tracing::warn!(
                target: "vw_desktop",
                error = %err,
                "failed to list themes while resolving custom id"
            );
            return None;
        }
    };
    let target_name = theme.to_string();
    let target_palette = theme.palette();
    for definition in definitions {
        let Ok(candidate) = to_iced_theme(&definition) else {
            continue;
        };
        if candidate.to_string() == target_name && candidate.palette() == target_palette {
            return Some(definition.id);
        }
    }
    None
}

/// 若配置值是 `custom:<id>`，提取 id。
pub fn custom_theme_id_from_config(name: &str) -> Option<&str> {
    name.strip_prefix(CUSTOM_THEME_PREFIX).filter(|id| !id.is_empty())
}
