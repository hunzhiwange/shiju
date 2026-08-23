//! 验证浏览器工具的后端选择、执行结果和安全策略衔接。

use super::*;
use crate::app::agent::security::AutonomyLevel;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn tool_with_backend(security: SecurityPolicy, backend: &str) -> BrowserTool {
    BrowserTool::new_with_backend(
        Arc::new(security),
        vec!["example.com".into()],
        Some("tool-tests".into()),
        backend.into(),
        true,
        "http://127.0.0.1:9515".into(),
        None,
        super::super::ComputerUseConfig::default(),
    )
}

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);

        let Some(header_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buffer[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        if buffer.len() >= header_end + 4 + content_length {
            break;
        }
    }

    String::from_utf8_lossy(&buffer).into_owned()
}

async fn computer_use_tool_server(
    status: u16,
    body: &'static str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (_probe, _) = listener.accept().await.unwrap();

        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut stream).await;
        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            503 => "Service Unavailable",
            _ => "Error",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        request
    });

    (format!("http://{address}/v1/actions"), handle)
}

async fn reachable_endpoint() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (_probe, _) = listener.accept().await.unwrap();
    });

    (format!("http://{address}/v1/actions"), handle)
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

fn computer_use_tool(endpoint: String, security: SecurityPolicy) -> BrowserTool {
    BrowserTool::new_with_backend(
        Arc::new(security),
        vec!["example.com".into()],
        Some("tool-tests".into()),
        "computer_use".into(),
        true,
        "http://127.0.0.1:9515".into(),
        None,
        super::super::ComputerUseConfig {
            endpoint,
            timeout_ms: 1_000,
            window_allowlist: vec!["Browser".into()],
            max_coordinate_x: Some(200),
            max_coordinate_y: Some(200),
            ..super::super::ComputerUseConfig::default()
        },
    )
}

#[test]
fn new_uses_legacy_agent_browser_defaults() {
    let tool = BrowserTool::new(
        Arc::new(SecurityPolicy::default()),
        vec![" Example.COM ".into()],
        Some("legacy-session".into()),
    );

    assert_eq!(tool.allowed_domains, vec!["example.com"]);
    assert_eq!(tool.session_name.as_deref(), Some("legacy-session"));
    assert_eq!(tool.backend, "agent_browser");
    assert!(tool.native_headless);
    assert_eq!(tool.native_webdriver_url, "http://127.0.0.1:9515");
    assert_eq!(tool.native_chrome_path, None);
    assert_eq!(tool.computer_use.endpoint, "http://127.0.0.1:8787/v1/actions");
    assert_eq!(tool.computer_use.api_key, None);
    assert_eq!(tool.computer_use.timeout_ms, 15_000);
    assert!(!tool.computer_use.allow_remote_endpoint);
    assert!(tool.computer_use.window_allowlist.is_empty());
    assert_eq!(tool.computer_use.max_coordinate_x, None);
    assert_eq!(tool.computer_use.max_coordinate_y, None);
}

#[test]
fn new_with_backend_normalizes_domains_and_preserves_configuration() {
    let computer_use = super::super::ComputerUseConfig {
        endpoint: "https://sidecar.example.com/v1/actions".into(),
        timeout_ms: 2_500,
        allow_remote_endpoint: true,
        window_allowlist: vec!["Browser".into()],
        max_coordinate_x: Some(1_920),
        max_coordinate_y: Some(1_080),
        ..super::super::ComputerUseConfig::default()
    };

    let tool = BrowserTool::new_with_backend(
        Arc::new(SecurityPolicy::default()),
        vec!["  Example.COM  ".into(), String::new(), "*.Docs.Example.COM".into()],
        Some("session-a".into()),
        "computer_use".into(),
        false,
        "http://127.0.0.1:4444".into(),
        Some("/opt/chrome".into()),
        computer_use,
    );

    assert_eq!(tool.allowed_domains, vec!["example.com", "*.docs.example.com"]);
    assert_eq!(tool.session_name.as_deref(), Some("session-a"));
    assert_eq!(tool.backend, "computer_use");
    assert!(!tool.native_headless);
    assert_eq!(tool.native_webdriver_url, "http://127.0.0.1:4444");
    assert_eq!(tool.native_chrome_path.as_deref(), Some("/opt/chrome"));
    assert_eq!(tool.computer_use.endpoint, "https://sidecar.example.com/v1/actions");
    assert_eq!(tool.computer_use.timeout_ms, 2_500);
    assert!(tool.computer_use.allow_remote_endpoint);
    assert_eq!(tool.computer_use.window_allowlist, vec!["Browser".to_string()]);
    assert_eq!(tool.computer_use.max_coordinate_x, Some(1_920));
    assert_eq!(tool.computer_use.max_coordinate_y, Some(1_080));
}

