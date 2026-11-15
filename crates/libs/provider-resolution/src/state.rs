//! 聚合缓存及配置指纹。
use crate::State;
use provider_state_cache as cache;
use provider_config_reader as config;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::UNIX_EPOCH;
#[cfg(not(target_arch = "wasm32"))]
use crate::native::load_state_impl;
#[cfg(target_arch = "wasm32")]
use crate::gateway::load_state_impl;

/// 启用过滤后的 provider 状态缓存 key。
const CACHE_KEY_FILTERED: &str = "provider:filtered";
/// 设置页可见的 provider 状态缓存 key。
const CACHE_KEY_SETTINGS: &str = "provider:settings";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ConfigFingerprint {
    path: String,
    exists: bool,
    modified_nanos: u128,
    len: u64,
}

static CONFIG_FINGERPRINT: Lazy<Mutex<Option<ConfigFingerprint>>> = Lazy::new(|| Mutex::new(None));

/// 返回面向实际使用场景过滤后的 provider 状态。
///
/// 该状态会过滤掉不可用的模型，更适合正常对话或推理流程使用。
pub(crate) async fn state_filtered() -> Arc<State> {
    invalidate_cache_if_config_changed().await;
    cache::get_or_init(CACHE_KEY_FILTERED, || async { Arc::new(load_state_impl(true).await) }).await
}

/// 返回设置页使用的 provider 状态，不应用启用过滤。
///
/// 设置页通常需要看到更完整的信息，因此这里保留未启用的模型项。
pub(crate) async fn state_for_settings() -> Arc<State> {
    invalidate_cache_if_config_changed().await;
    cache::get_or_init(CACHE_KEY_SETTINGS, || async { Arc::new(load_state_impl(false).await) })
        .await
}

#[cfg(not(target_arch = "wasm32"))]
fn read_config_fingerprint() -> ConfigFingerprint {
    let Some(path) = config::config_path() else {
        return ConfigFingerprint::default();
    };

    let mut fingerprint = ConfigFingerprint {
        path: path.to_string_lossy().to_string(),
        exists: false,
        modified_nanos: 0,
        len: 0,
    };

    let Ok(metadata) = std::fs::metadata(&path) else {
        return fingerprint;
    };

    fingerprint.exists = true;
    fingerprint.len = metadata.len();
    fingerprint.modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    fingerprint
}

#[cfg(target_arch = "wasm32")]
fn read_config_fingerprint() -> ConfigFingerprint {
    ConfigFingerprint::default()
}

async fn invalidate_cache_if_config_changed() {
    let next = read_config_fingerprint();
    let changed = {
        let mut guard = CONFIG_FINGERPRINT.lock().unwrap_or_else(|e| e.into_inner());
        let changed = guard.as_ref().is_some_and(|current| current != &next);
        *guard = Some(next);
        changed
    };

    if changed {
        invalidate_cache().await;
    }
}

/// 使 provider 聚合缓存失效。
///
/// 适用于配置变更、认证状态变化或模型元数据刷新之后。
pub async fn invalidate_cache() {
    if let Ok(mut guard) = CONFIG_FINGERPRINT.lock() {
        *guard = None;
    }
    cache::invalidate(CACHE_KEY_FILTERED).await;
    cache::invalidate(CACHE_KEY_SETTINGS).await;
}


#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "state_tests.rs"]
mod tests;
