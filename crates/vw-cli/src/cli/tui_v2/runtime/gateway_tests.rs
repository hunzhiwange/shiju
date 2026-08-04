use super::*;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use vw_gateway_client::GatewayChatStreamRequest;

fn unique_temp_config_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("vw-cli-gateway-{name}-{}-{nanos}.json", std::process::id()))
}

fn spawn_health_response_server(response: &'static [u8]) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind health response fixture");
    let port = listener.local_addr().expect("read fixture addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept health response client");
        let mut buffer = [0_u8; 512];
        let _ = stream.read(&mut buffer);
        stream.write_all(response).expect("write health response fixture");
    });
    (port, server)
}

fn spawn_stream_response_server(response_body: &'static str) -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stream response fixture");
    let port = listener.local_addr().expect("read stream fixture addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept stream response client");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 512];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).expect("read stream request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).expect("write stream response fixture");
        String::from_utf8(request).expect("stream request should be utf8")
    });
    (port, server)
}

fn spawn_json_sequence_server(
    responses: Vec<(&'static str, &'static str)>,
) -> (u16, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind JSON sequence fixture");
    let port = listener.local_addr().expect("read JSON fixture addr").port();
    let server = thread::spawn(move || {
        responses
            .into_iter()
            .map(|(status, body)| {
                let (mut stream, _) = listener.accept().expect("accept JSON fixture client");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 512];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = stream.read(&mut buffer).expect("read JSON fixture request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write JSON fixture response");
                String::from_utf8(request).expect("JSON fixture request should be utf8")
            })
            .collect()
    });
    (port, server)
}

fn runtime_for_port(port: u16) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port))
            .expect("client should construct"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")).with_id(Some("runtime-session".into())),
    )
}

#[test]
fn cd_command_rebinds_runtime_only_after_validated_project_resolution() {
    let workspace = tempfile::tempdir().expect("workspace");
    let file = workspace.path().join("not-a-directory");
    fs::write(&file, "x").expect("fixture file");
    let original = workspace.path().join("original");
    let target = workspace.path().join("target");
    fs::create_dir_all(&original).expect("original");
    fs::create_dir_all(&target).expect("target");

    let mut runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).expect("client"),
        GatewaySessionSeed::new(original.canonicalize().expect("canonical original")),
    );
    assert!(runtime.rebind_validated_directory(&file).is_err());
    assert_eq!(runtime.directory(), original.canonicalize().unwrap());
    assert!(runtime.rebind_validated_directory(&workspace.path().join("missing")).is_err());
    assert_eq!(runtime.directory(), original.canonicalize().unwrap());

    let canonical = runtime.rebind_validated_directory(&target).expect("valid directory");
    assert_eq!(canonical, target.canonicalize().unwrap());
    assert_eq!(runtime.directory(), canonical);
}

#[test]
fn gateway_health_probe_keeps_write_side_open_until_response_arrives() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind health probe fixture");
    let port = listener.local_addr().expect("read fixture addr").port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept health probe client");
        stream.set_read_timeout(Some(Duration::from_millis(50))).expect("set fixture read timeout");

        let mut request = Vec::new();
        let mut buffer = [0_u8; 512];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).expect("read health probe request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buffer[..read]);
        }

        let mut extra = [0_u8; 1];
        match stream.read(&mut extra) {
            Ok(0) => return,
            Ok(_) => {}
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(err) => panic!("unexpected fixture read error: {err}"),
        }

        stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
                )
                .expect("write health probe response");
    });

    let endpoint = GatewayEndpoint::new("127.0.0.1", port);

    assert!(gateway_health_ready(&endpoint));

    server.join().expect("join health probe fixture");
}

