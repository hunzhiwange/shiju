use serde_json::Value;
use tempfile::TempDir;
use vw_api_types::id::TenantScope;

use super::{
    Config, GatewaySkey,
    config_save::{
        merge_patch_between, merge_raw_project_patch, prepare_project_override,
        save_config_to_store, save_project_config_to_store, select_override_shape,
        update_project_config_from_configs,
    },
};
use crate::app::agent::config::SystemConfigStore;
use crate::app::agent::security::SecretStore;
use crate::global_database::GlobalDatabase;

fn store(temp: &TempDir) -> SystemConfigStore {
    SystemConfigStore::new(
        GlobalDatabase::new(temp.path().join("database").join("_index.sqlite3"))
            .expect("global database"),
    )
    .expect("system config store")
}

fn stored_json(store: &SystemConfigStore, tenant: &TenantScope) -> Value {
    let payload = store.get(tenant).expect("stored payload").expect("tenant row");
    serde_json::from_str(&payload).expect("stored json")
}

#[test]
fn save_config_to_store_encrypts_secrets_without_writing_user_config_file() {
    let temp = TempDir::new().expect("temp dir");
    let store = store(&temp);
    let tenant = TenantScope::default();
    let mut config = Config::default();
    config.config_path = temp.path().join("vibewindow.json");
    config.default_model = Some("gpt-4o-mini".to_string());
    config.api_key = Some("sk-plain".to_string());
    config.gateway.paired_tokens = vec!["pair-token".to_string()];

    save_config_to_store(&config, &tenant, &store).expect("save config");

    let persisted = stored_json(&store, &tenant);
    assert_eq!(persisted["default_model"], "gpt-4o-mini");
    assert!(SecretStore::is_encrypted(persisted["api_key"].as_str().expect("api key")));
    assert!(SecretStore::is_encrypted(
        persisted["gateway"]["paired_tokens"][0].as_str().expect("paired token")
    ));
    assert!(!config.config_path.exists());
}

#[test]
fn save_config_to_store_omits_gateway_skeys_from_system_payload() {
    let temp = TempDir::new().expect("temp dir");
    let store = store(&temp);
    let tenant = TenantScope::new("platform-a", "company-a").expect("tenant");
    let raw_skey = "  sk-super-secret  ";
    let mut config = Config::default();
    config.gateway.skeys = vec![
        GatewaySkey {
            enabled: true,
            skey: Some(raw_skey.to_string()),
            skey_hash: "ignored".to_string(),
            masked_skey: String::new(),
            name: "  \t ".to_string(),
            expires_at: Some("  2026-12-31T23:59:59Z  ".to_string()),
        },
        GatewaySkey {
            enabled: false,
            skey: None,
            skey_hash: format!("  {}  ", "A".repeat(64)),
            masked_skey: "saved-mask".to_string(),
            name: "  Existing Name  ".to_string(),
            expires_at: Some(" \n ".to_string()),
        },
    ];

    save_config_to_store(&config, &tenant, &store).expect("save config");

    let persisted = stored_json(&store, &tenant);
    assert_eq!(persisted["gateway"]["skeys"], serde_json::json!([]));
    assert!(!serde_json::to_string(&persisted).expect("persisted json").contains(raw_skey));
    assert_eq!(config.gateway.skeys[0].skey.as_deref(), Some(raw_skey));
}

#[test]
fn save_config_to_store_updates_only_target_tenant() {
    let temp = TempDir::new().expect("temp dir");
    let store = store(&temp);
    let first = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second = TenantScope::new("platform-b", "company-b").expect("second tenant");
    let mut first_config = Config::default();
    first_config.default_provider = Some("first-before".to_string());
    let mut second_config = Config::default();
    second_config.default_provider = Some("second-provider".to_string());
    save_config_to_store(&first_config, &first, &store).expect("first initial save");
    save_config_to_store(&second_config, &second, &store).expect("second initial save");

    first_config.default_provider = Some("first-after".to_string());
    save_config_to_store(&first_config, &first, &store).expect("first update");

    assert_eq!(stored_json(&store, &first)["default_provider"], "first-after");
    assert_eq!(stored_json(&store, &second)["default_provider"], "second-provider");
}

#[test]
fn save_config_to_store_rejects_invalid_config_without_persisting_tenant() {
    let temp = TempDir::new().expect("temp dir");
    let store = store(&temp);
    let tenant = TenantScope::new("platform-a", "company-a").expect("tenant");
    let mut config = Config::default();
    config.gateway.host = " ".to_string();

    let error = save_config_to_store(&config, &tenant, &store).expect_err("invalid config");

    assert!(error.to_string().contains("gateway.host"));
    assert!(store.get(&tenant).expect("tenant lookup").is_none());
}

#[test]
fn save_project_config_to_store_writes_database_without_project_file() {
    let temp = TempDir::new().expect("temp dir");
    let store = store(&temp);
    let tenant = TenantScope::new("platform", "company").expect("tenant");
    let payload = serde_json::json!({"default_provider": "project-provider"});

    save_project_config_to_store(&store, &tenant, temp.path(), &payload)
        .expect("save project config");

    assert_eq!(
        serde_json::from_str::<Value>(
            &store
                .get_project(&tenant, temp.path().to_str().expect("project path"))
                .expect("read project config")
                .expect("stored project config")
        )
        .expect("project config json"),
        payload
    );
    assert!(!temp.path().join(".vibewindow.json").exists());
}

