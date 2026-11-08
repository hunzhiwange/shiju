//! 验证桌面配置的默认值、文件转换和兼容性处理。
//!
//! 临时目录中的桌面配置覆盖缺失、旧版与完整文件，确认默认合并不丢字段。

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use tempfile::TempDir;
use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};

const SOURCE: &str = include_str!("config_desktop.rs");

#[derive(Debug, Clone)]
pub(crate) struct RecordedRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) body: String,
}

pub(crate) struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
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

pub(crate) struct TestServer {
    pub(crate) port: u16,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handle: Option<JoinHandle<()>>,
}

impl TestServer {
    pub(crate) fn start(
        expected_requests: usize,
        handler: impl Fn(&RecordedRequest) -> (u16, &'static str) + Send + Sync + 'static,
    ) -> Self {
        Self::start_matching(expected_requests, |_| true, handler)
    }

    pub(crate) fn start_matching(
        expected_requests: usize,
        matches: impl Fn(&RecordedRequest) -> bool + Send + 'static,
        handler: impl Fn(&RecordedRequest) -> (u16, &'static str) + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let port = listener.local_addr().expect("test server address should be available").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let handler = Arc::new(handler);
        let handle = std::thread::spawn(move || {
            let mut remaining = expected_requests;
            for stream in listener.incoming() {
                let mut stream = stream.expect("request stream should open");
                let request = read_http_request(&mut stream);
                if !matches(&request) {
                    write_http_response(&mut stream, 500, "{}");
                    continue;
                }
                let (status, body) = handler(&request);
                server_requests.lock().expect("requests lock should not be poisoned").push(request);
                write_http_response(&mut stream, status, body);
                remaining -= 1;
                if remaining == 0 {
                    break;
                }
            }
        });

        Self { port, requests, handle: Some(handle) }
    }

    pub(crate) fn take_requests(mut self) -> Vec<RecordedRequest> {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("test server should finish");
        }
        self.requests.lock().expect("requests lock should not be poisoned").clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.handle.take();
    }
}

fn read_http_request(stream: &mut TcpStream) -> RecordedRequest {
    let mut buffer = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let read = stream.read(&mut chunk).expect("request should be readable");
        assert!(read > 0, "request should contain headers");
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let header_end = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("request header terminator should exist")
        + 4;
    let header_text =
        String::from_utf8(buffer[..header_end].to_vec()).expect("headers should be utf8");
    let mut lines = header_text.lines();
    let request_line = lines.next().expect("request line should exist");
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().expect("method should exist").to_string();
    let path = request_parts.next().expect("path should exist").to_string();
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content length should parse"))
        })
        .unwrap_or(0);

    while buffer.len() < header_end + content_length {
        let read = stream.read(&mut chunk).expect("body should be readable");
        assert!(read > 0, "request body ended early");
        buffer.extend_from_slice(&chunk[..read]);
    }

    let body = String::from_utf8(buffer[header_end..header_end + content_length].to_vec())
        .expect("body should be utf8");
    RecordedRequest { method, path, body }
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = if status == 200 { "OK" } else { "ERROR" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("response should write");
}

fn env_lock() -> MutexGuard<'static, ()> {
    crate::app::TEST_PROCESS_STATE_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

pub(crate) fn temp_home() -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let lock = env_lock();
    let home = tempfile::tempdir().expect("temp home should be created");
    let guard = EnvGuard::set_os("HOME", home.path().as_os_str());
    (lock, home, guard)
}

fn gateway_config(port: u16) -> GatewayClientSystemSettingsConfig {
    let mut config = GatewayClientSystemSettingsConfig::default();
    config.set_servers(
        vec![GatewayClientServerConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            bearer_token: "test-token".to_string(),
            ..GatewayClientServerConfig::default()
        }],
        "test".to_string(),
    );
    config
}

fn configure_gateway(port: u16) -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let home = temp_home();
    set_gateway_port(port);
    home
}

