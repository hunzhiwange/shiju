use serde_json::json;
use tempfile::TempDir;
use vw_api_types::data::{
    AiDataConnectionDto, AiDataConnectionKind, AiDataReportDto, AiDataSettings, AiDataSourceMode,
};
use vw_api_types::id::TenantScope;

use super::*;
use crate::app::agent::storage;
use crate::global_database::GlobalDatabase;
use crate::security::SecretStore;

struct TestAiDataConfigStore {
    _home: TempDir,
    database: GlobalDatabase,
    secrets_dir: std::path::PathBuf,
    store: AiDataConfigStore,
}

impl TestAiDataConfigStore {
    fn new() -> Self {
        let home = tempfile::tempdir().expect("temp home");
        let database =
            GlobalDatabase::new(home.path().join("database.sqlite3")).expect("global database");
        let secrets_dir = home.path().join("secrets");
        let store = AiDataConfigStore::new(
            database.clone(),
            SecretStore::new(&secrets_dir, true),
            TenantScope::default(),
        )
        .expect("AI-DATA config store");
        Self { _home: home, database, secrets_dir, store }
    }

    fn restarted(&self) -> AiDataConfigStore {
        AiDataConfigStore::new(
            self.database.clone(),
            SecretStore::new(&self.secrets_dir, true),
            TenantScope::default(),
        )
        .expect("restarted AI-DATA config store")
    }
}

struct AiDataStorageSnapshot {
    reports: Option<Vec<AiDataReportDto>>,
}

impl AiDataStorageSnapshot {
    async fn capture() -> Self {
        Self { reports: storage::read(&["ai_data", "reports"]).await.ok() }
    }

