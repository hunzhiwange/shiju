//! 本地插件安装及启用、停用、移除等生命周期文件操作。

use anyhow::{Context, Result, bail};
#[cfg(test)]
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use super::compat::identity::{metadata_path, read_install_metadata};
use super::{PluginIdentity, PluginManifest, load_plugin_manifest};

const DISABLED_MARKER_SUFFIX: &str = ".disabled";

#[cfg(test)]
thread_local! {
    static NEXT_POST_COPY_AUDIT_RESULT: RefCell<Option<Result<PluginManifest, String>>> =
        const { RefCell::new(None) };
    static NEXT_POST_COPY_OBSERVER: RefCell<Option<PostCopyObserver>> =
        const { RefCell::new(None) };
}

#[cfg(test)]
type PostCopyObserver = Box<dyn FnOnce(&Path)>;

/// Result of a local plugin installation.
#[derive(Debug, Clone, PartialEq)]
/// 插件安装成功后返回的规范化清单与最终目录。
pub struct PluginInstallResult {
    pub installed_dir: PathBuf,
    pub manifest: PluginManifest,
    pub identity: PluginIdentity,
}

/// Return the plugin installation directory for a loaded runtime config.
/// 根据配置解析本地插件安装根目录。
pub(crate) fn install_dir_from_config(config: &crate::config::Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
        .join("plugins")
}

/// Install a plugin package from a local directory source.
/// 从本地目录安全复制并安装插件。
///
/// 在同文件系统暂存并审计原包与包外来源记录，完成后禁止覆盖地发布。
pub fn install_plugin_from_source(source: &str, plugins_dir: &Path) -> Result<PluginInstallResult> {
    install_plugin(source, plugins_dir, None)
}

/// 从已物化的市场包安装，市场归属与原包清单分开保存。
pub fn install_plugin_from_marketplace(
    source: &str,
    plugins_dir: &Path,
    marketplace: &str,
) -> Result<PluginInstallResult> {
    install_plugin(source, plugins_dir, Some(marketplace))
}

fn install_plugin(
    source: &str,
    plugins_dir: &Path,
    marketplace: Option<&str>,
) -> Result<PluginInstallResult> {
    let source = source.trim();
    if source.is_empty() {
        bail!("Plugin source cannot be empty");
    }
    if is_unsupported_remote_source(source) {
        bail!("Plugin source is not supported yet: {source}");
    }

    let source_path = PathBuf::from(source);
    if !source_path.exists() {
        bail!("Plugin source does not exist: {source}");
    }

    let source_manifest = load_plugin_manifest(&source_path)?;
    validate_plugin_name(&source_manifest.name)?;
    let identity = match marketplace {
        Some(marketplace) => PluginIdentity::marketplace(&source_manifest.name, marketplace)?,
        None => PluginIdentity::local(&source_manifest.name),
    };
    super::compat::transaction_install::install_package(&source_manifest, identity, plugins_dir)
}

pub(super) fn observe_post_copy(plugin_dir: &Path) {
    #[cfg(test)]
    if let Some(observer) = NEXT_POST_COPY_OBSERVER.with(|slot| slot.borrow_mut().take()) {
        observer(plugin_dir);
    }
    let _ = plugin_dir;
}

#[cfg(test)]
pub(super) fn set_next_post_copy_observer_for_test(observer: impl FnOnce(&Path) + 'static) {
    NEXT_POST_COPY_OBSERVER.with(|slot| *slot.borrow_mut() = Some(Box::new(observer)));
}

pub(super) fn load_plugin_manifest_for_post_copy_audit(
    plugin_dir: &Path,
) -> Result<PluginManifest> {
    #[cfg(test)]
    if let Some(result) = NEXT_POST_COPY_AUDIT_RESULT.with(|slot| slot.borrow_mut().take()) {
        return result.map_err(anyhow::Error::msg);
    }

    load_plugin_manifest(plugin_dir)
}

#[cfg(test)]
pub(super) fn set_next_post_copy_audit_result_for_test(result: Result<PluginManifest, String>) {
    NEXT_POST_COPY_AUDIT_RESULT.with(|slot| {
        *slot.borrow_mut() = Some(result);
    });
}

/// Enable or disable an installed plugin by toggling its marker file.
/// 写入或移除禁用标记以切换已安装插件状态。
pub fn set_plugin_enabled(plugins_dir: &Path, name: &str, enabled: bool) -> Result<()> {
    let plugin_dir = plugin_dir_for_name(plugins_dir, name)?;
    ensure_installed_plugin_dir(&plugin_dir, name)?;
    let marker = disabled_marker_path(plugins_dir, name);

    if enabled {
        match fs::remove_file(&marker) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to remove {}", marker.display()));
            }
        }
    } else {
        match fs::OpenOptions::new().write(true).create_new(true).open(&marker) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&marker)
                    .with_context(|| format!("failed to inspect {}", marker.display()))?;
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    bail!("plugin disabled marker must be a regular file: {}", marker.display());
                }
            }
            Err(error) => {
                return Err(error).with_context(|| format!("failed to write {}", marker.display()));
            }
        }
        super::compat::mcp_connection_sessions::close_plugin(&plugin_dir);
    }

    Ok(())
}

