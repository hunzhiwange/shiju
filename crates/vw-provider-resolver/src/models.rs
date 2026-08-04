//! 模型基线元数据的加载、缓存与刷新。
//!
//! 本模块负责维护 provider 基线元数据，主要能力包括：
//! - 解析内置的 `assets/model/api.json`
//! - 读取本地缓存的模型元数据文件
//! - 兼容不同 JSON 包装结构
//! - 在进程内缓存解析结果，避免重复读取与反序列化

use crate::flag;
use crate::global;
#[cfg(not(target_arch = "wasm32"))]
use crate::installation;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use tracing::error;
use tracing::info;

pub use vw_shared::provider::models::{
    Model, ModelCost, ModelCostOver200k, ModelInterleaved, ModelLimit, ModelModalities,
    ModelProviderInfo, Provider,
};

/// 打包内置的模型元数据，作为本地兜底数据源。
const BUNDLED_MODELS_TEXT: &str = include_str!("../assets/model/api.json");
/// 仓库自有的识矩 provider 与离线模型目录。
const BUNDLED_SHIJU_MODELS_TEXT: &str = include_str!("../assets/model/shiju.json");

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
        Err(err) => {
            info!(
                error = %err,
                "{label}_parse_invalid"
            );
            return None;
        }
    };

    match &value {
        Value::Object(obj) => {
            let keys: Vec<&String> = obj.keys().take(5).collect();
            info!(
                kind = "object",
                keys_count = obj.len(),
                keys = ?keys,
                "{label}_parse_shape"
            );

            if let Some(providers) = obj.get("providers")
                && let Ok(json) =
                    serde_json::from_value::<HashMap<String, Provider>>(providers.clone())
            {
                info!(providers = json.len(), "{label}_parsed_wrapped");
                return Some(json);
            }

            if let Some(data) = obj.get("data")
                && let Ok(json) = serde_json::from_value::<HashMap<String, Provider>>(data.clone())
            {
                info!(providers = json.len(), "{label}_parsed_data");
                return Some(json);
            }

            match serde_json::from_value::<HashMap<String, Provider>>(value.clone()) {
                Ok(json) => {
                    info!(providers = json.len(), "{label}_parsed");
                    return Some(json);
                }
                Err(err) => {
                    info!(
                        error = %err,
                        "{label}_parse_failed"
                    );
                }
            }
        }
        Value::Array(arr) => {
            info!(kind = "array", len = arr.len(), "{label}_parse_shape");

            if let Ok(list) = serde_json::from_value::<Vec<Provider>>(value) {
                let map = list
                    .into_iter()
                    .filter(|p| !p.id.is_empty())
                    .map(|p| (p.id.clone(), p))
                    .collect::<HashMap<_, _>>();
                info!(providers = map.len(), "{label}_parsed_array");
                return Some(map);
            }
        }
        _ => {
            info!(kind = "other", "{label}_parse_shape");
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

#[allow(dead_code)]
/// 进程内缓存，避免重复解析模型元数据。
static CACHE: Lazy<Mutex<Option<HashMap<String, Provider>>>> = Lazy::new(|| Mutex::new(None));

/// 清空进程内模型元数据缓存。
///
/// 下次调用 [`get`] 时会重新从磁盘缓存或内置资源加载，适合在设置页需要
/// 立即反映最新 `models.json` 内容时使用。
pub fn invalidate_cache() {
    if let Ok(mut lock) = CACHE.lock() {
        *lock = None;
    }
}

#[cfg(target_arch = "wasm32")]
/// wasm 环境下当前无需主动刷新模型缓存。
pub async fn refresh() {}

#[cfg(target_arch = "wasm32")]
/// 读取模型元数据，优先复用进程内缓存。
///
/// 若缓存为空，则触发实际加载并回填缓存。
pub async fn get() -> HashMap<String, Provider> {
    if let Ok(lock) = CACHE.lock() {
        if let Some(v) = lock.clone() {
            return v;
        }
    }

    let data = load().await;

    if let Ok(mut lock) = CACHE.lock() {
        *lock = Some(data.clone());
    }

    data
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取模型元数据，优先复用进程内缓存。
///
/// 若缓存为空，则触发实际加载并回填缓存。
pub async fn get() -> HashMap<String, Provider> {
    if let Ok(lock) = CACHE.lock()
        && let Some(v) = lock.clone()
    {
        return v;
    }

    let data = load().await;

    if let Ok(mut lock) = CACHE.lock() {
        *lock = Some(data.clone());
    }
    data
}

#[cfg(not(target_arch = "wasm32"))]
/// 从远端刷新模型元数据，并在成功后清空进程内缓存。
pub async fn refresh() {
    if *flag::VIBEWINDOW_DISABLE_MODELS_FETCH {
        return;
    }

    refresh_from_url(cache_path(), format!("{}/api.json", url())).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn refresh_from_url(path: PathBuf, api_url: String) {
    info!(
        path = %path.to_string_lossy(),
        url = api_url,
        "refresh"
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(api_url)
        .header("User-Agent", installation::user_agent())
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    let Ok(resp) = resp else {
        error!("Failed to fetch models.dev");
        return;
    };

    info!(status = resp.status().as_u16(), "refresh_status");

    if !resp.status().is_success() {
        return;
    }

    let Ok(text) = resp.text().await else {
        return;
    };

    info!(bytes = text.len(), "refresh_body");

    let _ = tokio::fs::write(&path, text).await;

    if let Ok(mut lock) = CACHE.lock() {
        *lock = None;
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 按“本地缓存 -> 内置数据”的顺序加载模型元数据。
///
/// 当本地缓存不存在或解析失败时，会回退到内置数据；若当前未指定自定义
/// models 路径，还会把内置数据写回缓存文件，作为后续启动的基线。
async fn load() -> HashMap<String, Provider> {
    let path = flag::VIBEWINDOW_MODELS_PATH.clone().map(PathBuf::from).unwrap_or_else(cache_path);
    let custom_path = flag::VIBEWINDOW_MODELS_PATH.is_some();

    info!(
        path = %path.to_string_lossy(),
        models_path_env = custom_path,
        disable_fetch = *flag::VIBEWINDOW_DISABLE_MODELS_FETCH,
        url = format!("{}/api.json", url()),
        "load"
    );

    if let Ok(content) = tokio::fs::read_to_string(&path).await
        && let Some(mut json) = parse_models_text("load_cache", &content)
        && !json.is_empty()
    {
        merge_bundled_shiju(&mut json, "load_cache");
        return json;
    }

    let bundled = bundled_models("load_bundled");
    if bundled.is_empty() {
        info!("load_bundled_empty");
        return HashMap::new();
    }

    if !custom_path {
        let _ = tokio::fs::write(&path, BUNDLED_MODELS_TEXT).await;
    }

    bundled
}

#[cfg(target_arch = "wasm32")]
/// wasm 环境仅使用内置模型数据。
async fn load() -> HashMap<String, Provider> {
    bundled_models("load_bundled")
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
