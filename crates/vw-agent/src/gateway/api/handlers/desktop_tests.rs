// 覆盖桌面偏好、项目设置、外部应用及服务命令接口。

use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use super::*;
use crate::app::agent::storage;
use crate::gateway::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;
use vw_api_types::id::TenantScope;

static DESKTOP_STORAGE_LOCK: Mutex<()> = Mutex::const_new(());

struct DesktopStorageSnapshot {
    json_tool_content: Option<ToolContentBody>,
    sql_tool_content: Option<ToolContentBody>,
    html_tool_content: Option<ToolContentBody>,
}

impl DesktopStorageSnapshot {
    async fn capture() -> Self {
        Self {
            json_tool_content: storage::read(&["desktop", "tool_content", "json"]).await.ok(),
            sql_tool_content: storage::read(&["desktop", "tool_content", "sql"]).await.ok(),
            html_tool_content: storage::read(&["desktop", "tool_content", "html"]).await.ok(),
        }
    }

    async fn restore(self) {
        restore_key(&["desktop", "tool_content", "json"], self.json_tool_content).await;
        restore_key(&["desktop", "tool_content", "sql"], self.sql_tool_content).await;
        restore_key(&["desktop", "tool_content", "html"], self.html_tool_content).await;
    }
}

async fn restore_key<T: serde::Serialize>(key: &[&str], value: Option<T>) {
    match value {
        Some(value) => storage::write(key, &value).await.expect("restore desktop storage key"),
        None => storage::remove(key).await.expect("remove desktop storage key"),
    }
}

async fn clear_desktop_storage() {
    storage::remove(&["desktop", "tool_content", "json"]).await.expect("clear json content");
    storage::remove(&["desktop", "tool_content", "sql"]).await.expect("clear sql content");
    storage::remove(&["desktop", "tool_content", "html"]).await.expect("clear html content");
}

#[test]
fn merge_preferences_patch_initializes_empty_preferences() {
    let mut current = Value::Null;

    merge_preferences_patch(
        &mut current,
        &json!({
            "model": "openai/gpt-5.1-codex-max",
            "auto_model": false
        }),
    );

    assert_eq!(
        current,
        json!({
            "model": "openai/gpt-5.1-codex-max",
            "auto_model": false
        })
    );
}

#[test]
fn merge_preferences_patch_null_removes_existing_key() {
    let mut current = json!({
        "model": "openai/gpt-5.1-codex-max",
        "auto_model": false
    });

    merge_preferences_patch(&mut current, &json!({ "model": null }));

    assert_eq!(current, json!({ "auto_model": false }));
}

#[test]
fn merge_preferences_patch_ignores_non_object_patch() {
    let mut current = json!({ "theme": "dark" });

    merge_preferences_patch(&mut current, &json!(["ignored"]));

    assert_eq!(current, json!({ "theme": "dark" }));
}

#[test]
fn merge_preferences_patch_replaces_nested_values_without_deep_merge() {
    let mut current = json!({
        "editor": {
            "font_size": 14,
            "word_wrap": true
        },
        "theme": "dark"
    });

    merge_preferences_patch(
        &mut current,
        &json!({
            "editor": {
                "font_size": 16
            }
        }),
    );

    assert_eq!(
        current,
        json!({
            "editor": {
                "font_size": 16
            },
            "theme": "dark"
        })
    );
}

#[test]
fn merge_preferences_patch_normalizes_scalar_current_before_ignoring_scalar_patch() {
    let mut current = json!("legacy-scalar");

    merge_preferences_patch(&mut current, &json!(false));

    assert_eq!(current, json!({}));
}

#[test]
fn service_command_normalization_allows_only_lifecycle_commands() {
    assert_eq!(normalize_service_command(" STATUS "), Some("status"));
    assert_eq!(normalize_service_command("install"), Some("install"));
    assert_eq!(normalize_service_command("start"), Some("start"));
    assert_eq!(normalize_service_command("stop"), Some("stop"));
    assert_eq!(normalize_service_command("restart"), Some("restart"));
    assert_eq!(normalize_service_command("uninstall"), Some("uninstall"));
    assert_eq!(normalize_service_command("daemon"), None);
    assert_eq!(normalize_service_command("../status"), None);
}