/// Remove an installed plugin and its disabled marker.
/// 删除经过名称与目录归属校验的已安装插件。
pub fn remove_installed_plugin(plugins_dir: &Path, name: &str) -> Result<()> {
    let plugin_dir = plugin_dir_for_name(plugins_dir, name)?;
    ensure_installed_plugin_dir(&plugin_dir, name)?;
    super::compat::mcp_connection_sessions::close_plugin(&plugin_dir);
    fs::remove_dir_all(&plugin_dir)
        .with_context(|| format!("failed to remove plugin: {}", plugin_dir.display()))?;

    let metadata = metadata_path(&plugin_dir)?;
    match fs::remove_file(&metadata) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("failed to remove plugin installation metadata"),
    }

    let marker = disabled_marker_path(plugins_dir, name);
    match fs::remove_file(&marker) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("failed to remove {}", marker.display()));
        }
    }

    Ok(())
}

/// 返回插件禁用标记的稳定路径。
pub(crate) fn disabled_marker_path(plugins_dir: &Path, name: &str) -> PathBuf {
    plugins_dir.join(format!("{name}{DISABLED_MARKER_SUFFIX}"))
}

fn is_unsupported_remote_source(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("git://")
        || lower.starts_with("ssh://")
        || lower.starts_with("git+")
        || lower.starts_with("git@")
        || lower.starts_with("npm:")
        || lower.starts_with("marketplace:")
}

pub(super) fn ensure_install_root(plugins_dir: &Path) -> Result<()> {
    match fs::symlink_metadata(plugins_dir) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!("Plugin install directory must not be a symlink: {}", plugins_dir.display());
            }
            if !metadata.is_dir() {
                bail!("Plugin install path must be a directory: {}", plugins_dir.display());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(plugins_dir)
                .with_context(|| format!("failed to create {}", plugins_dir.display()))?;
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect {}", plugins_dir.display()));
        }
    }

    Ok(())
}

fn plugin_dir_for_name(plugins_dir: &Path, name: &str) -> Result<PathBuf> {
    validate_plugin_name(name)?;
    Ok(plugins_dir.join(name))
}

fn validate_plugin_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        bail!("Invalid plugin name: {name}");
    }
    Ok(())
}

fn ensure_installed_plugin_dir(plugin_dir: &Path, name: &str) -> Result<()> {
    let metadata =
        fs::symlink_metadata(plugin_dir).with_context(|| format!("Plugin not found: {name}"))?;
    if metadata.file_type().is_symlink() {
        bail!("Installed plugin must not be a symlink: {}", plugin_dir.display());
    }
    if !metadata.is_dir() {
        bail!("Installed plugin must be a directory: {}", plugin_dir.display());
    }

    let manifest = load_plugin_manifest(plugin_dir)?;
    let metadata = read_install_metadata(plugin_dir, &manifest)?;
    let identity = metadata
        .map(|metadata| metadata.identity)
        .unwrap_or_else(|| PluginIdentity::local(&manifest.name));
    if identity.as_key() != name {
        bail!("Installed plugin name mismatch: requested '{}', found '{}'", name, manifest.name);
    }

    Ok(())
}

pub(super) fn copy_dir_recursive_secure(src: &Path, dest: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(src)
        .with_context(|| format!("failed to read metadata for {}", src.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("Refusing to copy symlinked plugin source path: {}", src.display());
    }
    if !metadata.is_dir() {
        bail!("Plugin source must be a directory: {}", src.display());
    }

    fs::create_dir_all(dest)
        .with_context(|| format!("failed to create destination {}", dest.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("failed to read {}", src.display()))? {
        let entry = entry.with_context(|| format!("failed to read entry in {}", src.display()))?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let metadata = fs::symlink_metadata(&src_path)
            .with_context(|| format!("failed to read metadata for {}", src_path.display()))?;

        if metadata.file_type().is_symlink() {
            bail!("Refusing to copy symlink within plugin source: {}", src_path.display());
        }
        if metadata.is_dir() {
            copy_dir_recursive_secure(&src_path, &dest_path)?;
        } else if metadata.is_file() {
            fs::copy(&src_path, &dest_path).with_context(|| {
                format!(
                    "failed to copy plugin file from {} to {}",
                    src_path.display(),
                    dest_path.display()
                )
            })?;
        } else {
            bail!("Unsupported plugin source entry: {}", src_path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
pub(super) fn copy_dir_recursive_secure_for_test(src: &Path, dest: &Path) -> Result<()> {
    copy_dir_recursive_secure(src, dest)
}
