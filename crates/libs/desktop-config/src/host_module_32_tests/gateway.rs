//! 验证网关配置的地址解析、默认值和环境覆盖规则。
//!
//! 地址样例覆盖主机、端口、协议和环境来源，非法输入必须走明确回退。

use config_ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn bootstrap_store() -> crate::config_system_settings::BootstrapStore {
    crate::config_system_settings::BootstrapStore { legacy_config_dir: None }
}
fn gateway_client_endpoint() -> gateway_endpoint::GatewayEndpoint {
    crate::config_gateway::gateway_client_endpoint_from_config(
        &bootstrap_store().load_gateway_client_bootstrap_config(),
    )
}
fn gateway_client() -> Result<vw_gateway_client::GatewayClient, String> {
    vw_gateway_client::GatewayClient::new(gateway_client_endpoint())
}
fn load_tools_list_via_gateway() -> Vec<String> {
    crate::ConfigAccess::new(gateway_client_endpoint(), None).load_tools_list_via_gateway()
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
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

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir should be created");
    }
    std::fs::write(path, serde_json::to_string(&value).expect("json should serialize"))
        .expect("json file should be written");
}

fn bootstrap_config_path(home: &Path) -> PathBuf {
    config_runtime_paths::home_config_dir(home).join("gateway-client-bootstrap.json")
}

fn write_gateway_bootstrap_config(home: &Path, cfg: &GatewayClientSystemSettingsConfig) {
    let path = bootstrap_config_path(home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("bootstrap dir should be created");
    }
    std::fs::write(
        path,
        serde_json::to_string(cfg).expect("gateway bootstrap config should serialize"),
    )
    .expect("gateway bootstrap config should be written");
}

fn read_http_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buf = [0_u8; 512];
    loop {
        let read = stream.read(&mut buf).expect("request should be readable");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buf[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(request).expect("request should be utf8")
}

fn write_http_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("response should be written");
}

fn spawn_http_stub(
    handler: impl Fn(String) -> (&'static str, String) + Send + Sync + 'static,
    expected_requests: usize,
) -> (u16, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("stub server should bind");
    let port = listener.local_addr().expect("stub local addr should exist").port();
    let handler = std::sync::Arc::new(handler);
    let join = std::thread::spawn(move || {
        for stream in listener.incoming().take(expected_requests) {
            let mut stream = stream.expect("stub request should connect");
            let request = read_http_request(&mut stream);
            let (status, body) = handler(request);
            write_http_response(&mut stream, status, &body);
        }
    });
    (port, join)
}

fn active_gateway_config(port: u16) -> GatewayClientSystemSettingsConfig {
    let server = GatewayClientServerConfig {
        id: "stub".to_string(),
        name: "Stub".to_string(),
        host: "127.0.0.1".to_string(),
        port,
        platform_id: "20000".to_string(),
        company_id: "200000001".to_string(),
        bearer_token: " token ".to_string(),
        username: " user ".to_string(),
        password: " pass ".to_string(),
        skey: " skey ".to_string(),
    };
    let mut cfg = GatewayClientSystemSettingsConfig::default();
    cfg.set_servers(vec![server], "stub".to_string());
    cfg
}

#[test]
fn gateway_client_endpoint_trims_auth_and_clamps_port() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let home = tempfile::tempdir().expect("temp home should be created");
    let server = GatewayClientServerConfig {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: " gateway.internal ".to_string(),
        port: 0,
        platform_id: " 20000 ".to_string(),
        company_id: " 200000001 ".to_string(),
        bearer_token: " token ".to_string(),
        username: " user ".to_string(),
        password: " pass ".to_string(),
        skey: " skey ".to_string(),
    };
    let mut cfg = GatewayClientSystemSettingsConfig::default();
    cfg.set_servers(vec![server], "remote".to_string());
    write_gateway_bootstrap_config(home.path(), &cfg);
    let _home = EnvVarGuard::set("HOME", home.path());
    let _config = EnvVarGuard::remove("VIBEWINDOW_CONFIG");
    let _config_dir = EnvVarGuard::remove("VIBEWINDOW_CONFIG_DIR");

    let endpoint = gateway_client_endpoint();

    assert_eq!(endpoint.normalized_host(), "gateway.internal");
    assert_eq!(endpoint.port, 1);
    assert_eq!(endpoint.tenant_scope().platform_id().as_ref(), "20000");
    assert_eq!(endpoint.tenant_scope().company_id().as_ref(), "200000001");
    let auth = endpoint.auth.expect("auth should be present");
    assert_eq!(auth.skey.as_deref(), Some("skey"));
}

