#![cfg(target_os = "macos")]

use serde_json::{Value, json};
use gateway_pairing::pairing::hash_skey;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::time::{sleep, timeout};
use {
    vw_api_app_identity::AppId,
    vw_api_app_diagnostics::DiagnosticJobIdentity,
    vw_api_app_diagnostics::DiagnosticJobSnapshot,
    vw_api_app_diagnostics::DiagnosticJobState,
    vw_api_app_diagnostics::DiagnosticOutcome,
    vw_api_app_diagnostics::DiagnosticResult,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::TaskId,
    vw_api_transport::id::TenantScope,
};
use {gateway_endpoint::{GatewayAuth, GatewayEndpoint}, {gateway_app_routing::AppRoutingError, vw_gateway_client::GatewayClient}};

const HOST: &str = "routing-mac";
const SKEY: &str = "fixture-outbound-scan-paired-gateway-key";
const APP: AppId = AppId::Codex;

#[tokio::test]
#[ignore = "started by the scan integration test with a private HOME and policy"]
async fn gateway_child() {
    let home = std::env::current_dir().unwrap().canonicalize().unwrap();
    assert_eq!(std::env::var_os("VIBEWINDOW_TEST_ROOT").unwrap(), home.as_os_str());
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut config = vw_agent::Config {
        workspace_dir: home.join("workspace"),
        config_path: home.join("config.toml"),
        ..Default::default()
    };
    assert_eq!(agent_paths::paths().home, home);
    assert_eq!(std::env::var_os("CODEX_HOME").unwrap(), home.join(".codex").as_os_str());
    config.gateway.auth_enabled = true;
    vw_agent::config::replace_gateway_skeys_for_tenant(
        &TenantScope::default(),
        &[config_gateway::GatewaySkey {
            enabled: true,
            skey: None,
            skey_hash: hash_skey(SKEY),
            masked_skey: String::new(),
            name: "scan fixture".into(),
            expires_at: None,
        }],
    )
    .await
    .unwrap();
    fs::create_dir_all(&config.workspace_dir).unwrap();
    vw_agent::gateway::run_gateway("127.0.0.1", 0, config).await.unwrap();
}

struct GatewayProcess(Child);
impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn client(port: u16, company: &str) -> GatewayClient {
    GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", company).unwrap())
            .with_auth(GatewayAuth { skey: Some(SKEY.into()) }),
    )
    .unwrap()
}

fn assert_error(error: AppRoutingError, status: u16, code: &str) {
    assert!(
        matches!(&error, AppRoutingError::Http { status: actual, body: Some(body) }
        if *actual == status && body.error.code == code),
        "unexpected error: {error:?}"
    );
}

