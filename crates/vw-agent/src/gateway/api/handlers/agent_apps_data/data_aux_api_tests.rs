//! HTTP fixtures only; never use ambient home or user configuration.
use super::data_aux_api;
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
            "grants":[{"platform_id":"p","company_id":"c","host_id":"h","data_backups":["project"],"data_export":["project"]}]}],
            "hosts":[{"id":"h","online":true,"binding":"local","home":directory,
            "applications":directory,"search_paths":[],"data_backup_root":backups}]}).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        let tenant = TenantScope::new("p", "c").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
        let db = GlobalDatabase::new(directory.join("fixture.sqlite")).unwrap();
        GatewaySettingsStore::new(db.clone())
            .unwrap()
            .upsert(&tenant, "agent_apps_user", "reader", "{\"appOrder\":[\"codex\"],\"visibleApps\":[\"codex\"]}")
            .unwrap();
        let app = data_aux_api::router_with_policy(Some(policy))
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
            .post(format!("{}/agent-apps/data/{operation}", self.url))
            .header("x-agent-apps-token", "anonymous-backup-fixture-token-00001")
            .json(&request)
            .send()
            .await
            .unwrap()
    }
}
#[tokio::test]
async fn cc_data_36_export_api_returns_downloadable_package() {
    let f=Fixture::new().await;
    let r=f.call("export",json!({"target":f.target,"selections":[{"category":"preferences","entries":{"kind":"all"}}],"format":"native_package","idempotencyKey":"fixture"})).await;
    assert_eq!(r.status(),200);
    let handle:Value=r.json().await.unwrap();
    let r=f.call("export/download",json!({"target":f.target,"transferId":handle["transferId"]})).await;
    assert_eq!(r.status(),200);
    let bytes=r.bytes().await.unwrap();
    let digest: [u8;32]=Sha256::digest(&bytes).into();
    let records=vw_agent_apps::data_management::export::verify_portable_package(&bytes,&digest).unwrap();
    assert_eq!(records[0].items[0]["appOrder"],json!(["codex"]));
}
#[tokio::test]
async fn cc_data_36_aux_api_rejects_arbitrary_category_path() {
    let f=Fixture::new().await;
    let r=f.call("locations/delete",json!({"target":f.target,"categoryId":"../../private","confirmed":true})).await;
    // No administrator-owned roots configured: fail closed before storage access.
    assert_eq!(r.status(),501);
}