#[test]
fn gateway_client_endpoint_uses_trimmed_bearer_token_when_skey_is_blank() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let home = tempfile::tempdir().expect("temp home should be created");
    let mut cfg = active_gateway_config(42618);
    cfg.servers[0].skey = " \t ".to_string();
    cfg.servers[0].bearer_token = " legacy-token ".to_string();
    write_gateway_bootstrap_config(home.path(), &cfg);
    let _home = EnvVarGuard::set("HOME", home.path());
    let _config = EnvVarGuard::remove("VIBEWINDOW_CONFIG");
    let _config_dir = EnvVarGuard::remove("VIBEWINDOW_CONFIG_DIR");

    let endpoint = gateway_client_endpoint();

    let auth = endpoint.auth.expect("auth should be present");
    assert_eq!(auth.skey.as_deref(), Some("legacy-token"));
}

#[test]
fn gateway_client_endpoint_omits_auth_key_when_credentials_are_blank() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let home = tempfile::tempdir().expect("temp home should be created");
    let mut cfg = active_gateway_config(42618);
    cfg.servers[0].skey = " \t ".to_string();
    cfg.servers[0].bearer_token = " ".to_string();
    write_gateway_bootstrap_config(home.path(), &cfg);
    let _home = EnvVarGuard::set("HOME", home.path());
    let _config = EnvVarGuard::remove("VIBEWINDOW_CONFIG");
    let _config_dir = EnvVarGuard::remove("VIBEWINDOW_CONFIG_DIR");

    let endpoint = gateway_client_endpoint();

    let auth = endpoint.auth.expect("auth container should be present");
    assert!(auth.skey.is_none());
}

#[test]
fn gateway_client_builds_from_configured_endpoint() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let home = tempfile::tempdir().expect("temp home should be created");
    let mut cfg = GatewayClientSystemSettingsConfig::default();
    cfg.host = "gateway.internal".to_string();
    cfg.port = 42618;
    cfg.servers.clear();
    write_gateway_bootstrap_config(home.path(), &cfg);
    let _home = EnvVarGuard::set("HOME", home.path());
    let _config = EnvVarGuard::remove("VIBEWINDOW_CONFIG");
    let _config_dir = EnvVarGuard::remove("VIBEWINDOW_CONFIG_DIR");

    assert!(gateway_client().is_ok());
}

#[test]
fn load_tools_list_via_gateway_returns_normalized_tools() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let home = tempfile::tempdir().expect("temp home should be created");
    let (port, join) = spawn_http_stub(
        |request| {
            assert!(request.starts_with("GET /v1/tools "));
            (
                "200 OK",
                json!({
                    "items": [
                        {
                            "identity": {"namespace": null, "name": "shell"},
                            "source": "builtin",
                            "exposure": "direct",
                            "id": "shell",
                            "display_name": "Shell",
                            "description": "Run shell commands",
                            "input_schema": {}
                        },
                        {
                            "identity": {"namespace": null, "name": "file_read"},
                            "source": "builtin",
                            "exposure": "direct",
                            "id": "file_read",
                            "display_name": "Read",
                            "description": "Read files",
                            "input_schema": {}
                        },
                        {
                            "identity": {"namespace": null, "name": "shell"},
                            "source": "builtin",
                            "exposure": "direct",
                            "id": "shell",
                            "display_name": "Shell",
                            "description": "Run shell commands",
                            "input_schema": {}
                        }
                    ]
                })
                .to_string(),
            )
        },
        1,
    );
    write_gateway_bootstrap_config(home.path(), &active_gateway_config(port));
    let _home = EnvVarGuard::set("HOME", home.path());
    let _config = EnvVarGuard::remove("VIBEWINDOW_CONFIG");
    let _config_dir = EnvVarGuard::remove("VIBEWINDOW_CONFIG_DIR");

    let tools = load_tools_list_via_gateway();
    join.join().expect("stub should stop");

    assert_eq!(tools, vec!["file_read".to_string(), "shell".to_string()]);
}

#[test]
fn load_tools_list_via_gateway_returns_empty_vec_on_request_error() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let home = tempfile::tempdir().expect("temp home should be created");
    let (port, join) =
        spawn_http_stub(|_| ("500 Internal Server Error", json!({"error": "boom"}).to_string()), 1);
    write_gateway_bootstrap_config(home.path(), &active_gateway_config(port));
    let _home = EnvVarGuard::set("HOME", home.path());
    let _config = EnvVarGuard::remove("VIBEWINDOW_CONFIG");
    let _config_dir = EnvVarGuard::remove("VIBEWINDOW_CONFIG_DIR");

    let tools = load_tools_list_via_gateway();
    join.join().expect("stub should stop");

    assert!(tools.is_empty());
}