pub(crate) fn set_gateway_port(port: u16) {
    super::super::system_settings::save_gateway_client_bootstrap_config(&gateway_config(port));
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

#[test]
fn config_desktop_tests_keeps_planned_coverage_targets() {
    for name in [
        "load_app_config_async",
        "save_app_config_async",
        "update_agents_compat_registry_result_async",
        "update_agents_compat_registry_async",
        "load_app_config",
        "save_app_config",
        "set_config_field",
        "load_project_chat_preferences",
        "save_project_chat_preferences",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
fn load_app_config_async_returns_object_preferences() {
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/desktop/preferences");
        (200, r#"{"agent":{"enabled":true}}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    let value = super::super::gateway::run_gateway_call(super::load_app_config_async())
        .expect("preferences should load");

    assert_eq!(value, serde_json::json!({"agent":{"enabled":true}}));
    assert_eq!(server.take_requests().len(), 1);
}

#[test]
fn load_app_config_async_replaces_non_object_preferences() {
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/desktop/preferences");
        (200, r#"["invalid"]"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    let value = super::super::gateway::run_gateway_call(super::load_app_config_async())
        .expect("preferences should load");

    assert_eq!(value, serde_json::json!({}));
    assert_eq!(server.take_requests().len(), 1);
}

#[test]
fn load_app_config_falls_back_to_empty_object_on_gateway_error() {
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/desktop/preferences");
        (500, r#"{"error":"unavailable"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    let value = super::load_app_config();

    assert_eq!(value, serde_json::json!({}));
    assert_eq!(server.take_requests().len(), 1);
}

#[test]
fn save_app_config_async_sends_object_patch() {
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.path, "/v1/desktop/preferences");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
            serde_json::json!({"theme":"dark"})
        );
        (200, r#"{"theme":"dark"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    super::super::gateway::run_gateway_call(super::save_app_config_async(serde_json::json!({
        "theme": "dark"
    })))
    .expect("preferences should save");

    assert_eq!(server.take_requests().len(), 1);
}

#[test]
fn save_app_config_async_replaces_non_object_patch() {
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.path, "/v1/desktop/preferences");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
            serde_json::json!({})
        );
        (200, r#"{}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    super::super::gateway::run_gateway_call(super::save_app_config_async(serde_json::json!(42)))
        .expect("preferences should save");

    assert_eq!(server.take_requests().len(), 1);
}