#[test]
fn gateway_health_probe_rejects_non_ok_status_and_missing_ok_body() {
    let (status_port, status_server) = spawn_health_response_server(
        b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
    );
    assert!(!gateway_health_ready(&GatewayEndpoint::new("127.0.0.1", status_port)));
    status_server.join().expect("join status fixture");

    let (body_port, body_server) = spawn_health_response_server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 21\r\nConnection: close\r\n\r\n{\"status\":\"starting\"}",
    );
    assert!(!gateway_health_ready(&GatewayEndpoint::new("127.0.0.1", body_port)));
    body_server.join().expect("join body fixture");

    let (empty_port, empty_server) = spawn_health_response_server(b"");
    assert!(!gateway_health_ready(&GatewayEndpoint::new("127.0.0.1", empty_port)));
    empty_server.join().expect("join empty response fixture");

    let (spaced_body_port, spaced_body_server) = spawn_health_response_server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n{\"status\": \"ok\"}",
    );
    assert!(gateway_health_ready(&GatewayEndpoint::new("127.0.0.1", spaced_body_port)));
    spaced_body_server.join().expect("join spaced body fixture");
}

#[test]
fn gateway_preflight_outcome_reports_started_flag() {
    assert!(!GatewayPreflightOutcome::Ready.started_gateway());
    assert!(GatewayPreflightOutcome::Started.started_gateway());
}

#[test]
fn bootstrap_config_defaults_and_json_mapping_are_stable() {
    let mut config = GatewayClientBootstrapConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, vw_config_types::gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(config.platform_id, "10000");
    assert_eq!(config.company_id, "100000000");
    assert!(config.endpoint().auth.is_none());

    config.apply_json_root(&serde_json::json!({
        "gateway_client": {
            "host": " gateway.internal ",
            "port": 8080,
            "username": " user ",
            "password": " pass ",
            "skey": " key ",
            "platform_id": "20000",
            "company_id": "200000001"
        }
    }));
    let endpoint = config.endpoint();
    assert_eq!(endpoint.normalized_host(), "gateway.internal");
    assert_eq!(endpoint.port, 8080);
    assert_eq!(endpoint.tenant_scope().platform_id().as_ref(), "20000");
    assert_eq!(endpoint.tenant_scope().company_id().as_ref(), "200000001");
    let auth = endpoint.auth.expect("auth should be attached");
    assert_eq!(auth.skey.as_deref(), Some("key"));

    config.apply_json_root(&serde_json::json!({
        "gateway_client": {
            "host": " ",
            "port": 0,
            "username": "",
            "password": "   ",
            "skey": null
        }
    }));
    assert_eq!(config.host, "gateway.internal");
    assert_eq!(config.port, 8080);
    assert_eq!(config.skey, None);
}

#[test]
fn gateway_health_probe_sends_tenant_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tenant health fixture");
    let port = listener.local_addr().expect("read tenant fixture addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept tenant health probe");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 512];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).expect("read tenant health request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
            )
            .expect("write tenant health response");
        String::from_utf8(request).expect("tenant health request should be utf8")
    });
    let endpoint = GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(
        vw_gateway_client::vw_api_types::id::TenantScope::new("20000", "200000001").unwrap(),
    );

    assert!(gateway_health_ready(&endpoint));
    let request = server.join().expect("join tenant health fixture");
    assert!(request.contains("X-Platform-ID: 20000\r\n"));
    assert!(request.contains("X-Company-ID: 200000001\r\n"));
}

#[test]
fn bootstrap_config_load_reads_cache_and_falls_back_for_missing_or_invalid_config() {
    let config_dir = unique_temp_config_path("valid");
    fs::create_dir(&config_dir).expect("create gateway config dir");
    let bootstrap_path = config_dir.join("gateway-client-bootstrap.json");
    fs::write(
        &bootstrap_path,
        r#"{"host":" gateway.local ","port":65535,"bearer_token":" bearer "}"#,
    )
    .expect("write gateway config");

    let config = Config { config_path: config_dir.join("vibewindow.json"), ..Config::default() };
    let bootstrap = GatewayClientBootstrapConfig::load(&config);
    fs::remove_file(&bootstrap_path).expect("remove gateway config");

    assert_eq!(bootstrap.host, "gateway.local");
    assert_eq!(bootstrap.port, 65535);
    assert_eq!(bootstrap.skey.as_deref(), Some("bearer"));

    assert_eq!(
        GatewayClientBootstrapConfig::load(&config),
        GatewayClientBootstrapConfig::default()
    );

    let invalid_path = config_dir.join("gateway-client-bootstrap.json");
    fs::write(&invalid_path, "{not-json").expect("write invalid gateway config");
    assert_eq!(
        GatewayClientBootstrapConfig::load(&config),
        GatewayClientBootstrapConfig::default()
    );
    fs::remove_file(invalid_path).expect("remove invalid gateway config");
    fs::remove_dir(config_dir).expect("remove gateway config dir");
}

