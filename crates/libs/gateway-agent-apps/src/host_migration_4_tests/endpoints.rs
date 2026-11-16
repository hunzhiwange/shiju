use app_provider_services::AppProviderStore;
use app_diagnostics_runtime::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticJobs};
use agent_database::GlobalDatabase;
use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tower::ServiceExt;
use vw_api_app_identity::AppId;
use vw_api_app_diagnostics::DiagnosticJobIdentity;
use vw_api_app_diagnostics::DiagnosticJobState;
use vw_api_app_provider_base::AppProviderWrite;
use vw_api_app_provider_base::ProviderProtocol;
use vw_api_transport::id::TenantScope;

const TOKEN: &str = "fixture-diagnostic-member-token-00001";

#[tokio::test]
async fn diagnostic_endpoints_api_is_read_only_and_scoped() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))?;
    let database = GlobalDatabase::for_home(root.path())?;
    let tenant = TenantScope::new("platform", "company")?;
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_url = format!("http://{}/v1", listener.local_addr()?);
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().fallback(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                async { StatusCode::OK }
            }),
        )
        .await
        .unwrap();
    });
    let store = app_settings_store::settings_store::GatewaySettingsStore::new(database.clone())?;
    let stored = json!({"baseUrl":"https://www.packyapi.ai/", "apiKey":"fixture-private-key",
        "settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://saved.example.test/"}},
        "customEndpoints":[{"url":"https://www.packyapi.ai", "kind":"baseUrl", "addedAt":1}]});
    store.upsert(&tenant, "app_providers.records", "\"claude\":provider", &stored.to_string())?;
    let before = AppProviderStore::preview_record(&database, &tenant, AppId::Claude, "provider")
        .map_err(anyhow::Error::msg)?;
    let policy = root.path().join("access.json");
    std::fs::write(&policy, json!({
        "principals":[{"id":"member","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{
            "platform_id":"platform","company_id":"company","host_id":"host",
            "provider_config_read":true,"routing_manage":true,
            "routing_apps":[{"project_id":"project","app_id":"claude"}]}]}],
        "hosts":[{"id":"host","binding":"local","online":true,"home":root.path(),
            "applications":root.path(),"search_paths":[]}]
    }).to_string())?;
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600))?;
    let app = crate::agent_apps_diagnostics::router_with_policy::<super::probe_host::NativeProbeHost>(Some(policy.clone()))
        .layer(Extension(tenant.clone()))
        .layer(Extension(database.clone()));
    let request = |body: Value, project: &str, host: &str, app_id: &str, token: &str| {
        Request::builder().method("POST")
            .uri(format!("/agent-apps/diagnostics/endpoint-candidates/query?projectId={project}&targetHostId={host}&appId={app_id}"))
            .header("content-type","application/json").header("x-agent-apps-token",token)
            .body(Body::from(body.to_string())).unwrap()
    };
    let body = json!({"providerId":"provider","presetName":"PackyCode",
        "draftUrls":[" https://www.packyapi.ai/// ",local_url]});
    let response =
        app.clone().oneshot(request(body.clone(), "project", "host", "claude", TOKEN)).await?;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "authorized candidate query must be available without starting a diagnostic"
    );
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = axum::body::to_bytes(response.into_body(), 32768).await?;
    assert!(!String::from_utf8_lossy(&bytes).contains("fixture-private-key"));
    let result: Value = serde_json::from_slice(&bytes)?;
    let candidates = result["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 6);
    assert_eq!(candidates[0]["displayUrl"], "https://www.packyapi.ai");
    assert_eq!(candidates[0]["sources"], json!(["draft", "saved_config", "preset", "custom"]));
    assert_eq!(candidates[0]["id"], hex::encode(Sha256::digest(b"https://www.packyapi.ai")));
    assert_eq!(candidates[1]["displayUrl"], local_url);
    for (project, host, app_id, token, expected) in [
        ("foreign", "host", "claude", TOKEN, StatusCode::FORBIDDEN),
        ("project", "foreign", "claude", TOKEN, StatusCode::FORBIDDEN),
        ("project", "host", "codex", TOKEN, StatusCode::FORBIDDEN),
        ("project", "host", "claude", "invalid", StatusCode::UNAUTHORIZED),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(body.clone(), project, host, app_id, token))
                .await?
                .status(),
            expected
        );
    }
    let foreign_tenant = crate::agent_apps_diagnostics::router_with_policy::<super::probe_host::NativeProbeHost>(Some(policy.clone()))
        .layer(Extension(TenantScope::new("platform", "foreign")?))
        .layer(Extension(database.clone()));
    assert_eq!(
        foreign_tenant
            .oneshot(request(body.clone(), "project", "host", "claude", TOKEN))
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    let original_policy = std::fs::read(&policy)?;
    let mut revoked: Value = serde_json::from_slice(&original_policy)?;
    revoked["principals"][0]["grants"][0]["provider_config_read"] = json!(false);
    std::fs::write(&policy, revoked.to_string())?;
    assert_eq!(
        app.clone()
            .oneshot(request(body.clone(), "project", "host", "claude", TOKEN))
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    std::fs::write(&policy, original_policy)?;
    for invalid in [
        "file:///tmp/service",
        "https://user:secret@example.test/",
        "https://@example.test/",
        "https://example.test/?token=secret",
        "http://example.test/",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(
                    json!({"providerId":"provider","draftUrls":[invalid]}),
                    "project",
                    "host",
                    "claude",
                    TOKEN
                ))
                .await?
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(request(json!({"providerId":"missing"}), "project", "host", "claude", TOKEN))
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                json!({"providerId":"provider","presetName":"missing"}),
                "project",
                "host",
                "claude",
                TOKEN
            ))
            .await?
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        before,
        AppProviderStore::preview_record(&database, &tenant, AppId::Claude, "provider")
            .map_err(anyhow::Error::msg)?
    );
    assert!(!root.path().join("diagnostics.json").exists());
    assert_eq!(
        requests.load(Ordering::SeqCst),
        0,
        "candidate queries must never connect to draft URLs"
    );
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    Ok(())
}