#[test]
fn merge_raw_project_patch_recurses_into_objects_and_replaces_other_values() {
    let mut target = serde_json::json!({
        "nested": {"retained": true, "changed": "before"},
        "scalar": 1,
        "untouched": "value"
    });

    merge_raw_project_patch(
        &mut target,
        serde_json::json!({
            "nested": {"changed": "after", "added": 2},
            "scalar": {"replacement": true},
            "null_value": null
        }),
    );

    assert_eq!(
        target,
        serde_json::json!({
            "nested": {"retained": true, "changed": "after", "added": 2},
            "scalar": {"replacement": true},
            "untouched": "value",
            "null_value": null
        })
    );
}

#[test]
fn merge_raw_project_patch_replaces_non_object_root() {
    let mut target = serde_json::json!("before");
    let patch = serde_json::json!({"after": true});

    merge_raw_project_patch(&mut target, patch.clone());

    assert_eq!(target, patch);
}

#[test]
fn select_override_shape_keeps_shape_and_uses_effective_leaf_values() {
    let shape = serde_json::json!({
        "api_key": "plain-project-key",
        "nested": {"changed": "plain-value", "missing": "fallback"},
        "explicit_null": null
    });
    let effective = serde_json::json!({
        "api_key": "encrypted-project-key",
        "nested": {"changed": "normalized-value"},
        "explicit_null": "ignored",
        "base_only": true
    });

    let selected = select_override_shape(&shape, Some(&effective));

    assert_eq!(
        selected,
        serde_json::json!({
            "api_key": "encrypted-project-key",
            "nested": {"changed": "normalized-value", "missing": "fallback"},
            "explicit_null": null
        })
    );
}

#[test]
fn select_override_shape_falls_back_to_leaf_shape_without_effective_value() {
    let shape = serde_json::json!([concat!("pro", "ject"), "value"]);

    assert_eq!(select_override_shape(&shape, None), shape);
}

#[test]
fn merge_patch_between_returns_none_for_equal_values() {
    let value = serde_json::json!({"nested": [1, 2, 3]});

    assert_eq!(merge_patch_between(&value, &value), None);
}

#[tokio::test]
async fn unchanged_project_config_does_not_create_override_file() {
    let temp = TempDir::new().expect("temp dir");
    let mut config = Config::default();
    config.workspace_dir = temp.path().to_path_buf();
    config.config_path = temp.path().join(".vibewindow.json");

    let result = update_project_config_from_configs(temp.path(), &config, &config)
        .await
        .expect("unchanged config");

    assert_eq!(
        serde_json::to_value(result).expect("result json"),
        serde_json::to_value(config).expect("config json")
    );
    assert!(!temp.path().join(".vibewindow.json").exists());
}

#[test]
fn merge_patch_between_reports_nested_changes_additions_and_removals() {
    let before = serde_json::json!({
        "nested": {"unchanged": true, "changed": "before", "removed": 1},
        "top_removed": false
    });
    let after = serde_json::json!({
        "nested": {"unchanged": true, "changed": "after", "added": 2},
        "top_added": [1, 2]
    });

    assert_eq!(
        merge_patch_between(&before, &after),
        Some(serde_json::json!({
            "nested": {"changed": "after", "removed": null, "added": 2},
            "top_removed": null,
            "top_added": [1, 2]
        }))
    );
}

#[test]
fn merge_patch_between_replaces_non_object_value() {
    let before = serde_json::json!(["before"]);
    let after = serde_json::json!({"after": true});

    assert_eq!(merge_patch_between(&before, &after), Some(after));
}

#[test]
fn prepare_project_override_encrypts_secrets_and_removes_gateway_skeys() {
    let temp = TempDir::new().expect("temp dir");
    let mut base = Config::default();
    base.config_path = temp.path().join("vibewindow.json");
    base.workspace_dir = temp.path().join("workspace");
    let plain_secret = "project-api-key";
    let raw_override = serde_json::json!({
        "default_provider": "before",
        "agent": {"max_iterations": 1},
        "gateway": {"skeys": [{"skey": "must-not-persist"}]}
    });
    let patch = serde_json::json!({
        "default_provider": "after",
        "api_key": plain_secret
    });

    let prepared =
        prepare_project_override(&base, raw_override, patch).expect("prepare project override");

    assert_eq!(prepared["default_provider"], "after");
    assert!(SecretStore::is_encrypted(prepared["api_key"].as_str().expect("encrypted api key")));
    assert!(prepared.get("agent").is_none());
    assert!(prepared["gateway"].get("skeys").is_none());
    assert!(!serde_json::to_string(&prepared).expect("prepared json").contains(plain_secret));
}

#[test]
fn prepare_project_override_rejects_undecryptable_secret() {
    let temp = TempDir::new().expect("temp dir");
    let mut base = Config::default();
    base.config_path = temp.path().join("vibewindow.json");
    base.workspace_dir = temp.path().join("workspace");

    let error = prepare_project_override(
        &base,
        serde_json::json!({}),
        serde_json::json!({"api_key": "enc2:not-hex"}),
    )
    .expect_err("invalid encrypted secret");

    assert!(error.to_string().contains("Failed to decrypt config.api_key"));
}

#[test]
fn prepare_project_override_rejects_base_config_path_without_parent() {
    let mut base = Config::default();
    base.config_path = std::path::PathBuf::from("/");

    let error = prepare_project_override(
        &base,
        serde_json::json!({}),
        serde_json::json!({"default_provider": "project-provider"}),
    )
    .expect_err("base config path without parent");

    assert!(error.to_string().contains("Tenant base config path must have a parent directory"));
}