#[test]
fn bootstrap_json_mapping_ignores_invalid_ports_and_prefers_skey_over_bearer_token() {
    let mut config = GatewayClientBootstrapConfig::default();

    config.apply_json_root(&serde_json::json!({
        "gateway_client": {
            "host": " gateway.example ",
            "port": 70000_u64,
            "skey": " primary ",
            "bearer_token": " fallback "
        }
    }));

    assert_eq!(config.host, "gateway.example");
    assert_eq!(config.port, vw_config_types::gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(config.skey.as_deref(), Some("primary"));

    config.apply_json_root(&serde_json::json!({
        "gateway_client": {
            "port": "8080",
            "skey": " ",
            "bearer_token": " bearer "
        }
    }));

    assert_eq!(config.port, vw_config_types::gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(config.skey.as_deref(), Some("bearer"));
}

#[test]
fn bootstrap_json_mapping_ignores_non_object_roots() {
    let mut config = GatewayClientBootstrapConfig::default();

    config.apply_json_root(&serde_json::json!(["not", "an", "object"]));

    assert_eq!(config, GatewayClientBootstrapConfig::default());
}

#[test]
fn session_seed_normalizes_optional_values_and_preserves_directory() {
    let seed = GatewaySessionSeed::new(PathBuf::from("/tmp/work"))
        .with_id(Some(" session-1 ".to_string()))
        .with_scope(Some(" project ".to_string()))
        .with_title(Some(" Title ".to_string()));

    assert_eq!(seed.id(), Some("session-1"));
    assert_eq!(seed.scope(), Some("project"));
    assert_eq!(seed.title(), Some("Title"));
    assert_eq!(seed.directory(), PathBuf::from("/tmp/work").as_path());

    let cleared = seed
        .clone()
        .with_id(Some(" ".to_string()))
        .with_scope(None)
        .with_title(Some(String::new()));
    assert_eq!(cleared.id(), None);
    assert_eq!(cleared.scope(), None);
    assert_eq!(cleared.title(), None);
}

#[test]
fn gateway_runtime_accessors_binding_resolution_and_directory_value_are_stable() {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 42617))
        .expect("client should construct");
    let mut runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")).with_id(Some("seed".to_string())),
    );

    assert_eq!(runtime.endpoint().describe(), "127.0.0.1:42617");
    assert_eq!(runtime.session_id(), Some("seed"));
    assert_eq!(runtime.resolve_session_id(None), Ok("seed"));
    assert_eq!(runtime.resolve_session_id(Some(" explicit ")), Ok("explicit"));
    assert_eq!(runtime.directory_value().as_deref(), Some("/tmp/work"));

    runtime.bind_session_seed(
        Some(" rebound ".to_string()),
        Some(" scope ".to_string()),
        Some(" title ".to_string()),
    );
    assert_eq!(runtime.session_id(), Some("rebound"));
    assert_eq!(runtime.scope(), Some("scope"));
    assert_eq!(runtime.title(), Some("title"));

    let unbound = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 42617))
            .expect("client should construct"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")),
    );
    assert_eq!(
        unbound.resolve_session_id(None),
        Err("gateway runtime session id is required".to_string())
    );
}

