//! 主题包图片资源：扩展名校验、相对名生成与复制。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// 主题资源种类（封面、窗口背景或聊天分区图）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeAssetKind {
    Cover,
    Background,
    ChatHeader,
    ChatBody,
    ChatInput,
}

impl ThemeAssetKind {
    fn file_stem(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::Background => "background",
            Self::ChatHeader => "chat_header",
            Self::ChatBody => "chat_body",
            Self::ChatInput => "chat_input",
        }
    }
}

/// 保存主题时待复制的外部图片源路径。
#[derive(Debug, Clone, Default)]
pub struct ThemeAssetSources {
    pub cover: Option<PathBuf>,
    pub background: Option<PathBuf>,
    pub chat_header: Option<PathBuf>,
    pub chat_body: Option<PathBuf>,
    pub chat_input: Option<PathBuf>,
}

/// 若路径扩展名为支持的图片格式则返回小写扩展名，否则 `None`。
pub fn supported_image_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => Some(ext),
        _ => None,
    }
}

/// 按资源种类与源文件扩展名生成包内相对名（如 `cover.png`）。
pub fn relative_asset_name(kind: ThemeAssetKind, source: &Path) -> Result<String> {
    let Some(ext) = supported_image_extension(source) else {
        let display = source.extension().and_then(|e| e.to_str()).unwrap_or("(none)");
        bail!(
            "unsupported image extension `.{display}` for theme asset (allowed: png, jpg, jpeg, webp)"
        );
    };
    Ok(format!("{}.{}", kind.file_stem(), ext))
}

/// 将源图片复制进主题目录，返回写入的相对文件名。
pub fn copy_theme_asset(kind: ThemeAssetKind, source: &Path, theme_dir: &Path) -> Result<String> {
    if !source.is_file() {
        bail!("theme asset source is not a file: {}", source.display());
    }
    let relative = relative_asset_name(kind, source)?;
    let dest = resolve_theme_asset_path(theme_dir, &relative)?;
    fs::create_dir_all(theme_dir)
        .with_context(|| format!("failed to create theme directory {}", theme_dir.display()))?;
    fs::copy(source, &dest).with_context(|| {
        format!("failed to copy theme asset from {} to {}", source.display(), dest.display())
    })?;
    Ok(relative)
}

/// 在主题目录下解析已校验的相对资源名。
pub fn resolve_theme_asset_path(theme_dir: &Path, relative_name: &str) -> Result<PathBuf> {
    validate_relative_asset_name(relative_name)?;
    Ok(theme_dir.join(relative_name))
}

/// 相对名不得为空、含分隔符、`..` 或绝对路径。
pub fn validate_relative_asset_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("invalid asset name: expected non-empty relative filename");
    }
    if name.starts_with('/') || name.starts_with('\\') {
        bail!("invalid asset name: absolute paths are not allowed, got `{name}`");
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        bail!("invalid asset name: expected a single relative filename, got `{name}`");
    }
    Ok(())
}
