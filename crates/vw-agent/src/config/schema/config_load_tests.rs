// 覆盖 JSON、TOML 与租户存储配置的加载、合并及错误传播。

use super::config_load::{
    config_json_path, load_config_for_tenant_from_store, load_config_from_json_root,
    load_config_from_toml_path,
};
use super::workspace::ConfigResolutionSource;
use crate::app::agent::config::SystemConfigStore;
use crate::global_database::GlobalDatabase;
use serde_json::json;
use vw_api_types::id::TenantScope;

// 缺失、空文件、旧格式和非法载荷必须走不同分支，避免把配置损坏误判为未配置。
#[test]
fn config_json_path_appends_stable_filename() {
    assert_eq!(
        config_json_path(std::path::Path::new("/tmp/vw")),
        std::path::Path::new("/tmp/vw/vibewindow.json")
    );
}

// 错误路径还要确认原配置或存储保持不变，防止校验失败后留下部分更新。
#[tokio::test]
async fn load_config_from_json_root_returns_none_for_missing_or_empty_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("vibewindow.json");

    let loaded = load_config_from_json_root(
        &path,
        tmp.path().join("workspace"),
        tmp.path(),
        ConfigResolutionSource::DefaultConfigDir,
    )
    .await
    .unwrap();

    assert!(loaded.is_none());
}

#[tokio::test]
async fn load_config_from_json_root_treats_non_object_root_as_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("vibewindow.json");
    tokio::fs::write(&path, "[]").await.unwrap();

    let loaded = load_config_from_json_root(
        &path,
        tmp.path().join("workspace"),
        tmp.path(),
        ConfigResolutionSource::DefaultConfigDir,
    )
    .await
    .unwrap();

    assert!(loaded.is_none());
}

#[tokio::test]
async fn load_config_from_json_root_reports_parse_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("vibewindow.json");
    tokio::fs::write(&path, "{").await.unwrap();

    let error = load_config_from_json_root(
        &path,
        tmp.path().join("workspace"),
        tmp.path(),
        ConfigResolutionSource::DefaultConfigDir,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("Failed to parse config file"));
}

#[tokio::test]
async fn load_config_from_json_root_propagates_deserialize_secret_and_validation_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("vibewindow.json");

    for (contents, expected_error) in [
        (r#"{"default_temperature":"warm"}"#, "Failed to deserialize config payload"),
        (
            r#"{"default_temperature":0.7,"api_key":"enc2:not-hex"}"#,
            "Failed to decrypt config.api_key",
        ),
        (
            r#"{"default_temperature":0.7,"autonomy":{"max_actions_per_hour":0}}"#,
            "autonomy.max_actions_per_hour must be greater than 0",
        ),
    ] {
        tokio::fs::write(&path, contents).await.unwrap();

        let error = load_config_from_json_root(
            &path,
            tmp.path().join("workspace"),
            tmp.path(),
            ConfigResolutionSource::DefaultConfigDir,
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains(expected_error), "unexpected error: {error:#}");
    }
}

#[tokio::test]
async fn load_config_from_toml_path_sets_runtime_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("legacy.toml");
    tokio::fs::write(&path, "default_model = \"model\"\ndefault_temperature = 0.7\n")
        .await
        .unwrap();
    let workspace = tmp.path().join("workspace");

    let loaded = load_config_from_toml_path(
        &path,
        workspace.clone(),
        tmp.path(),
        ConfigResolutionSource::DefaultConfigDir,
    )
    .await
    .unwrap();

    assert_eq!(loaded.config_path, path);
    assert_eq!(loaded.workspace_dir, workspace);
    assert_eq!(loaded.default_model.as_deref(), Some("model"));
}

#[tokio::test]
async fn load_config_from_toml_path_normalizes_legacy_gateway_table() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("legacy.toml");
    tokio::fs::write(&path, "default_temperature = 0.7\n[Gateway]\nauth_enabled = true\n")
        .await
        .unwrap();

    let loaded = load_config_from_toml_path(
        &path,
        tmp.path().join("workspace"),
        tmp.path(),
        ConfigResolutionSource::DefaultConfigDir,
    )
    .await
    .unwrap();

    assert!(loaded.gateway.auth_enabled);
}

#[tokio::test]
async fn load_config_from_toml_path_reports_read_and_parse_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("legacy.toml");

    let read_error = load_config_from_toml_path(
        &path,
        tmp.path().join("workspace"),
        tmp.path(),
        ConfigResolutionSource::DefaultConfigDir,
    )
    .await
    .unwrap_err();
    assert!(read_error.to_string().contains("Failed to read config file"));

    tokio::fs::write(&path, "default_temperature = [").await.unwrap();
    let parse_error = load_config_from_toml_path(
        &path,
        tmp.path().join("workspace"),
        tmp.path(),
        ConfigResolutionSource::DefaultConfigDir,
    )
    .await
    .unwrap_err();
    assert!(parse_error.to_string().contains("Failed to parse config file"));
}

