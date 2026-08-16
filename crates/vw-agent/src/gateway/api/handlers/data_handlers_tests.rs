use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::extract::Extension;
use axum::http::StatusCode;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use vw_api_types::data::{
    AiDataConnectionKind, AiDataCountMode, AiDataQueryKind, AiDataReportSourceDto, AiDataSourceMode,
};
use vw_api_types::id::TenantScope;

use crate::app::agent::storage;

use storage_support::AI_DATA_STORAGE_TEST_LOCK as AI_DATA_HANDLER_STORAGE_LOCK;

fn tenant() -> Extension<TenantScope> {
    Extension(TenantScope::default())
}

struct UnusedProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for UnusedProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(String::new())
    }
}

struct AiDataStorageSnapshot {
    settings: Option<AiDataSettings>,
    connections: Option<Vec<AiDataConnectionDto>>,
    reports: Option<Vec<AiDataReportDto>>,
}

impl AiDataStorageSnapshot {
    async fn capture() -> Self {
        let config_store = storage_support::local_config_store(TenantScope::default())
            .expect("AI-DATA config store");
        Self {
            settings: config_store.load_settings_optional().expect("capture AI-DATA settings"),
            connections: Some(
                config_store.load_connections().expect("capture AI-DATA connections"),
            ),
            reports: storage::read(&["ai_data", "reports"]).await.ok(),
        }
    }

    async fn restore(self) {
        let config_store = storage_support::local_config_store(TenantScope::default())
            .expect("AI-DATA config store");
        config_store.delete_settings().expect("clear AI-DATA settings before restore");
        if let Some(settings) = self.settings {
            config_store.save_settings(&settings).expect("restore AI-DATA settings");
        }
        config_store
            .save_connections(self.connections.as_deref().unwrap_or_default())
            .expect("restore AI-DATA connections");
        restore_key(&["ai_data", "reports"], self.reports).await;
    }
}

async fn restore_key<T: serde::Serialize>(key: &[&str], value: Option<T>) {
    match value {
        Some(value) => storage::write(key, &value).await.expect("restore ai-data storage key"),
        None => storage::remove(key).await.expect("remove ai-data storage key"),
    }
}

async fn clear_ai_data_storage() {
    let config_store =
        storage_support::local_config_store(TenantScope::default()).expect("AI-DATA config store");
    config_store.delete_settings().expect("clear AI-DATA settings");
    config_store.save_connections(&[]).expect("clear AI-DATA connections");
    storage::remove(&["ai_data", "reports"]).await.expect("clear ai-data reports");
}

fn connection_body(kind: AiDataConnectionKind) -> AiDataConnectionUpsertBody {
    AiDataConnectionUpsertBody {
        name: " Main ".to_string(),
        kind,
        description: Some("  ".to_string()),
        enabled: true,
        read_only: true,
        base_url: Some(" https://api.example.test ".to_string()),
        connection_url: Some(" sqlite://unused ".to_string()),
        sqlite_path: Some(" /tmp/example.db ".to_string()),
        default_path: Some(" /v1 ".to_string()),
        auth_token: Some(" token ".to_string()),
        headers: BTreeMap::new(),
        schema_hint: Some(" hint ".to_string()),
    }
}

fn connection(id: &str, name: &str, updated_at_ms: u64) -> AiDataConnectionDto {
    AiDataConnectionDto {
        id: id.to_string(),
        name: name.to_string(),
        kind: AiDataConnectionKind::Sqlite,
        description: None,
        enabled: true,
        read_only: true,
        base_url: None,
        connection_url: None,
        sqlite_path: Some("/tmp/example.db".to_string()),
        default_path: None,
        auth_token: None,
        headers: BTreeMap::new(),
        schema_hint: None,
        updated_at_ms,
        last_used_ms: None,
    }
}

#[tokio::test]
async fn data_connection_handlers_use_the_extension_tenant_for_same_id_records() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let first_tenant =
        TenantScope::new("data-handler-platform-a", "data-handler-company-a").expect("first");
    let second_tenant =
        TenantScope::new("data-handler-platform-b", "data-handler-company-b").expect("second");
    let first_store =
        storage_support::local_config_store(first_tenant.clone()).expect("first store");
    let second_store =
        storage_support::local_config_store(second_tenant.clone()).expect("second store");
    let first = vec![connection("shared", "Tenant A", 10)];
    let second = vec![connection("shared", "Tenant B", 20)];
    first_store.save_connections(&first).expect("save first tenant");
    second_store.save_connections(&second).expect("save second tenant");

    let Json(first_loaded) = data_connections_list(Extension(first_tenant)).await.expect("first");
    let Json(second_loaded) =
        data_connections_list(Extension(second_tenant)).await.expect("second");

    assert_eq!(first_loaded, first);
    assert_eq!(second_loaded, second);
    first_store.save_connections(&[]).expect("clear first tenant");
    second_store.save_connections(&[]).expect("clear second tenant");
}

