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
    let jobs = DiagnosticJobs::open(root).unwrap();
    let state = agent_gateway::gateway_test_fixture::state::<(), (), ()>(
        Default::default(), true, &[TOKEN.to_owned()],
    );
    let app = crate::agent_apps_diagnostics::router()
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
    assert!(!root.join("diagnostics.json").exists(), "denied requests must not create jobs");
    for (body, expected) in [
        (json!({"kind":"mcp","serverIds":["sentinel","missing"]}), StatusCode::GONE),
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
        assert_eq!(count.load(Ordering::SeqCst), expected_count);
    }
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
