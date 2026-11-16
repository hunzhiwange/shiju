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
async fn diagnostic_api_start_authorizes_before_execution() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))?;
    let database = GlobalDatabase::for_home(root.path())?;
    let tenant = TenantScope::new("platform", "company")?;
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/v1", listener.local_addr()?);
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().fallback(move |req: Request<Body>| {
                let observed = observed.clone();
                async move {
                    observed.fetch_add(1, Ordering::SeqCst);
                    let body = axum::body::to_bytes(req.into_body(), 32768).await.unwrap();
                    let payload: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                    if payload["method"] == "initialize" {
                        return axum::Json(json!({"jsonrpc":"2.0","id":payload["id"], "result":{
                            "protocolVersion":"2025-03-26", "capabilities":{}, "serverInfo":{"name":"fixture","version":"1"}
                        }})).into_response();
                    }
                    if payload["method"].is_string() { return StatusCode::ACCEPTED.into_response(); }
                    if payload["stream"] == true {
                        let sample = super::stream_fixtures::samples()
                            .into_iter().find(|(protocol, _, _)| *protocol == "anthropic").unwrap().2;
                        return ([("content-type", "text/event-stream")], sample).into_response();
                    }
                    if payload["model"].is_string() {
                        return axum::Json(json!({"type":"message","role":"assistant", "model":"fixture-model", "content":[{"type":"text","text":"OK"}]})).into_response();
                    }
                    StatusCode::OK.into_response()
                }
            }),
        )
        .await
        .unwrap();
    });
    let providers = AppProviderStore::new(database.clone()).map_err(anyhow::Error::msg)?;
    providers
        .save(
            &tenant,
            AppId::Claude,
            "provider",
            &AppProviderWrite {
                name: "Fixture".into(),
                protocol: ProviderProtocol::Anthropic,
                api_key: "fixture-key".into(),
                base_url: url.clone(),
                model: "fixture-model".into(),
                request_options: None,
            },
        )
        .map_err(anyhow::Error::msg)?;
    let project = vw_api_transport::ProjectScope::new(tenant.clone(), "project".into())?;
    let catalog = agent_app_services::mcp::McpCatalogStore::new(database.clone()).unwrap();
    let mcp_scope = vw_api_mcp_management::McpCatalogScope::Project { scope: project };
    let mcp_url = format!("{url}/mcp");
    let draft = |id: &str, connection| vw_agent_apps::mcp::catalog::McpCatalogDraft {
        id: id.into(),
        name: None,
        description: None,
        homepage: None,
        documentation: None,
        tags: Vec::new(),
        connection,
    };
    let mcp = catalog
        .create(
            &mcp_scope,
            draft(
                "http",
                vw_api_mcp_management::McpConnectionDefinition::StreamableHttp {
                    url: mcp_url.clone(),
                    headers: Default::default(),
                },
            ),
        )
        .unwrap();
    let marker = root.path().join("stdio-started");
    catalog
        .create(
            &mcp_scope,
            draft(
                "sentinel",
                vw_api_mcp_management::McpConnectionDefinition::Stdio {
                    command: "/usr/bin/touch".into(),
                    args: vec![marker.to_string_lossy().into_owned()],
                    env: Default::default(),
                    cwd: None,
                },
            ),
        )
        .unwrap();
    let policy = root.path().join("access.json");
    let grant = json!({"platform_id":"platform", "company_id":"company", "host_id":"host",
        "provider_config_read":true,"routing_manage":true,
        "routing_apps":[{"project_id":"project","app_id":"claude"}],
        "mcp_probe":[{"project_id":"project", "app_id":"claude", "record_id":"http",
            "connection_revision":mcp.connection_revision,
            "endpoint_sha256":hex::encode(Sha256::digest(mcp_url.as_bytes()))}]});
    std::fs::write(&policy, json!({
        "principals":[{"id":"member", "token_sha256":hex::encode(Sha256::digest(TOKEN)), "grants":[grant]}],
        "hosts":[{"id":"host", "binding":"local", "online":true, "home":root.path(), "applications":root.path(), "search_paths":[], "credential_root":root.path()}]
    }).to_string())?;
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600))?;
    let jobs = DiagnosticJobs::open(root.path()).unwrap();
    let app = crate::agent_apps_diagnostics::router_with_policy::<super::probe_host::NativeProbeHost>(Some(policy.clone()))
        .layer(Extension(tenant))
        .layer(Extension(database))
        .layer(Extension(jobs.clone()));
    let request = |body: Value, project: &str, host: &str, app_id: &str, token: &str| {
        Request::builder().method("POST")
            .uri(format!("/agent-apps/diagnostics/jobs?projectId={project}&targetHostId={host}&appId={app_id}"))
            .header("content-type", "application/json").header("x-agent-apps-token", token)
            .body(Body::from(body.to_string())).unwrap()
    };
    for body in [
        json!({"kind":"reachability","providerId":"provider"}),
        json!({"kind":"speed","providerId":"provider","endpointIds":["invalid"]}),
        json!({"kind":"model","providerId":"provider","modelId":"fixture-model"}),
        json!({"kind":"streaming","providerId":"provider","modelId":"fixture-model"}),
        json!({"kind":"mcp","serverIds":["sentinel"]}),
    ] {
        let response =
            app.clone().oneshot(request(body, "foreign", "host", "claude", TOKEN)).await?;
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "reject before submitting any diagnostic"
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(!marker.exists(), "denied stdio configuration must not execute");
    assert!(!root.path().join("diagnostics.json").exists(), "denied requests must not create jobs");
    for (project, host, app_id, token, expected) in [
        ("project", "foreign", "claude", TOKEN, StatusCode::FORBIDDEN),
        ("project", "host", "codex", TOKEN, StatusCode::FORBIDDEN),
        ("project", "host", "claude", "invalid-token", StatusCode::UNAUTHORIZED),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(
                    json!({"kind":"reachability","providerId":"provider"}),
                    project,
                    host,
                    app_id,
                    token
                ))
                .await?
                .status(),
            expected
        );
    }
    for (body, expected) in [
        (json!({"kind":"mcp","serverIds":["http","sentinel"]}), StatusCode::FORBIDDEN),
        (
            json!({"kind":"model","providerId":"provider","modelId":"fixture-model","baseUrl":url}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"kind":"reachability","providerId":"provider","modelId":"fixture-model"}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response =
            app.clone().oneshot(request(body, "project", "host", "claude", TOKEN)).await?;
        assert_eq!(response.status(), expected);
        assert!(!marker.exists());
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
    let response = app
        .clone()
        .oneshot(request(
            json!({"kind":"model","providerId":"provider","modelId":"missing-model"}),
            "project",
            "host",
            "claude",
            TOKEN,
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    for (body, expected_count) in [
        (json!({"kind":"reachability","providerId":"provider"}), 1),
        (
            json!({"kind":"speed","providerId":"provider","endpointIds":[hex::encode(Sha256::digest(url.as_bytes()))]}),
            3,
        ),
        (json!({"kind":"model","providerId":"provider","modelId":"fixture-model"}), 4),
        (json!({"kind":"streaming","providerId":"provider","modelId":"fixture-model"}), 5),
        (json!({"kind":"mcp","serverIds":["http"]}), 0),
    ] {
        let response =
            app.clone().oneshot(request(body, "project", "host", "claude", TOKEN)).await?;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let job: DiagnosticJobIdentity =
            serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 16384).await?)?;
        let owner = DiagnosticJobOwner { scope: job.scope.clone(), principal: "member".into() };
        let snapshot = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let snapshot = jobs.snapshot(&owner, &job.task_id).unwrap();
                if matches!(
                    snapshot.state,
                    DiagnosticJobState::Succeeded {} | DiagnosticJobState::Failed { .. }
                ) {
                    break snapshot;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await?;
        assert_eq!(snapshot.state, DiagnosticJobState::Succeeded {});
        assert_eq!(snapshot.progress.completed, 1);
        let results = serde_json::to_value(snapshot.results.as_ref().unwrap())?;
        assert_eq!(results.as_array().unwrap().len(), 1);
        assert_eq!(results[0]["report"]["outcome"]["status"], "succeeded", "{results}");
        assert_eq!(results[0]["kind"], serde_json::to_value(job.kind)?);
        if expected_count == 0 {
            assert!(
                count.load(Ordering::SeqCst) >= 7,
                "MCP initialization and notification reached the fixture"
            );
        } else {
            assert_eq!(count.load(Ordering::SeqCst), expected_count);
        }
    }
    assert!(!marker.exists());
    let before_capacity = count.load(Ordering::SeqCst);
    // Saturate the same production registry; queued work must not bypass host capacity.
    let owner = DiagnosticJobOwner {
        scope: vw_api_app_diagnostics::DiagnosticScope {
            project: vw_api_transport::ProjectScope::new(
                TenantScope::new("platform", "company")?,
                "project".into(),
            )?,
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    let mut held = Vec::new();
    for _ in 0..128 {
        let registry = jobs.clone();
        let owner = owner.clone();
        held.push(tokio::task::spawn_blocking(move || registry.start(owner, vw_api_app_diagnostics::DiagnosticProbeKind::Reachability,
            vec![|cancel: tokio_util::sync::CancellationToken| async move {
                cancel.cancelled().await;
                Err(app_diagnostics_runtime::agent_apps_diagnostic_jobs::DiagnosticProbeError::Cancelled)
            }])).await?.unwrap());
    }
    let response = app
        .oneshot(request(
            json!({"kind":"reachability","providerId":"provider"}),
            "project",
            "host",
            "claude",
            TOKEN,
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(count.load(Ordering::SeqCst), before_capacity);
    for job in &held {
        jobs.cancel(&owner, &job.task_id).unwrap();
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if held.iter().all(|job| {
                jobs.snapshot(&owner, &job.task_id)
                    .is_ok_and(|snapshot| snapshot.state == DiagnosticJobState::Cancelled {})
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    server.abort();
    Ok(())
}
