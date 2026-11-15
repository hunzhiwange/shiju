//! 内置主题本地覆盖层：读写 `themes/overlays/<slug>/theme.json`，不修改 Iced 内置本体。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use iced::Theme;

use super::assets::{ThemeAssetKind, ThemeAssetSources, copy_theme_asset, relative_asset_name};
use super::convert::to_iced_theme;
use super::model::ThemeDefinition;
use super::store::ThemeStore;

/// 由内置主题显示名派生稳定 slug，满足主题 id 校验。
pub fn builtin_overlay_slug(builtin_name: &str) -> String {
    let mut out = String::new();
    let mut pending_sep: Option<char> = None;
    for c in builtin_name.chars() {
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            if let Some(sep) = pending_sep.take() {
                if !out.is_empty() {
                    out.push(sep);
                }
            }
            out.push(lower);
        } else if lower == '_' {
            pending_sep = Some('_');
        } else if c.is_whitespace() || lower == '-' {
            pending_sep = Some('-');
        }
    }
    if out.is_empty() {
        return "theme".to_string();
    }
    out.chars().take(64).collect()
}

/// 覆盖层目录：`<themes_root>/overlays/<slug>`。
pub fn overlay_dir(themes_root: impl AsRef<Path>, builtin_name: &str) -> PathBuf {
    themes_root.as_ref().join("overlays").join(builtin_overlay_slug(builtin_name))
}

/// 覆盖层是否存在（含可读 `theme.json`）。
pub fn builtin_overlay_exists(themes_root: impl AsRef<Path>, builtin_name: &str) -> bool {
    overlay_dir(themes_root, builtin_name).join("theme.json").is_file()
}

/// 加载内置覆盖；不存在时返回 `Ok(None)`。
pub fn load_builtin_overlay(
    themes_root: impl AsRef<Path>,
    builtin_name: &str,
) -> Result<Option<ThemeDefinition>> {
    let slug = builtin_overlay_slug(builtin_name);
    let path = overlay_dir(&themes_root, builtin_name).join("theme.json");
    if !path.is_file() {
        return Ok(None);
    }
    let store = ThemeStore::new(themes_root.as_ref().join("overlays"));
    let definition = store.load(&slug)?;
    Ok(Some(definition))
}

/// 保存内置覆盖层（校验后写入 `overlays/<slug>/theme.json`）。
pub fn save_builtin_overlay(
    themes_root: impl AsRef<Path>,
    builtin_name: &str,
    definition: &ThemeDefinition,
) -> Result<ThemeDefinition> {
    save_builtin_overlay_with_sources(
        themes_root,
        builtin_name,
        definition,
        &ThemeAssetSources::default(),
    )
}

/// 保存内置覆盖层，可选复制封面/背景源图。
pub fn save_builtin_overlay_with_sources(
    themes_root: impl AsRef<Path>,
    builtin_name: &str,
    definition: &ThemeDefinition,
    sources: &ThemeAssetSources,
) -> Result<ThemeDefinition> {
    let slug = builtin_overlay_slug(builtin_name);
    let mut normalized = definition.clone();
    normalized.id = slug.clone();
    if normalized.name.trim().is_empty() {
        normalized.name = builtin_name.to_string();
    }
    normalized.normalize();
    normalized.validate()?;

    if let Some(cover) = sources.cover.as_deref() {
        let _ = relative_asset_name(ThemeAssetKind::Cover, cover)?;
    }
    if let Some(background) = sources.background.as_deref() {
        let _ = relative_asset_name(ThemeAssetKind::Background, background)?;
    }
    if let Some(chat_header) = sources.chat_header.as_deref() {
        let _ = relative_asset_name(ThemeAssetKind::ChatHeader, chat_header)?;
    }
    if let Some(chat_body) = sources.chat_body.as_deref() {
        let _ = relative_asset_name(ThemeAssetKind::ChatBody, chat_body)?;
    }
    if let Some(chat_input) = sources.chat_input.as_deref() {
        let _ = relative_asset_name(ThemeAssetKind::ChatInput, chat_input)?;
    }

    let dir = overlay_dir(&themes_root, builtin_name);
    let existed = dir.exists();
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create overlay directory {}", dir.display()))?;

    let write_result = (|| -> Result<()> {
        if let Some(cover) = sources.cover.as_deref() {
            let relative = copy_theme_asset(ThemeAssetKind::Cover, cover, &dir)?;
            normalized.assets.cover = Some(relative);
        }
        if let Some(background) = sources.background.as_deref() {
            let relative = copy_theme_asset(ThemeAssetKind::Background, background, &dir)?;
            normalized.assets.background = Some(relative);
        }
        if let Some(chat_header) = sources.chat_header.as_deref() {
            let relative = copy_theme_asset(ThemeAssetKind::ChatHeader, chat_header, &dir)?;
            normalized.assets.chat_header = Some(relative);
        }
        if let Some(chat_body) = sources.chat_body.as_deref() {
            let relative = copy_theme_asset(ThemeAssetKind::ChatBody, chat_body, &dir)?;
            normalized.assets.chat_body = Some(relative);
        }
        if let Some(chat_input) = sources.chat_input.as_deref() {
            let relative = copy_theme_asset(ThemeAssetKind::ChatInput, chat_input, &dir)?;
            normalized.assets.chat_input = Some(relative);
        }
        normalized.validate()?;

        let path = dir.join("theme.json");
        let content = serde_json::to_string_pretty(&normalized)
            .context("failed to serialize builtin overlay")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write overlay file {}", path.display()))?;
        Ok(())
    })();

    if let Err(err) = write_result {
        if !existed {
            let _ = fs::remove_dir_all(&dir);
        }
        return Err(err);
    }

    Ok(normalized)
}

/// 删除内置覆盖层（恢复默认）；目录不存在时视为已恢复。
pub fn delete_builtin_overlay(themes_root: impl AsRef<Path>, builtin_name: &str) -> Result<()> {
    let dir = overlay_dir(themes_root, builtin_name);
    if !dir.exists() {
        return Ok(());
    }
    fs::remove_dir_all(&dir)
        .with_context(|| format!("failed to delete overlay directory {}", dir.display()))?;
    Ok(())
}

/// 覆盖层资源仓库（根为 `themes/overlays`）。
pub fn overlay_theme_store(themes_root: impl AsRef<Path>) -> ThemeStore {
    ThemeStore::new(themes_root.as_ref().join("overlays"))
}

/// 将内置主题与本地覆盖合并为可应用的 Iced Theme。
///
/// 无覆盖时返回原内置枚举值；有覆盖时返回同名 `Theme::Custom`（配置仍用内置显示名）。
pub fn resolve_builtin_with_overlay(builtin: &Theme, themes_root: Option<&Path>) -> Result<Theme> {
    if matches!(builtin, Theme::Custom(_)) {
        bail!("expected a builtin iced theme, got custom");
    }
    let Some(root) = themes_root else {
        return Ok(builtin.clone());
    };
    let name = builtin.to_string();
    let Some(mut definition) = load_builtin_overlay(root, &name)? else {
        return Ok(builtin.clone());
    };
    // 保持显示名与内置一致，便于配置值与画廊匹配。
    definition.name = name;
    to_iced_theme(&definition)
}

/// 若 `name` 对应内置主题，返回该主题引用。
pub fn find_builtin_theme(name: &str) -> Option<&'static Theme> {
    Theme::ALL.iter().find(|theme| theme.to_string() == name)
}
