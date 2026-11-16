use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::{body::Body, http::{Request, StatusCode}};
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};
use vw_api_app_identity::AppId;
use vw_api_app_diagnostics::DiagnosticJobIdentity;
use vw_api_app_provider_base::{AppProviderWrite, ProviderProtocol};
use vw_api_transport::id::TenantScope;

#[tokio::test]
async fn diagnostic_api_start_authorizes_before_execution_gateway() -> anyhow::Result<()> {
    use super::app_routing::lifecycle_api_tests::{HOST, MANAGER, spawn_gateway, write_policy};
    let root = tempfile::tempdir()?;
    let credentials = root.path().join("credentials");
    std::fs::create_dir(&credentials)?;
    std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700))?;
    let policy = root.path().join("access.json");
    write_policy(&policy, root.path(), 0);
    let mut access: Value = serde_json::from_slice(&std::fs::read(&policy)?)?;
    let grant = &mut access["principals"][0]["grants"][0];
    grant["provider_config_read"] = json!(true);
    grant["routing_apps"] = json!([{"project_id":"project", "app_id":"claude"}]);
    std::fs::write(&policy, access.to_string())?;
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/v1", listener.local_addr()?);
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().fallback(move || {
                let observed = observed.clone();
                async move {
                    observed.fetch_add(1, Ordering::SeqCst);
                    StatusCode::UNAUTHORIZED
                }
            }),
        )
        .await
        .unwrap();
    });
    let db = GlobalDatabase::for_home(root.path())?;
    AppProviderStore::new(db)
        .map_err(anyhow::Error::msg)?
        .save(
            &TenantScope::new("p", "owner")?,
            AppId::Claude,
            "provider",
            &AppProviderWrite {
                name: "Fixture".into(),
                protocol: ProviderProtocol::Anthropic,
                api_key: "fixture-key".into(),
                base_url: url,
                model: "fixture-model".into(),
                request_options: None,
            },
        )
        .map_err(anyhow::Error::msg)?;
    let (gateway, port) = spawn_gateway(root.path(), &policy).await;
    let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build()?;
    for (project, status) in
        [("foreign", reqwest::StatusCode::FORBIDDEN), ("project", reqwest::StatusCode::ACCEPTED)]
    {
        let response = client
            .post(format!("http://127.0.0.1:{port}/v1/agent-apps/diagnostics/jobs"))
            .header("x-platform-id", "p")
            .header("x-company-id", "owner")
            .header("x-agent-apps-token", MANAGER)
            .query(&[("projectId", project), ("targetHostId", HOST), ("appId", "claude")])
            .json(&json!({"kind":"reachability", "providerId":"provider"}))
            .send()
            .await?;
        assert_eq!(response.status(), status);
        if project == "foreign" {
            assert_eq!(count.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(response.headers()["cache-control"], "no-store");
            let identity: DiagnosticJobIdentity = response.json().await?;
            assert!(!identity.task_id.as_ref().is_empty());
        }
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while count.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    drop(gateway);
    server.abort();
    Ok(())
}
