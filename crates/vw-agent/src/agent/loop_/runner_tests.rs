use crate::app::agent::config::Config;

use super::process_message;
use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    key: &'static str,
    old: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let old = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn serve_gateway_error_once() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("gateway listener should bind");
    let port = listener.local_addr().expect("gateway listener address").port().to_string();
    listener.set_nonblocking(true).expect("gateway listener should become nonblocking");
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 1024];
                    let _ = stream.read(&mut request);
                    let body = "test gateway closed";
                    let response = format!(
                        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    break;
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    (port, handle)
}

#[tokio::test]
async fn process_message_returns_error_for_unknown_provider() {
    let config =
        Config { default_provider: Some("__missing_provider__".to_string()), ..Config::default() };

    let result = process_message(config, "hello", "session").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn process_message_uses_default_provider_fallback_before_submit_error() {
    let _env = env_lock().lock().await;
    let config_dir = tempfile::tempdir().expect("config dir should be created");
    let workspace_dir = tempfile::tempdir().expect("workspace dir should be created");
    let (gateway_port, gateway) = serve_gateway_error_once();

    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());
    let _provider = EnvGuard::set("VIBEWINDOW_PROVIDER", "openrouter");
    let _host = EnvGuard::set("VIBEWINDOW_GATEWAY_HOST", "127.0.0.1");
    let _port = EnvGuard::set("VIBEWINDOW_GATEWAY_PORT", &gateway_port);
    let config = Config {
        default_provider: None,
        workspace_dir: workspace_dir.path().to_path_buf(),
        config_path: config_dir.path().join("vibewindow.json"),
        ..Config::default()
    };

    let error = process_message(config, "hello", "").await.expect_err("gateway should be closed");
    gateway.join().expect("gateway thread should finish");
    let message = error.to_string();

    assert!(!message.contains("Unknown provider"));
    assert!(message.contains("gateway stream failed: 503"));
    assert!(message.contains("test gateway closed"));
}
