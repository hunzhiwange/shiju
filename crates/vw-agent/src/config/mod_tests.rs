use super::{
    Error, State, deduplicate_plugins, directories, load_from_path_without_env, merge_json_value,
    plugin_name, remove_global_provider, take_gateway_skeys_from_patch, update, update_global,
};
use crate::app::agent::security::gateway_skey_store::GatewaySkeyStore;
use crate::app::agent::storage::{Error as StorageError, NotFoundError};
use crate::global_database::GlobalDatabase;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use serde_json::{Value, json};
use vw_api_types::id::TenantScope;
use vw_config_types::gateway::GatewaySkey;

#[test]
fn plugin_name_normalizes_versions_and_file_urls() {
    assert_eq!(plugin_name("tool@1.2.3"), "tool");
    assert_eq!(plugin_name("@scope/tool@1.2.3"), "@scope/tool");
    assert_eq!(plugin_name("@scope/tool"), "@scope/tool");
    assert_eq!(plugin_name("file:///tmp/my-plugin.wasm"), "my-plugin");
    assert_eq!(plugin_name("file:///tmp/plugin"), "plugin");
    assert_eq!(plugin_name("plain"), "plain");
}

#[test]
fn plugin_name_keeps_specs_without_extractable_name() {
    assert_eq!(plugin_name("file:///"), "file:///");
    assert_eq!(plugin_name("@"), "@");
}

#[test]
fn deduplicate_plugins_keeps_last_spec_for_each_name() {
    let plugins = vec!["a@1".to_string(), "b@1".to_string(), "a@2".to_string()];

    assert_eq!(deduplicate_plugins(plugins), vec!["b@1".to_string(), "a@2".to_string()]);
}

#[test]
fn deduplicate_plugins_normalizes_file_specs_by_stem() {
    let plugins = vec![
        "file:///tmp/one.wasm".to_string(),
        "file:///tmp/two.wasm".to_string(),
        "file:///opt/one.wasm".to_string(),
    ];

    assert_eq!(
        deduplicate_plugins(plugins),
        vec!["file:///tmp/two.wasm".to_string(), "file:///opt/one.wasm".to_string()]
    );
}

#[test]
fn deduplicate_plugins_handles_empty_input() {
    assert!(deduplicate_plugins(Vec::new()).is_empty());
}

#[test]
fn merge_json_value_recurses_and_removes_nulls() {
    let mut target = json!({"a": 1, "nested": {"keep": true, "drop": 1}});

    merge_json_value(&mut target, json!({"nested": {"drop": null, "add": 2}, "a": 3}));

    assert_eq!(target, json!({"a": 3, "nested": {"keep": true, "add": 2}}));
}

#[test]
fn merge_json_value_replaces_non_objects_and_inserts_new_values() {
    let mut target = json!({"nested": 1});

    merge_json_value(&mut target, json!({"nested": {"now": true}, "new": [1, 2]}));

    assert_eq!(target, json!({"nested": {"now": true}, "new": [1, 2]}));

    merge_json_value(&mut target, json!(false));

    assert_eq!(target, json!(false));
}

#[test]
fn take_gateway_skeys_from_patch_returns_none_without_gateway_object() {
    let mut patch = json!({"model": "gpt-4"});

    let skeys = take_gateway_skeys_from_patch(&mut patch).unwrap();

    assert!(skeys.is_none());
    assert_eq!(patch, json!({"model": "gpt-4"}));
}

#[test]
fn take_gateway_skeys_from_patch_removes_valid_skeys_from_patch() {
    let hash = "a".repeat(64);
    let mut patch = json!({
        "gateway": {
            "auth_enabled": true,
            "skeys": [{
                "enabled": false,
                "skey_hash": hash,
                "masked_skey": "sk-test***************tail",
                "name": "ci"
            }]
        }
    });

    let skeys = take_gateway_skeys_from_patch(&mut patch).unwrap().unwrap();

    assert_eq!(
        skeys,
        vec![GatewaySkey {
            enabled: false,
            skey: None,
            skey_hash: "a".repeat(64),
            masked_skey: "sk-test***************tail".to_string(),
            name: "ci".to_string(),
            expires_at: None,
        }]
    );
    assert_eq!(patch, json!({"gateway": {"auth_enabled": true}}));
}

