// 覆盖配置加载入口、默认路径与跨来源解析行为。

#[cfg(not(target_arch = "wasm32"))]
use super::load_from_path_without_env;
use super::{
    Error, State, deduplicate_plugins, merge_json_value, plugin_name,
    take_gateway_skeys_from_patch, update_global_for_tenant,
};
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use vw_api_types::id::TenantScope;

// 正常值、缺省值与非法输入成组验证，用于固定配置兼容边界和失败语义。
#[test]
fn plugin_name_strips_trailing_empty_version() {
    assert_eq!(plugin_name("registry@"), "registry");
}

#[test]
fn deduplicate_plugins_matches_file_and_version_specs_by_name() {
    let plugins =
        vec!["file:///tmp/tool.wasm".to_string(), "helper@1".to_string(), "tool@2".to_string()];

    assert_eq!(deduplicate_plugins(plugins), vec!["helper@1".to_string(), "tool@2".to_string()]);
}

#[test]
fn merge_json_value_ignores_null_for_missing_object_field() {
    let mut target = json!({"keep": 1});

    merge_json_value(&mut target, json!({"missing": null}));

    assert_eq!(target, json!({"keep": 1}));
}

#[test]
fn merge_json_value_replaces_root_with_null() {
    let mut target = json!({"keep": 1});

    merge_json_value(&mut target, json!(null));

    assert!(target.is_null());
}

#[test]
fn take_gateway_skeys_from_patch_leaves_non_object_gateway_untouched() {
    let mut patch = json!({"gateway": false});

    let skeys = take_gateway_skeys_from_patch(&mut patch).expect("non-object gateway is ignored");

    assert!(skeys.is_none());
    assert_eq!(patch, json!({"gateway": false}));
}

#[test]
fn take_gateway_skeys_from_patch_returns_none_when_skeys_are_absent() {
    let mut patch = json!({"gateway": {"auth_enabled": true}});

    let skeys = take_gateway_skeys_from_patch(&mut patch).expect("missing skeys are allowed");

    assert!(skeys.is_none());
    assert_eq!(patch, json!({"gateway": {"auth_enabled": true}}));
}

#[test]
fn take_gateway_skeys_from_patch_accepts_and_removes_empty_list() {
    let mut patch = json!({"gateway": {"auth_enabled": true, "skeys": []}});

    let skeys = take_gateway_skeys_from_patch(&mut patch)
        .expect("empty skey list is valid")
        .expect("present skey list is returned");

    assert!(skeys.is_empty());
    assert_eq!(patch, json!({"gateway": {"auth_enabled": true}}));
}

#[test]
fn state_default_uses_null_config() {
    let state = State::default();

    assert!(state.config.is_null());
}

#[tokio::test]
async fn update_global_for_tenant_rejects_non_object_before_storage_access() {
    let tenant = TenantScope::new("platform", "company").expect("tenant scope");

    let error = update_global_for_tenant(&tenant, json!(null)).await.unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message == "config patch must be a JSON object")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_from_path_without_env_loads_json_and_sets_runtime_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let config_path = temp.path().join("config.json");
    let workspace_dir = temp.path().join("workspace");
    tokio::fs::write(&config_path, r#"{"default_model":"json-model"}"#)
        .await
        .expect("write config");

    let config = load_from_path_without_env(&config_path, workspace_dir.clone())
        .await
        .expect("load json config");

    assert_eq!(config.config_path, config_path);
    assert_eq!(config.workspace_dir, workspace_dir);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_from_path_without_env_reports_missing_fallback_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    let config_path = temp.path().join("missing.toml");

    let error =
        load_from_path_without_env(&config_path, temp.path().join("workspace")).await.unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message.contains("Failed to read config file"))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_from_path_without_env_reports_invalid_json() {
    let temp = tempfile::tempdir().expect("temp dir");
    let config_path = temp.path().join("invalid.json");
    tokio::fs::write(&config_path, "{").await.expect("write invalid config");

    let error =
        load_from_path_without_env(&config_path, temp.path().join("workspace")).await.unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message.contains("Failed to parse config file"))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_from_path_without_env_blocking_rejects_invalid_path_inside_runtime() {
    let error =
        super::load_from_path_without_env_blocking(Path::new("/"), PathBuf::from("/workspace"))
            .unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message == "config path must have a parent directory")
    );
}
