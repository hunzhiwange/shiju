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
use vw_api_types::{app_routing::RoutingState, id::TenantScope};
use vw_gateway_client::{AppRoutingError, GatewayClient, GatewayEndpoint};

const CHILD_TEST: &str = "gateway::api::handlers::app_routing::lifecycle_api_tests::gateway_child";
const GATEWAY_LINE: &str = "VibeWindow Gateway listening on http://127.0.0.1:";
pub(super) const HOST: &str = "routing-mac";
pub(super) const MANAGER: &str = "fixture-routing-manager-token-00001";
pub(super) const READER: &str = "fixture-routing-reader-token-000002";
pub(super) const MEMBER: &str = "fixture-routing-member-token-000003";

#[tokio::test]
#[ignore = "started as a separate process with a private gateway configuration"]
async fn gateway_child() {
    let home = std::env::current_dir().unwrap();
    let config = crate::Config {
        workspace_dir: home.join("workspace"),
        config_path: home.join("config.toml"),
        ..Default::default()
    };
    fs::create_dir_all(&config.workspace_dir).unwrap();
    crate::gateway::run_gateway("127.0.0.1", 0, config).await.unwrap();
}

pub(super) struct GatewayProcess(Child);

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

pub(super) fn write_policy(path: &Path, home: &Path, port: u16) {
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

pub(super) async fn spawn_gateway(home: &Path, policy: &Path) -> (GatewayProcess, u16) {
    let output = tempfile::NamedTempFile::new_in(home).unwrap();
    let log = output.reopen().unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", CHILD_TEST, "--nocapture"])
        .current_dir(home)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home)
        .env("TMPDIR", home)
        .env("SHIJU_AGENT_APPS_ACCESS_FILE", policy)
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log);
    for key in ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let mut process = GatewayProcess(command.spawn().unwrap());
    let port = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let log = fs::read_to_string(output.path()).unwrap();
            assert!(process.0.try_wait().unwrap().is_none(), "gateway exited: {log}");
            if let Some(port) = log.lines().find_map(|line| line.strip_prefix(GATEWAY_LINE))
                && let Ok(port) = port.parse()
                && let Ok(response) =
                    reqwest::get(format!("http://127.0.0.1:{port}/v1/health")).await
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