#[test]
fn gateway_runtime_from_config_uses_loaded_endpoint_and_preserves_session_seed() {
    let config_dir = unique_temp_config_path("runtime");
    fs::create_dir(&config_dir).expect("create runtime config dir");
    let bootstrap_path = config_dir.join("gateway-client-bootstrap.json");
    fs::write(
        &bootstrap_path,
        r#"{"gateway_client":{"host":" cli.gateway ","port":43123,"skey":" runtime-key "}}"#,
    )
    .expect("write runtime config");

    let config = Config { config_path: config_dir.join("vibewindow.json"), ..Config::default() };
    let session = GatewaySessionSeed::new(PathBuf::from("/tmp/runtime-work"))
        .with_id(Some(" runtime-session ".to_string()))
        .with_scope(Some(" workspace-write ".to_string()))
        .with_title(Some(" Runtime Title ".to_string()));
    let runtime = GatewayUiRuntime::from_config(&config, session).expect("runtime should build");
    fs::remove_file(bootstrap_path).expect("remove runtime config");
    fs::remove_dir(config_dir).expect("remove runtime config dir");

    assert_eq!(runtime.endpoint().describe(), "cli.gateway:43123");
    assert_eq!(
        runtime.endpoint().auth.as_ref().and_then(|auth| auth.skey.as_deref()),
        Some("runtime-key")
    );
    assert_eq!(runtime.session_id(), Some("runtime-session"));
    assert_eq!(runtime.scope(), Some("workspace-write"));
    assert_eq!(runtime.title(), Some("Runtime Title"));
    assert_eq!(runtime.directory(), PathBuf::from("/tmp/runtime-work").as_path());
    assert_eq!(runtime.session().directory(), PathBuf::from("/tmp/runtime-work").as_path());
    assert!(runtime.client().endpoint().auth.is_some());
}

#[test]
fn gateway_runtime_for_workspace_uses_current_directory_seed() {
    let config_dir = unique_temp_config_path("workspace");
    fs::create_dir(&config_dir).expect("create workspace config dir");
    let config = Config { config_path: config_dir.join("vibewindow.json"), ..Config::default() };

    let runtime = GatewayUiRuntime::for_workspace(&config).expect("runtime should build");
    let current_dir = std::env::current_dir().expect("read current directory");
    fs::remove_dir(config_dir).expect("remove workspace config dir");

    assert_eq!(runtime.directory(), current_dir.as_path());
    assert_eq!(runtime.endpoint().describe(), "127.0.0.1:42617");
}

#[test]
fn enabled_acp_backend_keys_load_enabled_config_and_sort_names() {
    let (port, server) = spawn_json_sequence_server(vec![
        (
            "200 OK",
            r#"{"codex":{"command":"codex"},"alpha":{"command":"alpha"},"zeta":{"command":"zeta"}}"#,
        ),
        (
            "200 OK",
            r#"{"acp":{"zeta":{"command":"zeta"},"alpha":{"command":"alpha"}}}"#,
        ),
    ]);
    let runtime = runtime_for_port(port);

    assert_eq!(
        runtime.enabled_acp_backend_keys_blocking().expect("load enabled ACP keys"),
        ["alpha", "zeta"]
    );
    let requests = server.join().expect("join ACP config fixture");
    assert!(requests[0].starts_with("GET /v1/global/config/acp "));
    assert!(requests[1].starts_with("GET /v1/global/config "));
}

