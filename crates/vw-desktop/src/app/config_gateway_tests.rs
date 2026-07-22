use super::{
    apply_main_agent_overrides, gateway_blocking_runtime, gateway_client, gateway_client_endpoint,
    load_config_value_at_path, load_tools_list_via_gateway, load_vibewindow_root_json,
    normalize_gateway_host, normalize_identity_format, normalize_tool_ids,
    project_config_override_path, run_gateway_call, server_config_unreachable_error,
    set_config_value_at_path, should_attempt_tools_list_request,
};
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use vw_config_types::agent::AgentDefinitionConfig;
use vw_config_types::config::Config;
use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
use vw_gateway_client::GatewayEndpoint;

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
    vw_config_types::paths::home_config_dir(home).join("gateway-client-bootstrap.json")
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
fn normalize_identity_format_always_returns_openclaw() {
    assert_eq!(normalize_identity_format(None), "openclaw");
    assert_eq!(normalize_identity_format(Some(" aieos ")), "openclaw");
}

#[test]
fn normalize_gateway_host_trims_and_defaults_empty_values() {
    assert_eq!(normalize_gateway_host(" gateway.internal "), "gateway.internal");
    assert_eq!(normalize_gateway_host(" \t "), "127.0.0.1");
}

#[test]
fn apply_main_agent_overrides_uses_non_empty_main_agent_fields() {
    let mut cfg = Config::default();
    let mut main = AgentDefinitionConfig {
        provider: " openai ".to_string(),
        model: " gpt-5 ".to_string(),
        temperature: Some(0.2),
        identity_format: Some("custom".to_string()),
        ..AgentDefinitionConfig::default()
    };
    cfg.agents.insert("main".to_string(), main.clone());

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("openai"));
    assert_eq!(cfg.default_model.as_deref(), Some("openai/gpt-5"));
    assert_eq!(cfg.default_temperature, 0.2);
    assert_eq!(cfg.identity.format, "openclaw");

    main.provider = " ".to_string();
    main.model = "ignored".to_string();
    main.temperature = None;
    main.identity_format = None;
    cfg.default_provider = Some("previous".to_string());
    cfg.default_model = Some("previous/model".to_string());
    cfg.default_temperature = 0.9;
    cfg.agents.insert("main".to_string(), main);

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("previous"));
    assert_eq!(cfg.default_model.as_deref(), Some("previous/model"));
    assert_eq!(cfg.default_temperature, 0.9);
}

#[test]
fn apply_main_agent_overrides_noops_without_main_agent() {
    let mut cfg = Config::default();
    cfg.default_provider = Some("kept".to_string());
    cfg.default_model = Some("kept/model".to_string());

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("kept"));
    assert_eq!(cfg.default_model.as_deref(), Some("kept/model"));
}

#[test]
fn apply_main_agent_overrides_updates_provider_without_erasing_model_for_blank_model() {
    let mut cfg = Config::default();
    cfg.default_provider = Some("old-provider".to_string());
    cfg.default_model = Some("old-provider/old-model".to_string());
    cfg.agents.insert(
        "main".to_string(),
        AgentDefinitionConfig {
            provider: " new-provider ".to_string(),
            model: " \t ".to_string(),
            ..AgentDefinitionConfig::default()
        },
    );

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("new-provider"));
    assert_eq!(cfg.default_model.as_deref(), Some("old-provider/old-model"));
}

#[test]
fn apply_main_agent_overrides_applies_independent_fields_for_blank_provider() {
    let mut cfg = Config::default();
    cfg.default_provider = Some("kept-provider".to_string());
    cfg.default_model = Some("kept-provider/kept-model".to_string());
    cfg.identity.format = "aieos".to_string();
    cfg.identity.aieos_path = Some("identity.json".to_string());
    cfg.identity.aieos_inline = Some("inline identity".to_string());
    cfg.agents.insert(
        "main".to_string(),
        AgentDefinitionConfig {
            provider: " ".to_string(),
            model: "ignored".to_string(),
            temperature: Some(0.45),
            identity_format: Some("aieos".to_string()),
            ..AgentDefinitionConfig::default()
        },
    );

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("kept-provider"));
    assert_eq!(cfg.default_model.as_deref(), Some("kept-provider/kept-model"));
    assert_eq!(cfg.default_temperature, 0.45);
    assert_eq!(cfg.identity.format, "openclaw");
    assert!(cfg.identity.aieos_path.is_none());
    assert!(cfg.identity.aieos_inline.is_none());
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
fn set_config_value_at_path_replaces_root_when_path_is_empty() {
    let mut root = json!({"old": true});

    set_config_value_at_path(&mut root, &[], json!(["new"]));

    assert_eq!(root, json!(["new"]));
}

