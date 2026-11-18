use super::fixture::{HOST, assert_error, client, spawn_gateway, write_policy};
use agent_gateway::agent_apps_diagnostic_jobs::DiagnosticProbeError;
use serde_json::{Value, json};
use std::{fs, future::pending, os::unix::fs::PermissionsExt, process::Command, time::Duration};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    time::{Instant, sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use vw_api_app_diagnostics::DiagnosticFailureCode;
use vw_api_app_diagnostics::DiagnosticJobState;
use vw_api_app_diagnostics::DiagnosticOutcome;
use vw_api_app_diagnostics::DiagnosticResult;

use vw_api_app_identity::AppId;

#[tokio::test]
async fn cc_route_030_outbound_scan_expected_behavior() {
    assert_eq!(
        CANDIDATES.iter().map(|(port, _)| *port).collect::<Vec<_>>(),
        [7890, 7891, 1080, 8080, 8888, 3128, 10808, 10809]
    );
    const CHILD: &str = "SHIJU_OUTBOUND_SCAN_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
            .current_dir(&path)
            .env_clear()
            .env(CHILD, &path)
            .env("HOME", &path)
            .env("VIBEWINDOW_TEST_ROOT", &path)
            .env("CODEX_HOME", path.join(".codex"))
            .env("XDG_CONFIG_HOME", path.join(".config"))
            .env("XDG_DATA_HOME", path.join(".local/share"))
            .env("XDG_STATE_HOME", path.join(".local/state"))
            .env("XDG_CACHE_HOME", path.join(".cache"))
            .env("TMPDIR", &path)
            .status()
            .unwrap();
        assert!(status.success(), "isolated scan test failed: {status}");
        return;
    }
    let home = std::path::PathBuf::from(std::env::var_os(CHILD).unwrap());
    assert_eq!(std::env::current_dir().unwrap().canonicalize().unwrap(), home);
    assert_eq!(std::env::var_os("VIBEWINDOW_TEST_ROOT").unwrap(), home.as_os_str());
    assert_eq!(agent_paths::paths().home, home);
    assert_eq!(std::env::var_os("CODEX_HOME").unwrap(), home.join(".codex").as_os_str());
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir_all(home.join(".local/state/shiju")).unwrap();
    let policy = home.join("access.json");
    write_policy(&policy, &home, 0);
    let (_gateway, port) = spawn_gateway(&home, &policy).await;
    let raw = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let start = |body: Value| {
        raw.post(format!(
            "http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}/projects/fixture/apps/codex/egress/scans"
        ))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner")
        .json(&body)
        .send()
    };
    let owner = client(port, "owner");
    let project = "fixture".into();
    for body in [
        json!({"ip":"10.0.0.1"}),
        json!({"ports":[22]}),
        json!({"url":"http://example.org"}),
        json!({"timeoutMs":99999}),
    ] {
        assert_eq!(start(body).await.unwrap().status(), 400);
    }
    let response = start(json!({})).await.unwrap();
    assert_eq!(response.status(), 202);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let job: vw_api_app_diagnostics::DiagnosticJobIdentity =
        response.json().await.unwrap();
    assert_error(
        crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_outbound_probe_status(HOST, &project, AppId::Codex, &job.task_id, "")
            .await
            .unwrap_err(),
        404,
        "not_found",
    );
    // The registry still isolates existing tasks by tenant, project, host and app.
    for (tenant, host, project, app) in [
        ("other", HOST, "fixture", AppId::Codex),
        ("owner", "other-mac", "fixture", AppId::Codex),
        ("owner", HOST, "other", AppId::Codex),
        ("owner", HOST, "fixture", AppId::Claude),
    ] {
        let routing = crate::GatewayAppRoutingClient::new(client(port, tenant));
        assert_error(
            routing
                .app_routing_outbound_scan_status(host, &project.into(), app, &job.task_id, "")
                .await
                .unwrap_err(),
            404,
            "not_found",
        );
    }
    let snapshot = timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_outbound_scan_status(
                    HOST,
                    &project,
                    AppId::Codex,
                    &job.task_id,
                    "",
                )
                .await
                .unwrap();
            if snapshot.state == (DiagnosticJobState::Succeeded {}) {
                break snapshot;
            }
            assert!(matches!(
                snapshot.state,
                DiagnosticJobState::Running {} | DiagnosticJobState::Queued {}
            ));
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(snapshot.progress.completed, 8);
    assert_eq!(snapshot.progress.total, Some(8));
    let mut ports = Vec::new();
    for result in snapshot.results.unwrap() {
        let DiagnosticResult::OutboundScan { report } = result else { panic!("wrong task result") };
        let DiagnosticOutcome::Succeeded { evidence } = report.outcome else {
            panic!("scan failed")
        };
        ports.push(evidence.port);
        assert_eq!(evidence.protocol_candidates.is_empty(), !evidence.tcp_open);
    }
    ports.sort_unstable();
    assert_eq!(ports, [1080, 3128, 7890, 7891, 8080, 8888, 10808, 10809]);
    // Cancellation remains safe even if the short scan already finished.
    let job =
        crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_start_outbound_scan(HOST, &project, AppId::Codex, "").await.unwrap();
    let cancelled = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_cancel_outbound_scan(HOST, &project, AppId::Codex, &job.task_id, "")
        .await
        .unwrap();
    assert!(matches!(
        cancelled.state,
        DiagnosticJobState::Cancelling {}
            | DiagnosticJobState::Cancelled {}
            | DiagnosticJobState::Succeeded {}
    ));
}

