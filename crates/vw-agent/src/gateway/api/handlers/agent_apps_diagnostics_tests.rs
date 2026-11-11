use crate::{
    app_providers::AppProviderStore,
    gateway::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticJobs},
    global_database::GlobalDatabase,
};
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
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{DiagnosticJobIdentity, DiagnosticJobState},
    app_providers::{AppProviderWrite, ProviderProtocol},
    id::TenantScope,
};

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
                        let sample = crate::model_relay::stream_observer::tests::samples()
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
    let project = vw_api_types::id::ProjectScope::new(tenant.clone(), "project".into())?;
    let catalog = crate::agent_apps::mcp::McpCatalogStore::new(database.clone()).unwrap();
    let mcp_scope = vw_api_types::mcp_management::McpCatalogScope::Project { scope: project };
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
                vw_api_types::mcp_management::McpConnectionDefinition::StreamableHttp {
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
                vw_api_types::mcp_management::McpConnectionDefinition::Stdio {
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
    let app = super::agent_apps_diagnostics::router_with_policy(Some(policy.clone()))
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
        scope: vw_api_types::agent_apps_diagnostics::DiagnosticScope {
            project: vw_api_types::id::ProjectScope::new(
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
        held.push(tokio::task::spawn_blocking(move || registry.start(owner, vw_api_types::agent_apps_diagnostics::DiagnosticProbeKind::Reachability,
            vec![|cancel: tokio_util::sync::CancellationToken| async move {
                cancel.cancelled().await;
                Err(crate::gateway::agent_apps_diagnostic_jobs::DiagnosticProbeError::Cancelled)
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

#[tokio::test]
async fn diagnostic_api_read_and_cancel_are_scope_bound() -> anyhow::Result<()> {
    use crate::gateway::agent_apps_diagnostic_jobs::DiagnosticProbeError;
    use vw_api_types::agent_apps_diagnostics::*;
    let root = tempfile::tempdir()?;
    let policy = root.path().join("access.json");
    let other_token = "fixture-other-diagnostic-member-token-00002";
    let grant = json!({"platform_id":"platform","company_id":"company","host_id":"host",
        "provider_config_read":true,"routing_manage":true,
        "routing_apps":[{"project_id":"project","app_id":"claude"}]});
    let mut foreign_tenant_grant = grant.clone();
    foreign_tenant_grant["company_id"] = json!("foreign-company");
    std::fs::write(&policy, json!({"principals":[
        {"id":"member","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[grant.clone(), foreign_tenant_grant]},
        {"id":"other","token_sha256":hex::encode(Sha256::digest(other_token)),"grants":[grant]}],
        "hosts":[{"id":"host","binding":"local","online":true,"home":root.path(),"applications":root.path(),"search_paths":[]}]
    }).to_string())?;
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600))?;
    let jobs = DiagnosticJobs::open(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company")?;
    let owner = DiagnosticJobOwner {
        scope: DiagnosticScope {
            project: vw_api_types::id::ProjectScope::new(tenant.clone(), "project".into())?,
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    let app = super::agent_apps_diagnostics::router_with_policy(Some(policy.clone()))
        .layer(Extension(tenant))
        .layer(Extension(jobs.clone()));
    let request = |id: &str, cancel: bool, project: &str, host: &str, app_id: &str, token: &str| {
        Request::builder().method(if cancel {"POST"} else {"GET"})
            .uri(format!("/agent-apps/diagnostics/jobs/{id}{}?projectId={project}&targetHostId={host}&appId={app_id}", if cancel {"/cancel"} else {""}))
            .header("x-agent-apps-token", token).body(Body::empty()).unwrap()
    };
    let foreign_tenant_app =
        super::agent_apps_diagnostics::router_with_policy(Some(policy.clone()))
            .layer(Extension(TenantScope::new("platform", "foreign-company")?))
            .layer(Extension(jobs.clone()));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/", listener.local_addr()?);
    let server_gate = gate.clone();
    let server_requests = requests.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().fallback(move || {
                let gate = server_gate.clone();
                let requests = server_requests.clone();
                async move {
                    requests.fetch_add(1, Ordering::SeqCst);
                    gate.acquire().await.unwrap().forget();
                    StatusCode::UNAUTHORIZED
                }
            }),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build()?;
    let completed = Arc::new(AtomicUsize::new(0));
    let registry = jobs.clone();
    let submitted_owner = owner.clone();
    let observed = completed.clone();
    let job = tokio::task::spawn_blocking(move || registry.start(submitted_owner, DiagnosticProbeKind::Reachability,
        (0..2).map(|_| {
            let client = client.clone(); let url = url.clone(); let observed = observed.clone();
            move |cancel: tokio_util::sync::CancellationToken| async move {
                tokio::select! { biased;
                    () = cancel.cancelled() => Err(DiagnosticProbeError::Cancelled),
                    response = client.get(url).send() => {
                        let response = response.unwrap();
                        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
                        observed.fetch_add(1, Ordering::SeqCst);
                        Ok(DiagnosticResult::Reachability { provider_id: "provider".into(), report: DiagnosticReport {
                            source: DiagnosticEvidenceSource::ControlledFixture, timings: vec![],
                            outcome: DiagnosticOutcome::Succeeded { evidence: ReachabilityEvidence { http_status: 401, slow: false } }
                        }})
                    }
                }
            }
        }).collect())).await?.unwrap();
    let id = job.task_id.as_ref();
    let response =
        app.clone().oneshot(request(id, false, "project", "host", "claude", TOKEN)).await?;
    assert_eq!(response.status(), StatusCode::OK, "same scope must read a live diagnostic job");
    assert_eq!(response.headers()["cache-control"], "no-store");
    let initial: DiagnosticJobSnapshot =
        serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 32768).await?)?;
    assert_eq!(initial.progress.completed, 0);
    for cancel in [false, true] {
        for (task, project, host, app_id, token, status) in [
            (id, "project", "host", "claude", other_token, StatusCode::NOT_FOUND),
            ("unknown", "project", "host", "claude", TOKEN, StatusCode::NOT_FOUND),
            (id, "foreign", "host", "claude", TOKEN, StatusCode::FORBIDDEN),
            (id, "project", "foreign", "claude", TOKEN, StatusCode::FORBIDDEN),
            (id, "project", "host", "codex", TOKEN, StatusCode::FORBIDDEN),
            (id, "project", "host", "claude", "invalid", StatusCode::UNAUTHORIZED),
        ] {
            let response =
                app.clone().oneshot(request(task, cancel, project, host, app_id, token)).await?;
            assert_eq!(response.status(), status);
            let body = String::from_utf8(
                axum::body::to_bytes(response.into_body(), 32768).await?.to_vec(),
            )?;
            assert!(
                !body.contains(id)
                    && !body.contains("principal")
                    && !body.contains(root.path().to_str().unwrap())
            );
        }
    }
    for cancel in [false, true] {
        assert_eq!(
            foreign_tenant_app
                .clone()
                .oneshot(request(id, cancel, "project", "host", "claude", TOKEN))
                .await?
                .status(),
            StatusCode::NOT_FOUND,
            "a member authorized in both tenants must not cross tenant boundaries"
        );
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while requests.load(Ordering::SeqCst) != 2 {
        assert!(std::time::Instant::now() < deadline, "both HTTP probes must reach the fixture");
        tokio::task::yield_now().await;
    }
    gate.add_permits(1);
    loop {
        assert!(std::time::Instant::now() < deadline, "partial result deadline");
        let response =
            app.clone().oneshot(request(id, false, "project", "host", "claude", TOKEN)).await?;
        let partial: DiagnosticJobSnapshot =
            serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 32768).await?)?;
        assert!(partial.progress.completed >= initial.progress.completed);
        if partial.progress.completed == 1 {
            assert_eq!(partial.results.unwrap().len(), 1);
            break;
        }
        tokio::task::yield_now().await;
    }
    let response =
        app.clone().oneshot(request(id, true, "project", "host", "claude", TOKEN)).await?;
    assert_eq!(response.status(), StatusCode::OK);
    loop {
        assert!(std::time::Instant::now() < deadline, "cancellation cleanup deadline");
        let snapshot = jobs.snapshot(&owner, &job.task_id).unwrap();
        if snapshot.state == (DiagnosticJobState::Cancelled {}) {
            break;
        }
        tokio::task::yield_now().await;
    }
    for cancel in [false, true, true] {
        let response =
            app.clone().oneshot(request(id, cancel, "project", "host", "claude", TOKEN)).await?;
        let snapshot: DiagnosticJobSnapshot =
            serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 32768).await?)?;
        assert_eq!(snapshot.state, DiagnosticJobState::Cancelled {});
        assert_eq!(snapshot.progress.completed, 1);
        assert_eq!(snapshot.results.unwrap().len(), 1);
    }
    gate.add_permits(1);
    tokio::task::yield_now().await;
    assert_eq!(completed.load(Ordering::SeqCst), 1, "cancelled worker must not resume I/O");
    // Successful completion is also stable under cancellation.
    let registry = jobs.clone();
    let submitted_owner = owner.clone();
    let success = tokio::task::spawn_blocking(move || {
        registry.start(
            submitted_owner,
            DiagnosticProbeKind::Reachability,
            vec![|_: tokio_util::sync::CancellationToken| async {
                Ok(DiagnosticResult::Reachability {
                    provider_id: "provider".into(),
                    report: DiagnosticReport {
                        source: DiagnosticEvidenceSource::ControlledFixture,
                        timings: vec![],
                        outcome: DiagnosticOutcome::Succeeded {
                            evidence: ReachabilityEvidence { http_status: 200, slow: false },
                        },
                    },
                })
            }],
        )
    })
    .await?
    .unwrap();
    loop {
        assert!(std::time::Instant::now() < deadline, "success deadline");
        if jobs.snapshot(&owner, &success.task_id).unwrap().state
            == (DiagnosticJobState::Succeeded {})
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(request(success.task_id.as_ref(), true, "project", "host", "claude", TOKEN))
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let snapshot: DiagnosticJobSnapshot =
            serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 32768).await?)?;
        assert_eq!(snapshot.state, DiagnosticJobState::Succeeded {});
        assert_eq!(snapshot.progress.completed, 1);
    }
    // A completed job retains its terminal outcome after repeated cancellation.
    let registry = jobs.clone();
    let submitted_owner = owner.clone();
    let finished = tokio::task::spawn_blocking(move || {
        registry.start(
            submitted_owner,
            DiagnosticProbeKind::Reachability,
            vec![|_: tokio_util::sync::CancellationToken| async {
                Err(DiagnosticProbeError::Failed(DiagnosticFailure {
                    phase: DiagnosticPhase::Connect,
                    code: DiagnosticFailureCode::Transport,
                }))
            }],
        )
    })
    .await?
    .unwrap();
    loop {
        assert!(std::time::Instant::now() < deadline, "terminal failure deadline");
        if matches!(
            jobs.snapshot(&owner, &finished.task_id).unwrap().state,
            DiagnosticJobState::Failed { .. }
        ) {
            break;
        }
        tokio::task::yield_now().await;
    }
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(request(finished.task_id.as_ref(), true, "project", "host", "claude", TOKEN))
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let snapshot: DiagnosticJobSnapshot =
            serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 32768).await?)?;
        assert_eq!(
            snapshot.state,
            DiagnosticJobState::Failed {
                error: DiagnosticFailure {
                    phase: DiagnosticPhase::Connect,
                    code: DiagnosticFailureCode::Transport
                }
            }
        );
    }
    // An MCP-only submitter can poll and cancel without provider/routing permissions.
    let registry = jobs.clone();
    let submitted_owner = owner.clone();
    let mcp = tokio::task::spawn_blocking(move || {
        registry.start(
            submitted_owner,
            DiagnosticProbeKind::Mcp,
            vec![|cancel: tokio_util::sync::CancellationToken| async move {
                cancel.cancelled().await;
                Err(DiagnosticProbeError::Cancelled)
            }],
        )
    })
    .await?
    .unwrap();
    let mut access: Value = serde_json::from_slice(&std::fs::read(&policy)?)?;
    let original = access.clone();
    access["principals"][0]["grants"][0] = json!({"platform_id":"platform","company_id":"company","host_id":"host",
        "mcp_probe":[{"project_id":"project","app_id":"claude","record_id":"fixture","connection_revision":1}]});
    std::fs::write(&policy, access.to_string())?;
    for cancel in [false, true] {
        assert_eq!(
            app.clone()
                .oneshot(request(mcp.task_id.as_ref(), cancel, "project", "host", "claude", TOKEN))
                .await?
                .status(),
            StatusCode::OK
        );
    }
    loop {
        assert!(std::time::Instant::now() < deadline, "MCP cleanup deadline");
        if jobs.snapshot(&owner, &mcp.task_id).unwrap().state == (DiagnosticJobState::Cancelled {})
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    access["principals"][0]["grants"] = json!([]);
    std::fs::write(&policy, access.to_string())?;
    assert_eq!(
        app.clone().oneshot(request(id, false, "project", "host", "claude", TOKEN)).await?.status(),
        StatusCode::FORBIDDEN,
        "revoked membership is checked again before lookup"
    );
    std::fs::write(&policy, original.to_string())?;
    assert_eq!(requests.load(Ordering::SeqCst), 2, "polling and cancelling never replay probes");
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(601)).await;
    for cancel in [false, true] {
        assert_eq!(
            app.clone()
                .oneshot(request(id, cancel, "project", "host", "claude", TOKEN))
                .await?
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    Ok(())
}
