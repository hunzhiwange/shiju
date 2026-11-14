//! HTTP fixtures only; never use ambient home or user configuration.
use super::backup_crud_api;
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
            .upsert(&tenant, "agent_apps_user", "reader", "{\"fixture\":true}")
            .unwrap();
        let app = backup_crud_api::router_with_policy(Some(policy))
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
async fn cc_data_34_backup_api_maintains_isolated_backups() {
    let fixture = Fixture::new().await;
    let created = fixture.create().await;
    assert_eq!(fixture.create().await["backupId"], created["backupId"]);
    let listed: Vec<Value> =
        fixture.call("list", json!({"target":fixture.target})).await.json().await.unwrap();
    assert_eq!(listed.len(), 1);
    let renamed = fixture
        .call(
            "rename",
            json!({"target":fixture.target,"backupId":created["backupId"],
        "expectedRevision":created["revision"],"name":"renamed 中文"}),
        )
        .await;
    assert_eq!(renamed.status(), 200);
    let renamed: Value = renamed.json().await.unwrap();
    assert_eq!(renamed["backupId"], created["backupId"]);
    assert_ne!(renamed["revision"], created["revision"]);
    assert_eq!(
        fixture
            .call(
                "delete",
                json!({"target":fixture.target,"backupId":created["backupId"],
        "expectedRevision":created["revision"],"confirmed":true})
            )
            .await
            .status(),
        409
    );
    assert_eq!(
        fixture
            .call(
                "delete",
                json!({"target":fixture.target,"backupId":created["backupId"],
        "expectedRevision":renamed["revision"],"confirmed":true})
            )
            .await
            .status(),
        200
    );
    let listed: Vec<Value> =
        fixture.call("list", json!({"target":fixture.target})).await.json().await.unwrap();
    assert!(listed.is_empty());
}
#[tokio::test]
async fn cc_data_34_backup_api_rejects_cross_tenant_id() {
    let fixture = Fixture::new().await;
    let created = fixture.create().await;
    let mut foreign = fixture.target.clone();
    foreign["scope"] = serde_json::to_value(
        ProjectScope::new(TenantScope::new("p", "foreign").unwrap(), "project".into()).unwrap(),
    )
    .unwrap();
    let response = fixture
        .call(
            "delete",
            json!({"target":foreign,"backupId":created["backupId"],
        "expectedRevision":created["revision"],"confirmed":true}),
        )
        .await;
    assert_eq!(response.status(), 403);
    assert!(!response.text().await.unwrap().contains("fixture.sqlite"));
    let listed: Vec<Value> =
        fixture.call("list", json!({"target":fixture.target})).await.json().await.unwrap();
    assert_eq!(listed.len(), 1);
}