pub(super) fn client(port: u16, company: &str) -> GatewayClient {
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
async fn cc_route_011_lifecycle_reads_the_persisted_listener_config() -> anyhow::Result<()> {
    use crate::{
        app_routing::{
            RoutingListener,
            config_apply::{ListenerConfig, RoutingConfigStore},
        },
        global_database::GlobalDatabase,
    };
    use std::sync::Arc;

    let home = tempfile::tempdir()?;
    let policy = home.path().join("access.json");
    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let fallback = occupied.local_addr()?;
    write_policy(&policy, home.path(), fallback.port());
    let mut value: Value = serde_json::from_slice(&fs::read(&policy)?)?;
    value["hosts"][0]["routing_startup"]["enabled"] = json!(true);
    fs::write(&policy, value.to_string())?;

    let store = Arc::new(RoutingConfigStore::new(
        GlobalDatabase::for_home(home.path())?,
        TenantScope::new("p", "owner")?,
        HOST.into(),
    )?);
    let saved = ListenerConfig {
        enabled: false,
        bind: (Ipv4Addr::LOCALHOST, 0).into(),
        logging_enabled: true,
    };
    store
        .apply_listener(
            &Arc::new(RoutingListener::default()),
            ListenerConfig { enabled: true, bind: fallback, logging_enabled: false },
            saved,
            || Ok(()),
        )
        .await?;

    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let client = client(port, "owner");
    assert_eq!(client.app_routing_status(HOST, READER).await?.state, RoutingState::Stopped);
    let started = client.app_routing_start(HOST, MANAGER).await?;
    assert_eq!(started.state, RoutingState::Running);
    assert!(started.bound_port.is_some_and(|port| port != 0 && port != fallback.port()));
    client.app_routing_stop(HOST, MANAGER).await?;
    Ok(())
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
        assert_error(owner.app_routing_start(host, token).await.unwrap_err(), 403, "forbidden");
    }
    assert_error(other.app_routing_start(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(owner.app_routing_health(HOST, MEMBER).await.unwrap_err(), 403, "forbidden");
    assert_error(owner.app_routing_health(HOST, READER).await.unwrap_err(), 503, "not_ready");
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap().state, RoutingState::Stopped);

    assert_error(owner.app_routing_start(HOST, MANAGER).await.unwrap_err(), 409, "address_in_use");
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap().state, RoutingState::Failed);
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

    let running = owner.app_routing_start(HOST, MANAGER).await.unwrap();
    assert_eq!(running.state, RoutingState::Running);
    assert_eq!(running.bound_address, Some(address.ip()));
    assert_eq!(running.bound_port, Some(address.port()));
    assert!(running.started_at.is_some());
    assert_eq!(owner.app_routing_start(HOST, MANAGER).await.unwrap(), running);
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap(), running);
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

    assert_error(other.app_routing_stop(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(owner.app_routing_stop(HOST, READER).await.unwrap_err(), 403, "forbidden");
    assert!(tokio::net::TcpStream::connect(address).await.is_ok());
    let stopped = owner.app_routing_stop(HOST, MANAGER).await.unwrap();
    assert_eq!(stopped.state, RoutingState::Stopped);
    assert_eq!(
        (stopped.bound_address, stopped.bound_port, stopped.started_at.as_deref()),
        (None, None, None)
    );
    assert_eq!(owner.app_routing_stop(HOST, MANAGER).await.unwrap(), stopped);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
}

pub(super) fn write_attached_app(home: &Path) -> (std::path::PathBuf, String) {
    use vw_agent_apps::providers::live::commit::{ExecutionHostId, ProviderTransactions};

    let root = home.join("credentials");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let _store = ProviderTransactions::new(&root, ExecutionHostId::new(HOST).unwrap()).unwrap();
    let host_state = fs::read_dir(root.join("provider-live"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("codex.state.json");
    let attached = json!({
        "mode": {"mode": "proxy", "attached": true, "proxy_route": "fixture-target", "contract": null},
        "mode_tenant": null,
        "pointer": null
    }).to_string();
    fs::write(&host_state, &attached).unwrap();
    fs::set_permissions(&host_state, fs::Permissions::from_mode(0o600)).unwrap();
    (host_state, attached)
}

#[tokio::test]
async fn cc_route_010_lifecycle_stop_preserves_attached_apps() {
    let home = tempfile::TempDir::new().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let owner = client(port, "owner");
    let running = owner.app_routing_start(HOST, MANAGER).await.unwrap();
    let address = (running.bound_address.unwrap(), running.bound_port.unwrap());

    let (host_state, attached) = write_attached_app(home.path());

    assert_error(owner.app_routing_stop(HOST, MANAGER).await.unwrap_err(), 409, "apps_attached");
    assert_eq!(fs::read_to_string(&host_state).unwrap(), attached);
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap(), running);
    assert!(tokio::net::TcpStream::connect(address).await.is_ok());
    fs::remove_file(host_state).unwrap();
    assert_eq!(owner.app_routing_stop(HOST, MANAGER).await.unwrap().state, RoutingState::Stopped);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn cc_route_010_lifecycle_policy_reload_cannot_move_a_running_listener() {
    let home = tempfile::TempDir::new().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let owner = client(port, "owner");
    let other = client(port, "other");
    let running = owner.app_routing_start(HOST, MANAGER).await.unwrap();
    let address = (running.bound_address.unwrap(), running.bound_port.unwrap());

    let original = fs::read_to_string(&policy).unwrap();
    let mut edited: Value = serde_json::from_str(&original).unwrap();
    edited["principals"][0]["grants"][0]["company_id"] = json!("other");
    fs::write(&policy, edited.to_string()).unwrap();
    assert_error(owner.app_routing_stop(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(other.app_routing_stop(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(other.app_routing_status(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert!(tokio::net::TcpStream::connect(address).await.is_ok());

    fs::write(&policy, original).unwrap();
    assert_eq!(owner.app_routing_stop(HOST, MANAGER).await.unwrap().state, RoutingState::Stopped);
}
