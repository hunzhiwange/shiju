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

#[test]
fn cc_route_007_gateway_owner_rejects_untrusted_startup() {
    use crate::gateway::api::handlers::agent_apps::routing_setup;

    let fixture = TempDir::new().unwrap();
    let policy = fixture.path().join("access.json");
    let authorized = host(fixture.path(), 0);
    let read = || routing_setup(&policy).map(|setup| setup.startup_address);
    assert!(read().is_err(), "missing policy");

    for (field, value) in [("online", json!(false)), ("binding", json!("remote"))] {
        let mut denied = authorized.clone();
        denied[field] = value;
        write_policy(&policy, &[denied]);
        assert_eq!(read().unwrap(), None);
    }
    let mut denied = authorized.clone();
    denied["routing_startup"]["enabled"] = json!(false);
    write_policy(&policy, &[denied]);
    assert_eq!(read().unwrap(), None);
    let mut absent = authorized.clone();
    absent.as_object_mut().unwrap().remove("routing_startup");
    write_policy(&policy, &[absent]);
    assert_eq!(read().unwrap(), None);

    let mut non_loopback = authorized.clone();
    non_loopback["routing_startup"]["listen_address"] = json!("0.0.0.0");
    write_policy(&policy, &[non_loopback]);
    assert!(read().is_err());

    let mut second = authorized.clone();
    second["id"] = json!("another-host");
    write_policy(&policy, &[authorized.clone(), second]);
    assert!(read().is_err(), "ambiguous host policy");

    write_policy(&policy, &[authorized]);
    assert_eq!(read().unwrap().unwrap(), (Ipv4Addr::LOCALHOST, 0).into());
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(read().is_err(), "policy is writable by another principal");
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&policy, b"{broken").unwrap();
    assert!(read().is_err(), "damaged policy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cc_route_007_gateway_owner_waits_for_connections_before_returning() {
    use super::gateway_owner::run_with_routing;
    use std::sync::{Mutex, mpsc};
    use tokio::sync::oneshot;

    struct ConnectionCleanup {
        started: Option<oneshot::Sender<()>>,
        release: mpsc::Receiver<()>,
    }

    impl Drop for ConnectionCleanup {
        fn drop(&mut self) {
            let _ = self.started.take().unwrap().send(());
            let _ = self.release.recv_timeout(TIMEOUT);
        }
    }

    let (ready, bound) = oneshot::channel();
    let (accepted, connected) = oneshot::channel();
    let (stop, shutdown) = oneshot::channel();
    let (cleanup_start, cleanup_started) = oneshot::channel();
    let (release_cleanup, cleanup_release) = mpsc::channel();
    let mut owner = tokio::spawn(run_with_routing(
        async |routing| {
            let connection = Mutex::new(Some((accepted, cleanup_start, cleanup_release)));
            let address = routing.start_at((Ipv4Addr::LOCALHOST, 0).into(), move |stream| {
                let (accepted, started, release) = connection.lock().unwrap().take()?;
                Some(async move {
                    let _stream = stream;
                    let _cleanup = ConnectionCleanup { started: Some(started), release };
                    let _ = accepted.send(());
                    std::future::pending::<()>().await;
                })
            })?;
            ready.send(address).unwrap();
            std::future::pending().await
        },
        async {
            shutdown.await.unwrap();
            Ok(())
        },
    ));
    let client = tokio::time::timeout(TIMEOUT, async {
        let address = bound.await.unwrap();
        let client = tokio::net::TcpStream::connect(address).await.unwrap();
        connected.await.unwrap();
        client
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    tokio::time::timeout(TIMEOUT, cleanup_started).await.unwrap().unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut owner).await.is_err(),
        "gateway returned while a connection was still being cleaned up"
    );
    release_cleanup.send(()).unwrap();
    tokio::time::timeout(TIMEOUT, owner).await.unwrap().unwrap().unwrap();
    assert!(tokio::net::TcpStream::connect(client.peer_addr().unwrap()).await.is_err());
}
