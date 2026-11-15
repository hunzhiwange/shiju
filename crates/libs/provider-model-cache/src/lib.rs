//! 模型基线元数据的加载、缓存与刷新。
//!
//! 本模块负责维护 provider 基线元数据，主要能力包括：
//! - 通过 provider-model-catalog 解析并合并内置目录
//! - 读取本地缓存的模型元数据文件
//! - 兼容不同 JSON 包装结构
//! - 在进程内缓存解析结果，避免重复读取与反序列化

use provider_request_identity::flag;
use provider_runtime_paths as global;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;
use std::time::SystemTime;

#[cfg(not(target_arch = "wasm32"))]
#[path = "models_cache.rs"]
mod native;

pub use model_reasoning::ReasoningOption;

pub use provider_model_catalog::{
    Model, ModelCost, ModelCostOver200k, ModelInterleaved, ModelLimit, ModelModalities,
    ModelProviderInfo, Provider,
};
#[cfg(target_arch = "wasm32")]
use provider_model_catalog::bundled_models;

#[allow(dead_code)]
/// 返回 models.dev 的基础地址。
fn url() -> String {
    models_url(flag::VIBEWINDOW_MODELS_URL.as_deref())
}

fn models_url(configured: Option<&str>) -> String {
    configured.unwrap_or("https://models.dev").to_string()
}

#[allow(dead_code)]
/// 返回本地缓存文件路径。
fn cache_path() -> PathBuf {
    global::paths().cache.join("models.json")
}

/// The result of explicitly requesting a models.dev refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelsRefreshStatus {
    Updated,
    Fresh,
    Disabled,
    Failed,
}

/// Fetch metadata for the shared remote cache. Failed attempts leave `fetched_at` unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelsRefresh {
    pub status: ModelsRefreshStatus,
    pub fetched_at: Option<SystemTime>,
    pub last_error: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
/// Catalog used by the resolver, including the bundled Shiju overlay.
pub struct ModelsSnapshot {
    pub providers: HashMap<String, Provider>,
    /// Modification time of the managed models.json file, including legacy caches.
    pub fetched_at: Option<SystemTime>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ModelsSnapshot {
    /// Returns no freshness status for bundled or custom snapshots without a download time.
    pub fn cache_is_stale(&self) -> Option<bool> {
        let elapsed = self.fetched_at?.elapsed().ok()?;
        Some(elapsed >= native::STALE_TIME)
    }
}

#[cfg(not(target_arch = "wasm32"))]
static CACHE: Lazy<Arc<native::ModelsCache>> = Lazy::new(|| {
    Arc::new(native::ModelsCache::new(
        cache_path(),
        flag::VIBEWINDOW_MODELS_PATH.as_ref().map(PathBuf::from),
        format!("{}/api.json", url()),
    ))
});

#[cfg(target_arch = "wasm32")]
static CACHE: Lazy<Mutex<Option<HashMap<String, Provider>>>> = Lazy::new(|| Mutex::new(None));

/// 清空进程内模型元数据缓存，使下一次读取重新加载磁盘目录。
pub fn invalidate_cache() {
    #[cfg(not(target_arch = "wasm32"))]
    CACHE.invalidate();
    #[cfg(target_arch = "wasm32")]
    if let Ok(mut lock) = CACHE.lock() {
        *lock = None;
    }
}

/// Refresh the shared models.dev cache and report the outcome to the caller.
pub async fn refresh() -> ModelsRefresh {
    #[cfg(not(target_arch = "wasm32"))]
    return CACHE.refresh(*flag::VIBEWINDOW_DISABLE_MODELS_FETCH).await;

    #[cfg(target_arch = "wasm32")]
    ModelsRefresh { status: ModelsRefreshStatus::Disabled, fetched_at: None, last_error: None }
}

#[cfg(target_arch = "wasm32")]
/// 读取内置目录并复用进程内缓存。
pub async fn get() -> HashMap<String, Provider> {
    if let Ok(lock) = CACHE.lock() {
        if let Some(v) = lock.clone() {
            return v;
        }
    }

    let data = bundled_models("load_bundled");

    if let Ok(mut lock) = CACHE.lock() {
        *lock = Some(data.clone());
    }

    data
}

#[cfg(not(target_arch = "wasm32"))]
/// Read the effective catalog and its fetch time without using the network.
pub async fn snapshot() -> Arc<ModelsSnapshot> {
    CACHE.snapshot().await
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取模型元数据，优先复用进程内缓存。
pub async fn get() -> HashMap<String, Provider> {
    snapshot().await.providers.clone()
}

#[cfg(target_arch = "wasm32")]
/// wasm 环境无需额外初始化。
pub fn init() {}

#[cfg(not(target_arch = "wasm32"))]
/// 预留初始化入口，便于调用方显式触发模块加载。
///
/// 当前实现保持为空，但保留该入口可让上层显式表达初始化时机。
pub fn init() {
    static INIT: Lazy<()> = Lazy::new(|| {});

    Lazy::force(&INIT);
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