async fn terminal(
    client: &GatewayClient,
    project: &ProjectId,
    task: &TaskId,
) -> DiagnosticJobSnapshot {
    timeout(Duration::from_secs(8), async {
        loop {
            let snapshot = client
                .app_routing_outbound_scan_status(HOST, project, APP, task, "")
                .await
                .unwrap();
            if !matches!(
                snapshot.state,
                DiagnosticJobState::Queued {}
                    | DiagnosticJobState::Running {}
                    | DiagnosticJobState::Cancelling {}
            ) {
                return snapshot;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("scan must finish or finish cancellation within its bounded lifetime")
}

#[tokio::test]
async fn cc_route_030_production_gateway_preserves_fixed_candidates_and_job_boundaries() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let root = tempfile::tempdir().unwrap();
    let home = root.path().canonicalize().unwrap();
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["credentials", "Applications"] {
        let directory = home.join(name);
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let policy = home.join("access.json");
    // Routing startup still reads the policy to configure its local listener.
    let principals = json!([{
        "id":"scan-fixture",
        "token_sha256":hash_skey("unused-scan-routing-policy-fixture"),
        "grants":[{"platform_id":"p","company_id":"owner","host_id":HOST,
            "routing_read":true,"routing_manage":true}]
    }]);
    let host = |id| {
        json!({"id":id,"online":true,"binding":"local","home":&home,
        "search_paths":[],"applications":home.join("Applications"),"credential_root":home.join("credentials")})
    };
    let mut local = host(HOST);
    local["routing_startup"] = json!({"enabled":false,"listen_port":0});
    fs::write(
        &policy,
        json!({"hosts":[local,host("other-mac")],"principals":principals}).to_string(),
    )
    .unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();

    let output = tempfile::NamedTempFile::new_in(&home).unwrap();
    let log = output.reopen().unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", "gateway_child", "--nocapture"])
        .current_dir(&home)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .env("VIBEWINDOW_TEST_ROOT", &home)
        .env("CODEX_HOME", home.join(".codex"))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("XDG_STATE_HOME", home.join(".local/state"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("TMPDIR", &home)
        .env("USER", "shiju-scan-fixture")
        .env("SHIJU_AGENT_APPS_ACCESS_FILE", &policy)
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log);
    for key in ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let mut gateway = GatewayProcess(command.spawn().unwrap());
    let raw =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap();
    let port = timeout(Duration::from_secs(20), async {
        loop {
            let text = fs::read_to_string(output.path()).unwrap();
            assert!(gateway.0.try_wait().unwrap().is_none(), "Gateway exited: {text}");
            if let Some(port) = text.lines().find_map(|line| {
                line.strip_prefix("VibeWindow Gateway listening on http://127.0.0.1:")
                    .and_then(|port| port.parse::<u16>().ok())
            }) && gateway_system::GatewaySystemClient::new(client(port, "owner").transport().clone()).system_health().await.is_ok()
            {
                break port;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("private production Gateway must become ready");
    let owner = client(port, "owner");
    let project = ProjectId::from("fixture");
    let scan_url = format!(
        "http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}/projects/fixture/apps/codex/egress/scans"
    );
    let request = |body: Value| {
        raw.post(&scan_url)
            .header("x-platform-id", "p")
            .header("x-company-id", "owner")
            .json(&body)
    };
    assert_eq!(request(json!({})).send().await.unwrap().status(), 401);
    assert_eq!(
        request(json!({})).bearer_auth("unpaired-key").send().await.unwrap().status(),
        401
    );
    let start = |body: Value| request(body).bearer_auth(SKEY).send();
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
    let job: DiagnosticJobIdentity = response.json().await.unwrap();
    for (tenant, host, project, app) in [
        ("other", HOST, "fixture", APP),
        ("owner", "other-mac", "fixture", APP),
        ("owner", HOST, "other", APP),
        ("owner", HOST, "fixture", AppId::Claude),
    ] {
        let routing = client(port, tenant);
        assert_error(
            routing
                .app_routing_outbound_scan_status(host, &project.into(), app, &job.task_id, "")
                .await
                .unwrap_err(),
            404,
            "not_found",
        );
        assert_error(
            routing
                .app_routing_cancel_outbound_scan(host, &project.into(), app, &job.task_id, "")
                .await
                .unwrap_err(),
            404,
            "not_found",
        );
    }
    assert_error(
        owner
            .app_routing_outbound_probe_status(HOST, &project, APP, &job.task_id, "")
            .await
            .unwrap_err(),
        404,
        "not_found",
    );
    let snapshot = terminal(&owner, &project, &job.task_id).await;
    assert_eq!(snapshot.state, DiagnosticJobState::Succeeded {});
    assert_eq!(snapshot.progress.completed, 8);
    assert_eq!(snapshot.progress.total, Some(8));
    assert!(!serde_json::to_string(&snapshot).unwrap().contains(SKEY));
    let mut ports = Vec::new();
    for result in snapshot.results.unwrap() {
        let DiagnosticResult::OutboundScan { report } = result else { panic!("wrong result kind") };
        let DiagnosticOutcome::Succeeded { evidence } = report.outcome else {
            panic!("scan failed")
        };
        let hints: &[&str] = match evidence.port {
            7890 => &["http", "socks5"],
            7891 | 1080 | 10808 => &["socks5"],
            8080 | 8888 | 3128 | 10809 => &["http"],
            _ => panic!("scanner reached a non-source port"),
        };
        let expected: Vec<String> = if evidence.tcp_open {
            hints.iter().map(|hint| (*hint).to_owned()).collect()
        } else {
            Vec::new()
        };
        assert_eq!(evidence.protocol_candidates, expected);
        ports.push(evidence.port);
    }
    ports.sort_unstable();
    assert_eq!(ports, [1080, 3128, 7890, 7891, 8080, 8888, 10808, 10809]);
    let job = owner.app_routing_start_outbound_scan(HOST, &project, APP, "").await.unwrap();
    let cancelled = owner
        .app_routing_cancel_outbound_scan(HOST, &project, APP, &job.task_id, "")
        .await
        .unwrap();
    assert!(matches!(
        cancelled.state,
        DiagnosticJobState::Cancelling {}
            | DiagnosticJobState::Cancelled {}
            | DiagnosticJobState::Succeeded {}
    ));
    let final_state = terminal(&owner, &project, &job.task_id).await;
    assert!(matches!(
        final_state.state,
        DiagnosticJobState::Cancelled {} | DiagnosticJobState::Succeeded {}
    ));
    assert_eq!(
        owner
            .app_routing_cancel_outbound_scan(HOST, &project, APP, &job.task_id, "")
            .await
            .unwrap(),
        final_state
    );
}
