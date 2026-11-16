#![cfg(not(target_arch = "wasm32"))]
use crate::ConfigAccess;
use gateway_endpoint::GatewayEndpoint;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

fn config_access(port: u16) -> ConfigAccess {
    let endpoint = GatewayEndpoint::new("127.0.0.1", port)
        .with_auth(gateway_endpoint::GatewayAuth { skey: Some("test-token".into()) });
    ConfigAccess::new(endpoint, None)
}
#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
    body: String,
}

struct TestServer {
    port: u16,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handle: Option<JoinHandle<()>>,
}

impl TestServer {
    fn start(
        expected_requests: usize,
        handler: impl Fn(&RecordedRequest) -> (u16, &'static str) + Send + Sync + 'static,
    ) -> Self {
        Self::start_matching(expected_requests, |_| true, handler)
    }

    fn start_matching(
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

    fn take_requests(mut self) -> Vec<RecordedRequest> {
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

#[test]
fn load_app_config_async_returns_object_preferences() {
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/desktop/preferences");
        (200, r#"{"agent":{"enabled":true}}"#)
    });
    let access = config_access(server.port);

    let value = crate::access::run_gateway_call(access.load_app_config_async())
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
    let access = config_access(server.port);

    let value = crate::access::run_gateway_call(access.load_app_config_async())
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
    let access = config_access(server.port);

    let value = access.load_app_config();

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
    let access = config_access(server.port);

    crate::access::run_gateway_call(access.save_app_config_async(serde_json::json!({
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
    let access = config_access(server.port);

    crate::access::run_gateway_call(access.save_app_config_async(serde_json::json!(42)))
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
    let access = config_access(server.port);

    crate::access::run_gateway_call(access.update_agents_compat_registry_result_async(|agent| {
        agent.insert("compat_registry".to_string(), serde_json::json!("enabled"));
    }))
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
    let access = config_access(server.port);

    let loaded =
        crate::access::run_gateway_call(access.load_project_chat_preferences_async("/tmp/demo"))
            .expect("project preferences should load");
    crate::access::run_gateway_call(access.save_project_chat_preferences_async(
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
    let access = config_access(server.port);

    crate::access::run_gateway_call(access.save_json_tool_content_async("{}"))
        .expect("json tool content should save");
    crate::access::run_gateway_call(access.save_sql_tool_content_async("select 1"))
        .expect("sql tool content should save");
    crate::access::run_gateway_call(access.save_html_tool_content_async("<main></main>"))
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
    let access = config_access(server.port);

    crate::access::run_gateway_call(access.update_agents_compat_registry_result_async(|agent| {
        agent.insert("compat_registry".to_string(), serde_json::json!("enabled"));
    }))
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
    let access = config_access(server.port);
    let mut update_called = false;

    let result = crate::access::run_gateway_call(
        access.update_agents_compat_registry_result_async(|_| update_called = true),
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
    let access = config_access(server.port);

    access.set_config_field("show_terminal", serde_json::json!(true));

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
    let access = config_access(server.port);

    let loaded = access.load_project_chat_preferences("/tmp/empty");
    access.save_project_chat_preferences("/tmp/empty", "local/model", true, Some("reviewer"));

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
    let access = config_access(server.port);

    assert_eq!(access.load_json_tool_content(), r#"{"ready":true}"#);
    assert_eq!(access.load_sql_tool_content(), "select 1");
    assert_eq!(access.load_html_tool_content(), "<main>ready</main>");
    access.save_json_tool_content("{}");
    access.save_sql_tool_content("select 2");
    access.save_html_tool_content("<main>saved</main>");

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
    let access = config_access(server.port);

    assert_eq!(access.load_project_chat_preferences("/tmp/missing"), None);
    assert_eq!(access.load_json_tool_content(), "");
    access.save_app_config(&serde_json::json!({"theme":"dark"}));
    access.save_project_chat_preferences("/tmp/missing", "model", false, None);
    access.save_sql_tool_content("select 1");

    assert_eq!(server.take_requests().len(), 5);
}
