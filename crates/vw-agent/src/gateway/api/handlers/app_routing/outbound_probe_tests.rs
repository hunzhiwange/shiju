use super::{
    lifecycle_api_tests::{
        HOST, MANAGER, READER, assert_error, client, spawn_gateway, write_policy,
    },
    outbound_probe::probe,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Instant, sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticFailureCode, DiagnosticJobSnapshot, DiagnosticJobState, DiagnosticOutcome,
        DiagnosticProbeKind, DiagnosticResult,
    },
    app_routing::{RoutingEgressMode, RoutingEgressUpdate},
    id::{ProjectId, TaskId},
};
use vw_gateway_client::GatewayClient;

const APP: AppId = AppId::Codex;
const PEER: &str = "fixture-routing-peer-token-00000001";
const GUARD: Duration = Duration::from_secs(5);

async fn read_headers(socket: &mut TcpStream) -> String {
    timeout(GUARD, async {
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            assert!(headers.len() < 4096);
            headers.push(socket.read_u8().await.unwrap());
        }
        String::from_utf8(headers).unwrap()
    })
    .await
    .unwrap()
}

async fn accept_connect(proxy: &TcpListener) -> TcpStream {
    let (mut socket, _) = timeout(GUARD, proxy.accept()).await.unwrap().unwrap();
    let headers = read_headers(&mut socket).await;
    assert!(headers.starts_with("CONNECT www.shiju.cloud:443 HTTP/1.1\r\n"));
    assert!(headers.lines().filter_map(|line| line.split_once(':')).any(|(name, value)| {
        name.eq_ignore_ascii_case("proxy-authorization") && value.trim() == "Basic Zml4dHVyZTp0ZXN0"
    }));
    assert!(!headers.to_ascii_lowercase().contains("x-agent-apps-token"));
    socket
}

async fn assert_closed(socket: &mut TcpStream) {
    let result = timeout(GUARD, socket.read(&mut [0_u8])).await.unwrap();
    match result {
        Ok(size) => assert_eq!(size, 0),
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset),
    }
}

async fn terminal(
    client: &GatewayClient,
    project: &ProjectId,
    task: &TaskId,
    budget: Duration,
) -> DiagnosticJobSnapshot {
    timeout(budget, async {
        loop {
            let snapshot = client
                .app_routing_outbound_probe_status(HOST, project, APP, task, MANAGER)
                .await
                .unwrap();
            match snapshot.state {
                DiagnosticJobState::Queued {}
                | DiagnosticJobState::Running {}
                | DiagnosticJobState::Cancelling {} => sleep(Duration::from_millis(25)).await,
                _ => return snapshot,
            }
        }
    })
    .await
    .expect("the probe never published a terminal state")
}