fn report_source() -> AiDataReportSourceDto {
    AiDataReportSourceDto {
        source_key: "main".to_string(),
        connection_id: "conn-1".to_string(),
        query_kind: AiDataQueryKind::Sql,
        sql: Some("SELECT 1".to_string()),
        count_sql: None,
        cube_query: None,
        http_method: "GET".to_string(),
        http_path: None,
        http_body: None,
        append_pagination: true,
    }
}

fn report(id: &str, slug: &str, updated_at_ms: u64) -> AiDataReportDto {
    AiDataReportDto {
        id: id.to_string(),
        name: id.to_string(),
        slug: slug.to_string(),
        data_source: AiDataSourceMode::Normal,
        default_source_key: Some("main".to_string()),
        report_config: json!({
            "modules": [
                { "type": "table", "show": true },
                { "type": "chart", "show": false }
            ]
        }),
        sources: vec![report_source()],
        updated_at_ms,
    }
}

fn report_body() -> AiDataReportUpsertBody {
    AiDataReportUpsertBody {
        name: " Sales ".to_string(),
        slug: " sales ".to_string(),
        data_source: AiDataSourceMode::Normal,
        default_source_key: Some("main".to_string()),
        report_config: json!({ "modules": [] }),
        sources: vec![report_source()],
    }
}

fn make_sqlite_fixture(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT);
        INSERT INTO items VALUES (1, 'apple'), (2, 'pear');
        "#,
    )
    .unwrap();
}

fn app_state() -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider: Arc::new(UnusedProvider),
        model: "test-model".to_string(),
        temperature: 0.1,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

#[test]
fn validate_connection_upsert_accepts_required_fields_by_kind() {
    validate_connection_upsert(&connection_body(AiDataConnectionKind::Sqlite)).unwrap();
    validate_connection_upsert(&connection_body(AiDataConnectionKind::Mysql)).unwrap();
    validate_connection_upsert(&connection_body(AiDataConnectionKind::Postgres)).unwrap();
    validate_connection_upsert(&connection_body(AiDataConnectionKind::Cube)).unwrap();
    validate_connection_upsert(&connection_body(AiDataConnectionKind::Http)).unwrap();
}

#[test]
fn validate_connection_upsert_rejects_missing_required_fields() {
    let mut body = connection_body(AiDataConnectionKind::Sqlite);
    body.name = "   ".to_string();
    let err = validate_connection_upsert(&body).unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("连接名称"));

    let mut body = connection_body(AiDataConnectionKind::Sqlite);
    body.sqlite_path = Some(" ".to_string());
    assert!(validate_connection_upsert(&body).unwrap_err().message.contains("sqlite_path"));

    let mut body = connection_body(AiDataConnectionKind::Mysql);
    body.connection_url = None;
    assert!(validate_connection_upsert(&body).unwrap_err().message.contains("connection_url"));

    let mut body = connection_body(AiDataConnectionKind::Postgres);
    body.connection_url = Some("".to_string());
    assert!(validate_connection_upsert(&body).unwrap_err().message.contains("connection_url"));

    let mut body = connection_body(AiDataConnectionKind::Http);
    body.base_url = None;
    assert!(validate_connection_upsert(&body).unwrap_err().message.contains("base_url"));

    let mut body = connection_body(AiDataConnectionKind::Cube);
    body.base_url = Some(" ".to_string());
    assert!(validate_connection_upsert(&body).unwrap_err().message.contains("base_url"));
}

#[test]
fn validate_report_upsert_accepts_valid_payload_and_rejects_boundaries() {
    validate_report_upsert(&report_body()).unwrap();

    let mut body = report_body();
    body.name = " ".to_string();
    let err = validate_report_upsert(&body).unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("报表名称"));

    let mut body = report_body();
    body.slug = " ".to_string();
    assert!(validate_report_upsert(&body).unwrap_err().message.contains("slug"));

    let mut body = report_body();
    body.sources.clear();
    assert!(validate_report_upsert(&body).unwrap_err().message.contains("至少需要一个数据源"));
}

