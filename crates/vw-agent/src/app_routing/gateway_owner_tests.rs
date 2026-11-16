use serde_json::{Value, json};
use std::{
    fs,
    net::{Ipv4Addr, TcpListener},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(20);
const GATEWAY_LINE: &str = "VibeWindow Gateway listening on http://127.0.0.1:";
const CHILD_TEST: &str = "app_routing::gateway_owner_tests::gateway_child";

#[tokio::test]
#[ignore = "launched with isolated state by the gateway lifecycle acceptance test"]
async fn gateway_child() {
    let home = std::env::current_dir().unwrap();
    let port = std::env::var("ROUTING_TEST_PORT").unwrap().parse().unwrap();
    let config = crate::Config {
        workspace_dir: home.join("workspace"),
        config_path: home.join("config.toml"),
        ..Default::default()
    };
    fs::create_dir_all(&config.workspace_dir).unwrap();
    if std::env::var_os("ROUTING_TEST_DAEMON").is_some() {
        crate::daemon::run(config, "127.0.0.1".into(), 0).await.unwrap();
    } else {
        crate::gateway::run_gateway("127.0.0.1", 0, config).await.unwrap();
    }
    // Check before process exit while the runtime is still alive.
    let rebound = super::RoutingListener::default();
    assert_eq!(rebound.start(port).unwrap().port(), port);
    rebound.stop().await.unwrap();
}

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn host(home: &Path, port: u16) -> Value {
    json!({
        "id": "route-test-host",
        "online": true,
        "binding": "local",
        "home": home,
        "search_paths": [],
        "applications": home.join("Applications"),
        "routing_startup": { "enabled": true, "listen_port": port },
    })
}

fn write_policy(path: &Path, hosts: &[Value]) {
    fs::write(path, json!({ "principals": [], "hosts": hosts }).to_string()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[tokio::test]
async fn cc_route_007_gateway_owner_expected_behavior() {
    let fixture = TempDir::new().unwrap();
    let policy = fixture.path().join("access.json");
    let route = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let route_address = route.local_addr().unwrap();
    let route_port = route_address.port();
    write_policy(&policy, &[host(fixture.path(), route_port)]);
    let client_dir = fixture.path().join(".claude");
    fs::create_dir(&client_dir).unwrap();
    let client_settings = client_dir.join("settings.json");
    let original_settings = br#"{"env":{"TEST_ONLY":"leave untouched"}}"#;
    fs::write(&client_settings, original_settings).unwrap();
    drop(route);

    // Restart with the same stored policy, first standalone and then supervised by the daemon.
    for (daemon, restore) in [(false, true), (true, true), (false, false)] {
        let output = tempfile::NamedTempFile::new_in(fixture.path()).unwrap();
        let log = output.reopen().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--ignored", "--exact", CHILD_TEST, "--nocapture"])
            .current_dir(fixture.path())
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", fixture.path())
            .env("TMPDIR", fixture.path())
            .env("ROUTING_TEST_PORT", route_port.to_string())
            .stdin(Stdio::null())
            .stdout(log.try_clone().unwrap())
            .stderr(log);
        for key in ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        if daemon {
            command.env("ROUTING_TEST_DAEMON", "1");
        }
        if restore {
            command.env("SHIJU_AGENT_APPS_ACCESS_FILE", &policy);
        }
        let mut gateway = GatewayProcess(command.spawn().unwrap());
        tokio::time::timeout(TIMEOUT, async {
            loop {
                let log = fs::read_to_string(output.path()).unwrap();
                assert!(gateway.0.try_wait().unwrap().is_none(), "gateway exited: {log}");
                if let Some(port) = log.lines().find_map(|line| line.strip_prefix(GATEWAY_LINE))
                    && let Ok(response) =
                        reqwest::get(format!("http://127.0.0.1:{port}/v1/health")).await
                    && response.status().is_success()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("gateway health never became ready");

        if restore {
            let client = tokio::net::TcpStream::connect(route_address)
                .await
                .expect("headless gateway did not restore its authorized routing listener");
            drop(client);
            assert!(tokio::net::TcpStream::connect(route_address).await.is_ok());
        } else {
            assert!(tokio::net::TcpStream::connect(route_address).await.is_err());
        }
        assert_eq!(fs::read(&client_settings).unwrap(), original_settings);
        let mut kill = tokio::process::Command::new("/bin/kill");
        assert!(
            kill.args(["-TERM", &gateway.0.id().to_string()]).status().await.unwrap().success()
        );
        let status = tokio::time::timeout(TIMEOUT, async {
            loop {
                if let Some(status) = gateway.0.try_wait().unwrap() {
                    break status;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("gateway did not exit after SIGTERM");
        assert!(
            status.success(),
            "gateway shutdown failed: {}",
            fs::read_to_string(output.path()).unwrap()
        );
        assert!(tokio::net::TcpStream::connect(route_address).await.is_err());
    }
}
