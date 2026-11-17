//! HTTP fixtures only; never use ambient home or user configuration.
use super::{backup_crud_api, backup_restore_api};
use axum::Extension;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};
use {agent_database::GlobalDatabase, app_settings_store::settings_store::GatewaySettingsStore};
use {vw_api_transport::id::ProjectScope, vw_api_transport::id::TenantScope};

// Resolve the process-global runtime paths before any route opens storage.
fn isolated_test() -> bool {
    const CHILD: &str = "SHIJU_BACKUP_RESTORE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", root.path().canonicalize().unwrap())
        .status()
        .unwrap();
    assert!(status.success(), "isolated backup restore child failed: {status}");
    true
}

struct Fixture {
    _root: tempfile::TempDir,
    server: tokio::task::JoinHandle<()>,
    url: String,
    target: Value,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let directory = root.path().canonicalize().unwrap();
        let tenant = TenantScope::new("p", "c").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
        let db = GlobalDatabase::new(directory.join("fixture.sqlite")).unwrap();
        GatewaySettingsStore::new(db.clone())
            .unwrap()
            .upsert(
                &tenant,
                "agent_apps_user",
                "desktop-local-user",
                r#"{"appOrder":[],"visibleApps":[]}"#,
            )
            .unwrap();
        let app = backup_crud_api::router_with_policy(None)
            .merge(backup_restore_api::router_with_policy(None))
            .layer(Extension(tenant))
            .layer(Extension(db));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            _root: root,
            server,
            url,
            target: json!({"scope":project,"targetHostId":local_app_auth::HOST_ID,"principalId":"desktop-local-user"}),
        }
    }
    async fn call(&self, operation: &str, request: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/agent-apps/data/backups/{operation}", self.url))
            .json(&request)
            .send()
            .await
            .unwrap()
    }
    async fn create(&self) -> Value {
        let response = self
            .call(
                "create",
                json!({"target":self.target,"categories":["preferences"],
            "kind":"local_rollback","name":"fixture","idempotencyKey":"fixture-request"}),
            )
            .await;
        assert_eq!(response.status(), 200);
        response.json().await.unwrap()
    }
}

#[tokio::test]
async fn cc_data_35_restore_api_reports_safety_snapshot() {
    if isolated_test() {
        return;
    }
    let fixture = Fixture::new().await;
    let created = fixture.create().await;
    let preview = fixture
        .call("restore/preview", json!({"target":fixture.target,"backupId":created["backupId"]}))
        .await;
    assert_eq!(preview.status(), 200);
    let connection =
        rusqlite::Connection::open(fixture._root.path().join("fixture.sqlite")).unwrap();
    connection
        .execute(
            "UPDATE gateway_settings SET payload_json=?1 WHERE namespace='agent_apps_user'",
            [r#"{"appOrder":["fixture-change"],"visibleApps":[]}"#],
        )
        .unwrap();
    let request = json!({"target":fixture.target,"backupId":created["backupId"],"expectedRevision":created["revision"],"categories":["preferences"],"idempotencyKey":"restore-fixture","confirmed":true});
    let response = fixture.call("restore", request.clone()).await;
    assert_eq!(response.status(), 200);
    let queued: Value = response.json().await.unwrap();
    let mut terminal = None;
    for _ in 0..100 {
        let response = fixture
            .call(
                "restore/get",
                json!({"target":fixture.target,"operationId":queued["operationId"]}),
            )
            .await;
        assert_eq!(response.status(), 200);
        let progress: Value = response.json().await.unwrap();
        if progress["status"] == "completed" || progress["status"] == "failed" {
            terminal = Some(progress);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let terminal = terminal.expect("bounded restore completion");
    assert_eq!(terminal["status"], "completed");
    let restored: String = connection
        .query_row(
            "SELECT payload_json FROM gateway_settings WHERE namespace='agent_apps_user'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&restored).unwrap(),
        json!({"appOrder":[],"visibleApps":[]})
    );

    assert_eq!(terminal["result"]["committed"], true);
    assert!(terminal["rollbackBackupId"].as_str().is_some_and(|id| !id.is_empty()));
    let replay: Value = fixture.call("restore", request).await.json().await.unwrap();
    assert_eq!(replay["operationId"], queued["operationId"]);
    let diagnosis = fixture.call("diagnose", json!({"target":fixture.target})).await;
    assert_eq!(diagnosis.status(), 200);
    let diagnosis: Value = diagnosis.json().await.unwrap();
    assert_eq!(diagnosis["targetHealth"], "healthy");
}
#[tokio::test]
async fn cc_data_35_restore_api_rejects_unauthorized_without_io() {
    if isolated_test() {
        return;
    }
    let fixture = Fixture::new().await;
    // Authentication is owned by Gateway middleware. This route must reject
    // a tenant outside its authenticated Extension before opening owner storage.
    let mut foreign = fixture.target.clone();
    foreign["scope"] = serde_json::to_value(
        ProjectScope::new(TenantScope::new("p", "foreign").unwrap(), "project".into()).unwrap(),
    )
    .unwrap();
    let runtime_root = agent_paths::paths().data.join("apps");
    assert!(!runtime_root.exists());
    let database_before = fs::read(fixture._root.path().join("fixture.sqlite")).unwrap();
    let before: Vec<_> =
        fs::read_dir(fixture._root.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
    for (path, request) in [
        (
            "restore",
            json!({"target":foreign,"backupId":"missing","expectedRevision":"missing","categories":["preferences"],"idempotencyKey":"unauthorized","confirmed":true}),
        ),
        ("restore/preview", json!({"target":foreign,"backupId":"missing"})),
        ("diagnose", json!({"target":foreign})),
        ("restore/get", json!({"target":foreign,"operationId":"missing"})),
    ] {
        let response = reqwest::Client::new()
            .post(format!("{}/agent-apps/data/backups/{path}", fixture.url))
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
    let after: Vec<_> =
        fs::read_dir(fixture._root.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(before, after);
    assert!(!runtime_root.exists());
    assert_eq!(database_before, fs::read(fixture._root.path().join("fixture.sqlite")).unwrap());
}