#[test]
fn update_agents_compat_registry_result_async_creates_agent_section() {
    let server =
        TestServer::start(2, |request| match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/desktop/preferences") => (200, r#"{"agent":"legacy"}"#),
            ("PATCH", "/v1/desktop/preferences") => {
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                    serde_json::json!({"agent":{"compat_registry":"enabled"}})
                );
                (200, r#"{"agent":{"compat_registry":"enabled"}}"#)
            }
            _ => (404, r#"{"error":"unexpected"}"#),
        });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    super::super::gateway::run_gateway_call(super::update_agents_compat_registry_result_async(
        |agent| {
            agent.insert("compat_registry".to_string(), serde_json::json!("enabled"));
        },
    ))
    .expect("agent registry should update");

    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn project_chat_preferences_async_round_trips_gateway_shape() {
    let server = TestServer::start(2, |request| match request.method.as_str() {
        "GET" => {
            assert_eq!(request.path, "/v1/desktop/project-preferences?project_path=%2Ftmp%2Fdemo");
            (200, r#"{"model":"openai/gpt-5","auto_model":true,"acp_agent":"coder"}"#)
        }
        "PUT" => {
            assert_eq!(request.path, "/v1/desktop/project-preferences?project_path=%2Ftmp%2Fdemo");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                serde_json::json!({"model":"local/model","auto_model":false,"acp_agent":null})
            );
            (200, r#"{}"#)
        }
        _ => (404, r#"{"error":"unexpected"}"#),
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    let loaded = super::super::gateway::run_gateway_call(
        super::load_project_chat_preferences_async("/tmp/demo"),
    )
    .expect("project preferences should load");
    super::super::gateway::run_gateway_call(super::save_project_chat_preferences_async(
        "/tmp/demo",
        "local/model",
        false,
        None,
    ))
    .expect("project preferences should save");

    assert_eq!(loaded, Some(("openai/gpt-5".to_string(), true, Some("coder".to_string()))));
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn tool_content_wrappers_use_expected_tool_types() {
    let server = TestServer::start(3, |request| match request.path.as_str() {
        "/v1/desktop/tool-content/json" => {
            assert_eq!(request.method, "PUT");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                serde_json::json!({"content":"{}"})
            );
            (200, r#"{}"#)
        }
        "/v1/desktop/tool-content/sql" => {
            assert_eq!(request.method, "PUT");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                serde_json::json!({"content":"select 1"})
            );
            (200, r#"{}"#)
        }
        "/v1/desktop/tool-content/html" => {
            assert_eq!(request.method, "PUT");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                serde_json::json!({"content":"<main></main>"})
            );
            (200, r#"{}"#)
        }
        _ => (404, r#"{"error":"unexpected"}"#),
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    super::super::gateway::run_gateway_call(super::save_json_tool_content_async("{}"))
        .expect("json tool content should save");
    super::super::gateway::run_gateway_call(super::save_sql_tool_content_async("select 1"))
        .expect("sql tool content should save");
    super::super::gateway::run_gateway_call(super::save_html_tool_content_async("<main></main>"))
        .expect("html tool content should save");

    assert_eq!(server.take_requests().len(), 3);
}

#[test]
fn update_agents_compat_registry_result_async_preserves_existing_preferences() {
    let server =
        TestServer::start(2, |request| match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/desktop/preferences") => {
                (200, r#"{"theme":"dark","agent":{"mode":"existing"}}"#)
            }
            ("PATCH", "/v1/desktop/preferences") => {
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                    serde_json::json!({
                        "theme": "dark",
                        "agent": {
                            "mode": "existing",
                            "compat_registry": "enabled"
                        }
                    })
                );
                (200, r#"{}"#)
            }
            _ => (404, r#"{"error":"unexpected"}"#),
        });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    super::super::gateway::run_gateway_call(super::update_agents_compat_registry_result_async(
        |agent| {
            agent.insert("compat_registry".to_string(), serde_json::json!("enabled"));
        },
    ))
    .expect("agent registry should update");

    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn update_agents_compat_registry_result_async_skips_update_after_load_error() {
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/desktop/preferences");
        (500, r#"{"error":"unavailable"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let mut update_called = false;

    let result = super::super::gateway::run_gateway_call(
        super::update_agents_compat_registry_result_async(|_| update_called = true),
    );

    assert!(result.is_err());
    assert!(!update_called);
    assert_eq!(server.take_requests().len(), 1);
}

#[test]
fn set_config_field_preserves_other_preferences() {
    let server =
        TestServer::start(2, |request| match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/desktop/preferences") => {
                (200, r#"{"theme":"light","show_terminal":false}"#)
            }
            ("PATCH", "/v1/desktop/preferences") => {
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                    serde_json::json!({"theme":"light","show_terminal":true})
                );
                (200, r#"{}"#)
            }
            _ => (404, r#"{"error":"unexpected"}"#),
        });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    super::set_config_field("show_terminal", serde_json::json!(true));

    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn project_chat_preferences_handles_empty_model_and_sync_save() {
    let server = TestServer::start(2, |request| match request.method.as_str() {
        "GET" => {
            assert_eq!(request.path, "/v1/desktop/project-preferences?project_path=%2Ftmp%2Fempty");
            (200, r#"{"model":"  ","auto_model":true,"acp_agent":"coder"}"#)
        }
        "PUT" => {
            assert_eq!(request.path, "/v1/desktop/project-preferences?project_path=%2Ftmp%2Fempty");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                serde_json::json!({
                    "model": "local/model",
                    "auto_model": true,
                    "acp_agent": "reviewer"
                })
            );
            (200, r#"{}"#)
        }
        _ => (404, r#"{"error":"unexpected"}"#),
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    let loaded = super::load_project_chat_preferences("/tmp/empty");
    super::save_project_chat_preferences("/tmp/empty", "local/model", true, Some("reviewer"));

    assert_eq!(loaded, None);
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn tool_content_sync_wrappers_use_expected_routes_and_values() {
    let server =
        TestServer::start(6, |request| match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/desktop/tool-content/json") => (200, r#"{"content":"{\"ready\":true}"}"#),
            ("GET", "/v1/desktop/tool-content/sql") => (200, r#"{"content":"select 1"}"#),
            ("GET", "/v1/desktop/tool-content/html") => {
                (200, r#"{"content":"<main>ready</main>"}"#)
            }
            ("PUT", "/v1/desktop/tool-content/json") => {
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                    serde_json::json!({"content":"{}"})
                );
                (200, r#"{}"#)
            }
            ("PUT", "/v1/desktop/tool-content/sql") => {
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                    serde_json::json!({"content":"select 2"})
                );
                (200, r#"{}"#)
            }
            ("PUT", "/v1/desktop/tool-content/html") => {
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
                    serde_json::json!({"content":"<main>saved</main>"})
                );
                (200, r#"{}"#)
            }
            _ => (404, r#"{"error":"unexpected"}"#),
        });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    assert_eq!(super::load_json_tool_content(), r#"{"ready":true}"#);
    assert_eq!(super::load_sql_tool_content(), "select 1");
    assert_eq!(super::load_html_tool_content(), "<main>ready</main>");
    super::save_json_tool_content("{}");
    super::save_sql_tool_content("select 2");
    super::save_html_tool_content("<main>saved</main>");

    assert_eq!(server.take_requests().len(), 6);
}

#[test]
fn synchronous_wrappers_fall_back_or_absorb_gateway_errors() {
    let server = TestServer::start(5, |request| {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/desktop/project-preferences?project_path=%2Ftmp%2Fmissing")
            | ("GET", "/v1/desktop/tool-content/json")
            | ("PATCH", "/v1/desktop/preferences")
            | ("PUT", "/v1/desktop/project-preferences?project_path=%2Ftmp%2Fmissing")
            | ("PUT", "/v1/desktop/tool-content/sql") => {}
            _ => panic!("unexpected request: {request:?}"),
        }
        (500, r#"{"error":"unavailable"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);

    assert_eq!(super::load_project_chat_preferences("/tmp/missing"), None);
    assert_eq!(super::load_json_tool_content(), "");
    super::save_app_config(&serde_json::json!({"theme":"dark"}));
    super::save_project_chat_preferences("/tmp/missing", "model", false, None);
    super::save_sql_tool_content("select 1");

    assert_eq!(server.take_requests().len(), 5);
}

#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_project_preferences_are_safe_noops() {
    assert_eq!(super::load_project_chat_preferences("/project"), None);
    super::save_project_chat_preferences("/project", "model", true, Some("agent"));
}

#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_sync_app_config_uses_safe_empty_fallback() {
    assert_eq!(super::load_app_config(), serde_json::json!({}));
    super::save_app_config(&serde_json::json!({"theme":"dark"}));
    super::set_config_field("theme", serde_json::json!("light"));
}

#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_tool_content_wrappers_are_safe_empty_noops() {
    assert_eq!(super::load_json_tool_content(), "");
    assert_eq!(super::load_sql_tool_content(), "");
    assert_eq!(super::load_html_tool_content(), "");
    super::save_json_tool_content("{}");
    super::save_sql_tool_content("select 1");
    super::save_html_tool_content("<main></main>");
}

#[test]
fn app_update_source_patch_and_restart_read_use_gateway_preferences() {
    use vw_shared::update::UpdateSource;
    let stored = Arc::new(Mutex::new(serde_json::json!({"theme":"dark"})));
    let server_state = stored.clone();
    let server = TestServer::start(2, move |request| {
        assert_eq!(request.path, "/v1/desktop/preferences");
        if request.method == "PATCH" {
            let patch: serde_json::Value = serde_json::from_str(&request.body).unwrap();
            assert_eq!(patch, serde_json::json!({"app_update_source":"github"}));
            server_state.lock().unwrap()["app_update_source"] = patch["app_update_source"].clone();
        } else {
            assert_eq!(request.method, "GET");
            assert_eq!(server_state.lock().unwrap()["app_update_source"], "github");
        }
        (200, r#"{"app_update_source":"github","theme":"dark"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    super::super::gateway::run_gateway_call(super::save_app_config_async(
        crate::app::desktop_update::source_patch(UpdateSource::Github),
    ))
    .unwrap();
    let config = super::load_app_config();
    assert_eq!(
        crate::app::desktop_update::DesktopUpdate::from_config(&config).source,
        Some(UpdateSource::Github)
    );
    assert_eq!(config["theme"], "dark");
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn app_update_source_gateway_save_failure_reaches_unsaved_toast() {
    use crate::app::{Message, message::ViewMessage};
    use iced_runtime::futures::futures::StreamExt;
    use vw_shared::update::UpdateSource;
    let mut app = crate::app::App::new().0;
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.path, "/v1/desktop/preferences");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
            serde_json::json!({"app_update_source":"github"})
        );
        (500, r#"{"error":"preferences unavailable"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let task = crate::app::message::view::update(
        &mut app,
        ViewMessage::AppUpdateSourceSelected(UpdateSource::Github),
    );
    let result = super::super::gateway::run_gateway_call(async {
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        match stream.next().await {
            Some(iced_runtime::Action::Output(Message::View(
                ViewMessage::AppUpdateSourceSaved(result),
            ))) => Ok(result),
            _ => Err("missing persistence result".into()),
        }
    })
    .unwrap();
    assert!(result.is_err());
    let _ = crate::app::message::view::update(&mut app, ViewMessage::AppUpdateSourceSaved(result));
    assert_eq!(app.desktop_update.source, Some(UpdateSource::Github));
    assert!(app.active_toast.as_ref().unwrap().message.contains("未保存"));
    assert_eq!(server.take_requests().len(), 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn preference_save_failure_keeps_new_edits_and_preserves_unrelated_fields() {
    use crate::app::{App, Message};
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::{Value, json};
    use std::time::Duration;

    let mut app = App::new().0;
    let stored = Arc::new(Mutex::new(json!({"theme": "dark", "obsolete": true})));
    let server_state = stored.clone();
    let server = TestServer::start(2, move |request| {
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.path, "/v1/desktop/preferences");
        let fields: Value = serde_json::from_str(&request.body).unwrap();
        if fields == json!({"show_settings": true}) {
            return (500, r#"{"error":"preference rejected"}"#);
        }
        assert_eq!(fields, json!({"show_settings": false, "obsolete": null}));
        let mut stored = server_state.lock().unwrap();
        let stored = stored.as_object_mut().unwrap();
        for (key, value) in fields.as_object().unwrap() {
            if value.is_null() {
                stored.remove(key);
            } else {
                stored.insert(key.clone(), value.clone());
            }
        }
        (200, "{}")
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let receive_save = |task| {
        super::super::gateway::run_gateway_call(async {
            let mut actions = iced_runtime::task::into_stream(task).expect("save task");
            match tokio::time::timeout(Duration::from_secs(4), actions.next()).await {
                Ok(Some(iced_runtime::Action::Output(
                    message @ Message::DesktopPreferencesSaved(..),
                ))) => Ok(message),
                _ => Err("missing desktop preference save result".to_string()),
            }
        })
        .unwrap()
    };

    let first = app.save_desktop_preference("show_settings", true.into());
    let queued = app.save_desktop_preferences(
        json!({"show_settings": false, "obsolete": null}).as_object().unwrap().clone(),
    );
    assert!(iced_runtime::task::into_stream(queued).is_none());
    let failed = receive_save(first);
    assert!(matches!(&failed, Message::DesktopPreferencesSaved(_, Err(_))));
    let second = app.update(failed);
    assert!(app.active_toast.as_ref().unwrap().message.contains("未保存"));

    let saved = receive_save(second);
    assert!(matches!(&saved, Message::DesktopPreferencesSaved(_, Ok(()))));
    let done = app.update(saved);
    assert!(iced_runtime::task::into_stream(done).is_none());
    assert_eq!(*stored.lock().unwrap(), json!({"theme": "dark", "show_settings": false}));
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn preference_writes_are_serialized_per_endpoint_and_tenant() {
    use super::{DesktopPreferencesPersistence, GatewayEndpoint, TenantScope};
    use serde_json::Map;

    let mut queue = DesktopPreferencesPersistence::default();
    let endpoint = GatewayEndpoint::new("localhost", 7777);
    let same_endpoint = GatewayEndpoint::new("LOCALHOST", 7777);
    let other_endpoint = GatewayEndpoint::new("localhost", 7778);
    let other_tenant =
        endpoint.clone().with_tenant_scope(TenantScope::new("test", "other").unwrap());
    let fields = || Map::from_iter([("show_settings".into(), true.into())]);
    let has_task = |task| iced_runtime::task::into_stream(task).is_some();

    assert!(has_task(queue.enqueue(endpoint, fields())));
    assert!(!has_task(queue.enqueue(same_endpoint, fields())));
    assert!(has_task(queue.enqueue(other_endpoint, fields())));
    assert!(has_task(queue.enqueue(other_tenant, fields())));
}

async fn collect_preference_messages(
    task: iced::Task<crate::app::Message>,
) -> Vec<crate::app::Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        return Vec::new();
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut messages = Vec::new();
        while let Some(action) = stream.next().await {
            if let iced_runtime::Action::Output(message) = action {
                messages.push(message);
            }
        }
        messages
    })
    .await
    .expect("preference tasks should finish")
}

#[test]
fn project_puts_keep_the_original_gateway_and_drain_after_a_failed_save() {
    use crate::app::{App, Message, message::ChatMessage};
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (_lock, _home, _guard) = temp_home();
    let (mut app, _) = App::new();
    let writes = AtomicUsize::new(0);
    let first_gateway = TestServer::start(4, move |request| match request.method.as_str() {
        "PATCH" => (200, "{}"),
        "PUT" if writes.fetch_add(1, Ordering::SeqCst) == 0 => (500, "{}"),
        "PUT" => (200, "{}"),
        _ => panic!("unexpected desktop request {request:?}"),
    });
    let second_gateway = TestServer::start(1, |request| {
        assert_eq!(request.method, "PATCH");
        (200, "{}")
    });
    set_gateway_port(first_gateway.port);
    app.project_path = Some("/repo/a".to_string());

    let first = app.update(Message::Chat(ChatMessage::ModelSelected("first".into())));
    let next = app.update(Message::Chat(ChatMessage::ModelSelected("last".into())));
    let idle = app.desktop_preferences_persistence.wait_for_idle().unwrap();
    assert!(!*idle.borrow());

    set_gateway_port(second_gateway.port);
    app.project_path = None;
    let other = app.update(Message::Chat(ChatMessage::ModelSelected("other".into())));
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        assert!(collect_preference_messages(next).await.is_empty());
        let messages = collect_preference_messages(other).await;
        assert_eq!(messages.len(), 1);
        for message in messages {
            let _ = app.update(message);
        }
        assert!(!*idle.borrow());
        let messages = collect_preference_messages(first).await;
        assert_eq!(messages.len(), 2);
        let follow_up = iced::Task::batch(messages.into_iter().map(|message| app.update(message)));
        assert!(!*idle.borrow());
        assert!(app.active_toast.as_ref().unwrap().message.contains("项目聊天偏好未保存"));
        let messages = collect_preference_messages(follow_up).await;
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(
                    message,
                    Message::DesktopPreferencesSaved(..) | Message::ProjectChatPreferencesSaved(..)
                ))
                .count(),
            2
        );
        for message in messages {
            let _ = app.update(message);
        }
    });

    assert!(*idle.borrow());
    assert_eq!(app.model, "other");
    let requests = first_gateway.take_requests();
    let projects: Vec<_> = requests.iter().filter(|request| request.method == "PUT").collect();
    assert_eq!(projects.len(), 2);
    for (request, model) in projects.into_iter().zip(["first", "last"]) {
        assert_eq!(request.path, "/v1/desktop/project-preferences?project_path=%2Frepo%2Fa");
        assert_eq!(
            serde_json::from_str::<Value>(&request.body).unwrap(),
            json!({"model":model,"auto_model":false,"acp_agent":null})
        );
    }
    assert_eq!(second_gateway.take_requests().len(), 1);
}

#[test]
fn reasoning_and_send_preferences_preserve_unrelated_sessions_and_remove_legacy_key() {
    use crate::app::{
        App, Message,
        message::{ChatMessage, ViewMessage},
        state::ChatSendBehavior,
    };
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::{Value, json};
    use std::sync::mpsc;
    use std::time::Duration;
    use vw_gateway_client::vw_api_types::ReasoningEffortDto;

    let (_lock, _home, _guard) = temp_home();
    let (mut app, _) = App::new();
    let (release_send_save, allow_send_save) = mpsc::channel();
    let allow_send_save = Mutex::new(allow_send_save);
    let server = TestServer::start(3, move |request| {
        assert_eq!(request.path, "/v1/desktop/preferences");
        match request.method.as_str() {
            "GET" => (200, r#"{"session_reasoning_efforts":{"unopened":"future","one":"high"}}"#),
            "PATCH" => {
                if request.body.contains("chat_send_behavior") {
                    allow_send_save.lock().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
                }
                (200, "{}")
            }
            _ => panic!("unexpected desktop request {request:?}"),
        }
    });
    set_gateway_port(server.port);
    app.active_session_id = Some("active".into());
    app.current_session_runtime_mut().is_requesting = true;
    let behavior =
        app.update(Message::View(ViewMessage::SelectChatSendBehavior(ChatSendBehavior::Queue)));
    app.active_session_id = Some("one".into());
    let _ = app.update(Message::Chat(ChatMessage::ReasoningEffortSelected(None)));
    app.active_session_id = Some("two".into());
    let _ = app.update(Message::Chat(ChatMessage::ReasoningEffortSelected(Some(
        ReasoningEffortDto::High,
    ))));

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut actions = iced_runtime::task::into_stream(behavior).unwrap();
        let send = tokio::time::timeout(Duration::from_secs(5), actions.next()).await.unwrap();
        assert!(matches!(
            send,
            Some(iced_runtime::Action::Output(Message::Chat(ChatMessage::SendPressed)))
        ));
        release_send_save.send(()).unwrap();
        let saved = tokio::time::timeout(Duration::from_secs(5), actions.next()).await.unwrap();
        let Some(iced_runtime::Action::Output(save @ Message::DesktopPreferencesSaved(..))) = saved
        else {
            panic!("missing send preference save completion");
        };
        let reasoning = app.update(save);
        let messages = collect_preference_messages(reasoning).await;
        assert_eq!(messages.len(), 1);
        let _ = app.update(messages.into_iter().next().unwrap());
    });
    let requests = server.take_requests();
    assert_eq!(
        requests.iter().map(|req| req.method.as_str()).collect::<Vec<_>>(),
        ["PATCH", "GET", "PATCH"]
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[0].body).unwrap(),
        json!({"chat_send_behavior":"queue","dialogue_flow_follow_up_behavior":null})
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap(),
        json!({"session_reasoning_efforts":{"unopened":"future","one":null,"two":"high"}})
    );
}
