//! 验证网关配置的地址解析、默认值和环境覆盖规则。
//!
//! 地址样例覆盖主机、端口、协议和环境来源，非法输入必须走明确回退。

use super::{gateway_client, gateway_client_endpoint, load_config_value_at_path,
    load_tools_list_via_gateway, load_vibewindow_root_json, project_config_override_path};
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use config_ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};

#[test]
fn interactive_config_paths_reject_blocking_gateway_wrappers() {
    // Cover the entry points and persistence helpers migrated by the desktop config plan.
    // Session, Git and MCP operations have separate latency boundaries.
    let sources = [
        ("App::new", include_str!("impls/app_init/new.rs")),
        ("startup", include_str!("impls/app_init/startup.rs")),
        ("App::update", include_str!("impls/app_update.rs")),
        ("App::view", include_str!("impls/app_view.rs")),
        ("projects", include_str!("projects.rs")),
        ("project preferences", include_str!("config_project_chat.rs")),
        ("chat", include_str!("message/chat/session.rs")),
        ("layout", include_str!("message/view/layout.rs")),
        ("chat popovers", include_str!("message/view/panel/popovers.rs")),
        ("file tree", include_str!("message/project/file_tree.rs")),
        ("task board", include_str!("message/task_board/update.rs")),
        ("theme", include_str!("message/view/theme.rs")),
        ("general settings", include_str!("message/settings/general.rs")),
        ("dialogue flow", include_str!("message/settings/dialogue_flow.rs")),
        ("Gateway settings", include_str!("message/settings/gateway_client.rs")),
    ];
    let blocking_wrappers = [
        "run_gateway_call",
        "block_on_gateway",
        "block_on",
        "set_config_field",
        "save_app_config",
        "load_app_config",
        "load_project_chat_preferences",
        "save_project_chat_preferences",
        "load_system_settings_config",
        "update_system_settings_config",
        "load_full_agent_config",
        "load_full_agent_config_result",
        "load_global_acp_config_result",
        "load_enabled_acp_config_result",
        "load_gateway_config_result",
        "load_tools_list_via_gateway",
        "gateway_load_archived_session_ids",
    ];
    for (name, source) in sources {
        for token in source.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            assert!(!blocking_wrappers.contains(&token), "{name} uses blocking {token}");
        }
    }
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
fn project_config_override_path_uses_explicit_workspace() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", workspace.path());

    assert_eq!(project_config_override_path(), Some(workspace.path().join(".vibewindow.json")));
}

#[test]
fn project_config_override_path_ignores_legacy_user_config_env() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG", temp.path().join("vibewindow.json"));
    let _config_dir = EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().join("config"));

    assert_eq!(project_config_override_path(), Some(workspace.join(".vibewindow.json")));
}

#[test]
fn project_config_override_path_falls_back_to_current_dir_for_blank_workspace() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", " \t ");
    let current_dir = std::env::current_dir().expect("current dir should be available");

    assert_eq!(project_config_override_path(), Some(current_dir.join(".vibewindow.json")));
}

#[test]
fn load_vibewindow_root_json_reads_only_project_override() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    write_json(&workspace.join(".vibewindow.json"), json!({"from_project": true}));
    write_json(&temp.path().join("vibewindow.json"), json!({"from_legacy": true}));
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG", temp.path().join("vibewindow.json"));

    assert_eq!(load_vibewindow_root_json(), json!({"from_project": true}));
}

#[test]
fn load_vibewindow_root_json_returns_empty_object_without_readable_candidates() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", temp.path().join("missing"));

    assert_eq!(load_vibewindow_root_json(), json!({}));
}

#[test]
fn load_vibewindow_root_json_returns_empty_object_for_invalid_json() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    std::fs::write(workspace.join(".vibewindow.json"), "{not-json")
        .expect("invalid json fixture should be written");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);

    assert_eq!(load_vibewindow_root_json(), json!({}));
}

#[test]
fn load_config_value_at_path_deserializes_existing_value() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    let config = workspace.join(".vibewindow.json");
    write_json(
        &config,
        json!({"app_ui": {"system_settings": {"gateway_client": {"host": "gw", "port": 9000}}}}),
    );
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);

    let value = load_config_value_at_path::<GatewayClientSystemSettingsConfig>(&[
        "app_ui",
        "system_settings",
        "gateway_client",
    ])
    .expect("gateway client config should deserialize");

    assert_eq!(value.host, "gw");
    assert_eq!(value.port, 9000);
    assert!(load_config_value_at_path::<String>(&["missing"]).is_none());
    assert!(
        load_config_value_at_path::<u16>(&["app_ui", "system_settings", "gateway_client", "host"])
            .is_none()
    );
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