#[test]
fn enabled_acp_backend_keys_use_known_defaults_and_propagate_load_errors() {
    let catalog = HashMap::from([
        ("codex".to_string(), AcpAgentConfig::default()),
        ("claude".to_string(), AcpAgentConfig::default()),
        ("custom".to_string(), AcpAgentConfig::default()),
    ]);
    assert_eq!(
        resolve_enabled_acp_backend_keys(&catalog, &serde_json::json!({"acp": {}})),
        ["claude", "codex"]
    );

    let (port, server) =
        spawn_json_sequence_server(vec![("500 Internal Server Error", r#"{"error":"offline"}"#)]);
    let runtime = runtime_for_port(port);
    assert!(runtime.enabled_acp_backend_keys_blocking().is_err());
    server.join().expect("join failed ACP config fixture");
}

#[test]
fn prepare_stream_request_fills_missing_session_without_overwriting_explicit_session() {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 42617))
            .expect("client should construct"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")).with_id(Some("runtime-session".into())),
    );

    let prepared = runtime.prepare_stream_request(&GatewayChatStreamRequest::default());
    assert_eq!(prepared.session_id.as_ref().map(|id| id.as_ref()), Some("runtime-session"));

    let explicit = GatewayChatStreamRequest {
        session_id: Some(SessionId::from("explicit-session")),
        model: Some("gpt".to_string()),
        ..GatewayChatStreamRequest::default()
    };
    let prepared = runtime.prepare_stream_request(&explicit);
    assert_eq!(prepared.session_id.as_ref().map(|id| id.as_ref()), Some("explicit-session"));
    assert_eq!(prepared.model.as_deref(), Some("gpt"));
}

#[test]
fn prepare_stream_request_leaves_session_unset_when_runtime_is_unbound() {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 42617))
            .expect("client should construct"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")),
    );

    let prepared = runtime.prepare_stream_request(&GatewayChatStreamRequest::default());

    assert!(prepared.session_id.is_none());
}

#[tokio::test]
async fn stream_chat_adapts_gateway_events_and_returns_done_terminal() {
    let (port, server) = spawn_stream_response_server(
        "data: {\"type\":\"chat.delta\",\"delta\":\"hello\"}\n\n\
         data: {\"type\":\"chat.done\",\"finish_reason\":\"stop\",\"message_id\":\"assistant-1\",\"parent_message_id\":\"user-1\"}\n\n",
    );
    let runtime = runtime_for_port(port);
    let mut events = Vec::new();

    let terminal = runtime
        .stream_chat(&GatewayChatStreamRequest::default(), |event| {
            events.push(event);
            true
        })
        .await;
    let request = server.join().expect("join async stream fixture");

    assert!(request.starts_with("POST /v1/chat/stream?directory="));
    assert_eq!(events[0], UiRuntimeEvent::Delta("hello".to_string()));
    assert_eq!(
        terminal,
        UiRuntimeTerminalEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: None,
            message_id: Some("assistant-1".to_string()),
            parent_message_id: Some("user-1".to_string()),
        }
    );
}

#[tokio::test]
async fn stream_chat_returns_cancelled_terminal_when_consumer_stops_before_terminal_event() {
    let (port, server) = spawn_stream_response_server(
        "data: {\"type\":\"chat.delta\",\"delta\":\"first\"}\n\n\
         data: {\"type\":\"chat.delta\",\"delta\":\"second\"}\n\n",
    );
    let runtime = runtime_for_port(port);
    let mut events = Vec::new();

    let terminal = runtime
        .stream_chat(&GatewayChatStreamRequest::default(), |event| {
            events.push(event);
            false
        })
        .await;
    server.join().expect("join async cancel stream fixture");

    assert_eq!(events, vec![UiRuntimeEvent::Delta("first".to_string())]);
    assert!(matches!(
        terminal,
        UiRuntimeTerminalEvent::Cancelled { reason: Some(_), usage: None, .. }
    ));
}

#[test]
fn stream_chat_blocking_returns_cancelled_terminal_when_consumer_stops() {
    let (port, server) = spawn_stream_response_server(
        "data: {\"type\":\"chat.delta\",\"delta\":\"first\"}\r\n\r\n\
         data: {\"type\":\"chat.delta\",\"delta\":\"second\"}\r\n\r\n",
    );
    let runtime = runtime_for_port(port);
    let mut events = Vec::new();

    let terminal = runtime.stream_chat_blocking(&GatewayChatStreamRequest::default(), |event| {
        events.push(event);
        false
    });
    let request = server.join().expect("join blocking stream fixture");

    assert!(request.starts_with("POST /v1/chat/stream?directory="));
    assert_eq!(events, vec![UiRuntimeEvent::Delta("first".to_string())]);
    assert!(matches!(
        terminal,
        UiRuntimeTerminalEvent::Cancelled { reason: Some(_), usage: None, .. }
    ));
}