#[tokio::test]
async fn load_config_from_toml_path_propagates_deserialize_secret_and_validation_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("legacy.toml");

    for (contents, expected_error) in [
        ("default_temperature = \"warm\"\n", "Failed to deserialize config file"),
        (
            "default_temperature = 0.7\napi_key = \"enc2:not-hex\"\n",
            "Failed to decrypt config.api_key",
        ),
        (
            "default_temperature = 0.7\n[autonomy]\nmax_actions_per_hour = 0\n",
            "autonomy.max_actions_per_hour must be greater than 0",
        ),
    ] {
        tokio::fs::write(&path, contents).await.unwrap();

        let error = load_config_from_toml_path(
            &path,
            tmp.path().join("workspace"),
            tmp.path(),
            ConfigResolutionSource::DefaultConfigDir,
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains(expected_error), "unexpected error: {error:#}");
    }
}

#[tokio::test]
async fn load_config_merges_tenant_base_project_override_then_environment() {
    let temp_home = tempfile::tempdir().unwrap();
    let config_dir = temp_home.path().join("config");
    let project_dir = temp_home.path().join("project");
    tokio::fs::create_dir_all(&config_dir).await.unwrap();
    tokio::fs::create_dir_all(&project_dir).await.unwrap();

    let store = SystemConfigStore::new(
        GlobalDatabase::new(temp_home.path().join("database/_index.sqlite3")).unwrap(),
    )
    .unwrap();
    let tenant = TenantScope::new("platform-a", "company-a").unwrap();
    let other_tenant = TenantScope::new("platform-a", "company-b").unwrap();
    store
        .upsert(
            &tenant,
            &json!({
                "default_model": "database-model",
                "default_provider": "database-provider",
                "default_temperature": 0.2
            })
            .to_string(),
        )
        .unwrap();
    store
        .upsert(
            &other_tenant,
            &json!({
                "default_model": "other-model",
                "default_provider": "other-provider",
                "default_temperature": 0.9
            })
            .to_string(),
        )
        .unwrap();

    let override_contents = json!({
        "default_provider": "project-provider",
        "default_model": "project-model"
    })
    .to_string();
    store.upsert_project(&tenant, project_dir.to_str().unwrap(), &override_contents).unwrap();

    let loaded = load_config_for_tenant_from_store(
        &store,
        &tenant,
        Some(&project_dir),
        &config_dir,
        project_dir.clone(),
        ConfigResolutionSource::DefaultConfigDir,
        |config| config.default_model = Some("environment-model".to_string()),
    )
    .await
    .unwrap();

    assert_eq!(loaded.default_temperature, 0.2, "database base must be retained");
    assert_eq!(loaded.default_provider.as_deref(), Some("project-provider"));
    assert_eq!(loaded.default_model.as_deref(), Some("environment-model"));
    assert_eq!(loaded.workspace_dir, project_dir);
    assert_eq!(loaded.config_path, config_json_path(&config_dir));
    assert!(!project_dir.join(".vibewindow.json").exists());

    let other_loaded = load_config_for_tenant_from_store(
        &store,
        &other_tenant,
        None,
        &config_dir,
        config_dir.join("workspace"),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(other_loaded.default_model.as_deref(), Some("other-model"));
    assert_eq!(other_loaded.default_provider.as_deref(), Some("other-provider"));
    assert_eq!(other_loaded.default_temperature, 0.9);

    let clean_project = temp_home.path().join("nested/clean-project");
    tokio::fs::create_dir_all(&clean_project).await.unwrap();
    tokio::fs::write(
        clean_project.join(".vibewindow.json"),
        r#"{"default_provider":"must-not-be-scanned"}"#,
    )
    .await
    .unwrap();
    let clean_project_loaded = load_config_for_tenant_from_store(
        &store,
        &tenant,
        Some(&clean_project),
        &config_dir,
        clean_project.clone(),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(clean_project_loaded.default_model.as_deref(), Some("database-model"));
    assert_eq!(clean_project_loaded.default_provider.as_deref(), Some("database-provider"));
    assert!(clean_project.join(".vibewindow.json").exists());

    let new_tenant = TenantScope::new("platform-b", "company-new").unwrap();
    let initialized = load_config_for_tenant_from_store(
        &store,
        &new_tenant,
        None,
        &config_dir,
        config_dir.join("workspace"),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(initialized.default_provider.as_deref(), Some("shiju"));
    let initialized_payload: serde_json::Value =
        serde_json::from_str(&store.get(&new_tenant).unwrap().unwrap()).unwrap();
    assert!(initialized_payload.is_object());
    assert!(initialized_payload.get("gateway").is_some());
}

#[tokio::test]
async fn load_config_from_store_normalizes_legacy_model_aliases_without_overriding_current_keys() {
    let temp_home = tempfile::tempdir().unwrap();
    let config_dir = temp_home.path().join("config");
    tokio::fs::create_dir_all(&config_dir).await.unwrap();
    let store = SystemConfigStore::new(
        GlobalDatabase::new(temp_home.path().join("database/_index.sqlite3")).unwrap(),
    )
    .unwrap();
    let legacy_tenant = TenantScope::new("platform", "legacy-company").unwrap();
    let conflict_tenant = TenantScope::new("platform", "conflict-company").unwrap();
    store
        .upsert(
            &legacy_tenant,
            &json!({
                "model": "legacy-model",
                "model_provider": "legacy-provider",
                "default_temperature": 0.7
            })
            .to_string(),
        )
        .unwrap();
    store
        .upsert(
            &conflict_tenant,
            &json!({
                "default_model": "current-model",
                "model": "legacy-model",
                "default_provider": "current-provider",
                "model_provider": "legacy-provider",
                "default_temperature": 0.7
            })
            .to_string(),
        )
        .unwrap();

    let legacy = load_config_for_tenant_from_store(
        &store,
        &legacy_tenant,
        None,
        &config_dir,
        config_dir.join("legacy-workspace"),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(legacy.default_model.as_deref(), Some("legacy-model"));
    assert_eq!(legacy.default_provider.as_deref(), Some("legacy-provider"));

    let conflict = load_config_for_tenant_from_store(
        &store,
        &conflict_tenant,
        None,
        &config_dir,
        config_dir.join("conflict-workspace"),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(conflict.default_model.as_deref(), Some("current-model"));
    assert_eq!(conflict.default_provider.as_deref(), Some("current-provider"));
}

#[tokio::test]
async fn load_config_from_store_rejects_malformed_and_non_object_base_payloads() {
    let temp_home = tempfile::tempdir().unwrap();
    let config_dir = temp_home.path().join("config");
    tokio::fs::create_dir_all(&config_dir).await.unwrap();
    let store = SystemConfigStore::new(
        GlobalDatabase::new(temp_home.path().join("database/_index.sqlite3")).unwrap(),
    )
    .unwrap();
    let malformed_tenant = TenantScope::new("platform", "malformed-company").unwrap();
    let array_tenant = TenantScope::new("platform", "array-company").unwrap();
    store.upsert(&malformed_tenant, "{").unwrap();
    store.upsert(&array_tenant, "[]").unwrap();

    let malformed_error = load_config_for_tenant_from_store(
        &store,
        &malformed_tenant,
        None,
        &config_dir,
        config_dir.join("workspace"),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(malformed_error.to_string().contains("Failed to parse tenant system config payload"));

    let array_error = load_config_for_tenant_from_store(
        &store,
        &array_tenant,
        None,
        &config_dir,
        config_dir.join("workspace"),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(
        array_error.to_string().contains("tenant system config payload root must be an object")
    );
}

#[tokio::test]
async fn load_config_rejects_invalid_project_override_without_mutating_base() {
    let temp_home = tempfile::tempdir().unwrap();
    let config_dir = temp_home.path().join("config");
    let project_dir = temp_home.path().join("project");
    tokio::fs::create_dir_all(&config_dir).await.unwrap();
    tokio::fs::create_dir_all(&project_dir).await.unwrap();

    let store = SystemConfigStore::new(
        GlobalDatabase::new(temp_home.path().join("database/_index.sqlite3")).unwrap(),
    )
    .unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let base_payload = json!({
        "default_model": "database-model",
        "gateway": { "host": "127.0.0.1" }
    })
    .to_string();
    store.upsert(&tenant, &base_payload).unwrap();

    let project_key = project_dir.to_str().unwrap();
    store.upsert_project(&tenant, project_key, "[]").unwrap();

    let error = load_config_for_tenant_from_store(
        &store,
        &tenant,
        Some(&project_dir),
        &config_dir,
        project_dir.clone(),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("tenant project config payload root must be an object"));
    assert_eq!(store.get(&tenant).unwrap().as_deref(), Some(base_payload.as_str()));
    assert_eq!(store.get_project(&tenant, project_key).unwrap().as_deref(), Some("[]"));

    let missing_tenant = TenantScope::new("platform", "missing-company").unwrap();
    let missing = load_config_for_tenant_from_store(
        &store,
        &missing_tenant,
        Some(&project_dir),
        &config_dir,
        project_dir.clone(),
        ConfigResolutionSource::DefaultConfigDir,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(missing.default_provider.as_deref(), Some("shiju"));
    assert!(store.get(&missing_tenant).unwrap().is_some());
    assert_eq!(store.get_project(&missing_tenant, project_key).unwrap(), None);

    for (contents, expected_error) in [
        (r#"{"gateway":{"host":""}}"#, "gateway.host must not be empty"),
        (r#"{"api_key":"enc2:not-hex"}"#, "Failed to decrypt config.api_key"),
    ] {
        store.upsert_project(&tenant, project_key, contents).unwrap();
        let error = load_config_for_tenant_from_store(
            &store,
            &tenant,
            Some(&project_dir),
            &config_dir,
            project_dir.clone(),
            ConfigResolutionSource::DefaultConfigDir,
            |_| {},
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains(expected_error), "unexpected error: {error:#}");
        assert_eq!(store.get(&tenant).unwrap().as_deref(), Some(base_payload.as_str()));
        assert_eq!(store.get_project(&tenant, project_key).unwrap().as_deref(), Some(contents));
        assert!(!project_dir.join(".vibewindow.json").exists());
    }
}