#[tokio::test]
async fn cc_route_029_outbound_probe_expected_behavior() {
    let home = tempfile::tempdir().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;

    let raw = reqwest::Client::builder().no_proxy().timeout(GUARD).build().unwrap();
    let start = |body: Value| {
        raw.post(format!(
            "http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}/projects/fixture/apps/codex/egress/probes"
        ))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner")
        .header("x-agent-apps-token", MANAGER)
        .json(&body)
        .send()
    };
    let response = start(json!({})).await.unwrap();
    assert_eq!(response.status(), 403, "host management alone must not authorize a project probe");

    let mut access: Value = serde_json::from_slice(&fs::read(&policy).unwrap()).unwrap();
    for index in [0, 1] {
        access["principals"][index]["grants"][0]["routing_apps"] = json!([
            {"project_id": "fixture", "app_id": "codex"},
            {"project_id": "other", "app_id": "codex"}
        ]);
    }
    let mut peer = access["principals"][0].clone();
    peer["id"] = json!("peer");
    peer["token_sha256"] = json!(hex::encode(Sha256::digest(PEER)));
    access["principals"].as_array_mut().unwrap().push(peer);
    fs::write(&policy, access.to_string()).unwrap();

    let owner = client(port, "owner");
    let project = ProjectId::from("fixture");
    assert_error(
        owner.app_routing_start_outbound_probe(HOST, &project, APP, READER).await.unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        client(port, "other")
            .app_routing_start_outbound_probe(HOST, &project, APP, MANAGER)
            .await
            .unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        owner
            .app_routing_start_outbound_probe("other-mac", &project, APP, MANAGER)
            .await
            .unwrap_err(),
        403,
        "forbidden",
    );
    let direct = RoutingEgressUpdate { mode: RoutingEgressMode::Direct, proxy_url: None };
    owner.app_routing_set_egress(HOST, MANAGER, &direct).await.unwrap();
    let job = owner.app_routing_start_outbound_probe(HOST, &project, APP, MANAGER).await.unwrap();
    let snapshot = terminal(&owner, &project, &job.task_id, GUARD).await;
    assert!(matches!(
        snapshot.state,
        DiagnosticJobState::Failed { error } if error.code == DiagnosticFailureCode::Unsupported
    ));

    let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let custom = RoutingEgressUpdate {
        mode: RoutingEgressMode::Custom,
        proxy_url: Some(format!("http://fixture:test@{}", proxy.local_addr().unwrap())),
    };
    owner.app_routing_set_egress(HOST, MANAGER, &custom).await.unwrap();
    for body in [
        json!({"url": "http://127.0.0.1:9"}),
        json!({"proxyUrl": "http://127.0.0.1:9"}),
        json!({"timeoutMs": 60000}),
    ] {
        let response = start(body).await.unwrap();
        assert_eq!(response.status(), 400);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    assert!(timeout(Duration::from_millis(50), proxy.accept()).await.is_err());

    let response = start(json!({})).await.unwrap();
    assert_eq!(response.status(), 202);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let job: vw_api_types::agent_apps_diagnostics::DiagnosticJobIdentity =
        response.json().await.unwrap();
    assert_eq!(job.kind, DiagnosticProbeKind::OutboundProxy);
    let mut socket = accept_connect(&proxy).await;
    assert_error(
        owner
            .app_routing_cancel_outbound_probe(HOST, &project, APP, &job.task_id, PEER)
            .await
            .unwrap_err(),
        404,
        "not_found",
    );
    assert_error(
        owner
            .app_routing_outbound_probe_status(HOST, &"other".into(), APP, &job.task_id, MANAGER)
            .await
            .unwrap_err(),
        404,
        "not_found",
    );
    let cancelled = owner
        .app_routing_cancel_outbound_probe(HOST, &project, APP, &job.task_id, MANAGER)
        .await
        .unwrap();
    assert!(matches!(
        cancelled.state,
        DiagnosticJobState::Cancelling {} | DiagnosticJobState::Cancelled {}
    ));
    assert_closed(&mut socket).await;
    let snapshot = terminal(&owner, &project, &job.task_id, GUARD).await;
    assert_eq!(snapshot.state, DiagnosticJobState::Cancelled {});
    assert_eq!(snapshot.results, Some(vec![]));
    assert_eq!(
        owner
            .app_routing_cancel_outbound_probe(HOST, &project, APP, &job.task_id, MANAGER)
            .await
            .unwrap(),
        snapshot
    );

    let job = owner.app_routing_start_outbound_probe(HOST, &project, APP, MANAGER).await.unwrap();
    let mut socket = accept_connect(&proxy).await;
    let snapshot = terminal(&owner, &project, &job.task_id, Duration::from_secs(13)).await;
    assert!(matches!(
        snapshot.state,
        DiagnosticJobState::Failed { error } if error.code == DiagnosticFailureCode::Timeout
    ));
    assert_closed(&mut socket).await;
    assert!(timeout(Duration::from_millis(50), proxy.accept()).await.is_err());

    let job = owner.app_routing_start_outbound_probe(HOST, &project, APP, MANAGER).await.unwrap();
    let mut socket = accept_connect(&proxy).await;
    socket
        .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n")
        .await
        .unwrap();
    let snapshot = terminal(&owner, &project, &job.task_id, GUARD).await;
    assert!(matches!(
        snapshot.state,
        DiagnosticJobState::Failed { error } if error.code == DiagnosticFailureCode::Transport
    ));
    assert_closed(&mut socket).await;
    let wire = serde_json::to_string(&snapshot).unwrap();
    assert!(!wire.contains("fixture:test") && !wire.contains("Zml4dHVyZTp0ZXN0"));
}

#[tokio::test]
async fn cc_route_029_probe_reports_headers_without_following_redirects() {
    let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/manifest", server.local_addr().unwrap()).parse().unwrap();
    let fixture = async {
        let (mut socket, _) = timeout(GUARD, server.accept()).await.unwrap().unwrap();
        assert!(read_headers(&mut socket).await.starts_with("HEAD /manifest HTTP/1.1\r\n"));
        socket.write_all(
            b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/unapproved\r\nContent-Length: 0\r\n\r\n"
        ).await.unwrap();
    };
    let client = crate::model_relay::upstream::upstream_http_client_builder_without_total_timeout()
        .no_proxy()
        .build()
        .unwrap();
    let (result, ()) = tokio::join!(
        probe(client, target, Instant::now() + GUARD, CancellationToken::new()),
        fixture
    );
    let Ok(DiagnosticResult::OutboundProxy { report }) = result else {
        panic!("expected a proxy reachability report");
    };
    assert!(matches!(
        report.outcome,
        DiagnosticOutcome::Succeeded { evidence } if evidence.http_status == 302
    ));
    assert_eq!(report.timings.len(), 1);
}