#[tokio::test]
async fn settings_handlers_return_defaults_and_clamp_updates() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;

    let Json(settings) = data_settings_get(tenant()).await.unwrap();
    assert_eq!(settings, AiDataSettings::default());

    let Json(settings) = data_settings_put(
        tenant(),
        Json(AiDataSettingsUpdateBody { default_limit: 0, default_timeout_secs: 999 }),
    )
    .await
    .unwrap();
    assert_eq!(settings.default_limit, 1);
    assert_eq!(settings.default_timeout_secs, 600);

    let Json(settings) = data_settings_put(
        tenant(),
        Json(AiDataSettingsUpdateBody { default_limit: 20_000, default_timeout_secs: 0 }),
    )
    .await
    .unwrap();
    assert_eq!(settings.default_limit, 10_000);
    assert_eq!(settings.default_timeout_secs, 1);
    assert_eq!(storage_support::load_settings(TenantScope::default()).await, settings);

    snapshot.restore().await;
}

#[tokio::test]
async fn connection_update_preserves_last_used_and_missing_mutations_are_rejected() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    let mut existing = connection("existing", "Existing", 10);
    existing.last_used_ms = Some(42);
    storage_support::save_connections(TenantScope::default(), &[existing]).await.unwrap();

    let Json(updated) = data_connection_update(
        tenant(),
        axum::extract::Path("existing".to_string()),
        Json(connection_body(AiDataConnectionKind::Sqlite)),
    )
    .await
    .unwrap();
    assert_eq!(updated.last_used_ms, Some(42));
    assert_eq!(
        storage_support::load_settings(TenantScope::default())
            .await
            .selected_connection_id
            .as_deref(),
        Some("existing")
    );

    let update_error = data_connection_update(
        tenant(),
        axum::extract::Path("missing".to_string()),
        Json(connection_body(AiDataConnectionKind::Sqlite)),
    )
    .await
    .unwrap_err();
    assert_eq!(update_error.status, StatusCode::NOT_FOUND);

    let activate_error =
        data_connection_activate(tenant(), axum::extract::Path("missing".to_string()))
            .await
            .unwrap_err();
    assert_eq!(activate_error.status, StatusCode::NOT_FOUND);

    snapshot.restore().await;
}

#[tokio::test]
async fn connection_delete_keeps_unrelated_selection_and_clears_last_selection() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    storage_support::save_connections(
        TenantScope::default(),
        &[connection("selected", "Selected", 10), connection("other", "Other", 20)],
    )
    .await
    .unwrap();
    storage_support::save_settings(
        TenantScope::default(),
        &AiDataSettings {
            selected_connection_id: Some("selected".to_string()),
            ..AiDataSettings::default()
        },
    )
    .await
    .unwrap();

    let Json(deleted) =
        data_connection_delete(tenant(), axum::extract::Path("other".to_string())).await.unwrap();
    assert_eq!(deleted["deleted_id"], "other");
    assert_eq!(
        storage_support::load_settings(TenantScope::default())
            .await
            .selected_connection_id
            .as_deref(),
        Some("selected")
    );

    let _ = data_connection_delete(tenant(), axum::extract::Path("selected".to_string()))
        .await
        .unwrap();
    assert!(
        storage_support::load_settings(TenantScope::default())
            .await
            .selected_connection_id
            .is_none()
    );

    snapshot.restore().await;
}