#[tokio::test]
async fn service_command_handler_rejects_unknown_command_before_loading_config() {
    let err = service_command_post(Path(ServiceCommandPath { command: "../../shell".to_string() }))
        .await
        .expect_err("unknown service command should fail");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.to_string(), "unsupported service command");
}

#[test]
fn service_command_response_round_trips_through_json() {
    let response =
        ServiceCommandResponse { command: "status".to_string(), output: "running".to_string() };

    let payload = serde_json::to_string(&response).expect("serialize service command response");
    let decoded: ServiceCommandResponse =
        serde_json::from_str(&payload).expect("deserialize service command response");

    assert_eq!(decoded, response);
}

#[test]
fn preferences_handlers_default_patch_and_persist_values() {
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database.clone()).expect("gateway settings store");
    let preferences_file = vw_config_types::paths::home_config_dir(home.path())
        .join("storage/desktop/preferences.json");

    let tenant = TenantScope::default();
    let Json(defaults) =
        preferences_get_from_store(&store, &tenant).expect("preferences should load");
    assert_eq!(defaults, Value::Null);
    assert!(!preferences_file.exists());

    let Json(patched) = preferences_patch_in_store(
        &store,
        &tenant,
        Json(json!({
            "model": "openai/gpt-5.1-codex-max",
            "auto_model": false
        })),
    )
    .expect("preferences should patch");
    assert_eq!(patched["model"], "openai/gpt-5.1-codex-max");

    let Json(patched) = preferences_patch_in_store(
        &store,
        &tenant,
        Json(json!({
            "model": null,
            "temperature": 0.2
        })),
    )
    .expect("preferences should patch again");
    assert_eq!(patched, json!({ "auto_model": false, "temperature": 0.2 }));
    assert!(!preferences_file.exists());

    let reopened = GatewaySettingsStore::new(database).expect("reopen gateway settings store");
    let Json(stored) =
        preferences_get_from_store(&reopened, &tenant).expect("preferences should read back");
    assert_eq!(stored, patched);

    let legacy_home = tempfile::tempdir().expect("temporary legacy home");
    let legacy_store = GatewaySettingsStore::new(
        GlobalDatabase::for_home(legacy_home.path()).expect("legacy global database"),
    )
    .expect("legacy gateway settings store");
    let legacy_path = vw_config_types::paths::home_config_dir(legacy_home.path())
        .join("storage/desktop/preferences.json");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("create legacy directory");
    std::fs::write(&legacy_path, r#"{"theme":"legacy"}"#).expect("write legacy preferences");
    let Json(ignored) = preferences_get_from_store(&legacy_store, &tenant)
        .expect("legacy preferences should be ignored");
    assert_eq!(ignored, Value::Null);
    legacy_store
        .upsert(&tenant, "desktop.preferences", "default", r#"{"theme":"database"}"#)
        .expect("seed conflicting database value");

    let Json(conflict) =
        preferences_get_from_store(&legacy_store, &tenant).expect("database preference should win");
    assert_eq!(conflict, json!({ "theme": "database" }));
    assert_eq!(
        std::fs::read_to_string(&legacy_path).expect("legacy preferences remain"),
        r#"{"theme":"legacy"}"#
    );
}

#[test]
fn preferences_handlers_isolate_same_key_by_tenant_scope() {
    let home = tempfile::tempdir().expect("temporary home");
    let store =
        GatewaySettingsStore::new(GlobalDatabase::for_home(home.path()).expect("global database"))
            .expect("gateway settings store");
    let first = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second = TenantScope::new("platform-b", "company-b").expect("second tenant");

    let Json(_) = preferences_patch_in_store(&store, &first, Json(json!({"theme": "first"})))
        .expect("first preference");
    let Json(_) = preferences_patch_in_store(&store, &second, Json(json!({"theme": "second"})))
        .expect("second preference");

    let Json(first_value) =
        preferences_get_from_store(&store, &first).expect("first preference read");
    let Json(second_value) =
        preferences_get_from_store(&store, &second).expect("second preference read");
    assert_eq!(first_value, json!({"theme": "first"}));
    assert_eq!(second_value, json!({"theme": "second"}));
}

#[test]
fn preferences_get_reports_corrupt_stored_payload() {
    let home = tempfile::tempdir().expect("temporary home");
    let store =
        GatewaySettingsStore::new(GlobalDatabase::for_home(home.path()).expect("global database"))
            .expect("gateway settings store");
    let tenant = TenantScope::default();
    store
        .upsert(&tenant, DESKTOP_PREFERENCES_NAMESPACE, DEFAULT_CONFIG_KEY, "not-json")
        .expect("seed corrupt preferences");

    let err =
        preferences_get_from_store(&store, &tenant).expect_err("corrupt preferences should fail");

    assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(err.to_string().starts_with("invalid desktop preferences:"));
}

#[tokio::test]
async fn tool_content_handlers_validate_type_and_round_trip_content() {
    let _guard = DESKTOP_STORAGE_LOCK.lock().await;
    let snapshot = DesktopStorageSnapshot::capture().await;
    clear_desktop_storage().await;

    let Json(default_content) =
        tool_content_get(Path(ToolContentPath { tool_type: "json".to_string() }))
            .await
            .expect("default content should load");
    assert_eq!(default_content, json!({ "content": "" }));

    let Json(saved) = tool_content_put(
        Path(ToolContentPath { tool_type: "html".to_string() }),
        Json(ToolContentBody { content: "<main>Hi</main>".to_string() }),
    )
    .await
    .expect("content should save");
    assert_eq!(saved, json!({ "content": "<main>Hi</main>" }));

    let Json(read_back) = tool_content_get(Path(ToolContentPath { tool_type: "html".to_string() }))
        .await
        .expect("saved content should load");
    assert_eq!(read_back, saved);

    let Json(saved_sql) = tool_content_put(
        Path(ToolContentPath { tool_type: "sql".to_string() }),
        Json(ToolContentBody { content: "SELECT 1;".to_string() }),
    )
    .await
    .expect("sql content should save");
    let Json(read_sql) = tool_content_get(Path(ToolContentPath { tool_type: "sql".to_string() }))
        .await
        .expect("sql content should load");
    assert_eq!(read_sql, saved_sql);

    let err = tool_content_get(Path(ToolContentPath { tool_type: "yaml".to_string() }))
        .await
        .expect_err("unsupported get should fail");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.to_string(), "unsupported tool content type");

    let err = tool_content_put(
        Path(ToolContentPath { tool_type: "yaml".to_string() }),
        Json(ToolContentBody { content: "x".to_string() }),
    )
    .await
    .expect_err("unsupported put should fail");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);

    let Json(sql_after_rejection) =
        tool_content_get(Path(ToolContentPath { tool_type: "sql".to_string() }))
            .await
            .expect("valid content should remain readable");
    assert_eq!(sql_after_rejection, json!({ "content": "SELECT 1;" }));

    snapshot.restore().await;
}

