//! 持久化并校验识矩技能包的服务端版本缓存。

use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use vw_config_types::paths::home_config_dir;

const CACHE_FILE_NAME: &str = "shiju-package-version.json";
const MAX_CACHE_BYTES: u64 = 4 * 1024;
const MAX_VERSION_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// 最近一次确认的服务端版本及检测时间。
pub(crate) struct PackageVersionCacheEntry {
    /// 经过格式校验的服务端版本。
    pub(crate) server_version: String,
    /// 非零 Unix 秒级检测时间。
    pub(crate) checked_at_unix: u64,
}

impl PackageVersionCacheEntry {
    pub(crate) fn new(server_version: String, checked_at_unix: u64) -> Result<Self> {
        if !valid_version(&server_version) {
            bail!("识矩版本缓存包含无效版本");
        }
        if checked_at_unix == 0 {
            bail!("识矩版本缓存包含无效检测时间");
        }
        Ok(Self { server_version, checked_at_unix })
    }

    fn validate(self) -> Result<Self> {
        Self::new(self.server_version, self.checked_at_unix)
    }
}

pub(crate) fn package_version_cache_path() -> Result<PathBuf> {
    let home =
        BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()).context("无法确定用户主目录")?;
    Ok(home_config_dir(home).join(CACHE_FILE_NAME))
}

/// 读取有大小上限的缓存；文件不存在时返回 `None`，格式异常则报错。
pub(crate) fn load_package_version_cache(path: &Path) -> Result<Option<PackageVersionCacheEntry>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("无法检查 {}", path.display())),
    };
    if !metadata.is_file() || metadata.len() > MAX_CACHE_BYTES {
        bail!("识矩版本缓存不是有效的小型文件");
    }
    let bytes = fs::read(path).with_context(|| format!("无法读取 {}", path.display()))?;
    let entry = serde_json::from_slice::<PackageVersionCacheEntry>(&bytes)
        .context("识矩版本缓存格式无效")?
        .validate()?;
    Ok(Some(entry))
}

/// 校验条目后通过同目录临时文件原子替换缓存。
pub(crate) fn write_package_version_cache(
    path: &Path,
    entry: &PackageVersionCacheEntry,
) -> Result<()> {
    PackageVersionCacheEntry::new(entry.server_version.clone(), entry.checked_at_unix)?;
    let parent = path.parent().context("识矩版本缓存路径缺少父目录")?;
    fs::create_dir_all(parent).with_context(|| format!("无法创建 {}", parent.display()))?;
    let payload = serde_json::to_vec(entry).context("无法序列化识矩版本缓存")?;
    let mut temporary =
        NamedTempFile::new_in(parent).with_context(|| format!("无法暂存 {}", path.display()))?;
    temporary
        .write_all(&payload)
        .and_then(|_| temporary.as_file().sync_all())
        .with_context(|| format!("无法写入 {}", path.display()))?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("无法原子更新 {}", path.display()))?;
    Ok(())
}

fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= MAX_VERSION_BYTES
        && version.split('.').all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        })
}
