//! 模型基线元数据的加载、缓存与刷新。
//!
//! 本模块负责维护 provider 基线元数据，主要能力包括：
//! - 解析内置的 `assets/model/api.json`
//! - 读取本地缓存的模型元数据文件
//! - 兼容不同 JSON 包装结构
//! - 在进程内缓存解析结果，避免重复读取与反序列化

use crate::flag;
use crate::global;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;
use std::time::SystemTime;
use tracing::debug;

#[cfg(not(target_arch = "wasm32"))]
#[path = "models_cache.rs"]
mod native;

pub use vw_shared::provider::models::{
    Model, ModelCost, ModelCostOver200k, ModelInterleaved, ModelLimit, ModelModalities,
    ModelProviderInfo, Provider,
};

pub use vw_shared::provider::reasoning::ReasoningOption;

/// 打包内置的模型元数据，作为本地兜底数据源。
const BUNDLED_MODELS_TEXT: &str = include_str!("../assets/model/api.json");
/// 仓库自有的识矩 provider 与离线模型目录。
const BUNDLED_SHIJU_MODELS_TEXT: &str = include_str!("../assets/model/shiju.json");

/// Exact IDs only: prefer Shiju's curated entry, then upstream catalogs before relays.
/// Do not infer capabilities from a family name or strip provider prefixes.
pub(crate) fn shiju_model_metadata<'a>(
    catalog: &'a HashMap<String, Provider>,
    model_id: &str,
) -> Option<&'a Model> {
    [
        "shiju",
        "alibaba-cn",
        "alibaba-token-plan-cn",
        "moonshotai-cn",
        "deepseek",
        "zhipuai",
        "alibaba",
        "alibaba-token-plan",
        "moonshotai",
        "hyper",
    ]
    .into_iter()
    .find_map(|provider| catalog.get(provider)?.models.get(model_id))
}

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

#[allow(dead_code)]
/// 尝试兼容多种 JSON 结构并解析为 provider 映射。
///
/// 当前支持的输入形状包括：
/// - 直接的 `HashMap<String, Provider>`
/// - 带 `providers` 字段的对象
/// - 带 `data` 字段的对象
/// - `Vec<Provider>` 数组
fn parse_models_text(label: &str, text: &str) -> Option<HashMap<String, Provider>> {
    let value = match serde_json::from_str::<Value>(text) {
        Ok(v) => v,
        Err(_) => {
            debug!("{label}_parse_invalid");
            return None;
        }
    };

    match &value {
        Value::Object(obj) => {
            debug!(kind = "object", keys_count = obj.len(), "{label}_parse_shape");

            if let Some(providers) = obj.get("providers")
                && let Ok(json) =
                    serde_json::from_value::<HashMap<String, Provider>>(providers.clone())
            {
                debug!(providers = json.len(), "{label}_parsed_wrapped");
                return Some(json);
            }

            if let Some(data) = obj.get("data")
                && let Ok(json) = serde_json::from_value::<HashMap<String, Provider>>(data.clone())
            {
                debug!(providers = json.len(), "{label}_parsed_data");
                return Some(json);
            }

            match serde_json::from_value::<HashMap<String, Provider>>(value.clone()) {
                Ok(json) => {
                    debug!(providers = json.len(), "{label}_parsed");
                    return Some(json);
                }
                Err(_) => debug!("{label}_parse_failed"),
            }
        }
        Value::Array(arr) => {
            debug!(kind = "array", len = arr.len(), "{label}_parse_shape");

            if let Ok(list) = serde_json::from_value::<Vec<Provider>>(value) {
                let map = list
                    .into_iter()
                    .filter(|p| !p.id.is_empty())
                    .map(|p| (p.id.clone(), p))
                    .collect::<HashMap<_, _>>();
                debug!(providers = map.len(), "{label}_parsed_array");
                return Some(map);
            }
        }
        _ => {
            debug!(kind = "other", "{label}_parse_shape");
        }
    }
    None
}

/// 读取打包内置的模型数据。
fn bundled_models(label: &str) -> HashMap<String, Provider> {
    let mut providers = parse_models_text(label, BUNDLED_MODELS_TEXT).unwrap_or_default();
    merge_bundled_shiju(&mut providers, label);
    providers
}

fn merge_bundled_shiju(providers: &mut HashMap<String, Provider>, label: &str) {
    let overlay_label = format!("{label}_shiju");
    let Some(mut overlay) = parse_models_text(&overlay_label, BUNDLED_SHIJU_MODELS_TEXT) else {
        return;
    };
    let Some(shiju) = overlay.remove("shiju") else {
        return;
    };
    providers.insert("shiju".to_string(), shiju);
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
