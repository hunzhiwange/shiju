use super::fixture::{HOST, MANAGER, assert_error, spawn_gateway_with_pairing, write_policy};
use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Instant, sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use {
    vw_api_app_identity::AppId,
    vw_api_app_diagnostics::DiagnosticFailureCode,
    vw_api_app_diagnostics::DiagnosticJobSnapshot,
    vw_api_app_diagnostics::DiagnosticJobState,
    vw_api_app_diagnostics::DiagnosticOutcome,
    vw_api_app_diagnostics::DiagnosticProbeKind,
    vw_api_app_diagnostics::DiagnosticResult,
    vw_api_app_routing::RoutingEgressMode,
    vw_api_app_routing::RoutingEgressUpdate,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::TaskId,
    vw_api_transport::id::TenantScope,
};
use gateway_http::GatewayTransport;

const APP: AppId = AppId::Codex;
const SKEY: &str = "fixture-routing-gateway-skey-00001";

fn paired_client(port: u16, company: &str) -> GatewayTransport {
    GatewayTransport::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", company).unwrap())
            .with_auth(GatewayAuth { skey: Some(SKEY.to_owned()) }),
    )
    .unwrap()
}
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
    assert!(!headers.lines().filter_map(|line| line.split_once(':')).any(|(name, _)| {
        name.eq_ignore_ascii_case("authorization")
    }));
    assert!(!headers.contains(SKEY));
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
    client: &GatewayTransport,
    project: &ProjectId,
    task: &TaskId,
    budget: Duration,
) -> DiagnosticJobSnapshot {
    timeout(budget, async {
        loop {
            let snapshot = crate::GatewayAppRoutingClient::new(client.clone()).app_routing_outbound_probe_status(HOST, project, APP, task, "")
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
    const CHILD_ROOT: &str = "SHIJU_OUTBOUND_PROBE_CHILD_ROOT";
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        let root = tempfile::tempdir().unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "host_migration_7_tests::outbound_probe_tests::cc_route_029_outbound_probe_expected_behavior",
                "--nocapture",
            ])
            .env(CHILD_ROOT, root.path())
            .env("VIBEWINDOW_TEST_ROOT", root.path())
            .env_remove("VIBEWINDOW_TEST_HOME")
            .output()
            .unwrap();
        eprint!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "isolated probe test failed: {}", output.status);
        return;
    };
    let home = PathBuf::from(root);
    assert!(home.is_absolute() && home.is_dir());
    assert_eq!(std::env::var_os("VIBEWINDOW_TEST_ROOT").as_deref(), Some(home.as_os_str()));
    let paths = agent_paths::paths();
    assert_eq!(paths.home, home);
    assert!(paths.data.starts_with(&home) && paths.config.starts_with(&home) && paths.state.starts_with(&home));

    let policy = home.join("access.json");
    write_policy(&policy, &home, 0);
    // The fixture uses its explicit routing state path, independent of the runtime profile.
    let state = home.join(".local/state/shiju");
    fs::create_dir_all(&state).unwrap();
    let (_gateway, port) = spawn_gateway_with_pairing(&home, &policy, Some(&[SKEY.to_owned()])).await;
    assert!(state.join("agent-app-diagnostics/diagnostics.lock").is_file());

    let raw = reqwest::Client::builder().no_proxy().timeout(GUARD).build().unwrap();
    let probes = format!(
        "http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}/projects/fixture/apps/codex/egress/probes"
    );
    let start = |body: Value| {
        raw.post(&probes)
            .header("x-platform-id", "p")
            .header("x-company-id", "owner")
            .json(&body)
    };
    for request in [
        start(json!({})),
        start(json!({})).bearer_auth("invalid-skey"),
        start(json!({})).header("x-agent-apps-token", MANAGER),
    ] {
        assert_eq!(request.send().await.unwrap().status(), 401);
    }

    let owner = paired_client(port, "owner");
    let other_tenant = crate::GatewayAppRoutingClient::new(paired_client(port, "other"));
    let project = ProjectId::from("fixture");
    let direct = RoutingEgressUpdate { mode: RoutingEgressMode::Direct, proxy_url: None };
    crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_set_egress(HOST, "", &direct).await.unwrap();
    let job = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_start_outbound_probe(HOST, &project, APP, "").await.unwrap();
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
    crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_set_egress(HOST, "", &custom).await.unwrap();
    for body in [
        json!({"url": "http://127.0.0.1:9"}),
        json!({"proxyUrl": "http://127.0.0.1:9"}),
        json!({"timeoutMs": 60000}),
    ] {
        let response = start(body).bearer_auth(SKEY).send().await.unwrap();
        assert_eq!(response.status(), 400);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    assert!(timeout(Duration::from_millis(50), proxy.accept()).await.is_err());

    let response = start(json!({})).bearer_auth(SKEY).send().await.unwrap();
    assert_eq!(response.status(), 202);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let job: vw_api_app_diagnostics::DiagnosticJobIdentity =
        response.json().await.unwrap();
    assert_eq!(job.kind, DiagnosticProbeKind::OutboundProxy);
    let mut socket = accept_connect(&proxy).await;
    let job_url = format!("{probes}/{}", job.task_id.as_ref());
    for method in [reqwest::Method::GET, reqwest::Method::DELETE] {
        let response = raw.request(method, &job_url)
            .header("x-platform-id", "p")
            .header("x-company-id", "owner")
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
    }
    let owner_api = crate::GatewayAppRoutingClient::new(owner.clone());
    let other_project = ProjectId::from("other");
    for (api, host, project, app) in [
        (&other_tenant, HOST, &project, APP),
        (&owner_api, "other-mac", &project, APP),
        (&owner_api, HOST, &other_project, APP),
        (&owner_api, HOST, &project, AppId::Claude),
    ] {
        assert_error(api.app_routing_outbound_probe_status(host, project, app, &job.task_id, "")
            .await.unwrap_err(), 404, "not_found");
        assert_error(api.app_routing_cancel_outbound_probe(host, project, app, &job.task_id, "")
            .await.unwrap_err(), 404, "not_found");
    }
    let cancelled = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_cancel_outbound_probe(HOST, &project, APP, &job.task_id, "")
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
        crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_cancel_outbound_probe(HOST, &project, APP, &job.task_id, "")
            .await
            .unwrap(),
        snapshot
    );

    let job = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_start_outbound_probe(HOST, &project, APP, "").await.unwrap();
    let mut socket = accept_connect(&proxy).await;
    let snapshot = terminal(&owner, &project, &job.task_id, Duration::from_secs(13)).await;
    assert!(matches!(
        snapshot.state,
        DiagnosticJobState::Failed { error } if error.code == DiagnosticFailureCode::Timeout
    ));
    assert_closed(&mut socket).await;
    assert!(timeout(Duration::from_millis(50), proxy.accept()).await.is_err());

    let job = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_start_outbound_probe(HOST, &project, APP, "").await.unwrap();
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
    assert!(!wire.contains("fixture:test") && !wire.contains("Zml4dHVyZTp0ZXN0") && !wire.contains(SKEY));
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
    let client = model_http_client::upstream_http_client_builder_without_total_timeout()
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

use crate::server::outbound_probe::probe;
