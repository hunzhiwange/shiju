use gateway_app_routing::AppRoutingError;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::{Ipv4Addr, TcpListener},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};
use {
    vw_api_app_routing::RoutingState,
    vw_api_transport::id::TenantScope,
};
use {gateway_endpoint::{GatewayEndpoint}, {vw_gateway_client::GatewayClient}};

const GATEWAY_SKEY: &str = "fixture-local-routing-gateway-skey";
const CHILD_TEST: &str = "gateway::api::handlers::app_routing::lifecycle_api_tests::gateway_child";
const GATEWAY_LINE: &str = "VibeWindow Gateway listening on http://127.0.0.1:";
pub(in crate::gateway::api::handlers) const HOST: &str = "routing-mac";
pub(in crate::gateway::api::handlers) const MANAGER: &str = "fixture-routing-manager-token-00001";
pub(super) const READER: &str = "fixture-routing-reader-token-000002";
pub(super) const MEMBER: &str = "fixture-routing-member-token-000003";

#[tokio::test]
#[ignore = "started as a separate process with a private gateway configuration"]
async fn gateway_child() {
    let home = std::env::current_dir().unwrap().canonicalize().unwrap();
    assert_eq!(crate::global::paths().home, home);
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut config = crate::Config {
        workspace_dir: home.join("workspace"),
        config_path: home.join("config.toml"),
        ..Default::default()
    };
    if std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").is_none() {
        config.gateway.auth_enabled = true;
        let skeys = serde_json::from_value::<Vec<config_gateway::GatewaySkey>>(json!([{
            "skey_hash": hex::encode(Sha256::digest(GATEWAY_SKEY)),
            "name": "local routing fixture",
        }])).unwrap();
        crate::config::replace_gateway_skeys_for_tenant(&TenantScope::default(), &skeys)
            .await
            .unwrap();
    }
    if std::env::var_os("SHIJU_ROUTING_TEST_TUNNEL").is_some() {
        config.tunnel = serde_json::from_value(json!({
            "provider": "custom",
            "custom": {"start_command": "/usr/bin/true"},
        })).unwrap();
    }
    fs::create_dir_all(&config.workspace_dir).unwrap();
    crate::gateway::run_gateway("127.0.0.1", 0, config).await.unwrap();
}

pub(in crate::gateway::api::handlers) struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn grant(token: &str, read: bool, manage: bool) -> Value {
    json!({
        "id": token,
        "token_sha256": hex::encode(Sha256::digest(token)),
        "grants": [{
            "platform_id": "p", "company_id": "owner", "host_id": HOST,
            "routing_read": read, "routing_manage": manage
        }]
    })
}

pub(in crate::gateway::api::handlers) fn write_policy(path: &Path, home: &Path, port: u16) {
    let host = |id| {
        json!({
            "id": id,
            "online": true,
            "binding": "local",
            "home": home,
            "search_paths": [],
            "applications": home.join("Applications"),
            "credential_root": home.join("credentials"),
        })
    };
    let mut owner = host(HOST);
    owner["routing_startup"] = json!({"enabled": false, "listen_port": port});
    let policy = json!({
        "hosts": [owner, host("other-mac")],
        "principals": [
            grant(MANAGER, true, true), grant(READER, true, false), grant(MEMBER, false, false)
        ],
    });
    fs::write(path, policy.to_string()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

pub(in crate::gateway::api::handlers) async fn spawn_gateway(
    home: &Path,
    policy: &Path,
) -> (GatewayProcess, u16) {
    spawn_gateway_with_options(home, Some(policy), false).await
}

async fn spawn_gateway_with_options(
    home: &Path,
    policy: Option<&Path>,
    tunneled: bool,
) -> (GatewayProcess, u16) {
    let home = home.canonicalize().unwrap();
    let output = tempfile::NamedTempFile::new_in(&home).unwrap();
    let log = output.reopen().unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", CHILD_TEST, "--nocapture"])
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
        .env("USER", "shiju-routing-fixture")
        .env("TMPDIR", &home)
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log);
    if let Some(policy) = policy {
        command.env("SHIJU_AGENT_APPS_ACCESS_FILE", policy);
    }
    if tunneled {
        command.env("SHIJU_ROUTING_TEST_TUNNEL", "1");
    }
    for key in ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let mut process = GatewayProcess(command.spawn().unwrap());
    let health =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(2)).build().unwrap();
    let port = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let log = fs::read_to_string(output.path()).unwrap();
            assert!(process.0.try_wait().unwrap().is_none(), "gateway exited: {log}");
            if let Some(port) = log.lines().find_map(|line| line.strip_prefix(GATEWAY_LINE))
                && let Ok(port) = port.parse()
                && let Ok(response) =
                    health.get(format!("http://127.0.0.1:{port}/v1/health")).send().await
                && response.status().is_success()
            {
                break port;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("gateway health never became ready");
    (process, port)
}

pub(in crate::gateway::api::handlers) fn client(port: u16, company: &str) -> GatewayClient {
    GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", company).unwrap()),
    )
    .unwrap()
}

