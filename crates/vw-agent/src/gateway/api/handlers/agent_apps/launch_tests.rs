use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request as HttpRequest,
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use tower::ServiceExt;
const TOKEN: &str = "fixture-launch-api-authorized-token";
fn terminal(preferred: Option<&str>, _: &Path, material: &LaunchMaterial) -> Result<(), String> {
    assert_eq!(preferred, Some("terminal"));
    if material.args == ["fail-terminal"] {
        return Err("fixed failure".into());
    }
    std::fs::write(material.cwd.join("accepted"), serde_json::to_vec(&material.args).unwrap())
        .unwrap();
    Ok(())
}
async fn call(app: &Router, token: &str, value: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/agent-apps/launch?targetHostId=h")
                .header("content-type", "application/json")
                .header("x-agent-apps-token", token)
                .header("x-platform-id", "p")
                .header("x-company-id", "c")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
#[tokio::test]
async fn cc_apps_launch_api_target_scope_accepts_and_rejects_before_terminal() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let program = bin.join("codex");
    std::fs::write(&program, "#!/bin/sh\necho 'codex 1.0.0'\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy = root.path().join("policy.json");
    let mut value = json!({"principals":[{"id":"a","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"c","host_id":"h","launch":true}]}],"hosts":[{"id":"h","online":true,"binding":"local","home":root.path(),"search_paths":[bin],"applications":root.path()}]});
    std::fs::write(&policy, value.to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = vw_api_types::id::TenantScope::new("p", "c").unwrap();
    GatewaySettingsStore::new(db.clone())
        .unwrap()
        .upsert(&tenant, "agent_apps_host", "h", r#"{"preferredTerminal":"terminal"}"#)
        .unwrap();
    let app = router_with_terminal(Some(policy.clone()), terminal)
        .layer(Extension(db))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let observed = vw_agent_apps::apps::discovery::read::read(
        vw_api_types::agent_apps::AppId::Codex,
        &vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: "h".into(),
            home: root.path().into(),
            search_paths: vec![bin],
            applications: root.path().into(),
        },
    )
    .unwrap();
    let request = json!({"appId":"codex","installationId":observed.installations[0].id,"cwd":root.path(),"args":["hello"]});
    for (field, replacement, status) in [
        ("cwd", json!("/"), StatusCode::UNPROCESSABLE_ENTITY),
        ("installationId", json!("forged"), StatusCode::UNPROCESSABLE_ENTITY),
        (
            "args",
            json!(["--dangerously-bypass-approvals-and-sandbox"]),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        ("command", json!("touch injected"), StatusCode::UNPROCESSABLE_ENTITY),
        ("accountId", json!("wrong-account"), StatusCode::NOT_IMPLEMENTED),
        ("providerId", json!("unknown"), StatusCode::NOT_IMPLEMENTED),
    ] {
        let mut invalid = request.clone();
        invalid[field] = replacement;
        assert_eq!(call(&app, TOKEN, invalid).await.0, status, "{field}");
        assert!(!root.path().join("accepted").exists());
    }
    assert_eq!(
        call(&app, "invalid-token-with-sufficient-length", request.clone()).await.0,
        StatusCode::UNAUTHORIZED
    );
    value["principals"][0]["grants"][0]["launch"] = json!(false);
    std::fs::write(&policy, value.to_string()).unwrap();
    assert_eq!(call(&app, TOKEN, request.clone()).await.0, StatusCode::FORBIDDEN);
    assert!(!root.path().join("accepted").exists());
    value["principals"][0]["grants"][0]["launch"] = json!(true);
    std::fs::write(&policy, value.to_string()).unwrap();
    let mut failed = request.clone();
    failed["args"] = json!(["fail-terminal"]);
    assert_eq!(call(&app, TOKEN, failed).await, (StatusCode::SERVICE_UNAVAILABLE, Value::Null));
    assert!(!root.path().join("accepted").exists());
    let (status, result) = call(&app, TOKEN, request.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result, json!({"installationId":request["installationId"]}));
    assert_eq!(std::fs::read_to_string(root.path().join("accepted")).unwrap(), "[\"hello\"]");
}