#[test]
fn configured_backend_rejects_unknown_backend() {
    let tool = tool_with_backend(SecurityPolicy::default(), "playwright");

    let error = tool.configured_backend().expect_err("unknown backend should be rejected");

    assert!(error.to_string().contains("Unsupported browser backend 'playwright'"));
}

#[test]
fn rust_native_compilation_flag_matches_feature_configuration() {
    assert_eq!(BrowserTool::rust_native_compiled(), cfg!(feature = "browser-native"));
}

#[cfg(not(feature = "browser-native"))]
#[test]
fn rust_native_is_unavailable_without_compiled_feature() {
    let tool = tool_with_backend(SecurityPolicy::default(), "rust_native");

    assert!(!tool.rust_native_available());
}

#[cfg(not(feature = "browser-native"))]
#[tokio::test]
async fn resolve_backend_rejects_rust_native_without_compiled_feature() {
    let tool = tool_with_backend(SecurityPolicy::default(), "rust_native");

    let error = tool
        .resolve_backend()
        .await
        .expect_err("native backend should require its compile-time feature");

    assert_eq!(
        error.to_string(),
        "browser.backend='rust_native' requires build feature 'browser-native'"
    );
}

#[tokio::test]
async fn resolve_backend_propagates_invalid_computer_use_config_without_network() {
    let mut computer_use = super::super::ComputerUseConfig::default();
    computer_use.timeout_ms = 0;
    let tool = BrowserTool::new_with_backend(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".into()],
        None,
        "computer_use".into(),
        true,
        "http://127.0.0.1:9515".into(),
        None,
        computer_use,
    );

    let error = tool
        .resolve_backend()
        .await
        .expect_err("invalid sidecar configuration should stop before reachability probing");

    assert_eq!(error.to_string(), "browser.computer_use.timeout_ms must be > 0");
}

#[test]
fn to_result_formats_success_data_and_empty_success() {
    let tool = tool_with_backend(SecurityPolicy::default(), "agent_browser");
    let result = tool
        .to_result(AgentBrowserResponse {
            success: true,
            data: Some(serde_json::json!({"title": "VibeWindow", "count": 2})),
            error: None,
        })
        .expect("successful response should convert");

    assert!(result.success);
    assert_eq!(
        serde_json::from_str::<Value>(&result.output).expect("output should remain JSON"),
        serde_json::json!({"title": "VibeWindow", "count": 2})
    );
    assert_eq!(result.error, None);

    let empty = tool
        .to_result(AgentBrowserResponse { success: true, data: None, error: None })
        .expect("empty successful response should convert");
    assert!(empty.success);
    assert!(empty.output.is_empty());
    assert_eq!(empty.error, None);
}

