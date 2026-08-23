//! 验证会话界面配置的默认值、序列化与兼容字段行为。
//! 更新单个配置字段必须保留同级键；畸形旧配置不得被新写入静默覆盖。

use super::*;
use crate::gateway::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;
use vw_api_types::id::TenantScope;

static GLOBAL_PATHS_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();

#[test]
fn ui_config_round_trips_through_gateway_settings() {
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database.clone()).expect("gateway settings store");
    let legacy_path = vw_config_types::paths::home_config_dir(home.path())
        .join("storage/desktop/preferences.json");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("create legacy directory");
    std::fs::write(&legacy_path, r#"{"legacy":true}"#).expect("write legacy ui config");

    assert_eq!(
        load_app_config_from_store(&store, &TenantScope::default()).expect("load empty ui config"),
        serde_json::json!({})
    );
    set_config_field_in_store(
        &store,
        &TenantScope::default(),
        "panel",
        serde_json::json!({"enabled": true}),
    )
    .expect("save ui config");

    let reopened = GatewaySettingsStore::new(database).expect("reopen gateway settings store");
    let cfg =
        load_app_config_from_store(&reopened, &TenantScope::default()).expect("reload ui config");
    assert_eq!(cfg["panel"]["enabled"], true);
    let stored = reopened
        .get(&TenantScope::default(), "desktop.preferences", "default")
        .expect("read ui config")
        .expect("stored ui config");
    assert_eq!(serde_json::from_str::<Value>(&stored).expect("stored json"), cfg);
    assert_eq!(
        std::fs::read_to_string(&legacy_path).expect("legacy ui config remains"),
        r#"{"legacy":true}"#
    );
}

#[test]
fn ui_config_isolates_the_same_key_across_tenants() {
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database).expect("gateway settings store");
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-b", "company-b").expect("tenant B");

    set_config_field_in_store(&store, &tenant_a, "archived", serde_json::json!(["shared"]))
        .expect("save tenant A config");
    set_config_field_in_store(&store, &tenant_b, "archived", serde_json::json!(["other"]))
        .expect("save tenant B config");

    assert_eq!(
        load_app_config_from_store(&store, &tenant_a).expect("load tenant A")["archived"],
        serde_json::json!(["shared"])
    );
    assert_eq!(
        load_app_config_from_store(&store, &tenant_b).expect("load tenant B")["archived"],
        serde_json::json!(["other"])
    );
}

#[test]
fn load_app_config_treats_non_object_payloads_as_empty() {
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database).expect("gateway settings store");
    let tenant = TenantScope::default();

    for payload in ["null", "[]", r#""scalar""#, "42", "true"] {
        store
            .upsert(&tenant, UI_CONFIG_NAMESPACE, UI_CONFIG_KEY, payload)
            .expect("store non-object payload");

        assert_eq!(
            load_app_config_from_store(&store, &tenant).expect("load non-object payload"),
            serde_json::json!({}),
            "payload {payload} should not become UI config"
        );
    }
}

#[test]
fn load_app_config_reports_malformed_json() {
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database).expect("gateway settings store");
    let tenant = TenantScope::default();
    store
        .upsert(&tenant, UI_CONFIG_NAMESPACE, UI_CONFIG_KEY, r#"{"panel":true"#)
        .expect("store malformed payload");

    let error = load_app_config_from_store(&store, &tenant).expect_err("reject malformed payload");

    assert!(error.to_string().contains("decode desktop UI config"), "unexpected error: {error:#}");
}

#[test]
fn set_config_field_preserves_siblings_and_replaces_existing_field() {
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database).expect("gateway settings store");
    let tenant = TenantScope::default();
    store
        .upsert(
            &tenant,
            UI_CONFIG_NAMESPACE,
            UI_CONFIG_KEY,
            r#"{"panel":{"enabled":false},"theme":"dark"}"#,
        )
        .expect("store initial config");

    set_config_field_in_store(&store, &tenant, "panel", serde_json::json!({"enabled": true}))
        .expect("replace panel config");

    assert_eq!(
        load_app_config_from_store(&store, &tenant).expect("load updated config"),
        serde_json::json!({
            "panel": {"enabled": true},
            "theme": "dark",
        })
    );
}

#[test]
// 畸形旧值必须保留供诊断，局部更新不能借机把整个配置静默重置为空对象。
fn set_config_field_rejects_malformed_existing_payload_without_overwriting_it() {
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database).expect("gateway settings store");
    let tenant = TenantScope::default();
    let malformed = r#"{"panel":true"#;
    store
        .upsert(&tenant, UI_CONFIG_NAMESPACE, UI_CONFIG_KEY, malformed)
        .expect("store malformed payload");

    let error = set_config_field_in_store(&store, &tenant, "theme", serde_json::json!("dark"))
        .expect_err("malformed config must not be replaced");

    assert!(error.to_string().contains("decode desktop UI config"));
    assert_eq!(
        store.get(&tenant, UI_CONFIG_NAMESPACE, UI_CONFIG_KEY).expect("read payload").as_deref(),
        Some(malformed)
    );
}

#[test]
fn ui_config_public_wrapper_helpers_fallback_on_store_errors() {
    let tenant = TenantScope::default();
    let load =
        load_app_config_from_store_result(Err(anyhow::anyhow!("settings unavailable")), &tenant);
    assert_eq!(load, serde_json::json!({}));

    set_config_field_with_store_result(
        Err(anyhow::anyhow!("settings unavailable")),
        &tenant,
        "theme",
        serde_json::json!("dark"),
    );
}

#[test]
fn ui_config_public_wrappers_use_runtime_settings_store() {
    let _guard = GLOBAL_PATHS_ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("global paths env lock");
    let home = tempfile::tempdir().expect("temporary home");
    unsafe {
        std::env::set_var("VIBEWINDOW_TEST_HOME", home.path());
    }

    set_config_field(&TenantScope::default(), "theme", serde_json::json!("dark"));
    let cfg = load_app_config(&TenantScope::default());

    assert_eq!(cfg["theme"], "dark");
    unsafe {
        std::env::remove_var("VIBEWINDOW_TEST_HOME");
    }
}