#[test]
fn project_preferences_handlers_report_missing_and_round_trip() {
    let project_path = "/tmp/vw project prefs/work.demo";
    let home = tempfile::tempdir().expect("temporary home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let store = GatewaySettingsStore::new(database.clone()).expect("gateway settings store");
    let project_file = vw_config_types::paths::home_config_dir(home.path())
        .join("storage/desktop/project_prefs/tmp_vw_project_prefs_work_demo.json");

    let tenant = TenantScope::default();
    let missing = project_preferences_get_from_store(
        &store,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: project_path.to_string() }),
    )
    .expect_err("missing prefs should fail");
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(!project_file.exists());

    let body = ProjectPreferencesBody {
        model: "anthropic/claude-sonnet-4-5".to_string(),
        auto_model: true,
        acp_agent: Some("codex".to_string()),
    };
    let Json(saved) = project_preferences_put_in_store(
        &store,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: project_path.to_string() }),
        Json(body.clone()),
    )
    .expect("prefs should save");
    assert_eq!(saved.model, body.model);
    assert!(saved.auto_model);
    assert_eq!(saved.acp_agent, Some("codex".to_string()));

    let reopened = GatewaySettingsStore::new(database).expect("reopen gateway settings store");
    let Json(read_back) = project_preferences_get_from_store(
        &reopened,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: project_path.to_string() }),
    )
    .expect("prefs should read back");
    assert_eq!(read_back.model, body.model);
    assert!(read_back.auto_model);
    assert_eq!(read_back.acp_agent, Some("codex".to_string()));

    assert!(!project_file.exists());
    let stored = reopened
        .get(&tenant, "desktop.project_preferences", project_path)
        .expect("read exact project key")
        .expect("project preference row");
    assert_eq!(
        serde_json::from_str::<Value>(&stored).expect("stored project preferences"),
        serde_json::to_value(&body).expect("expected project preferences")
    );

    let legacy_home = tempfile::tempdir().expect("temporary legacy home");
    let legacy_store = GatewaySettingsStore::new(
        GlobalDatabase::for_home(legacy_home.path()).expect("legacy global database"),
    )
    .expect("legacy gateway settings store");
    let legacy_path = vw_config_types::paths::home_config_dir(legacy_home.path())
        .join("storage/desktop/project_prefs/tmp_vw_project_prefs_work_demo.json");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("create legacy project directory");
    std::fs::write(&legacy_path, r#"{"model":"legacy","auto_model":false,"acp_agent":null}"#)
        .expect("write legacy project preferences");
    let ignored = project_preferences_get_from_store(
        &legacy_store,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: project_path.to_string() }),
    )
    .expect_err("legacy project preferences should be ignored");
    assert_eq!(ignored.status, StatusCode::NOT_FOUND);
    legacy_store
        .upsert(&tenant, "desktop.project_preferences", project_path, &stored)
        .expect("seed conflicting project database value");
    let Json(conflict) = project_preferences_get_from_store(
        &legacy_store,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: project_path.to_string() }),
    )
    .expect("database project preferences should win");
    assert_eq!(conflict.model, body.model);
    assert_eq!(
        std::fs::read_to_string(&legacy_path).expect("legacy project preferences remain"),
        r#"{"model":"legacy","auto_model":false,"acp_agent":null}"#
    );

    let empty = project_preferences_get_from_store(
        &reopened,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: "  ".to_string() }),
    )
    .expect_err("blank project path should fail");
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
}