#[test]
fn to_result_preserves_failure_error_and_clears_output() {
    let tool = tool_with_backend(SecurityPolicy::default(), "agent_browser");

    let result = tool
        .to_result(AgentBrowserResponse {
            success: false,
            data: Some(serde_json::json!({"ignored": true})),
            error: Some("browser command failed".into()),
        })
        .expect("failed response should still convert to a tool result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("browser command failed"));
}

#[test]
fn sanitize_payload_redacts_nested_base64_strings_case_insensitively() {
    let payload = serde_json::json!({
        "imageBase64": "abcd",
        "nested": [
            {"BASE64_data": "é🙂"},
            {"base64_count": 7},
            "plain"
        ],
        "unchanged": true
    });

    let sanitized = BrowserTool::sanitize_payload(&payload);

    assert_eq!(
        sanitized,
        serde_json::json!({
            "imageBase64": "<base64 4 chars>",
            "nested": [
                {"BASE64_data": "<base64 2 chars>"},
                {"base64_count": 7},
                "plain"
            ],
            "unchanged": true
        })
    );
    assert_eq!(payload["imageBase64"], "abcd");
}

#[test]
fn summarize_action_covers_specific_and_fallback_messages() {
    let cases = vec![
        ("open", serde_json::json!({"url": "https://example.com"}), "Opened https://example.com"),
        ("open", serde_json::json!({}), "Opened page"),
        ("snapshot", serde_json::json!({}), "Captured page snapshot"),
        ("click", serde_json::json!({"selector": "#submit"}), "Clicked #submit"),
        ("click", serde_json::json!({}), "Clicked element"),
        ("fill", serde_json::json!({"selector": "#name"}), "Filled #name"),
        ("fill", serde_json::json!({}), "Filled field"),
        ("type", serde_json::json!({"selector": "#query"}), "Typed into #query"),
        ("type", serde_json::json!({}), "Typed text"),
        ("get_text", serde_json::json!({"selector": ".message"}), "Read text from .message"),
        ("get_text", serde_json::json!({}), "Read element text"),
        ("get_title", serde_json::json!({"title": "Dashboard"}), "Read title: Dashboard"),
        ("get_title", serde_json::json!({}), "Read page title"),
        (
            "get_url",
            serde_json::json!({"url": "https://example.com/current"}),
            "Read URL: https://example.com/current",
        ),
        ("get_url", serde_json::json!({}), "Read current URL"),
        (
            "screenshot",
            serde_json::json!({"path": "captures/page.png"}),
            "Captured screenshot to captures/page.png",
        ),
        ("screen_capture", serde_json::json!({}), "Captured screenshot"),
        ("wait", serde_json::json!({}), "Wait completed"),
        ("press", serde_json::json!({"key": "Enter"}), "Pressed Enter"),
        ("key_press", serde_json::json!({}), "Pressed key"),
        ("hover", serde_json::json!({"selector": ".menu"}), "Hovered .menu"),
        ("hover", serde_json::json!({}), "Hovered element"),
        ("scroll", serde_json::json!({"direction": "down"}), "Scrolled down"),
        ("scroll", serde_json::json!({}), "Scrolled page"),
        (
            "is_visible",
            serde_json::json!({"selector": "#dialog"}),
            "Checked visibility for #dialog",
        ),
        ("is_visible", serde_json::json!({}), "Checked element visibility"),
        ("close", serde_json::json!({}), "Closed browser"),
        ("find", serde_json::json!({"value": "Save"}), "Resolved element for Save"),
        ("find", serde_json::json!({}), "Resolved page element"),
        (
            "mouse_move",
            serde_json::json!({"backend": "computer_use"}),
            "Ran mouse_move via computer_use",
        ),
        ("custom", serde_json::json!({}), "Ran custom"),
    ];

    for (action, payload, expected) in cases {
        assert_eq!(BrowserTool::summarize_action(action, &payload), expected, "action={action}");
    }
}

#[test]
fn tool_contract_exposes_browser_schema_and_safety_flags() {
    let tool = tool_with_backend(SecurityPolicy::default(), "agent_browser");
    let schema = tool.parameters_schema();

    assert_eq!(tool.name(), "browser");
    assert!(tool.description().contains("browser.allowed_domains"));
    assert!(tool.description().contains("get_text"));
    assert!(tool.description().contains("require a non-empty 'selector'"));
    assert!(
        schema["properties"]["selector"]["description"].as_str().unwrap_or("").contains("get_text")
    );
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], serde_json::json!(["action"]));
    assert_eq!(
        schema["properties"]["action"]["enum"],
        serde_json::json!([
            "open",
            "snapshot",
            "click",
            "fill",
            "type",
            "get_text",
            "get_title",
            "get_url",
            "screenshot",
            "wait",
            "press",
            "hover",
            "scroll",
            "is_visible",
            "close",
            "find",
            "mouse_move",
            "mouse_click",
            "mouse_drag",
            "key_type",
            "key_press",
            "screen_capture"
        ])
    );
    assert_eq!(
        schema["properties"]["find_action"]["enum"],
        serde_json::json!(["click", "fill", "text", "hover", "check"])
    );

    let spec = tool.spec();
    assert_eq!(spec.id, crate::app::agent::tools::BROWSER_TOOL_ID);
    assert_eq!(spec.display_name, crate::app::agent::tools::BROWSER_TOOL_ID);
    assert_eq!(spec.name, crate::app::agent::tools::BROWSER_TOOL_ID);
    assert_eq!(spec.aliases, vec![crate::app::agent::tools::BROWSER_TOOL_ALIAS.to_string()]);
    assert_eq!(spec.input_schema, schema);
    assert_eq!(spec.parameters, spec.input_schema);
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[tokio::test]
async fn execute_rejects_read_only_policy_before_backend_resolution() {
    let mut security = SecurityPolicy::default();
    security.autonomy = AutonomyLevel::ReadOnly;
    let tool = tool_with_backend(security, "agent_browser");

    let result = tool
        .execute(serde_json::json!({"action": "open", "url": "https://example.com"}))
        .await
        .expect("policy rejection should be a tool result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Action blocked: autonomy is read-only"));
}

