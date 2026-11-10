use std::{
    future::pending,
    os::unix::fs::DirBuilderExt,
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::high_availability::circuit_breaker::{AttemptOutcome, CircuitBreaker};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthErrorCode,
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource::ControlledFixture, DiagnosticFailure, DiagnosticFailureCode,
        DiagnosticJobState, DiagnosticOutcome, DiagnosticPhase, DiagnosticProbeKind,
        DiagnosticResult, DiagnosticScope, ReachabilityEvidence,
    },
    agent_apps_high_availability::{CircuitBreakerPolicy, FailoverHealthState},
    app_providers::claude::{ClaudeConfig, ClaudeProfile},
    id::ProjectScope,
};

use super::{DiagnosticProbeError, Endpoint, ReachabilityPrepareError, ReachabilityProbe, probe};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    app_providers::AppProviderStore,
    gateway::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticJobs},
    global_database::GlobalDatabase,
    model_relay::upstream::upstream_http_client_builder,
};

async fn request_line(socket: &mut TcpStream) -> String {
    let mut request = Vec::new();
    while !request.ends_with(b"\r\n\r\n") {
        request.push(socket.read_u8().await.unwrap());
        assert!(request.len() < 8192);
    }
    String::from_utf8(request).unwrap()
}

#[tokio::test]
async fn diagnostic_reachability_does_not_change_breaker() {
    let directory = tempfile::tempdir().unwrap();
    let jobs = DiagnosticJobs::open(directory.path()).unwrap();
    let owner = DiagnosticJobOwner {
        scope: DiagnosticScope {
            project: ProjectScope::for_default_tenant("project".into()).unwrap(),
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    let policy = CircuitBreakerPolicy {
        failure_threshold: 2,
        success_threshold: 1,
        cooldown_secs: 60,
        error_rate_threshold: 1.0,
        min_requests: 5,
    };
    let mut breaker = CircuitBreaker::default();
    breaker.record(AttemptOutcome::ProviderFailure, &policy, Instant::now());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/exact/path/", listener.local_addr().unwrap());
    let (seen, mut requests) = mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut sockets = Vec::new();
        for status in [200, 302, 401, 500] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = request_line(&mut socket).await;
            assert!(request.starts_with("GET /exact/path/ HTTP/1.1\r\n"));
            assert!(!request.to_ascii_lowercase().contains("authorization:"));
            assert!(!request.contains("fixture-key"));
            assert!(request.to_ascii_lowercase().contains("user-agent: fixture-agent"));
            seen.send(status).unwrap();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status} Fixture\r\nLocation: /redirect\r\nContent-Length: 10\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            // Keep each response body pending until every header-only probe finishes.
            sockets.push(socket);
        }
        pending::<()>().await;
    });
    let client =
        upstream_http_client_builder().retry(reqwest::retry::never()).no_proxy().build().unwrap();
    let prepared = tokio::task::spawn_blocking({
        let home = directory.path().to_owned();
        let owner = owner.clone();
        move || {
            let database = GlobalDatabase::for_home(&home).unwrap();
            let credentials = home.join("credentials");
            std::fs::DirBuilder::new().mode(0o700).create(&credentials).unwrap();
            let accounts = AuthAccountStore::new(database.clone(), &credentials).unwrap();
            let store = AppProviderStore::new(database.clone()).unwrap();
            let tenant = owner.scope.project.tenant_scope();
            let profile = ClaudeProfile {
                name: "Controlled provider".into(),
                model: None,
                config: ClaudeConfig::ThirdParty {
                    base_url: endpoint,
                    api_key: "fixture-key".into(),
                    api_key_header: true,
                },
            };
            store
                .claude_save_with_metadata(
                    tenant,
                    "dynamic",
                    &profile,
                    None,
                    true,
                    Some(serde_json::json!({"meta": {"providerType": "github_copilot"}})),
                )
                .unwrap();
            assert!(
                matches!(
                    ReachabilityProbe::prepare(
                        &database,
                        &accounts,
                        owner.clone(),
                        "dynamic".into()
                    ),
                    Err(ReachabilityPrepareError::Account(AuthErrorCode::NotFound)),
                ),
                "a dynamic Copilot provider requires a scoped account"
            );
            store
                .claude_save_with_metadata(
                    tenant,
                    "provider",
                    &profile,
                    None,
                    true,
                    Some(serde_json::json!({
                        "meta": {
                            "providerType": "github_copilot",
                            "isFullUrl": true,
                            "customUserAgent": "fixture-agent"
                        }
                    })),
                )
                .unwrap();
            ReachabilityProbe::prepare(&database, &accounts, owner, "provider".into()).unwrap()
        }
    })
    .await
    .unwrap();
    let Endpoint::Configured(endpoint) = prepared.endpoint else {
        panic!("a full URL must bypass Copilot endpoint discovery");
    };
    for status in [200, 302, 401, 500] {
        let id = tokio::task::spawn_blocking({
            let jobs = jobs.clone();
            let owner = owner.clone();
            let client = client.clone();
            let endpoint = endpoint.clone();
            let user_agent = prepared.user_agent.clone();
            move || {
                let run = |cancel| async move {
                    probe(
                        &client,
                        "provider".into(),
                        endpoint,
                        user_agent,
                        ControlledFixture,
                        Duration::from_secs(8),
                        cancel,
                    )
                    .await
                };
                jobs.start(owner, DiagnosticProbeKind::Reachability, vec![run]).unwrap().task_id
            }
        })
        .await
        .unwrap();
        let snapshot = timeout(Duration::from_secs(3), async {
            loop {
                let snapshot = jobs.snapshot(&owner, &id).unwrap();
                if matches!(snapshot.state, DiagnosticJobState::Succeeded {}) {
                    break snapshot;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("header-only probe must complete while the body is pending");
        let results = snapshot.results.unwrap();
        let [DiagnosticResult::Reachability { report, .. }] = results.as_slice() else {
            panic!("expected reachability evidence, got {results:?}");
        };
        assert_eq!(report.source, ControlledFixture);
        assert_eq!(
            report.outcome,
            DiagnosticOutcome::Succeeded {
                evidence: ReachabilityEvidence { http_status: status, slow: false },
            },
            "any HTTP response establishes reachability"
        );
        assert_eq!(requests.try_recv().unwrap(), status);
        assert_eq!(breaker.state(), FailoverHealthState::Closed);
    }

    let refused = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", refused.local_addr().unwrap()).parse().unwrap();
    drop(refused);
    assert_failure(&client, endpoint, DiagnosticFailureCode::Transport).await;

    let tls = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("https://{}/", tls.local_addr().unwrap()).parse().unwrap();
    let handshake = tokio::spawn(async move {
        let (mut socket, _) = tls.accept().await.unwrap();
        assert_eq!(socket.read_u8().await.unwrap(), 22);
        // A fatal handshake_failure TLS alert.
        socket.write_all(&[21, 3, 3, 0, 2, 2, 40]).await.unwrap();
    });
    assert_failure(&client, endpoint, DiagnosticFailureCode::Transport).await;
    handshake.await.unwrap();

    let stalled = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint =
        format!("http://{}/", stalled.local_addr().unwrap()).parse::<reqwest::Url>().unwrap();
    let (entered, mut attempts) = mpsc::unbounded_channel();
    let stalled = tokio::spawn(async move {
        let mut sockets = Vec::new();
        loop {
            let (mut socket, _) = stalled.accept().await.unwrap();
            request_line(&mut socket).await;
            entered.send(()).unwrap();
            sockets.push(socket);
        }
    });
    assert_failure(&client, endpoint.clone(), DiagnosticFailureCode::Timeout).await;
    attempts.try_recv().unwrap();
    attempts.try_recv().unwrap();
    assert!(attempts.try_recv().is_err(), "only one timeout retry is allowed");

    let cancel = CancellationToken::new();
    let active = tokio::spawn({
        let (client, cancel) = (client.clone(), cancel.clone());
        async move {
            probe(
                &client,
                "provider".into(),
                endpoint,
                None,
                ControlledFixture,
                Duration::from_secs(8),
                cancel,
            )
            .await
        }
    });
    timeout(Duration::from_secs(3), attempts.recv()).await.unwrap().unwrap();
    cancel.cancel();
    assert!(matches!(
        timeout(Duration::from_secs(1), active).await.unwrap().unwrap(),
        Err(DiagnosticProbeError::Cancelled)
    ));

    assert_eq!(breaker.state(), FailoverHealthState::Closed);
    breaker.record(AttemptOutcome::ProviderFailure, &policy, Instant::now());
    assert_eq!(
        breaker.state(),
        FailoverHealthState::Open,
        "diagnostic successes must not reset the production failure streak"
    );
    for task in [server, stalled] {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
}

async fn assert_failure(
    client: &reqwest::Client,
    endpoint: reqwest::Url,
    code: DiagnosticFailureCode,
) {
    let result = probe(
        client,
        "provider".into(),
        endpoint,
        None,
        ControlledFixture,
        Duration::from_millis(200),
        CancellationToken::new(),
    )
    .await;
    let Ok(DiagnosticResult::Reachability { report, .. }) = result else {
        panic!("expected a reachability failure report");
    };
    assert_eq!(report.timings.len(), if code == DiagnosticFailureCode::Timeout { 2 } else { 1 });
    assert_eq!(
        report.outcome,
        DiagnosticOutcome::Failed {
            error: DiagnosticFailure { phase: DiagnosticPhase::ResponseHeaders, code },
            http_status: None,
        }
    );
}
