//! Controlled HTTP evidence: editor candidates are bound to the authorized diagnostic host.
use super::*;
use axum::{body::Body, http::Request};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tower::ServiceExt;
use vw_api_types::agent_apps::AppId;
async fn body(response: Response) -> Value {
    serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 32768).await.unwrap())
        .unwrap()
}
#[tokio::test]
async fn diagnostic_endpoint_draft_reference_executes_without_save() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))?;
    let database = GlobalDatabase::for_home(root.path())?;
    let tenant = TenantScope::new("platform", "company")?;
    let jobs = DiagnosticJobs::open(root.path()).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let draft = format!("http://{}/draft", listener.local_addr()?);
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().fallback(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                async { StatusCode::UNAUTHORIZED }
            }),
        )
        .await
        .unwrap()
    });
    let store = crate::gateway::settings_store::GatewaySettingsStore::new(database.clone())?;
    let stored = json!({"settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://saved.example.test"}},"customEndpoints":[]});
    store.upsert(&tenant, "app_providers.records", "\"claude\":provider", &stored.to_string())?;
    let before = AppProviderStore::preview_record(&database, &tenant, AppId::Claude, "provider")
        .map_err(anyhow::Error::msg)?;
    let policy = root.path().join("access.json");
    let grant = json!({"platform_id":"platform","company_id":"company","host_id":"host", "provider_config_read":true,"routing_manage":true,"routing_apps":[{"project_id":"project","app_id":"claude"}]});
    let access = json!({"principals":[{"id":"member","token_sha256":hex::encode(Sha256::digest("fixture-diagnostic-member-token-00001")),"grants":[grant.clone()]},{"id":"other","token_sha256":hex::encode(Sha256::digest("fixture-diagnostic-other-token-000002")),"grants":[grant]}],"hosts":[{"id":"host","binding":"local","online":true,"home":root.path(),"applications":root.path(),"search_paths":[]}]});
    std::fs::write(&policy, access.to_string())?;
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600))?;
    let app = router_with_policy(Some(policy.clone()))
        .layer(Extension(tenant.clone()))
        .layer(Extension(database.clone()))
        .layer(Extension(jobs));
    let request = |method: &str, resource: &str, token: &str, payload: Value| {
        Request::builder().method(method).uri(format!("/agent-apps/diagnostics/{resource}?projectId=project&targetHostId=host&appId=claude")).header("content-type","application/json").header("x-agent-apps-token",token).body(Body::from(payload.to_string())).unwrap()
    };
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "endpoint-candidates/query",
            "fixture-diagnostic-member-token-00001",
            json!({"providerId":"provider","draftUrls":[draft],"presetName":"PackyCode"}),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let candidates = body(response).await;
    let id = candidates["candidates"][0]["id"].clone();
    assert_eq!(candidates["candidates"][0]["displayUrl"], draft);
    assert!(
        candidates["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["sources"].as_array().unwrap().contains(&json!("preset")))
    );
    assert_eq!(count.load(Ordering::SeqCst), 0, "candidate query cannot start network I/O");
    let payload = json!({"kind":"speed","providerId":"provider","endpointIds":[id]});
    let response = app
        .clone()
        .oneshot(request("POST", "jobs", "fixture-diagnostic-other-token-000002", payload.clone()))
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "another authorized principal cannot consume the editor reference"
    );
    let mut revoked = access.clone();
    revoked["principals"][0]["grants"][0]["routing_manage"] = json!(false);
    std::fs::write(&policy, revoked.to_string())?;
    let response = app
        .clone()
        .oneshot(request("POST", "jobs", "fixture-diagnostic-member-token-00001", payload.clone()))
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "authorization is rechecked at execution time"
    );
    std::fs::write(&policy, access.to_string())?;
    let response = app
        .clone()
        .oneshot(request("POST", "jobs", "fixture-diagnostic-member-token-00001", payload))
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "unsaved candidate must resolve on its authorized host"
    );
    let job = body(response).await;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let snapshot = loop {
        let response = app
            .clone()
            .oneshot(request(
                "GET",
                &format!("jobs/{}", job["taskId"].as_str().unwrap()),
                "fixture-diagnostic-member-token-00001",
                Value::Null,
            ))
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let snapshot = body(response).await;
        if snapshot["state"]["status"] == "succeeded" {
            break snapshot;
        }
        assert!(tokio::time::Instant::now() < deadline, "{snapshot}");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };
    assert_eq!(snapshot["results"][0]["endpointId"], id);
    assert_eq!(snapshot["results"][0]["report"]["outcome"]["evidence"]["httpStatus"], 401);
    assert_eq!(count.load(Ordering::SeqCst), 2, "one warmup and one measured GET");
    assert_eq!(
        AppProviderStore::preview_record(&database, &tenant, AppId::Claude, "provider")
            .map_err(anyhow::Error::msg)?,
        before,
        "probing must never persist the draft"
    );
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    Ok(())
}