pub(super) fn assert_error(error: AppRoutingError, status: u16, code: &str) {
    assert!(
        matches!(&error, AppRoutingError::Http { status: actual, body: Some(body) }
            if *actual == status && body.error.code == code),
        "expected HTTP {status} {code}, got {error:?}"
    );
}


#[tokio::test]
async fn cc_route_010_lifecycle_api_expected_behavior() {
    let home = tempfile::TempDir::new().unwrap();
    let policy = home.path().join("access.json");
    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = occupied.local_addr().unwrap();
    write_policy(&policy, home.path(), address.port());
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;

    let base = format!("http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}");
    let raw = reqwest::Client::builder().no_proxy().build().unwrap();
    // Check the actual HTTP entry before invoking the SDK so its implementation
    // cannot hide a missing production route or an authorization bypass.
    let unauthorized = raw
        .post(format!("{base}/start"))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), 401, "a member credential is required to start");

    let read_only = raw
        .post(format!("{base}/start"))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner")
        .header("x-agent-apps-token", READER)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(read_only.status(), 403, "a read-only member cannot start the listener");

    let owner = client(port, "owner");
    let other = client(port, "other");
    for (host, token) in [(HOST, MEMBER), (HOST, READER), ("other-mac", MANAGER)] {
        assert_error(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_start(host, token).await.unwrap_err(), 403, "forbidden");
    }
    assert_error(gateway_app_routing::GatewayAppRoutingClient::new(other.transport().clone()).app_routing_start(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_health(HOST, MEMBER).await.unwrap_err(), 403, "forbidden");
    assert_error(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_health(HOST, READER).await.unwrap_err(), 503, "not_ready");
    assert_eq!(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_status(HOST, READER).await.unwrap().state, RoutingState::Stopped);

    assert_error(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_start(HOST, MANAGER).await.unwrap_err(), 409, "address_in_use");
    assert_eq!(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_status(HOST, READER).await.unwrap().state, RoutingState::Failed);
    drop(occupied);

    let invalid = raw
        .post(format!("{base}/start"))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner")
        .header("x-agent-apps-token", MANAGER)
        .json(&json!({"listenAddress": "0.0.0.0"}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), 400);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());

    let running = gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_start(HOST, MANAGER).await.unwrap();
    assert_eq!(running.state, RoutingState::Running);
    assert_eq!(running.bound_address, Some(address.ip()));
    assert_eq!(running.bound_port, Some(address.port()));
    assert!(running.started_at.is_some());
    assert_eq!(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_start(HOST, MANAGER).await.unwrap(), running);
    assert_eq!(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_status(HOST, READER).await.unwrap(), running);
    let health = raw
        .get(format!("{base}/health"))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner")
        .header("x-agent-apps-token", READER)
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), 200);
    assert_eq!(health.headers()["cache-control"], "no-store");
    assert_eq!(health.json::<Value>().await.unwrap(), json!({"ready":true}));

    assert_error(gateway_app_routing::GatewayAppRoutingClient::new(other.transport().clone()).app_routing_stop(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_stop(HOST, READER).await.unwrap_err(), 403, "forbidden");
    assert!(tokio::net::TcpStream::connect(address).await.is_ok());
    let stopped = gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_stop(HOST, MANAGER).await.unwrap();
    assert_eq!(stopped.state, RoutingState::Stopped);
    assert_eq!(
        (stopped.bound_address, stopped.bound_port, stopped.started_at.as_deref()),
        (None, None, None)
    );
    assert_eq!(gateway_app_routing::GatewayAppRoutingClient::new(owner.transport().clone()).app_routing_stop(HOST, MANAGER).await.unwrap(), stopped);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn routing_status_identifies_unconfigured_control() {
    let home = tempfile::TempDir::new().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let mut config: Value = serde_json::from_slice(&fs::read(&policy).unwrap()).unwrap();
    for member in config["principals"].as_array_mut().unwrap() {
        member["grants"][0]["routing_read"] = json!(false);
        member["grants"][0]["routing_manage"] = json!(false);
    }
    fs::write(&policy, config.to_string()).unwrap();

    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let gateway = client(port, "owner");
    let routing = gateway_app_routing::GatewayAppRoutingClient::new(gateway.transport().clone());
    assert_error(
        routing.app_routing_status(HOST, MANAGER).await.unwrap_err(),
        503,
        "routing_control_not_configured",
    );
}

#[tokio::test]
async fn local_routing_control_endpoints() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let home = tempfile::TempDir::new().unwrap();
    let (gateway, port) = spawn_gateway_with_options(home.path(), None, false).await;
    let host = vw_api_transport::LOCAL_HOST_ID;
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap();
    let request = |port, method, host, path, company| {
        client
            .request(method, format!("http://127.0.0.1:{port}/v1/app-routing/hosts/{host}/{path}"))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
    };
    use reqwest::Method;
    let local_request =
        |port, method, path| request(port, method, host, path, "owner").bearer_auth(GATEWAY_SKEY);
    let local = |method, path| local_request(port, method, path);
    let read = async |port, path| {
        let response = local_request(port, Method::GET, path).send().await.unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        assert_eq!(status, 200, "{path}: {body}");
        serde_json::from_str::<Value>(&body).unwrap()
    };

    assert_eq!(
        request(port, Method::GET, host, "config", "owner").send().await.unwrap().status(),
        401
    );
    assert_eq!(
        request(port, Method::GET, "other-mac", "config", "owner")
            .bearer_auth(GATEWAY_SKEY)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    let missing_tenant = client
        .get(format!("http://127.0.0.1:{port}/v1/app-routing/hosts/{host}/config"))
        .bearer_auth(GATEWAY_SKEY)
        .send()
        .await
        .unwrap();
    assert_eq!(missing_tenant.status(), 400);
    let mut config = read(port, "config").await;
    assert_eq!(config["enabled"], false);
    assert_eq!(config["listenAddress"], "127.0.0.1");
    assert_eq!(read(port, "status").await["state"], "stopped");

    let app_config = "projects/default/apps/codex/config";
    let mut timeouts = read(port, app_config).await;
    timeouts["idleTimeoutSecs"] = json!(0);
    let applied = local(Method::PUT, app_config).json(&timeouts).send().await.unwrap();
    assert_eq!(applied.status(), 200);
    assert_eq!(applied.json::<Value>().await.unwrap(), timeouts);

    assert_eq!(read(port, "client-host").await["effectiveHost"], "127.0.0.1");
    read(port, "projects/default/apps/codex/requests").await;
    assert_eq!(read(port, "egress").await["configuredMode"], "system");

    let egress =
        local(Method::PUT, "egress").json(&json!({"mode": "direct"})).send().await.unwrap();
    assert_eq!(egress.status(), 200);
    assert_eq!(egress.json::<Value>().await.unwrap()["configuredMode"], "direct");

    let other = request(port, Method::PUT, host, "egress", "other")
        .bearer_auth(GATEWAY_SKEY)
        .json(&json!({"mode": "system"}))
        .send()
        .await
        .unwrap();
    assert_eq!(other.status(), 200);
    assert_eq!(read(port, "egress").await["configuredMode"], "direct");

    let mut invalid = config.clone();
    invalid["listenAddress"] = json!("0.0.0.0");
    assert_eq!(local(Method::PUT, "config").json(&invalid).send().await.unwrap().status(), 400);
    config["listenPort"] = json!(0);
    config["enabled"] = json!(true);
    assert_eq!(local(Method::PUT, "config").json(&config).send().await.unwrap().status(), 200);
    let response = local(Method::POST, "start").json(&json!({})).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let running = response.json::<Value>().await.unwrap();
    assert_eq!(running["state"], "running");
    let listen_port = running["boundPort"].as_u64().unwrap() as u16;
    assert!(tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, listen_port)).await.is_ok());

    let response = local(Method::POST, "stop").json(&json!({})).send().await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.json::<Value>().await.unwrap()["state"], "stopped");
    assert!(tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, listen_port)).await.is_err());

    drop(gateway);
    let (gateway, port) = spawn_gateway_with_options(home.path(), None, false).await;
    assert_eq!(read(port, app_config).await, timeouts);
    assert_eq!(read(port, "config").await, config);
    assert_eq!(read(port, "status").await["state"], "stopped");
    assert_eq!(read(port, "egress").await["configuredMode"], "direct");

    drop(gateway);
    let (tunneled_gateway, port) = spawn_gateway_with_options(home.path(), None, true).await;
    let response = local_request(port, Method::GET, "config").send().await.unwrap();
    assert_eq!(response.status(), 503);

    drop(tunneled_gateway);
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (_policy_gateway, port) = spawn_gateway(home.path(), &policy).await;
    for (host, expected) in [(HOST, 200), (host, 403)] {
        let response = request(port, Method::GET, host, "config", "owner").send().await.unwrap();
        assert_eq!(response.status(), expected);
    }
}
