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

#[test]
fn diagnostic_api_start_authorizes_before_execution() -> anyhow::Result<()> {
    const CHILD: &str = "SHIJU_DIAGNOSTIC_START_ROOT";
    if let Some(root) = std::env::var_os(CHILD) {
        return tokio::runtime::Runtime::new()?.block_on(check_start(std::path::Path::new(&root)));
    }
    let root = tempfile::tempdir()?;
    let status = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "host_migration_4_tests::start::diagnostic_api_start_authorizes_before_execution",
            "--nocapture",
        ])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env(CHILD, root.path())
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("HOME", root.path())
        .env("TMPDIR", root.path())
        .env("XDG_CONFIG_HOME", root.path().join("xdg-config"))
        .env("XDG_CACHE_HOME", root.path().join("xdg-cache"))
        .env("XDG_DATA_HOME", root.path().join("xdg-data"))
        .env("XDG_STATE_HOME", root.path().join("xdg-state"))
        .env("CODEX_HOME", root.path().join("codex"))
        .env("CLAUDE_CONFIG_DIR", root.path().join("claude"))
        .env("VIBEWINDOW_CONFIG_DIR", root.path().join("shiju-config"))
        .status()?;
    anyhow::ensure!(status.success(), "isolated start test failed: {status}");
    Ok(())
}

async fn check_start(root: &std::path::Path) -> anyhow::Result<()> {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?;
    let database = GlobalDatabase::for_home(root)?;
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
                        return axum::Json(json!({"type":"message","role":"assistant", "model":"fixture-model", "stop_reason":"end_turn", "content":[{"type":"text","text":"OK"}]})).into_response();
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
    catalog
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
    let marker = root.join("stdio-started");
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
    let jobs = DiagnosticJobs::open(root).unwrap();
    let state = agent_gateway::gateway_test_fixture::state::<(), (), ()>(
        Default::default(), true, &[TOKEN.to_owned()],
    );
    let app = crate::agent_apps_diagnostics::router::<super::probe_host::NativeProbeHost>()
        .layer(Extension(tenant))
        .layer(Extension(database))
        .layer(Extension(jobs.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state, agent_gateway::request_policy::skey_auth_middleware::<(), (), ()>,
        ));
    let request = |body: Value, project: &str, host: &str, app_id: &str, token: &str| {
        Request::builder().method("POST")
            .uri(format!("/agent-apps/diagnostics/jobs?projectId={project}&targetHostId={host}&appId={app_id}"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
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
            app.clone().oneshot(request(body, "project", "desktop-local", "claude", "invalid-token")).await?;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "pairing rejects before submitting any diagnostic"
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(!marker.exists(), "denied stdio configuration must not execute");
    assert!(!root.join("diagnostics.json").exists(), "denied requests must not create jobs");
    // A paired client still cannot resolve an MCP record from another project.
    let response = app.clone().oneshot(request(
        json!({"kind":"mcp","serverIds":["http"]}),
        "foreign", "desktop-local", "claude", TOKEN,
    )).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    for (body, expected) in [
        (json!({"kind":"mcp","serverIds":["sentinel","missing"]}), StatusCode::NOT_FOUND),
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
            app.clone().oneshot(request(body, "project", "desktop-local", "claude", TOKEN)).await?;
        assert_eq!(response.status(), expected);
        assert!(!marker.exists());
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
    assert!(!root.join("diagnostics.json").exists(), "rejected inputs must not create jobs");
    let response = app
        .clone()
        .oneshot(request(
            json!({"kind":"model","providerId":"provider","modelId":"missing-model"}),
            "project",
            "desktop-local",
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
            app.clone().oneshot(request(body, "project", "desktop-local", "claude", TOKEN)).await?;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let job: DiagnosticJobIdentity =
            serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 16384).await?)?;
        let owner = DiagnosticJobOwner { scope: job.scope.clone(), principal: "desktop-local-user".into() };
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
            target_host_id: "desktop-local".into(),
            app_id: AppId::Claude,
        },
        principal: "desktop-local-user".into(),
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
            "desktop-local",
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