#[test]
fn project_preferences_handlers_isolate_same_project_by_tenant_scope() {
    let home = tempfile::tempdir().expect("temporary home");
    let store =
        GatewaySettingsStore::new(GlobalDatabase::for_home(home.path()).expect("global database"))
            .expect("gateway settings store");
    let first = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second = TenantScope::new("platform-b", "company-b").expect("second tenant");
    let query = || Query(ProjectPreferencesQuery { project_path: "/shared/project".to_string() });

    let Json(_) = project_preferences_put_in_store(
        &store,
        &first,
        query(),
        Json(ProjectPreferencesBody {
            model: "first-model".to_string(),
            auto_model: false,
            acp_agent: None,
        }),
    )
    .expect("first project preference");
    let Json(_) = project_preferences_put_in_store(
        &store,
        &second,
        query(),
        Json(ProjectPreferencesBody {
            model: "second-model".to_string(),
            auto_model: true,
            acp_agent: Some("codex".to_string()),
        }),
    )
    .expect("second project preference");

    let Json(first_value) = project_preferences_get_from_store(&store, &first, query())
        .expect("first project preference read");
    let Json(second_value) = project_preferences_get_from_store(&store, &second, query())
        .expect("second project preference read");
    assert_eq!(first_value.model, "first-model");
    assert_eq!(second_value.model, "second-model");
}

