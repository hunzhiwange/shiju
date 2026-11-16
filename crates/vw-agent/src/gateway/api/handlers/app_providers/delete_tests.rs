use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode},
};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use tower::ServiceExt;
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::delete::AppProviderDelete;
use vw_api_transport::id::TenantScope;
const TOKEN: &str = "fixture-delete-reader-token-over-32-chars";
fn tenant(company: &str) -> TenantScope {
    TenantScope::new("p", company).unwrap()
}
fn setup(root: &Path) -> (Router, GlobalDatabase) {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy = root.join("access.json");
    let value = serde_json::json!({"principals":[{"id":"reader","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true},{"platform_id":"p","company_id":"a","host_id":"metadata-only"}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},{"id":"metadata-only","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")}]});
    std::fs::write(&policy, value.to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::for_home(&root.join("database")).unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    for app in ["kimi", "opencode", "claude"] {
        for id in ["used", "unused"] {
            // These catalog records may precede provider-session profile support.
            settings
                .upsert(
                    &tenant("a"),
                    "app_providers.records",
                    &format!("\"{app}\":{id}"),
                    r#"{"name":"fixture","protocol":"openai_responses","baseUrl":"https://fixture.example.test","model":"fixture-model","apiKey":"fixture-secret"}"#,
                )
                .unwrap();
        }
    }
    let routes = super::delete::router_with_policy(Some(policy));
    let app = Router::new()
        .merge(routes)
        .layer(Extension(db.clone()))
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    (app, db)
}
fn rows(db: &GlobalDatabase) -> Vec<crate::gateway::settings_store::GatewaySetting> {
    GatewaySettingsStore::new(db.clone())
        .unwrap()
        .list(&tenant("a"), "app_providers.records")
        .unwrap()
}
#[tokio::test]
async fn cc04_delete_is_registered_in_formal_handler_router() {
    let root = tempfile::tempdir().unwrap();
    let (_, db) = setup(root.path());
    let app = crate::gateway::router::handler_router()
        .layer(Extension(db.clone()))
        .layer(Extension(tenant("a")));
    let before = rows(&db);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/app-providers/opencode/unused")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(rows(&db), before);
}