#[test]
fn set_config_value_at_path_creates_missing_objects_and_overwrites_scalars() {
    let mut root = json!({"app_ui": "not-object"});

    set_config_value_at_path(
        &mut root,
        &["app_ui", "system_settings", "gateway_client", "host"],
        json!("gateway.internal"),
    );

    assert_eq!(
        root,
        json!({"app_ui": {"system_settings": {"gateway_client": {"host": "gateway.internal"}}}})
    );
}

#[test]
fn set_config_value_at_path_replaces_non_object_root_with_object() {
    let mut root = json!(["not", "an", "object"]);

    set_config_value_at_path(&mut root, &["gateway"], json!("enabled"));

    assert_eq!(root, json!({"gateway": "enabled"}));
}

#[test]
fn set_config_value_at_path_preserves_existing_objects_and_siblings() {
    let mut root = json!({
        "app_ui": {
            "theme": "dark",
            "system_settings": {
                "gateway_client": {"host": "old", "port": 42618}
            }
        }
    });

    set_config_value_at_path(
        &mut root,
        &["app_ui", "system_settings", "gateway_client", "host"],
        json!("new"),
    );

    assert_eq!(root["app_ui"]["theme"], json!("dark"));
    assert_eq!(root["app_ui"]["system_settings"]["gateway_client"]["host"], json!("new"));
    assert_eq!(root["app_ui"]["system_settings"]["gateway_client"]["port"], json!(42618));
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
fn server_config_unreachable_error_omits_blank_details() {
    assert_eq!(
        server_config_unreachable_error(" \t "),
        "服务端配置不可达，请检查 Gateway 连接状态。"
    );
    assert_eq!(
        server_config_unreachable_error("network down"),
        "服务端配置不可达，请检查 Gateway 连接状态。network down"
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
fn should_attempt_tools_list_request_allows_optional_skey() {
    assert!(should_attempt_tools_list_request(&GatewayEndpoint::new("gateway.internal", 1)));
    assert!(should_attempt_tools_list_request(&GatewayEndpoint::new("127.0.0.1", 1)));
}

#[test]
fn normalize_tool_ids_sorts_and_dedups() {
    let tools = normalize_tool_ids(vec![
        "bash".to_string(),
        String::new(),
        "file_read".to_string(),
        "bash".to_string(),
    ]);

    assert_eq!(tools, vec!["bash".to_string(), "file_read".to_string()]);
}

#[test]
fn normalize_tool_ids_drops_blank_entries_without_trimming_non_blank_ids() {
    let tools =
        normalize_tool_ids(vec![" zed ".to_string(), " \t ".to_string(), "alpha".to_string()]);

    assert_eq!(tools, vec![" zed ".to_string(), "alpha".to_string()]);
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
                            "id": "shell",
                            "display_name": "Shell",
                            "description": "Run shell commands",
                            "input_schema": {}
                        },
                        {
                            "id": "file_read",
                            "display_name": "Read",
                            "description": "Read files",
                            "input_schema": {}
                        },
                        {
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

#[test]
fn run_gateway_call_runs_without_existing_runtime() {
    let value = run_gateway_call(async { Ok::<_, String>(42) }).expect("future should succeed");

    assert_eq!(value, 42);
}

#[test]
fn run_gateway_call_runs_inside_current_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime should build");

    let value = runtime.block_on(async {
        run_gateway_call(async { Ok::<_, String>("ok".to_string()) })
            .expect("future should run on blocking runtime")
    });

    assert_eq!(value, "ok");
}

#[test]
fn run_gateway_call_runs_inside_multithread_runtime() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("multi-thread runtime should build");

    let value = runtime.block_on(async {
        run_gateway_call(async { Ok::<_, String>("ok".to_string()) })
            .expect("future should run through block_in_place")
    });

    assert_eq!(value, "ok");
}

#[test]
fn run_gateway_call_returns_future_error() {
    let err = run_gateway_call(async { Err::<(), _>("boom".to_string()) })
        .expect_err("future error should be returned");

    assert_eq!(err, "boom");
}

#[test]
fn gateway_blocking_runtime_reuses_successful_runtime() {
    let first = gateway_blocking_runtime().expect("runtime should build") as *const _;
    let second = gateway_blocking_runtime().expect("runtime should be reused") as *const _;

    assert_eq!(first, second);
}