#[tokio::test]
async fn execute_rejects_exhausted_action_budget_before_backend_resolution() {
    let mut security = SecurityPolicy::default();
    security.max_actions_per_hour = 0;
    let tool = tool_with_backend(security, "agent_browser");

    let result = tool
        .execute(serde_json::json!({"action": "snapshot"}))
        .await
        .expect("rate-limit rejection should be a tool result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Action blocked: rate limit exceeded"));
}

#[tokio::test]
async fn execute_returns_backend_configuration_errors_as_failed_results() {
    let tool = tool_with_backend(SecurityPolicy::default(), "unsupported");

    let result = tool
        .execute(serde_json::json!({"action": "snapshot"}))
        .await
        .expect("backend selection failure should be a tool result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert!(
        result
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Unsupported browser backend 'unsupported'"))
    );
}

#[tokio::test]
async fn execute_action_rejects_computer_use_after_browser_action_parsing() {
    let tool = tool_with_backend(SecurityPolicy::default(), "computer_use");

    let error = tool
        .execute_action(BrowserAction::Close, ResolvedBackend::ComputerUse)
        .await
        .expect_err("computer-use must be handled before BrowserAction parsing");

    assert_eq!(
        error.to_string(),
        "Internal error: computer_use backend must be handled before BrowserAction parsing"
    );
}

#[cfg(not(feature = "browser-native"))]
#[tokio::test]
async fn rust_native_execution_rejects_build_without_feature() {
    let tool = tool_with_backend(SecurityPolicy::default(), "rust_native");

    let error = tool
        .execute_rust_native_action(BrowserAction::Close)
        .await
        .expect_err("missing native feature should reject execution");

    assert_eq!(
        error.to_string(),
        "Rust-native browser backend is not compiled. Rebuild with --features browser-native"
    );
}