#[tokio::test]
async fn connection_handlers_cover_create_list_get_update_activate_delete_and_not_found() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    storage_support::save_connections(
        TenantScope::default(),
        &[connection("old", "Old", 10), connection("new", "New", 20)],
    )
    .await
    .unwrap();
    storage_support::save_settings(
        TenantScope::default(),
        &AiDataSettings {
            selected_connection_id: Some("old".to_string()),
            ..AiDataSettings::default()
        },
    )
    .await
    .unwrap();

    let Json(list) = data_connections_list(tenant()).await.unwrap();
    assert_eq!(list.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), vec!["new", "old"]);

    let Json(existing) =
        data_connection_get(tenant(), axum::extract::Path("old".to_string())).await.unwrap();
    assert_eq!(existing.name, "Old");
    assert_eq!(
        data_connection_get(tenant(), axum::extract::Path("missing".to_string()))
            .await
            .unwrap_err()
            .status,
        StatusCode::NOT_FOUND
    );

    let Json(created) =
        data_connection_create(tenant(), Json(connection_body(AiDataConnectionKind::Sqlite)))
            .await
            .unwrap();
    assert_eq!(created.name, "Main");
    assert!(uuid::Uuid::parse_str(&created.id).is_ok());
    assert_eq!(
        storage_support::load_settings(TenantScope::default())
            .await
            .selected_connection_id
            .as_deref(),
        Some(created.id.as_str())
    );

    let mut body = connection_body(AiDataConnectionKind::Http);
    body.name = " Updated ".to_string();
    body.base_url = Some("https://example.test".to_string());
    body.description = Some(" ".to_string());
    body.schema_hint = Some(" ".to_string());
    let Json(updated) =
        data_connection_update(tenant(), axum::extract::Path(created.id.clone()), Json(body))
            .await
            .unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "Updated");
    assert!(updated.description.is_none());
    assert!(updated.schema_hint.is_none());
    assert_eq!(updated.kind, AiDataConnectionKind::Http);

    let Json(settings) =
        data_connection_activate(tenant(), axum::extract::Path("new".to_string())).await.unwrap();
    assert_eq!(settings.selected_connection_id.as_deref(), Some("new"));
    let activated = storage_support::load_connections(TenantScope::default())
        .await
        .into_iter()
        .find(|item| item.id == "new")
        .unwrap();
    assert!(activated.last_used_ms.is_some());

    let Json(deleted) =
        data_connection_delete(tenant(), axum::extract::Path("new".to_string())).await.unwrap();
    assert_eq!(deleted["deleted_id"], "new");
    assert_ne!(
        storage_support::load_settings(TenantScope::default())
            .await
            .selected_connection_id
            .as_deref(),
        Some("new")
    );
    assert_eq!(
        data_connection_delete(tenant(), axum::extract::Path("missing".to_string()))
            .await
            .unwrap_err()
            .status,
        StatusCode::NOT_FOUND
    );

    snapshot.restore().await;
}

#[tokio::test]
async fn connection_test_and_catalog_handlers_wrap_runtime_results() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("items.db");
    make_sqlite_fixture(&db_path);
    storage_support::save_settings(
        TenantScope::default(),
        &AiDataSettings { default_timeout_secs: 1, ..AiDataSettings::default() },
    )
    .await
    .unwrap();
    storage_support::save_connections(
        TenantScope::default(),
        &[AiDataConnectionDto {
            sqlite_path: Some(db_path.to_string_lossy().to_string()),
            ..connection("sqlite", "SQLite", 1)
        }],
    )
    .await
    .unwrap();

    let Json(test) =
        data_connection_test(tenant(), axum::extract::Path("sqlite".to_string())).await.unwrap();
    assert!(test.ok);
    assert!(test.message.contains("SQLite OK"));

    let Json(catalog) =
        data_connection_catalog(tenant(), axum::extract::Path("sqlite".to_string())).await.unwrap();
    assert_eq!(catalog.connection_id, "sqlite");
    assert_eq!(catalog.catalog["tables"][0]["name"], "items");

    assert_eq!(
        data_connection_test(tenant(), axum::extract::Path("missing".to_string()))
            .await
            .unwrap_err()
            .status,
        StatusCode::NOT_FOUND
    );

    snapshot.restore().await;
}

#[tokio::test]
async fn connection_runtime_failures_are_mapped_without_hiding_missing_records() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    storage_support::save_connections(
        TenantScope::default(),
        &[AiDataConnectionDto {
            kind: AiDataConnectionKind::Mysql,
            connection_url: Some("mysql://localhost/test".to_string()),
            sqlite_path: None,
            ..connection("mysql", "MySQL", 1)
        }],
    )
    .await
    .unwrap();

    let Json(test) =
        data_connection_test(tenant(), axum::extract::Path("mysql".to_string())).await.unwrap();
    assert!(!test.ok);
    assert!(test.message.contains("MySQL"));

    let err = data_connection_catalog(tenant(), axum::extract::Path("mysql".to_string()))
        .await
        .unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("MySQL"));
    assert_eq!(
        data_connection_catalog(tenant(), axum::extract::Path("missing".to_string()))
            .await
            .unwrap_err()
            .status,
        StatusCode::NOT_FOUND
    );

    snapshot.restore().await;
}