#[test]
fn stream_chat_blocking_adapts_done_terminal() {
    let (port, server) = spawn_stream_response_server(
        "data: {\"type\":\"chat.done\",\"finish_reason\":\"stop\",\"message_id\":\"assistant-2\"}\r\n\r\n",
    );
    let runtime = runtime_for_port(port);
    let mut events = Vec::new();

    let terminal = runtime.stream_chat_blocking(&GatewayChatStreamRequest::default(), |event| {
        events.push(event);
        true
    });
    server.join().expect("join blocking done stream fixture");

    assert_eq!(events.len(), 1);
    assert_eq!(
        terminal,
        UiRuntimeTerminalEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: None,
            message_id: Some("assistant-2".to_string()),
            parent_message_id: None,
        }
    );
}

#[tokio::test]
async fn stream_chat_returns_annotated_transport_error_terminal() {
    let runtime = runtime_for_port(9);
    let mut event_count = 0;

    let terminal = runtime
        .stream_chat(&GatewayChatStreamRequest::default(), |_| {
            event_count += 1;
            true
        })
        .await;

    assert_eq!(event_count, 0);
    assert!(
        matches!(terminal, UiRuntimeTerminalEvent::Error(message) if message.contains("Gateway endpoint 127.0.0.1:9 is unavailable"))
    );
}

#[test]
fn stream_chat_blocking_returns_annotated_transport_error_terminal() {
    let runtime = runtime_for_port(9);
    let mut event_count = 0;

    let terminal = runtime.stream_chat_blocking(&GatewayChatStreamRequest::default(), |_| {
        event_count += 1;
        true
    });

    assert_eq!(event_count, 0);
    assert!(
        matches!(terminal, UiRuntimeTerminalEvent::Error(message) if message.contains("Gateway endpoint 127.0.0.1:9 is unavailable"))
    );
}

#[test]
fn optional_string_normalization_error_annotation_and_stream_terminal_finalization() {
    assert_eq!(normalize_optional_str_ref(Some("  value  ")), Some("value"));
    assert_eq!(normalize_optional_str_ref(Some("  ")), None);
    assert_eq!(normalize_optional_str_ref(None), None);

    let endpoint = GatewayEndpoint::new("127.0.0.1", 9);
    let annotated =
        annotate_gateway_transport_error("connection refused by fixture".to_string(), &endpoint);
    assert!(annotated.contains("Gateway endpoint 127.0.0.1:9 is unavailable"));
    assert_eq!(
        annotate_gateway_transport_error("application error".to_string(), &endpoint),
        "application error"
    );
    assert!(looks_like_gateway_transport_error("tcp connect error"));
    assert!(!looks_like_gateway_transport_error("validation failed"));

    assert_eq!(
        finalize_stream_terminal(Ok(()), None),
        UiRuntimeTerminalEvent::Error("gateway stream closed before terminal event".to_string())
    );
    assert_eq!(
        finalize_stream_terminal(
            Err("transport failed".to_string()),
            Some(UiRuntimeTerminalEvent::Done {
                finish_reason: Some("stop".to_string()),
                usage: None,
                message_id: None,
                parent_message_id: None,
            }),
        ),
        UiRuntimeTerminalEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: None,
            message_id: None,
            parent_message_id: None,
        }
    );
    assert!(matches!(
        cancelled_by_consumer_terminal(),
        UiRuntimeTerminalEvent::Cancelled { reason: Some(_), .. }
    ));
    assert_eq!(
        finalize_stream_terminal(Err("cancel requested".to_string()), None),
        UiRuntimeTerminalEvent::Cancelled {
            reason: Some("cancel requested".to_string()),
            usage: None,
            message_id: None,
            parent_message_id: None,
        }
    );
}

#[test]
fn transport_error_annotation_normalizes_blank_and_recognizes_dns_failures() {
    let endpoint = GatewayEndpoint::new("gateway.invalid", 42617);

    assert_eq!(
        annotate_gateway_transport_error("  ".to_string(), &endpoint),
        "gateway request failed"
    );
    let annotated =
        annotate_gateway_transport_error("DNS error resolving host".to_string(), &endpoint);
    assert!(annotated.contains("Gateway endpoint gateway.invalid:42617 is unavailable"));
}

