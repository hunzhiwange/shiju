//! HTTP fixtures only; never use ambient home or user configuration.
use super::{backup_crud_api, backup_restore_api};
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::Extension;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectScope, TenantScope};

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
        let backups = directory.join("backups");
        fs::create_dir(&backups).unwrap();
        fs::set_permissions(&backups, fs::Permissions::from_mode(0o700)).unwrap();
        let policy = directory.join("policy.json");
        let token = "anonymous-backup-fixture-token-00001";
        fs::write(&policy, json!({"principals":[{"id":"reader", "token_sha256":hex::encode(Sha256::digest(token)),
            "grants":[{"platform_id":"p","company_id":"c","host_id":"h","data_backups":["project"]}]}],
            "hosts":[{"id":"h","online":true,"binding":"local","home":directory,
            "applications":directory,"search_paths":[],"data_backup_root":backups}]}).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        let tenant = TenantScope::new("p", "c").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
        let db = GlobalDatabase::new(directory.join("fixture.sqlite")).unwrap();
        GatewaySettingsStore::new(db.clone())
            .unwrap()
            .upsert(&tenant, "agent_apps_user", "reader", r#"{"appOrder":[],"visibleApps":[]}"#)
            .unwrap();
        let app = backup_crud_api::router_with_policy(Some(policy.clone()))
            .merge(backup_restore_api::router_with_policy(Some(policy)))
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
            target: json!({"scope":project,"targetHostId":"h","principalId":"reader"}),
        }
    }
    async fn call(&self, operation: &str, request: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/agent-apps/data/backups/{operation}", self.url))
            .header("x-agent-apps-token", "anonymous-backup-fixture-token-00001")
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
    let fixture = Fixture::new().await;
    let database_before = fs::read(fixture._root.path().join("fixture.sqlite")).unwrap();
    let before: Vec<_> = fs::read_dir(fixture._root.path().join("backups"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    for (path, request) in [
        (
            "restore",
            json!({"target":fixture.target,"backupId":"missing","expectedRevision":"missing","categories":["preferences"],"idempotencyKey":"unauthorized","confirmed":true}),
        ),
        ("restore/preview", json!({"target":fixture.target,"backupId":"missing"})),
        ("diagnose", json!({"target":fixture.target})),
        ("restore/get", json!({"target":fixture.target,"operationId":"missing"})),
    ] {
        let response = reqwest::Client::new()
            .post(format!("{}/agent-apps/data/backups/{path}", fixture.url))
            .json(&request)
            .send()
            .await
            .unwrap();
        assert!(matches!(response.status().as_u16(), 401 | 403));
    }
    let after: Vec<_> = fs::read_dir(fixture._root.path().join("backups"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before, after);
    assert_eq!(database_before, fs::read(fixture._root.path().join("fixture.sqlite")).unwrap());
}
