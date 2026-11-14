//! Anonymous HTTP fixtures; no ambient home, source data or credentials.
use super::migration_read_api;
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::Extension;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

struct Fixture {
    _root: tempfile::TempDir,
    server: tokio::task::JoinHandle<()>,
    url: String,
    target: Value,
    db: GlobalDatabase,
    tenant: TenantScope,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let backups = directory.join("backups");
        let sources = directory.join("sources");
        for path in [&backups, &sources] {
            fs::create_dir(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let source = sources.join("fixture.json");
        fs::write(&source, json!({"version":2,"claude":{"current":"fixture",
            "providers":{"fixture":{"name":"Anonymous","settingsConfig":{"env":{"ANTHROPIC_MODEL":"fixture-model"}}}}}}).to_string()).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        let policy = directory.join("policy.json");
        fs::write(
            &policy,
            json!({"principals":[{"id":"reader",
            "token_sha256":hex::encode(Sha256::digest("anonymous-migration-fixture-token-00001")),
            "grants":[{"platform_id":"p","company_id":"c","host_id":"h",
                "data_migration_read":["project","other"],"data_backups":["project","other"],
                "provider_config_read":true,"provider_config_write":true}]}],
            "hosts":[{"id":"h","online":true,"binding":"local","home":directory,
                "applications":directory,"search_paths":[],"data_backup_root":backups,
                "data_migration_root":sources,"data_migration_sources":{"fixture":source}}]})
            .to_string(),
        )
        .unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        let tenant = TenantScope::new("p", "c").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
        let db = GlobalDatabase::new(directory.join("fixture.sqlite")).unwrap();
        AppProviderStore::new(db.clone()).unwrap();
        let app = migration_read_api::router_with_policy(Some(policy))
            .layer(Extension(tenant.clone()))
            .layer(Extension(db.clone()));
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
            db,
            tenant,
        }
    }
    async fn call(&self, path: &str, body: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/agent-apps/data/{path}", self.url))
            .header("x-agent-apps-token", "anonymous-migration-fixture-token-00001")
            .json(&body)
            .send()
            .await
            .unwrap()
    }
    async fn preview(&self) -> Value {
        let source = self
            .call("sources/identify", json!({"target":self.target,"transferId":"fixture"}))
            .await;
        assert_eq!(source.status(), 200);
        let source: Value = source.json().await.unwrap();
        let preview = self.call("import/preview", json!({"target":self.target,
            "transferId":source["transferId"],"selections":[{"category":"providers","entries":{"kind":"all"}}]})).await;
        assert_eq!(preview.status(), 200);
        preview.json().await.unwrap()
    }
    fn request(&self, preview: &Value) -> Value {
        json!({"target":self.target,"previewId":preview["previewId"],"expectedRevision":preview["revision"],
            "conflicts":[],"idempotencyKey":"anonymous-request","confirmed":true})
    }
    async fn terminal(&self, operation: &Value) -> Value {
        for _ in 0..200 {
            let response = self
                .call(
                    "operations/get",
                    json!({"target":self.target,"operationId":operation["operationId"]}),
                )
                .await;
            assert_eq!(response.status(), 200);
            let current: Value = response.json().await.unwrap();
            if current["phase"] == "terminal" {
                return current;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("fixture operation did not terminate");
    }
}
#[tokio::test]
async fn cc_data_33_apply_api_exposes_operation_terminal_state() {
    let fixture = Fixture::new().await;
    let preview = fixture.preview().await;
    let mut request = fixture.request(&preview);
    request["expectedRevision"] = json!("stale");
    assert_eq!(fixture.call("import/execute", request.clone()).await.status(), 409);
    request["expectedRevision"] = preview["revision"].clone();
    let response = fixture.call("import/execute", request.clone()).await;
    assert_eq!(response.status(), 200);
    let operation: Value = response.json().await.unwrap();
    let terminal = fixture.terminal(&operation).await;
    assert_eq!(terminal["status"], "completed");
    assert_eq!(terminal["committed"], true);
    assert_eq!(terminal["result"]["kind"], "partial");
    assert_eq!(
        AppProviderStore::new(fixture.db.clone())
            .unwrap()
            .list(&fixture.tenant, AppId::Claude)
            .unwrap()
            .len(),
        1
    );
    let replay: Value = fixture.call("import/execute", request.clone()).await.json().await.unwrap();
    assert_eq!(replay, terminal);
    request["expectedRevision"] = json!("changed-input");
    assert_eq!(fixture.call("import/execute", request).await.status(), 409);
    let retry = fixture
        .call(
            "operations/retry-projections",
            json!({"target":fixture.target,"operationId":terminal["operationId"]}),
        )
        .await;
    assert_eq!(retry.status(), 200);
    let retry: Value = retry.json().await.unwrap();
    assert_eq!(retry["committed"], true);
    assert_eq!(retry["result"]["kind"], "partial");
    assert_eq!(
        AppProviderStore::new(fixture.db.clone())
            .unwrap()
            .list(&fixture.tenant, AppId::Claude)
            .unwrap()
            .len(),
        1
    );
}
#[tokio::test]
async fn cc_data_33_cancel_api_rejects_foreign_operation() {
    let fixture = Fixture::new().await;
    let preview = fixture.preview().await;
    let operation: Value =
        fixture.call("import/execute", fixture.request(&preview)).await.json().await.unwrap();
    let terminal = fixture.terminal(&operation).await;
    let mut foreign = fixture.target.clone();
    foreign["scope"] =
        serde_json::to_value(ProjectScope::new(fixture.tenant.clone(), "other".into()).unwrap())
            .unwrap();
    for path in ["operations/get", "operations/cancel", "operations/retry-projections"] {
        let response = fixture
            .call(path, json!({"target":foreign,"operationId":operation["operationId"]}))
            .await;
        assert_eq!(response.status(), 404);
        assert!(!response.text().await.unwrap().contains("fixture.sqlite"));
    }
    let cancelled: Value = fixture
        .call(
            "operations/cancel",
            json!({"target":fixture.target,"operationId":operation["operationId"]}),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(cancelled, terminal);
}
