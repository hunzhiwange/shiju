//! 本地插件清单的静态路径审计辅助逻辑。
//!
//! 解析前拒绝符号链接和越界路径，确保插件声明只访问其规范化根目录。

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::types::PluginPath;

/// 规范化插件根目录，并拒绝符号链接或非目录入口。
pub(super) fn canonical_plugin_root(plugin_root: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(plugin_root).with_context(|| {
        format!("failed to read plugin root metadata: {}", plugin_root.display())
    })?;
    if metadata.file_type().is_symlink() {
        bail!("Plugin root must not be a symlink: {}", plugin_root.display());
    }
    if !metadata.is_dir() {
        bail!("Plugin root must be a directory: {}", plugin_root.display());
    }

    plugin_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize plugin root: {}", plugin_root.display()))
}

/// 返回通过逐级符号链接检查的插件清单路径。
pub(super) fn plugin_manifest_path(canonical_root: &Path) -> Result<PathBuf> {
    let relative = Path::new(".vibewindow-plugin").join("plugin.json");
    ensure_no_symlink_components(canonical_root, &relative, "plugin.json")?;
    let path = canonical_root.join(&relative);
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("plugin manifest not found: {}", path.display()))?;
    if !canonical_path.starts_with(canonical_root) {
        bail!("Plugin manifest path escapes plugin root: {}", path.display());
    }
    if !canonical_path.is_file() {
        bail!("Plugin manifest must be a file: {}", path.display());
    }
    Ok(canonical_path)
}

/// 将清单中的相对组件路径解析到插件根目录内。
///
/// 绝对路径、父目录跳转、符号链接组件和根目录逃逸都会被明确拒绝。
pub(super) fn resolve_component_path(
    canonical_root: &Path,
    field: &str,
    raw: &str,
) -> Result<PluginPath> {
    if !raw.starts_with("./") {
        bail!("Plugin manifest field {field} path must start with './': {raw}");
    }

    let relative = Path::new(raw);
    if relative.is_absolute() {
        bail!("Plugin manifest field {field} path must be relative to plugin root: {raw}");
    }

    ensure_no_symlink_components(canonical_root, relative, field)?;

    let path = canonical_root.join(relative);
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("Plugin manifest field {field} path does not exist: {raw}"))?;
    if !canonical_path.starts_with(canonical_root) {
        bail!("Plugin manifest field {field} path escapes plugin root: {raw}");
    }

    Ok(PluginPath { raw: raw.to_string(), canonical_path })
}

fn ensure_no_symlink_components(canonical_root: &Path, relative: &Path, field: &str) -> Result<()> {
    let mut current = canonical_root.to_path_buf();

    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                current.pop();
            }
            Component::Normal(part) => {
                current.push(part);
                if let Ok(metadata) = fs::symlink_metadata(&current) {
                    if metadata.file_type().is_symlink() {
                        bail!(
                            "Plugin manifest field {field} path includes symlink: {}",
                            current.display()
                        );
                    }
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "Plugin manifest field {field} path must be relative: {}",
                    relative.display()
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "audit_tests.rs"]
mod audit_tests;
