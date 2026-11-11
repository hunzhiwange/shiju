use super::{
    lifecycle_api_tests::{
        HOST, MANAGER, READER, assert_error, client, spawn_gateway, write_policy,
    },
    outbound_scan::{CANDIDATES, scan_connection},
};
use crate::gateway::agent_apps_diagnostic_jobs::DiagnosticProbeError;
use serde_json::{Value, json};
use std::{fs, future::pending, time::Duration};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    time::{Instant, sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticFailureCode, DiagnosticJobState, DiagnosticOutcome, DiagnosticResult,
    },
};

#[tokio::test]
async fn cc_route_030_outbound_scan_expected_behavior() {
    assert_eq!(
        CANDIDATES.iter().map(|(port, _)| *port).collect::<Vec<_>>(),
        [7890, 7891, 1080, 8080, 8888, 3128, 10808, 10809]
    );
    let home = tempfile::tempdir().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let raw = reqwest::Client::builder().no_proxy().build().unwrap();
    let start = |body: Value| {
        raw.post(format!(
        "http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}/projects/fixture/apps/codex/egress/scans"
    )).header("x-platform-id","p").header("x-company-id","owner")
        .header("x-agent-apps-token",MANAGER).json(&body).send()
    };
    assert_eq!(start(json!({})).await.unwrap().status(), 403);
    let mut access: Value = serde_json::from_slice(&fs::read(&policy).unwrap()).unwrap();
    for index in [0, 1] {
        access["principals"][index]["grants"][0]["routing_apps"] =
            json!([{"project_id":"fixture","app_id":"codex"}]);
    }
    fs::write(&policy, access.to_string()).unwrap();
    let owner = client(port, "owner");
    let project = "fixture".into();
    assert_error(
        owner
            .app_routing_start_outbound_scan(HOST, &project, AppId::Codex, READER)
            .await
            .unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        client(port, "other")
            .app_routing_start_outbound_scan(HOST, &project, AppId::Codex, MANAGER)
            .await
            .unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        owner
            .app_routing_start_outbound_scan("other-mac", &project, AppId::Codex, MANAGER)
            .await
            .unwrap_err(),
        403,
        "forbidden",
    );
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
    let job: vw_api_types::agent_apps_diagnostics::DiagnosticJobIdentity =
        response.json().await.unwrap();
    assert_error(
        owner
            .app_routing_outbound_probe_status(HOST, &project, AppId::Codex, &job.task_id, MANAGER)
            .await
            .unwrap_err(),
        404,
        "not_found",
    );
    let snapshot = timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = owner
                .app_routing_outbound_scan_status(
                    HOST,
                    &project,
                    AppId::Codex,
                    &job.task_id,
                    MANAGER,
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
        owner.app_routing_start_outbound_scan(HOST, &project, AppId::Codex, MANAGER).await.unwrap();
    let cancelled = owner
        .app_routing_cancel_outbound_scan(HOST, &project, AppId::Codex, &job.task_id, MANAGER)
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