#[tokio::test]
async fn cc_route_030_tcp_candidate_closes_without_protocol_claim() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let connect = TcpStream::connect(listener.local_addr().unwrap());
    let result = scan_connection(
        connect,
        7890,
        &["http", "socks5"],
        Instant::now() + Duration::from_secs(2),
        CancellationToken::new(),
    )
    .await
    .unwrap_or_else(|_| panic!("scan failed"));
    let (mut socket, _) = listener.accept().await.unwrap();
    assert_eq!(timeout(Duration::from_secs(1), socket.read(&mut [0])).await.unwrap().unwrap(), 0);
    let DiagnosticResult::OutboundScan { report } = result else { panic!("wrong result") };
    assert!(
        matches!(report.outcome,DiagnosticOutcome::Succeeded {evidence} if evidence.tcp_open && evidence.protocol_candidates == ["http","socks5"])
    );
}

#[tokio::test]
async fn cc_route_030_cancel_and_deadlines_stop_pending_io() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = scan_connection(
        pending(),
        7890,
        &["http"],
        Instant::now() + Duration::from_secs(2),
        cancel,
    )
    .await;
    assert!(matches!(result, Err(DiagnosticProbeError::Cancelled)));
    let result =
        scan_connection(pending(), 7890, &["http"], Instant::now(), CancellationToken::new()).await;
    assert!(
        matches!(result,Err(DiagnosticProbeError::Failed(error)) if error.code == DiagnosticFailureCode::Timeout)
    );
    let result = scan_connection(
        pending(),
        7890,
        &["http"],
        Instant::now() + Duration::from_millis(10),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(result,Err(DiagnosticProbeError::Failed(error)) if error.code == DiagnosticFailureCode::Timeout)
    );
    let result = scan_connection(
        pending(),
        7890,
        &["http"],
        Instant::now() + Duration::from_secs(2),
        CancellationToken::new(),
    )
    .await
    .unwrap_or_else(|_| panic!("port budget must yield a closed candidate"));
    let DiagnosticResult::OutboundScan { report } = result else { panic!("wrong result") };
    assert!(
        matches!(report.outcome,DiagnosticOutcome::Succeeded {evidence} if !evidence.tcp_open && evidence.protocol_candidates.is_empty())
    );
}

#[tokio::test]
async fn cc_route_030_cancel_drops_inflight_connection_before_return() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let guard = Guard(dropped.clone());
    let cancel = CancellationToken::new();
    let connection = async move {
        let _guard = guard;
        pending().await
    };
    let (result, ()) = tokio::join!(
        scan_connection(
            connection,
            7890,
            &["http"],
            Instant::now() + Duration::from_secs(2),
            cancel.clone()
        ),
        async {
            sleep(Duration::from_millis(1)).await;
            cancel.cancel();
        }
    );
    assert!(matches!(result, Err(DiagnosticProbeError::Cancelled)));
    assert!(dropped.load(Ordering::SeqCst));
}

use crate::server::outbound_scan::{CANDIDATES, scan_connection};