#[test]
fn take_gateway_skeys_from_patch_rejects_invalid_skeys_shape() {
    let mut patch = json!({"gateway": {"skeys": {"name": "not-a-list"}}});

    let error = take_gateway_skeys_from_patch(&mut patch).unwrap_err();

    assert!(matches!(error, Error::Json(_)));
}

#[test]
fn hydrate_gateway_skeys_from_store_projects_explicit_tenant_metadata() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store =
        GatewaySkeyStore::new(GlobalDatabase::for_home(temp.path()).expect("global database"))
            .expect("gateway skey store");
    let tenant = TenantScope::new("platform-a", "company-a").expect("tenant");
    store
        .replace_all(
            &tenant,
            &[GatewaySkey {
                enabled: true,
                skey: None,
                skey_hash: "a".repeat(64),
                masked_skey: "shared-mask".to_string(),
                name: "shared".to_string(),
                expires_at: None,
            }],
        )
        .expect("seed tenant skey");
    let mut config = super::Config::default();
    config.gateway.skeys.push(GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: "b".repeat(64),
        masked_skey: "config-mask".to_string(),
        name: "config-payload".to_string(),
        expires_at: None,
    });

    super::hydrate_gateway_skeys_from_store(&mut config, &tenant, &store)
        .expect("hydrate tenant skeys");

    assert_eq!(config.gateway.skeys.len(), 1);
    assert_eq!(config.gateway.skeys[0].name, "shared");
}

#[test]
fn error_display_and_state_default_are_plain() {
    assert_eq!(Error::Invalid("bad config".to_string()).to_string(), "bad config");
    assert!(State::default().directories.is_empty());
}

#[test]
fn error_from_variants_display_inner_errors() {
    let io_error = Error::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"));
    assert_eq!(io_error.to_string(), "denied");

    let json_error = serde_json::from_str::<Value>("{").unwrap_err();
    assert!(!Error::from(json_error).to_string().is_empty());

    let yaml_error = serde_yaml::from_str::<Value>(":").unwrap_err();
    assert!(!Error::from(yaml_error).to_string().is_empty());

    let storage_error = StorageError::NotFound(NotFoundError { message: "missing".to_string() });
    assert_eq!(Error::from(storage_error).to_string(), "missing");
}

#[tokio::test]
async fn directories_returns_empty_list() {
    assert!(directories().await.is_empty());
}

#[tokio::test]
async fn update_rejects_non_object_patch() {
    let error = update(json!(true)).await.unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message == "config patch must be a JSON object")
    );
}

#[tokio::test]
async fn update_global_rejects_non_object_patch() {
    let error = update_global(json!(["not", "object"])).await.unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message == "config patch must be a JSON object")
    );
}

#[tokio::test]
async fn remove_global_provider_rejects_blank_provider_id() {
    let error = remove_global_provider(" \t\n ").await.unwrap_err();

    assert!(matches!(error, Error::Invalid(message) if message == "provider_id must not be empty"));
}

#[tokio::test]
async fn load_from_path_without_env_rejects_path_without_parent() {
    let error =
        load_from_path_without_env(Path::new("/"), PathBuf::from("/workspace")).await.unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message == "config path must have a parent directory")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_from_path_without_env_falls_back_to_empty_toml_config() {
    let temp = tempfile::tempdir().expect("temp dir");
    let config_path = temp.path().join("config.toml");
    let workspace_dir = temp.path().join("workspace");
    tokio::fs::write(&config_path, "   ").await.expect("write empty config");

    let config = load_from_path_without_env(&config_path, workspace_dir.clone())
        .await
        .expect("load empty TOML config");

    assert_eq!(config.config_path, config_path);
    assert_eq!(config.workspace_dir, workspace_dir);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn load_from_path_without_env_blocking_rejects_path_without_parent() {
    let error =
        super::load_from_path_without_env_blocking(Path::new("/"), PathBuf::from("/workspace"))
            .unwrap_err();

    assert!(
        matches!(error, Error::Invalid(message) if message == "config path must have a parent directory")
    );
}
