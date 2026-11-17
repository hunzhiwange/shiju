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
async fn diagnostic_api_read_and_cancel_are_scope_bound() -> anyhow::Result<()> {
    use app_diagnostics_runtime::agent_apps_diagnostic_jobs::DiagnosticProbeError;
    use vw_api_app_diagnostics::*;
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
            project: vw_api_transport::ProjectScope::new(tenant.clone(), "project".into())?,
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    let app = crate::agent_apps_diagnostics::router_with_policy::<super::probe_host::NativeProbeHost>(Some(policy.clone()))
        .layer(Extension(tenant))
        .layer(Extension(jobs.clone()));
    let request = |id: &str, cancel: bool, project: &str, host: &str, app_id: &str, token: &str| {
        Request::builder().method(if cancel {"POST"} else {"GET"})
            .uri(format!("/agent-apps/diagnostics/jobs/{id}{}?projectId={project}&targetHostId={host}&appId={app_id}", if cancel {"/cancel"} else {""}))
            .body(Body::empty()).unwrap()
    };
    let foreign_tenant_app =
        crate::agent_apps_diagnostics::router_with_policy::<super::probe_host::NativeProbeHost>(Some(policy.clone()))
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
