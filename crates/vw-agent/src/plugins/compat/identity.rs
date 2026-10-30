//! 稳定插件身份及原包之外的安装元数据边界。

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::super::{PluginIdentity, PluginInstallMetadata, PluginManifest};

impl PluginIdentity {
    /// 校验两个独立分段，创建冻结基线中的市场插件身份。
    pub fn marketplace(name: &str, marketplace: &str) -> Result<Self> {
        validate_segment(name, "plugin name", true)?;
        validate_segment(marketplace, "marketplace name", false)?;
        Ok(Self { name: name.to_owned(), marketplace: Some(marketplace.to_owned()) })
    }

    /// 解析精确的 `<plugin>@<marketplace>`，不添加名称别名。
    pub fn parse_marketplace(key: &str) -> Result<Self> {
        let (name, marketplace) =
            key.rsplit_once('@').context("invalid plugin key: expected <plugin>@<marketplace>")?;
        Self::marketplace(name, marketplace)
    }

    /// 返回用于发现、配置和生命周期操作的稳定标识。
    pub fn as_key(&self) -> String {
        match &self.marketplace {
            Some(marketplace) => format!("{}@{marketplace}", self.name),
            None => self.name.clone(),
        }
    }

    /// 返回原包中的插件名称。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回市场归属；本地插件没有推断出的市场。
    pub fn marketplace_name(&self) -> Option<&str> {
        self.marketplace.as_deref()
    }

    pub(crate) fn local(name: &str) -> Self {
        Self { name: name.to_owned(), marketplace: None }
    }
}

fn validate_segment(value: &str, kind: &str, allow_dots: bool) -> Result<()> {
    if value.is_empty() {
        bail!("invalid {kind}: must not be empty");
    }
    if allow_dots && (value.starts_with('.') || value.ends_with('.') || value.contains("..")) {
        bail!("invalid {kind}: dots must separate non-empty name segments");
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') || allow_dots && ch == '.')
    {
        bail!("invalid {kind}: unsupported characters");
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallRecord {
    schema_version: u32,
    name: String,
    marketplace: Option<String>,
    source: String,
    version: Option<String>,
}

/// sidecar 与原包是同级条目，清单和包内文件永不承载宿主状态。
pub(crate) fn metadata_path(plugin_dir: &Path) -> Result<PathBuf> {
    let mut name =
        plugin_dir.file_name().context("plugin directory must have a name")?.to_os_string();
    name.push(".install.json");
    Ok(plugin_dir.with_file_name(name))
}

pub(crate) fn read_install_metadata(
    plugin_dir: &Path,
    manifest: &PluginManifest,
) -> Result<Option<PluginInstallMetadata>> {
    let path = metadata_path(plugin_dir)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if plugin_dir
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.starts_with(&format!("{}@", manifest.name)))
            {
                bail!(
                    "marketplace plugin is missing installation metadata: {}",
                    plugin_dir.display()
                );
            }
            return Ok(None);
        }
        Err(error) => return Err(error).context("failed to inspect plugin installation metadata"),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("plugin installation metadata must be a regular file: {}", path.display());
    }
    if metadata.len() > 64 * 1024 {
        bail!("plugin installation metadata is too large");
    }
    let record: InstallRecord = serde_json::from_slice(
        &fs::read(&path).context("failed to read plugin installation metadata")?,
    )
    .context("invalid plugin installation metadata")?;
    if record.schema_version != 1 || record.source.trim().is_empty() {
        bail!("unsupported or incomplete plugin installation metadata");
    }
    let identity = match &record.marketplace {
        Some(marketplace) => PluginIdentity::marketplace(&record.name, marketplace)?,
        None => PluginIdentity::local(&record.name),
    };
    if identity.name() != manifest.name
        || record.version != manifest.version
        || plugin_dir.file_name() != Some(OsStr::new(&identity.as_key()))
    {
        bail!("plugin installation metadata identity or version mismatch");
    }
    Ok(Some(PluginInstallMetadata { identity, source: record.source, version: record.version }))
}

pub(crate) fn write_install_metadata(
    plugin_dir: &Path,
    metadata: &PluginInstallMetadata,
) -> Result<()> {
    let path = metadata_path(plugin_dir)?;
    let record = InstallRecord {
        schema_version: 1,
        name: metadata.identity.name().to_owned(),
        marketplace: metadata.identity.marketplace_name().map(str::to_owned),
        source: metadata.source.clone(),
        version: metadata.version.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&record)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .context("failed to create plugin installation metadata")?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(error).context("failed to persist plugin installation metadata");
    }
    Ok(())
}
