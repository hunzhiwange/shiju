//! 本地主题磁盘仓库：按 `<root>/<id>/theme.json` 读写删除。

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use super::assets::{ThemeAssetKind, ThemeAssetSources, copy_theme_asset, relative_asset_name};
use super::model::ThemeDefinition;

/// 以给定 themes 根目录为范围的本地主题仓库。
#[derive(Debug, Clone)]
pub struct ThemeStore {
    root: PathBuf,
}

impl ThemeStore {
    /// 使用指定 themes 根目录构造仓库。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 列出根目录下所有可解析的主题定义。
    pub fn list(&self) -> Result<Vec<ThemeDefinition>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut themes = Vec::new();
        let entries = fs::read_dir(&self.root)
            .with_context(|| format!("failed to read themes root {}", self.root.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| {
                format!("failed to read entry under themes root {}", self.root.display())
            })?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if id == "overlays" {
                continue;
            }
            match self.load(id) {
                Ok(definition) => themes.push(definition),
                Err(err) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        theme_id = %id,
                        error = %err,
                        "skipping unreadable theme directory"
                    );
                }
            }
        }
        themes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(themes)
    }

    /// 读取指定 id 的主题定义。
    pub fn load(&self, id: &str) -> Result<ThemeDefinition> {
        let path = self.theme_json_path(id);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read theme file {}", path.display()))?;
        let definition: ThemeDefinition = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse theme file {}", path.display()))?;
        if definition.id != id {
            bail!("theme id mismatch: directory `{id}` but theme.json has `{}`", definition.id);
        }
        definition.validate()?;
        Ok(definition)
    }

    /// 主题目录下指定相对资源的绝对路径。
    pub fn theme_asset_path(&self, id: &str, relative_name: &str) -> PathBuf {
        self.theme_dir(id).join(relative_name)
    }

    /// 校验并保存主题定义到 `<id>/theme.json`（不复制外部图片源）。
    pub fn save(&self, definition: &ThemeDefinition) -> Result<()> {
        self.save_with_sources(definition, &ThemeAssetSources::default()).map(|_| ())
    }

    /// 校验定义，可选复制封面/背景源图，再写入 `theme.json`。
    ///
    /// 返回写入磁盘后的定义（`assets` 已更新为包内相对名）。
    /// 非法源图扩展名会在落盘前失败，不留下残缺主题目录。
    pub fn save_with_sources(
        &self,
        definition: &ThemeDefinition,
        sources: &ThemeAssetSources,
    ) -> Result<ThemeDefinition> {
        let mut normalized = definition.clone();
        normalized.normalize();
        normalized.validate()?;

        // 先校验源扩展名，避免创建目录后因非法格式留下残缺主题。
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

        let dir = self.theme_dir(&normalized.id);
        let theme_existed = dir.exists();
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create theme directory {}", dir.display()))?;

        let copy_result = (|| -> Result<()> {
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
                .context("failed to serialize theme definition")?;
            fs::write(&path, content)
                .with_context(|| format!("failed to write theme file {}", path.display()))?;
            Ok(())
        })();

        if let Err(err) = copy_result {
            if !theme_existed {
                let _ = fs::remove_dir_all(&dir);
            }
            return Err(err);
        }

        Ok(normalized)
    }

    /// 删除指定 id 的主题目录（含资源文件）。
    pub fn delete(&self, id: &str) -> Result<()> {
        let dir = self.theme_dir(id);
        if !dir.exists() {
            bail!("theme `{id}` not found");
        }
        fs::remove_dir_all(&dir)
            .with_context(|| format!("failed to delete theme directory {}", dir.display()))?;
        Ok(())
    }

    fn theme_dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    fn theme_json_path(&self, id: &str) -> PathBuf {
        self.theme_dir(id).join("theme.json")
    }
}
