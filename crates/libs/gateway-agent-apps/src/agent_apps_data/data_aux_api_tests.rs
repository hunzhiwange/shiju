//! HTTP fixtures only; never use ambient home or user configuration.
use super::data_aux_api;
use {app_settings_store::settings_store::GatewaySettingsStore, agent_database::GlobalDatabase};
use axum::Extension;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use {
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};

// Runtime paths are cached globally, so each case resolves them in a fresh process.
fn isolated_test() -> bool {
    const CHILD: &str = "SHIJU_DATA_AUX_CHILD";
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
    assert!(status.success(), "isolated data auxiliary child failed: {status}");
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
            .upsert(&tenant, "agent_apps_user", "desktop-local-user", "{\"appOrder\":[\"codex\"],\"visibleApps\":[\"codex\"]}")
            .unwrap();
        let app = data_aux_api::router_with_policy(None)
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
            .post(format!("{}/agent-apps/data/{operation}", self.url))
            .json(&request)
            .send()
            .await
            .unwrap()
    }
}
#[tokio::test]
async fn cc_data_36_export_api_returns_downloadable_package() {
    if isolated_test() {
        return;
    }
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
    if isolated_test() {
        return;
    }
    let f=Fixture::new().await;
    let r=f.call("locations/delete",json!({"target":f.target,"categoryId":"../../private","confirmed":true})).await;
    // Registered roots do not allow arbitrary category paths.
    assert_eq!(r.status(),400);
}
