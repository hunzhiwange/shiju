//! 应用级目录与缓存版本管理。
//!
//! 本模块统一管理 provider 解析相关的 home、data、cache 目录，并在启动时
//! 基于缓存版本号执行一次性清理，避免旧缓存结构影响新版本行为。

use once_cell::sync::Lazy;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

/// 应用目录名。
pub const APP: &str = vw_config_types::paths::APP_DIR_NAME;
/// 缓存目录版本号；变更时会触发缓存清理。
pub const CACHE_VERSION: &str = "21";

/// 运行时会用到的全局路径集合。
///
/// 这些路径会被认证信息读取、模型缓存与其他本地状态共享使用。
#[derive(Debug, Clone)]
pub struct GlobalPaths {
    /// 用户主目录。
    pub home: PathBuf,
    /// 应用数据目录。
    pub data: PathBuf,
    /// 应用缓存目录。
    pub cache: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
static PATHS: Lazy<GlobalPaths> = Lazy::new(|| {
    let base = directories::BaseDirs::new();
    let home = std::env::var_os("VIBEWINDOW_TEST_HOME")
        .map(PathBuf::from)
        .or_else(|| base.as_ref().map(|b| b.home_dir().to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let data = base
        .as_ref()
        .map(|b| b.data_dir().join(APP))
        .unwrap_or_else(|| home.join(".local").join("share").join(APP));
    let cache = base
        .as_ref()
        .map(|b| b.cache_dir().join(APP))
        .unwrap_or_else(|| home.join(".cache").join(APP));

    let _ = std::fs::create_dir_all(&data);
    let _ = std::fs::create_dir_all(&cache);

    ensure_cache_version(&cache);

    GlobalPaths { home, data, cache }
});

#[cfg(not(target_arch = "wasm32"))]
fn ensure_cache_version(cache: &Path) {
    let version_path = cache.join("version");
    let version = std::fs::read_to_string(&version_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0".to_string());

    // 缓存版本变更后清空旧目录，避免旧结构与新格式混用。
    if version != CACHE_VERSION {
        if let Ok(entries) = std::fs::read_dir(cache) {
            for entry in entries.flatten() {
                let p = entry.path();
                let _ =
                    if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
            }
        }
        let _ = std::fs::write(&version_path, CACHE_VERSION);
    }
}

#[cfg(target_arch = "wasm32")]
static PATHS: Lazy<GlobalPaths> = Lazy::new(|| {
    let home = PathBuf::from("/");
    let data = home.join("data").join(APP);
    let cache = home.join("cache").join(APP);
    GlobalPaths { home, data, cache }
});

/// 返回当前进程的全局路径配置。
///
/// 该值在进程生命周期内只初始化一次。
pub fn paths() -> &'static GlobalPaths {
    &PATHS
}

#[cfg(test)]
#[path = "global_tests.rs"]
mod global_tests;
