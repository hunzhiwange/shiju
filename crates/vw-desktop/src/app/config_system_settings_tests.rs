//! 验证系统设置配置在应用状态与持久化结构之间的转换。
//!
//! 往返夹具比较持久化配置与应用设置状态，确保缺省值和可选字段不漂移。

use std::ffi::OsStr;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{TcpListener, TcpStream};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver};
use std::sync::{LazyLock, Mutex, MutexGuard};

use tempfile::TempDir;
use config_ui::{
    GatewayClientServerConfig, GatewayClientSystemSettingsConfig,
};

const SOURCE: &str = concat!(include_str!("config_system_settings.rs"), include_str!("../../../libs/desktop-config/src/config_system_settings.rs"));
static CONFIG_SYSTEM_SETTINGS_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(in crate::app) struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn env_lock() -> MutexGuard<'static, ()> {
    CONFIG_SYSTEM_SETTINGS_ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

fn temp_home() -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let lock = env_lock();
    let home = tempfile::tempdir().expect("temp home should be created");
    let guard = EnvGuard::set_os("HOME", home.path().as_os_str());
    (lock, home, guard)
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn isolated_settings() -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let lock = env_lock();
    let root = tempfile::tempdir().expect("temp settings directory");
    let guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", root.path().as_os_str());
    (lock, root, guard)
}

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

fn gateway_config(host: &str, port: u16) -> GatewayClientSystemSettingsConfig {
    let mut config = GatewayClientSystemSettingsConfig::default();
    let mut server = GatewayClientServerConfig::default();
    server.host = host.to_string();
    server.port = port;
    config.set_servers(vec![server], "local".to_string());
    config
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn read_http_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        let read = stream.read(&mut buffer).expect("request should be readable");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if let Some(header_end) =
            request.windows(4).position(|window| window == b"\r\n\r\n").map(|index| index + 4)
        {
            let headers = String::from_utf8_lossy(&request[..header_end]);
            if request.len() >= header_end + http_content_length(&headers) {
                break;
            }
        }
    }
    String::from_utf8(request).expect("request should be utf8")
}

#[cfg(not(target_arch = "wasm32"))]
fn http_content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if !name.eq_ignore_ascii_case("content-length") {
                return None;
            }
            value.trim().parse().ok()
        })
        .unwrap_or(0)
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn request_json_body(request: &str) -> serde_json::Value {
    let (_, body) = request.split_once("\r\n\r\n").expect("request should contain a body");
    serde_json::from_str(body).expect("request body should be json")
}

#[test]
fn config_system_settings_tests_keeps_planned_coverage_targets() {
    for name in [
        "fetch_desktop_system_settings_via_gateway",
        "patch_desktop_system_settings_via_gateway",
        "normalize_system_settings_config",
        "gateway_client_bootstrap_cache_path",
        "load_gateway_client_bootstrap_config",
        "save_gateway_client_bootstrap_config",
        "load_gateway_client_config",
        "update_gateway_client_config",
        "load_system_settings_config_async",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn update_gateway_client_config_persists_without_a_gateway() {
    let (_lock, _home, _guard) = temp_home();
    let bootstrap = gateway_config("127.0.0.1", 1);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let snapshot = super::update_gateway_client_config(|config| {
        config.username = "desktop-user".to_string();
    })
    .expect("local save should succeed with no Gateway");
    assert_eq!(snapshot.username, "desktop-user");
    assert_eq!(super::load_gateway_client_bootstrap_config(), snapshot);
}

#[test]
fn agent_unified_selection_config_override_isolates_window_bootstrap() {
    let _lock = env_lock();
    let root = TempDir::new().unwrap();
    let _guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", root.path().as_os_str());
    assert_eq!(
        super::gateway_client_bootstrap_cache_path().unwrap(),
        root.path().join("gateway-client-bootstrap.json")
    );
    assert_eq!(super::legacy_gateway_client_bootstrap_cache_path(), None);
    assert!(crate::app::project_dirs().unwrap().data_local_dir().starts_with(root.path()));
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn with_editor_settings_gateway(
    test: impl FnOnce(crate::app::App, Receiver<String>, mpsc::Sender<Option<String>>),
) {
    with_settings_gateway(|requests, responses| {
        let (app, _) = crate::app::App::new();
        test(app, requests, responses);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn with_settings_gateway(
    test: impl FnOnce(Receiver<String>, mpsc::Sender<Option<String>>),
) {
    use std::time::Duration;

    let (_lock, home, _home_guard) = temp_home();
    let _config_guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", home.path().as_os_str());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let bootstrap = gateway_config("127.0.0.1", address.port());
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();
    let bootstrap_path = super::gateway_client_bootstrap_cache_path().unwrap();
    let original_bootstrap = std::fs::read(&bootstrap_path).unwrap();

    let (request_tx, requests) = mpsc::channel();
    let (responses, response_rx) = mpsc::channel::<Option<String>>();
    let server = std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let request = read_http_request(&mut stream);
            if request.is_empty() {
                break;
            }
            if request_tx.send(request).is_err() {
                break;
            }
            let body = match response_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(Some(body)) => body,
                Ok(None) => continue, // Disconnect without a response.
                Err(_) => break,
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test(requests, responses);
    }));
    // Wake accept on success or panic; the response channel also closes if the test panics.
    drop(TcpStream::connect(address));
    server.join().unwrap();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
    assert_eq!(std::fs::read(bootstrap_path).unwrap(), original_bootstrap);
}

/// Poll the Iced save task while holding its HTTP reply until the caller has exercised the UI.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn drive_system_settings_patch(
    task: iced::Task<crate::app::Message>,
    requests: &Receiver<String>,
    responses: &mpsc::Sender<Option<String>>,
    expected: serde_json::Value,
    reply: Option<&str>,
    while_waiting: impl FnOnce(),
) -> crate::app::Message {
    use iced_runtime::{Action, futures::futures::StreamExt, task::into_stream};
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(4);
    let runtime =
        tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let completion = runtime.spawn(async move {
        let mut stream = into_stream(task).expect("Iced save task");
        let Some(Action::Output(message)) = stream.next().await else {
            panic!("expected a save result");
        };
        message
    });
    let request = requests.recv_timeout(TIMEOUT).expect("system settings PATCH");
    assert!(request.starts_with("PATCH /v1/global/config "), "{request}");
    assert_eq!(
        request_json_body(&request),
        serde_json::json!({"app_ui": {"system_settings": expected}})
    );
    while_waiting();
    responses.send(reply.map(str::to_owned)).unwrap();
    runtime.block_on(async { tokio::time::timeout(TIMEOUT, completion).await.unwrap().unwrap() })
}