#[tokio::test]
async fn call_lifts_policy_failure_into_structured_result() {
    let mut security = SecurityPolicy::default();
    security.autonomy = AutonomyLevel::ReadOnly;
    let tool = tool_with_backend(security, "agent_browser");

    let result = tool
        .call(serde_json::json!({"action": "click", "selector": "#save"}))
        .await
        .expect("policy failure should be represented structurally");

    assert_eq!(
        result.data,
        serde_json::json!({
            "success": false,
            "output": "",
            "error": "Action blocked: autonomy is read-only"
        })
    );
    assert_eq!(result.model_result, Value::String("Action blocked: autonomy is read-only".into()));
    assert!(result.content_blocks.is_empty());
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(false));

    let hint = result.render_hint.expect("failure should include a render hint");
    assert_eq!(hint.title.as_deref(), Some(crate::app::agent::tools::BROWSER_TOOL_ID));
    assert_eq!(hint.kind.as_deref(), Some("browser"));
    assert_eq!(hint.summary.as_deref(), Some("Failed to run click"));
    assert_eq!(hint.metadata, serde_json::json!({"action": "click"}));
}

#[tokio::test]
async fn resolve_backend_selects_reachable_computer_use_sidecar() {
    let (endpoint, server) = reachable_endpoint().await;
    let tool = computer_use_tool(endpoint, SecurityPolicy::default());

    let resolved = tool.resolve_backend().await.expect("reachable sidecar should resolve");

    assert_eq!(resolved, ResolvedBackend::ComputerUse);
    server.await.unwrap();
}

#[tokio::test]
async fn resolve_backend_reports_unreachable_computer_use_sidecar() {
    let tool = computer_use_tool("http://127.0.0.1:9/v1/actions".into(), SecurityPolicy::default());

    let error = tool
        .resolve_backend()
        .await
        .expect_err("unreachable sidecar should reject computer_use backend");

    assert_eq!(
        error.to_string(),
        "browser.backend='computer_use' but sidecar endpoint is unreachable. Check browser.computer_use.endpoint and sidecar status"
    );
}

#[tokio::test]
async fn execute_errors_when_action_is_missing_after_backend_resolution() {
    let (endpoint, server) = reachable_endpoint().await;
    let tool = computer_use_tool(endpoint, SecurityPolicy::default());

    let error = tool
        .execute(serde_json::json!({"url": "https://example.com"}))
        .await
        .expect_err("missing action should remain a hard input error");

    assert_eq!(error.to_string(), "Missing 'action' parameter");
    server.await.unwrap();
}

#[tokio::test]
async fn execute_returns_unknown_action_as_failed_result_after_backend_resolution() {
    let (endpoint, server) = reachable_endpoint().await;
    let tool = computer_use_tool(endpoint, SecurityPolicy::default());

    let result = tool
        .execute(serde_json::json!({"action": "dance"}))
        .await
        .expect("unknown action should be represented as a tool result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Unknown action: dance"));
    server.await.unwrap();
}

#[tokio::test]
async fn execute_routes_computer_use_action_to_sidecar_and_preserves_output() {
    let (endpoint, server) = computer_use_tool_server(
        200,
        r#"{"success":true,"data":{"backend":"computer_use","ok":true}}"#,
    )
    .await;
    let tool = computer_use_tool(endpoint, SecurityPolicy::default());

    let result = tool
        .execute(serde_json::json!({"action": "mouse_click", "x": 10, "y": 20}))
        .await
        .expect("computer_use action should execute through the sidecar");

    assert!(result.success);
    assert_eq!(
        serde_json::from_str::<Value>(&result.output).unwrap(),
        serde_json::json!({"backend":"computer_use","ok":true})
    );
    assert!(result.error.is_none());

    let request = server.await.unwrap();
    let payload = serde_json::from_str::<Value>(request_body(&request)).unwrap();
    assert_eq!(payload["action"], "mouse_click");
    assert_eq!(payload["params"], serde_json::json!({"x": 10, "y": 20}));
    assert_eq!(payload["metadata"]["session_name"], "tool-tests");
    assert_eq!(payload["policy"]["window_allowlist"], serde_json::json!(["Browser"]));
}

