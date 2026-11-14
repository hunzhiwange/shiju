use axum::{Extension, body::Body, http::{Request, StatusCode}};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use tower::ServiceExt;
use vw_api_types::id::TenantScope;
use serde_json::json;

const TOKEN: &str = "claude-control-test-token-000000000000";

struct Fixture { root: tempfile::TempDir, policy: std::path::PathBuf }
impl Fixture {
    fn new(grants: serde_json::Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        let policy = root.path().join("access.json");
        let mut grant = json!({"platform_id":"p","company_id":"c","host_id":"h"});
        grant.as_object_mut().unwrap().extend(grants.as_object().unwrap().clone());
        fs::write(&policy, json!({"principals":[{"id":"control","token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[grant]}],"hosts":[{"id":"h","online":true,"binding":"local","home":root.path(),
            "applications":root.path(),"search_paths":[],"credential_root":private}]}).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        Self { root, policy }
    }
    async fn call(&self, route: &str, body: Option<serde_json::Value>) -> (StatusCode, Vec<u8>) {
        let app = super::claude_api::router_with_policy(Some(self.policy.clone()))
            .layer(Extension(TenantScope::new("p", "c").unwrap()));
        let request = if let Some(body) = body {
            Request::builder().method("POST").uri(format!("/agent-apps/claude/{route}"))
                .header("content-type", "application/json").header("x-agent-apps-token", TOKEN)
                .body(Body::from(body.to_string())).unwrap()
        } else {
            Request::builder().uri(format!("/agent-apps/claude/{route}?projectId=project&targetHostId=h"))
                .header("x-agent-apps-token", TOKEN).body(Body::empty()).unwrap()
        };
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        (status, bytes.to_vec())
    }
}
fn action(action: &str) -> serde_json::Value {
    json!({"scope":{"projectId":"project","targetHostId":"h"},"action":action})
}

#[tokio::test]
async fn cp12_claude_api_success() {
    let f = Fixture::new(json!({"claude_config_read_projects":["project"],
        "claude_config_marker_projects":["project"],"claude_onboarding_projects":["project"]}));
    assert_eq!(f.call("config-marker", Some(action("enable"))).await.0, StatusCode::OK);
    let (status, bytes) = f.call("config-status", None).await;
    assert_eq!(status, StatusCode::OK);
    let result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["status"]["managed"], true);
    assert_eq!(result["status"]["primaryApiKeyPresent"], true);
    assert!(result["status"].get("primaryApiKey").is_none());
    assert_eq!(f.call("onboarding", Some(action("enable"))).await.0, StatusCode::OK);
    let onboarding: serde_json::Value = serde_json::from_slice(&fs::read(f.root.path().join(".claude.json")).unwrap()).unwrap();
    assert_eq!(onboarding["hasCompletedOnboarding"], true);
}

#[tokio::test]
async fn cp12_claude_api_rejects_without_side_effects() {
    let f = Fixture::new(json!({"provider_config_read":true,"provider_config_write":true,
        "claude_config_read_projects":["project"]}));
    for route in ["config-marker", "onboarding"] {
        assert_eq!(f.call(route, Some(action("enable"))).await.0, StatusCode::FORBIDDEN);
    }
    assert!(!f.root.path().join(".claude").exists());
    assert!(!f.root.path().join(".claude.json").exists());
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cp12_claude_api_user_credential_is_never_returned_or_cleared() {
    let f = Fixture::new(json!({"claude_config_read_projects":["project"],"claude_config_marker_projects":["project"]}));
    fs::create_dir(f.root.path().join(".claude")).unwrap();
    let path = f.root.path().join(".claude/config.json");
    let original = br#"{"primaryApiKey":"private-fixture-value","other":true}"#;
    fs::write(&path, original).unwrap();
    let (status, bytes) = f.call("config-status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!String::from_utf8(bytes).unwrap().contains("private-fixture-value"));
    assert_eq!(f.call("config-marker", Some(action("disable"))).await.0, StatusCode::CONFLICT);
    assert_eq!(fs::read(path).unwrap(), original);
    assert_eq!(f.call("onboarding", Some(action("enable"))).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cp12_claude_api_rejects_ungranted_project_host_and_arbitrary_json() {
    let f = Fixture::new(json!({"claude_config_marker_projects":["project"]}));
    for scope in [json!({"projectId":"other","targetHostId":"h"}),
        json!({"projectId":"project","targetHostId":"other"})] {
        let (status, _) = f.call("config-marker", Some(json!({"scope":scope,"action":"enable"}))).await;
        assert!(!status.is_success());
    }
    let mut body = action("enable");
    body["config"] = json!({"primaryApiKey":"arbitrary-value"});
    assert!(!f.call("config-marker", Some(body)).await.0.is_success());
    assert!(!f.root.path().join(".claude").exists());
    assert!(!f.root.path().join(".claude.json").exists());
}