    async fn restore(self) {
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
    storage::remove(&["ai_data", "reports"]).await.expect("clear ai-data reports");
}

fn connection(
    id: &str,
    name: &str,
    updated_at_ms: u64,
    last_used_ms: Option<u64>,
) -> AiDataConnectionDto {
    serde_json::from_value(json!({
        "id": id,
        "name": name,
        "kind": "sqlite",
        "sqlite_path": "/tmp/data.sqlite",
        "updated_at_ms": updated_at_ms,
        "last_used_ms": last_used_ms
    }))
    .expect("connection")
}

fn report(id: &str, name: &str, updated_at_ms: u64) -> AiDataReportDto {
    AiDataReportDto {
        id: id.to_string(),
        name: name.to_string(),
        slug: id.to_string(),
        data_source: AiDataSourceMode::Normal,
        default_source_key: None,
        report_config: json!({ "columns": [] }),
        sources: Vec::new(),
        updated_at_ms,
    }
}

fn store_raw_connection(
    fixture: &TestAiDataConfigStore,
    config_key: &str,
    connection: &AiDataConnectionDto,
) {
    let payload = serde_json::to_string(connection).expect("raw connection payload");
    fixture
        .store
        .settings_store
        .upsert(&fixture.store.tenant, AI_DATA_CONNECTIONS_NAMESPACE, config_key, &payload)
        .expect("store raw connection");
}

#[test]
fn internal_storage_error_preserves_message_and_uses_internal_status() {
    let error = internal_storage_error("storage unavailable");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(error.message, "storage unavailable");
}

#[test]
fn settings_optional_distinguishes_missing_saved_and_deleted_values() {
    let fixture = TestAiDataConfigStore::new();
    let settings = AiDataSettings {
        default_limit: 73,
        selected_connection_id: Some("selected".to_string()),
        ..AiDataSettings::default()
    };

    assert_eq!(fixture.store.load_settings_optional().expect("missing settings"), None);

    fixture.store.save_settings(&settings).expect("save settings");
    assert_eq!(fixture.store.load_settings_optional().expect("saved settings"), Some(settings));

    fixture.store.delete_settings().expect("delete settings");
    assert_eq!(fixture.store.load_settings_optional().expect("deleted settings"), None);
}

#[test]
fn settings_load_reports_malformed_database_payload() {
    let fixture = TestAiDataConfigStore::new();
    fixture
        .store
        .settings_store
        .upsert(&fixture.store.tenant, AI_DATA_SETTINGS_NAMESPACE, DEFAULT_CONFIG_KEY, "{not-json")
        .expect("store malformed settings");

    let error = fixture.store.load_settings_optional().expect_err("malformed settings should fail");

    assert!(
        format!("{error:#}").contains("decode AI-DATA settings from shared database"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn settings_are_isolated_by_tenant() {
    let fixture = TestAiDataConfigStore::new();
    let other_tenant =
        TenantScope::new("settings-platform-b", "settings-company-b").expect("other tenant");
    let other = AiDataConfigStore::new(
        fixture.database.clone(),
        SecretStore::new(&fixture.secrets_dir, true),
        other_tenant,
    )
    .expect("other tenant AI-DATA store");
    let first = AiDataSettings {
        default_limit: 11,
        selected_connection_id: Some("first".to_string()),
        ..AiDataSettings::default()
    };
    let second = AiDataSettings {
        default_limit: 22,
        selected_connection_id: Some("second".to_string()),
        ..AiDataSettings::default()
    };

    fixture.store.save_settings(&first).expect("save first tenant");
    other.save_settings(&second).expect("save second tenant");

    assert_eq!(fixture.store.load_settings().expect("load first tenant"), first);
    assert_eq!(other.load_settings().expect("load second tenant"), second);
}

#[test]
fn save_connections_rejects_duplicate_ids_without_mutating_existing_rows() {
    let fixture = TestAiDataConfigStore::new();
    let existing = vec![connection("existing", "Existing", 1, None)];
    fixture.store.save_connections(&existing).expect("save existing connection");
    let duplicates =
        vec![connection("duplicate", "First", 2, None), connection("duplicate", "Second", 3, None)];

    let error = fixture
        .store
        .save_connections(&duplicates)
        .expect_err("duplicate connection ids should fail");

    assert_eq!(error.to_string(), "duplicate AI-DATA connection id");
    assert_eq!(fixture.store.load_connections().expect("existing connections remain"), existing);
}

#[test]
fn save_connections_reports_encryption_field_context_without_mutating_existing_rows() {
    for (field, expected_context) in [
        ("connection_url", "encrypt AI-DATA connection URL"),
        ("auth_token", "encrypt AI-DATA auth token"),
        ("header", "encrypt AI-DATA header value"),
    ] {
        let fixture = TestAiDataConfigStore::new();
        let existing = vec![connection("existing", "Existing", 1, None)];
        fixture.store.save_connections(&existing).expect("save existing connection");
        let blocked_secret_dir = fixture._home.path().join("blocked-secret-dir");
        std::fs::write(&blocked_secret_dir, "not a directory").expect("create path blocker");
        let failing_store = AiDataConfigStore::new(
            fixture.database.clone(),
            SecretStore::new(&blocked_secret_dir, true),
            TenantScope::default(),
        )
        .expect("AI-DATA store with blocked secret path");
        let mut replacement = connection("replacement", "Replacement", 2, None);
        match field {
            "connection_url" => {
                replacement.connection_url = Some("postgres://database.test/app".to_string());
            }
            "auth_token" => replacement.auth_token = Some("auth-token".to_string()),
            "header" => {
                replacement.headers.insert("X-Api-Key".to_string(), "header-value".to_string());
            }
            _ => unreachable!("covered field cases"),
        }

        let error = failing_store
            .save_connections(&[replacement])
            .expect_err("blocked secret path should reject connection");

        assert!(
            format!("{error:#}").contains(expected_context),
            "unexpected error for {field}: {error:#}"
        );
        assert_eq!(
            fixture.store.load_connections().expect("existing connections remain"),
            existing
        );
    }
}

#[test]
fn save_connections_replaces_existing_set_and_removes_stale_rows() {
    let fixture = TestAiDataConfigStore::new();
    fixture
        .store
        .save_connections(&[
            connection("keep", "Original", 1, None),
            connection("stale", "Stale", 2, None),
        ])
        .expect("save initial connections");
    let replacements =
        vec![connection("keep", "Updated", 3, Some(4)), connection("new", "New", 5, None)];

    fixture.store.save_connections(&replacements).expect("replace connections");

    assert_eq!(fixture.store.load_connections().expect("load replacements"), replacements);
}

#[test]
fn save_connections_with_empty_slice_removes_all_rows() {
    let fixture = TestAiDataConfigStore::new();
    fixture
        .store
        .save_connections(&[
            connection("first", "First", 1, None),
            connection("second", "Second", 2, None),
        ])
        .expect("save initial connections");

    fixture.store.save_connections(&[]).expect("clear connections");

    assert!(fixture.store.load_connections().expect("load cleared connections").is_empty());
}

#[test]
fn load_connections_rejects_malformed_database_payload() {
    let fixture = TestAiDataConfigStore::new();
    fixture
        .store
        .settings_store
        .upsert(&fixture.store.tenant, AI_DATA_CONNECTIONS_NAMESPACE, "malformed", "{not-json")
        .expect("store malformed connection");

    let error = fixture.store.load_connections().expect_err("malformed connection should fail");

    assert!(
        format!("{error:#}").contains("decode AI-DATA connection from shared database"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn load_connections_rejects_id_that_differs_from_config_key() {
    let fixture = TestAiDataConfigStore::new();
    let connection = connection("payload-id", "Mismatched", 1, None);
    store_raw_connection(&fixture, "config-key", &connection);

    let error = fixture.store.load_connections().expect_err("mismatched id should fail");

    assert_eq!(error.to_string(), "AI-DATA connection id does not match its config key");
}

#[test]
fn load_connections_accepts_plaintext_legacy_secrets() {
    let fixture = TestAiDataConfigStore::new();
    let mut connection = connection("plaintext", "Plaintext", 1, None);
    connection.connection_url = Some("postgres://legacy-plaintext".to_string());
    connection.auth_token = Some("legacy-token".to_string());
    connection.headers.insert("X-Legacy".to_string(), "legacy-header-value".to_string());
    store_raw_connection(&fixture, "plaintext", &connection);

    assert_eq!(
        fixture.store.load_connections().expect("load plaintext legacy secrets"),
        vec![connection]
    );
}

#[test]
fn load_connections_reports_field_context_for_invalid_encrypted_secrets() {
    let mut invalid_connection_url = connection("connection-url", "Connection URL", 1, None);
    invalid_connection_url.connection_url = Some("enc2:not-hex".to_string());
    let mut invalid_auth_token = connection("auth-token", "Auth Token", 1, None);
    invalid_auth_token.auth_token = Some("enc2:not-hex".to_string());
    let mut invalid_header = connection("header", "Header", 1, None);
    invalid_header.headers.insert("X-Broken".to_string(), "enc2:not-hex".to_string());

    for (config_key, connection, expected_context) in [
        ("connection-url", invalid_connection_url, "decrypt AI-DATA connection URL"),
        ("auth-token", invalid_auth_token, "decrypt AI-DATA auth token"),
        ("header", invalid_header, "decrypt AI-DATA header value"),
    ] {
        let fixture = TestAiDataConfigStore::new();
        store_raw_connection(&fixture, config_key, &connection);

        let error = fixture.store.load_connections().expect_err("invalid secret should fail");
        let message = format!("{error:#}");
        assert!(message.contains(expected_context), "unexpected error: {message}");
    }
}

#[test]
fn ai_data_connections_encrypt_credentials_and_headers_in_gateway_settings() {
    let fixture = TestAiDataConfigStore::new();
    let connection = AiDataConnectionDto {
        id: "sensitive".to_string(),
        name: "Sensitive".to_string(),
        kind: AiDataConnectionKind::Postgres,
        description: None,
        enabled: true,
        read_only: true,
        base_url: None,
        connection_url: Some("postgres://plain-user:plain-password@localhost/db".to_string()),
        sqlite_path: None,
        default_path: None,
        auth_token: Some("plain-auth-token".to_string()),
        headers: [
            ("X-Api-Key".to_string(), "plain-header-value".to_string()),
            ("X-Second".to_string(), "plain-second-value".to_string()),
        ]
        .into_iter()
        .collect(),
        schema_hint: None,
        updated_at_ms: 10,
        last_used_ms: None,
    };

    fixture
        .store
        .save_connections(std::slice::from_ref(&connection))
        .expect("connection should save");

    let database = fixture.database.connect().expect("database connection");
    let tenant = TenantScope::default();
    let payload: String = database
        .query_row(
            "SELECT payload_json FROM gateway_settings \
             WHERE platform_id = ?1 AND company_id = ?2 \
               AND namespace = 'ai_data.connections' AND config_key = 'sensitive'",
            [tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
            |row| row.get(0),
        )
        .expect("encrypted gateway settings payload");
    for plaintext in [
        "postgres://plain-user:plain-password@localhost/db",
        "plain-auth-token",
        "plain-header-value",
        "plain-second-value",
    ] {
        assert!(!payload.contains(plaintext));
    }
    assert_eq!(payload.matches("enc2:").count(), 4);
    assert!(
        fixture.restarted().load_connections().expect("decrypted connections") == vec![connection]
    );
}

#[test]
fn ai_data_legacy_json_is_ignored_when_database_is_empty_or_conflicts() {
    let fixture = TestAiDataConfigStore::new();
    let legacy_dir = vw_config_types::paths::home_config_dir(fixture._home.path())
        .join("storage")
        .join("ai_data");
    std::fs::create_dir_all(&legacy_dir).expect("legacy AI-DATA dir");
    std::fs::write(
        legacy_dir.join("settings.json"),
        r#"{"default_limit":999,"default_timeout_secs":999,"selected_connection_id":"legacy"}"#,
    )
    .expect("legacy settings input");
    std::fs::write(
        legacy_dir.join("connections.json"),
        serde_json::to_vec(&vec![connection("legacy", "Legacy", 99, None)])
            .expect("legacy connections JSON"),
    )
    .expect("legacy connections input");

    assert_eq!(fixture.store.load_settings().expect("empty settings"), AiDataSettings::default());
    assert!(fixture.store.load_connections().expect("empty connections").is_empty());

    let database_settings = AiDataSettings {
        default_limit: 42,
        selected_connection_id: Some("database".to_string()),
        ..AiDataSettings::default()
    };
    let database_connections = vec![connection("database", "Database", 10, None)];
    fixture.store.save_settings(&database_settings).expect("database settings");
    fixture.store.save_connections(&database_connections).expect("database connections");

    assert_eq!(fixture.store.load_settings().expect("database settings load"), database_settings);
    assert_eq!(
        fixture.store.load_connections().expect("database connections load"),
        database_connections
    );
    assert!(legacy_dir.join("settings.json").exists());
    assert!(legacy_dir.join("connections.json").exists());
}

#[test]
fn ai_data_connections_are_isolated_by_tenant_with_the_same_id() {
    let fixture = TestAiDataConfigStore::new();
    let other_tenant =
        TenantScope::new("ai-data-platform-b", "ai-data-company-b").expect("other tenant");
    let other = AiDataConfigStore::new(
        fixture.database.clone(),
        SecretStore::new(&fixture.secrets_dir, true),
        other_tenant,
    )
    .expect("other tenant AI-DATA store");
    let first = vec![connection("shared", "Tenant A", 10, None)];
    let second = vec![connection("shared", "Tenant B", 20, None)];

    fixture.store.save_connections(&first).expect("save first tenant");
    other.save_connections(&second).expect("save second tenant");

    assert_eq!(fixture.store.load_connections().expect("load first tenant"), first);
    assert_eq!(other.load_connections().expect("load second tenant"), second);
}

#[tokio::test]
async fn load_helpers_return_defaults_when_storage_is_empty() {
    let _guard = AI_DATA_STORAGE_TEST_LOCK.lock().await;
    let fixture = TestAiDataConfigStore::new();
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;

    assert_eq!(fixture.store.load_settings().expect("default settings"), AiDataSettings::default());
    assert!(fixture.store.load_connections().expect("empty connections").is_empty());
    assert!(load_reports().await.is_empty());

    snapshot.restore().await;
}

#[tokio::test]
async fn save_helpers_persist_round_trippable_values() {
    let _guard = AI_DATA_STORAGE_TEST_LOCK.lock().await;
    let fixture = TestAiDataConfigStore::new();
    let snapshot = AiDataStorageSnapshot::capture().await;
    clear_ai_data_storage().await;

    let settings = AiDataSettings {
        default_limit: 250,
        default_timeout_secs: 12,
        selected_connection_id: Some("conn-a".to_string()),
        ..AiDataSettings::default()
    };
    let connections = vec![
        connection("conn-a", "alpha", 10, Some(30)),
        AiDataConnectionDto {
            id: "conn-b".to_string(),
            name: "beta".to_string(),
            kind: AiDataConnectionKind::Http,
            description: Some("HTTP source".to_string()),
            enabled: true,
            read_only: false,
            base_url: Some("https://example.test".to_string()),
            connection_url: None,
            sqlite_path: None,
            default_path: Some("/api".to_string()),
            auth_token: None,
            headers: Default::default(),
            schema_hint: None,
            updated_at_ms: 20,
            last_used_ms: None,
        },
    ];
    let reports = vec![report("report-a", "Revenue", 100), report("report-b", "Costs", 50)];

    fixture.store.save_settings(&settings).expect("settings should save");
    fixture.store.save_connections(&connections).expect("connections should save");
    save_reports(&reports).await.expect("reports should save");

    assert_eq!(fixture.restarted().load_settings().expect("settings should load"), settings);
    assert_eq!(
        fixture.restarted().load_connections().expect("connections should load"),
        connections
    );
    assert_eq!(load_reports().await, reports);
    let stored_reports: Vec<AiDataReportDto> =
        storage::read(&["ai_data", "reports"]).await.expect("reports file storage");
    assert_eq!(stored_reports, reports);

    snapshot.restore().await;
}

#[test]
fn sort_connections_prefers_last_used_then_updated_at_then_name() {
    let mut connections = vec![
        connection("old", "zeta", 10, None),
        connection("tie-b", "bravo", 20, Some(80)),
        connection("tie-a", "alpha", 20, Some(80)),
        connection("updated", "omega", 70, None),
    ];

    sort_connections(&mut connections);

    let ids = connections.into_iter().map(|item| item.id).collect::<Vec<_>>();
    assert_eq!(ids, vec!["tie-a", "tie-b", "updated", "old"]);
}

#[test]
fn sort_reports_prefers_recent_update_then_name() {
    let mut reports =
        vec![report("old", "zeta", 10), report("tie-b", "bravo", 20), report("tie-a", "alpha", 20)];

    sort_reports(&mut reports);

    let ids = reports.into_iter().map(|item| item.id).collect::<Vec<_>>();
    assert_eq!(ids, vec!["tie-a", "tie-b", "old"]);
}

#[test]
fn now_ms_returns_epoch_milliseconds() {
    assert!(now_ms() > 1_000_000_000_000);
}