#[tokio::test]
async fn execute_validates_screen_capture_output_path_before_sidecar_request() {
    let (endpoint, server) = reachable_endpoint().await;
    let mut security = SecurityPolicy::default();
    security.workspace_dir = PathBuf::from(".");
    let tool = BrowserTool::new_with_backend(
        Arc::new(security),
        vec!["example.com".into()],
        None,
        "computer_use".into(),
        true,
        "http://127.0.0.1:9515".into(),
        None,
        super::super::ComputerUseConfig { endpoint, timeout_ms: 1_000, ..Default::default() },
    );

    let error = tool
        .execute(serde_json::json!({"action": "screen_capture", "path": "../outside.png"}))
        .await
        .expect_err("invalid output path should stop before the sidecar request");

    assert_eq!(error.to_string(), "'path' path must stay within the workspace");
    server.await.unwrap();
}

#[tokio::test]
async fn call_wraps_successful_computer_use_json_with_sanitized_render_data() {
    let (endpoint, server) = computer_use_tool_server(
        200,
        r#"{"success":true,"data":{"backend":"computer_use","imageBase64":"abcdef","title":"Done"}}"#,
    )
    .await;
    let tool = computer_use_tool(endpoint, SecurityPolicy::default());

    let result = tool
        .call(serde_json::json!({"action": "screen_capture"}))
        .await
        .expect("successful legacy result should be lifted into rich call result");

    assert_eq!(
        result.data,
        serde_json::json!({
            "action": "screen_capture",
            "backend": "computer_use",
            "result": {
                "backend": "computer_use",
                "imageBase64": "<base64 6 chars>",
                "title": "Done"
            }
        })
    );
    assert_eq!(
        result.model_result,
        Value::String(
            serde_json::to_string_pretty(
                &serde_json::json!({"backend":"computer_use","imageBase64":"abcdef","title":"Done"})
            )
            .unwrap()
        )
    );
    assert_eq!(result.content_blocks.len(), 1);
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(true));

    let hint = result.render_hint.expect("success should include browser render metadata");
    assert_eq!(hint.summary.as_deref(), Some("Captured screenshot"));
    assert_eq!(
        hint.metadata,
        serde_json::json!({"action": "screen_capture", "backend": "computer_use"})
    );
    server.await.unwrap();
}

#[tokio::test]
async fn call_uses_browser_backend_when_success_output_is_not_json() {
    let (endpoint, server) = computer_use_tool_server(200, "plain sidecar output").await;
    let tool = computer_use_tool(endpoint, SecurityPolicy::default());

    let result = tool
        .call(serde_json::json!({"action": "key_press", "key": "Enter"}))
        .await
        .expect("plain successful output should still become a structured call result");

    assert_eq!(
        result.data,
        serde_json::json!({
            "action": "key_press",
            "backend": "browser",
            "result": "plain sidecar output"
        })
    );
    assert_eq!(result.model_result, Value::String("plain sidecar output".into()));
    let hint = result.render_hint.expect("success should include render metadata");
    assert_eq!(hint.summary.as_deref(), Some("Pressed key"));
    assert_eq!(hint.metadata, serde_json::json!({"action": "key_press", "backend": "browser"}));
    server.await.unwrap();
}

#[tokio::test]
async fn computer_use_endpoint_url_and_validate_url_delegate_to_client_policy() {
    let workspace = TempDir::new().unwrap();
    let mut security = SecurityPolicy::default();
    security.workspace_dir = workspace.path().to_path_buf();
    let tool = computer_use_tool("http://127.0.0.1:8787/v1/actions".into(), security);

    let endpoint = tool.computer_use_endpoint_url().expect("default local endpoint should parse");
    assert_eq!(endpoint.as_str(), "http://127.0.0.1:8787/v1/actions");
    tool.validate_url("https://example.com/docs").expect("allowlisted host should pass");

    let error = tool
        .validate_url("https://blocked.example.net")
        .expect_err("non-allowlisted host should be rejected");
    assert_eq!(error.to_string(), "Host 'blocked.example.net' not in browser.allowed_domains");
}