#[test]
fn project_preferences_put_rejects_blank_path_without_persisting() {
    let home = tempfile::tempdir().expect("temporary home");
    let store =
        GatewaySettingsStore::new(GlobalDatabase::for_home(home.path()).expect("global database"))
            .expect("gateway settings store");
    let tenant = TenantScope::default();
    let blank_path = " \t ";

    let err = project_preferences_put_in_store(
        &store,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: blank_path.to_string() }),
        Json(ProjectPreferencesBody {
            model: "unused".to_string(),
            auto_model: false,
            acp_agent: None,
        }),
    )
    .expect_err("blank project path should fail");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.to_string(), "project path must not be empty");
    assert_eq!(
        store
            .list(&tenant, DESKTOP_PROJECT_PREFERENCES_NAMESPACE)
            .expect("check rejected project namespace"),
        Vec::new()
    );
}

#[test]
fn project_preferences_get_reports_corrupt_stored_payload() {
    let home = tempfile::tempdir().expect("temporary home");
    let store =
        GatewaySettingsStore::new(GlobalDatabase::for_home(home.path()).expect("global database"))
            .expect("gateway settings store");
    let tenant = TenantScope::default();
    let project_path = "/tmp/corrupt-project-preferences";
    store
        .upsert(&tenant, DESKTOP_PROJECT_PREFERENCES_NAMESPACE, project_path, r#"{"model":42}"#)
        .expect("seed corrupt project preferences");

    let err = project_preferences_get_from_store(
        &store,
        &tenant,
        Query(ProjectPreferencesQuery { project_path: project_path.to_string() }),
    )
    .expect_err("corrupt project preferences should fail");

    assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(err.to_string().starts_with("invalid desktop project preferences:"));
}

#[test]
fn project_preferences_json_defaults_and_omits_optional_agent() {
    let body: ProjectPreferencesBody = serde_json::from_value(json!({
        "model": "default-model",
        "auto_model": true
    }))
    .expect("deserialize project preferences without agent");
    assert_eq!(body.acp_agent, None);

    let response = ProjectPreferencesResponse {
        model: body.model,
        auto_model: body.auto_model,
        acp_agent: body.acp_agent,
    };
    assert_eq!(
        serde_json::to_value(response).expect("serialize project preferences response"),
        json!({
            "model": "default-model",
            "auto_model": true
        })
    );
}

#[tokio::test]
async fn external_apps_get_reports_platform_and_finder() {
    let Json(response) = external_apps_get().await.expect("external apps should load");

    assert_eq!(response.platform, host_platform());
    assert!(response.apps.iter().any(|app| app.id == "finder" && app.available));
    assert_eq!(detect_external_apps().first().map(|app| app.id.as_str()), Some("finder"));
}

#[tokio::test]
async fn external_open_handler_rejects_unknown_target_without_spawning() {
    let err = external_apps_open_post(Json(ExternalOpenRequest {
        path: "/tmp".to_string(),
        target: "definitely-unsupported".to_string(),
    }))
    .await
    .expect_err("unknown target should fail");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.to_string(), "unsupported external app target");
}

#[test]
fn decode_file_url_path_decodes_plain_and_file_url_paths() {
    assert_eq!(decode_file_url_path("/tmp/space path"), "/tmp/space path");
    assert_eq!(decode_file_url_path("file:///tmp/space%20path"), "/tmp/space path");
    assert_eq!(decode_file_url_path("file://relative%2Fpath"), "relative/path");
}

#[test]
fn decode_file_url_path_preserves_invalid_percent_encoding() {
    assert_eq!(decode_file_url_path("plain%GGpath"), "plain%GGpath");
    #[cfg(not(windows))]
    {
        assert_eq!(decode_file_url_path("file:///tmp/%GG"), "/tmp/%GG");
        assert_eq!(decode_file_url_path("file:///"), "/");
    }
}

#[test]
fn validate_project_path_accepts_nonblank_and_rejects_whitespace() {
    assert!(super::validate_project_path("/tmp/project").is_ok());

    let error = super::validate_project_path(" \n\t ")
        .expect_err("whitespace-only project path should fail");
    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "project path must not be empty");
}

#[test]
fn router_can_be_constructed_for_unit_state() {
    let _router = router::<()>();
}