#[tokio::test]
async fn report_handlers_cover_crud_sorting_preparation_and_slug_conflicts() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    storage_support::save_reports(&[report("older", "older", 10), report("newer", "newer", 20)])
        .await
        .unwrap();

    let Json(list) = data_reports_list().await.unwrap();
    assert_eq!(
        list.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        vec!["newer", "older"]
    );
    assert_eq!(list[0].report_config["modules"].as_array().unwrap().len(), 1);

    let Json(existing) = data_report_get(axum::extract::Path("older".to_string())).await.unwrap();
    assert_eq!(existing.slug, "older");
    assert_eq!(
        data_report_get(axum::extract::Path("missing".to_string())).await.unwrap_err().status,
        StatusCode::NOT_FOUND
    );

    let Json(created) = data_report_create(Json(report_body())).await.unwrap();
    assert_eq!(created.name, "Sales");
    assert_eq!(created.slug, "sales");
    assert!(uuid::Uuid::parse_str(&created.id).is_ok());

    let duplicate = data_report_create(Json(report_body())).await.unwrap_err();
    assert_eq!(duplicate.status, StatusCode::BAD_REQUEST);
    assert!(duplicate.message.contains("slug 已存在"));

    let mut body = report_body();
    body.name = " Updated ".to_string();
    body.slug = " updated ".to_string();
    body.default_source_key = Some(" ".to_string());
    let Json(updated) =
        data_report_update(axum::extract::Path(created.id.clone()), Json(body)).await.unwrap();
    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.slug, "updated");
    assert!(updated.default_source_key.is_none());

    let mut conflict = report_body();
    conflict.slug = "older".to_string();
    assert!(
        data_report_update(axum::extract::Path(updated.id.clone()), Json(conflict))
            .await
            .unwrap_err()
            .message
            .contains("slug 已存在")
    );

    let Json(deleted) = data_report_delete(axum::extract::Path(updated.id.clone())).await.unwrap();
    assert_eq!(deleted["deleted_id"], updated.id);
    assert_eq!(
        data_report_delete(axum::extract::Path("missing".to_string())).await.unwrap_err().status,
        StatusCode::NOT_FOUND
    );

    snapshot.restore().await;
}

#[tokio::test]
async fn report_update_allows_own_slug_and_rejects_missing_report() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    storage_support::save_reports(&[report("existing", "stable", 10)]).await.unwrap();

    let mut body = report_body();
    body.slug = " stable ".to_string();
    let Json(updated) =
        data_report_update(axum::extract::Path("existing".to_string()), Json(body)).await.unwrap();
    assert_eq!(updated.id, "existing");
    assert_eq!(updated.slug, "stable");

    let mut missing_body = report_body();
    missing_body.slug = "unused".to_string();
    let error = data_report_update(axum::extract::Path("missing".to_string()), Json(missing_body))
        .await
        .unwrap_err();
    assert_eq!(error.status, StatusCode::NOT_FOUND);

    snapshot.restore().await;
}

#[tokio::test]
async fn data_query_handler_executes_with_persisted_runtime_state() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("items.db");
    make_sqlite_fixture(&db_path);
    storage_support::save_settings(
        TenantScope::default(),
        &AiDataSettings { default_limit: 1, ..AiDataSettings::default() },
    )
    .await
    .unwrap();
    storage_support::save_connections(
        TenantScope::default(),
        &[AiDataConnectionDto {
            sqlite_path: Some(db_path.to_string_lossy().to_string()),
            ..connection("conn-1", "SQLite", 1)
        }],
    )
    .await
    .unwrap();
    let mut persisted_report = report("report-1", "sales", 1);
    persisted_report.sources[0].sql = Some("SELECT id, name FROM items".to_string());
    storage_support::save_reports(&[persisted_report]).await.unwrap();

    let Json(response) = data_query(
        tenant(),
        Json(AiDataQueryRequest {
            report_id: Some("sales".to_string()),
            count: Some(AiDataCountMode::Enabled),
            debug: Some(true),
            ..Default::default()
        }),
    )
    .await
    .unwrap();
    assert_eq!(response.page.total_record, 2);
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.report_config.unwrap()["modules"].as_array().unwrap().len(), 1);
    assert_eq!(response.debug.unwrap()["query_kind"], "sql");

    let err = data_query(
        tenant(),
        Json(AiDataQueryRequest {
            connection_id: Some("missing".to_string()),
            ..Default::default()
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);

    snapshot.restore().await;
}

#[tokio::test]
async fn data_ai_query_handler_loads_persisted_state_before_context_validation() {
    let _guard = AI_DATA_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;

    let err = data_ai_query(
        tenant(),
        axum::extract::State(app_state()),
        Json(AiDataAiQueryRequest { prompt: "missing context".to_string(), ..Default::default() }),
    )
    .await
    .unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("report_id 或 connection_id"));

    snapshot.restore().await;
}