#[test]
fn runtime_directory_value_normalizes_blank_and_visible_paths() {
    assert_eq!(runtime_directory_value(Path::new("")), None);
    assert_eq!(runtime_directory_value(Path::new("  ")), None);
    assert_eq!(runtime_directory_value(Path::new(" /tmp/work ")).as_deref(), Some("/tmp/work"));
}

#[test]
fn loopback_detection_socket_resolution_and_health_failures_are_stable() {
    assert!(is_local_loopback_endpoint(&GatewayEndpoint::new("127.0.0.1", 1)));
    assert!(is_local_loopback_endpoint(&GatewayEndpoint::new(" localhost ", 1)));
    assert!(is_local_loopback_endpoint(&GatewayEndpoint::new("[::1]", 1)));
    assert!(!is_local_loopback_endpoint(&GatewayEndpoint::new("192.0.2.10", 1)));

    assert!(resolve_gateway_socket_address(&GatewayEndpoint::new("127.0.0.1", 1)).is_some());
    assert!(
        resolve_gateway_socket_address(&GatewayEndpoint::new("not a valid host name", 1)).is_none()
    );
    assert!(!gateway_health_ready(&GatewayEndpoint::new("not a valid host name", 1)));
    assert!(!gateway_health_ready(&GatewayEndpoint::new("127.0.0.1", 9)));
}

#[test]
fn wait_for_gateway_health_observes_ready_endpoint_and_short_timeout() {
    let (port, server) = spawn_health_response_server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
    );
    assert!(wait_for_gateway_health(
        &GatewayEndpoint::new("127.0.0.1", port),
        Duration::from_secs(1),
    ));
    server.join().expect("join wait fixture");

    assert!(!wait_for_gateway_health(
        &GatewayEndpoint::new("not a valid host name", 1),
        Duration::from_millis(1),
    ));
}

#[test]
fn preflight_returns_ready_when_health_probe_succeeds() {
    let (port, server) = spawn_health_response_server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
    );
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port))
            .expect("client should construct"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")),
    );

    let outcome = runtime.ensure_local_gateway_ready_blocking().expect("preflight should pass");
    server.join().expect("join preflight fixture");

    assert_eq!(outcome, GatewayPreflightOutcome::Ready);
    assert!(!outcome.started_gateway());
}

#[test]
fn preflight_rejects_unhealthy_non_loopback_endpoint_without_autostart() {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("not a valid host name", 42617))
            .expect("client should construct"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")),
    );

    let error = runtime
        .ensure_local_gateway_ready_blocking()
        .expect_err("non-loopback unhealthy endpoint should fail");

    assert!(error.contains("gateway preflight failed"));
    assert!(error.contains("Gateway endpoint not a valid host name:42617 is unavailable"));
}

#[test]
fn local_gateway_startup_failure_message_reports_spawn_and_timeout_failures() {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 42617);

    let spawn_error = local_gateway_startup_failure_message(
        &endpoint,
        Some("permission denied".to_string()),
        Duration::from_secs(2),
    );
    assert!(spawn_error.contains("gateway preflight failed"));
    assert!(spawn_error.contains("Auto-start failed: permission denied"));

    let timeout_error =
        local_gateway_startup_failure_message(&endpoint, None, Duration::from_secs(3));
    assert!(timeout_error.contains("Auto-start did not reach /v1/health"));
    assert!(timeout_error.contains("within 3s"));
}

#[test]
fn block_on_gateway_runs_ready_future_and_preserves_errors() {
    assert_eq!(block_on_gateway(async { Ok::<_, String>("ok") }), Ok("ok"));
    assert_eq!(
        block_on_gateway(async { Err::<(), _>("gateway failed".to_string()) }),
        Err("gateway failed".to_string())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn block_on_gateway_uses_current_tokio_runtime_handle() {
    assert_eq!(block_on_gateway(async { Ok::<_, String>("inside-runtime") }), Ok("inside-runtime"));
}
