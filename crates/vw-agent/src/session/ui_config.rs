//! 桌面 UI 配置读写入口，负责同步读取和更新持久化的偏好字段。

use anyhow::{Context, Result};
use serde_json::Value;
use vw_api_types::id::TenantScope;

use crate::gateway::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;

const UI_CONFIG_NAMESPACE: &str = "desktop.preferences";
const UI_CONFIG_KEY: &str = "default";

/// 执行 load_app_config 操作，并返回调用方需要的结果。
pub fn load_app_config(tenant: &TenantScope) -> Value {
    load_app_config_from_store_result(runtime_settings_store(), tenant)
}

/// 执行 set_config_field 操作，并返回调用方需要的结果。
pub fn set_config_field(tenant: &TenantScope, key: &str, value: Value) {
    set_config_field_with_store_result(runtime_settings_store(), tenant, key, value);
}

fn load_app_config_from_store_result(
    store: Result<GatewaySettingsStore>,
    tenant: &TenantScope,
) -> Value {
    store.and_then(|store| load_app_config_from_store(&store, tenant)).unwrap_or_else(|error| {
        tracing::error!(target: "vw_agent", error = %error, "load desktop UI config failed");
        serde_json::json!({})
    })
}

fn set_config_field_with_store_result(
    store: Result<GatewaySettingsStore>,
    tenant: &TenantScope,
    key: &str,
    value: Value,
) {
    let result = store.and_then(|store| set_config_field_in_store(&store, tenant, key, value));
    if let Err(error) = result {
        tracing::error!(target: "vw_agent", error = %error, "save desktop UI config failed");
    }
}

fn load_app_config_from_store(store: &GatewaySettingsStore, tenant: &TenantScope) -> Result<Value> {
    let Some(payload) = store.get(tenant, UI_CONFIG_NAMESPACE, UI_CONFIG_KEY)? else {
        return Ok(serde_json::json!({}));
    };
    let cfg: Value = serde_json::from_str(&payload).context("decode desktop UI config")?;
    Ok(if cfg.is_object() { cfg } else { serde_json::json!({}) })
}

fn set_config_field_in_store(
    store: &GatewaySettingsStore,
    tenant: &TenantScope,
    key: &str,
    value: Value,
) -> Result<()> {
    let mut cfg = load_app_config_from_store(store, tenant)?;
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert(key.to_string(), value);
    }

    let payload = serde_json::to_string(&cfg).context("encode desktop UI config")?;
    store.upsert(tenant, UI_CONFIG_NAMESPACE, UI_CONFIG_KEY, &payload)
}

fn runtime_settings_store() -> Result<GatewaySettingsStore> {
    let database = GlobalDatabase::for_home(&crate::global::paths().home)
        .context("open shared database for desktop UI config")?;
    GatewaySettingsStore::new(database).context("initialize desktop UI config store")
}
#[cfg(test)]
#[path = "ui_config_tests.rs"]
mod ui_config_tests;
