//! 验证计算机操作请求执行、策略元数据、响应和截图落盘。
//! 本地伪 sidecar 记录完整 HTTP 请求，验证认证裁剪、策略元数据、截图路径和断连上下文。

use super::*;
use crate::app::agent::security::SecurityPolicy;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// 原始 TCP 夹具保留请求字节，避免高级 HTTP 客户端替测试自动修正非法响应。
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

async fn raw_server(response: Vec<u8>) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut stream).await;
        stream.write_all(&response).await.unwrap();
        request
    });

    (format!("http://{address}/v1/actions"), handle)
}

async fn sidecar_server(status: u16, body: &str) -> (String, tokio::task::JoinHandle<String>) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes();
    raw_server(response).await
}

async fn disconnecting_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = read_http_request(&mut stream).await;
    });

    (format!("http://{address}/v1/actions"), handle)
}

fn client(endpoint: String, api_key: Option<&str>) -> ComputerUseClient {
    ComputerUseClient::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".to_string()],
        Some("test-session".to_string()),
        super::super::config::ComputerUseConfig {
            endpoint,
            api_key: api_key.map(str::to_string),
            timeout_ms: 1_000,
            window_allowlist: vec!["Test Browser".to_string()],
            max_coordinate_x: Some(1_920),
            max_coordinate_y: Some(1_080),
            ..Default::default()
        },
    )
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

#[tokio::test]
async fn execute_action_rejects_invalid_endpoint_before_network_call() {
    let client = ComputerUseClient::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".to_string()],
        Some("session".to_string()),
        super::super::config::ComputerUseConfig {
            endpoint: "not a url".to_string(),
            ..Default::default()
        },
    );

    let err = client
        .execute_action("open", &serde_json::json!({"url":"https://example.com"}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Invalid browser.computer_use.endpoint"));
}

#[tokio::test]
async fn execute_action_rejects_non_object_args_before_network_call() {
    let client = client("http://127.0.0.1:9/v1/actions".to_string(), None);

    let error = client.execute_action("key_press", &serde_json::json!([])).await.unwrap_err();

    assert!(error.to_string().contains("browser args must be a JSON object"));
}

#[tokio::test]
async fn execute_action_rejects_invalid_action_params_before_network_call() {
    let client = client("http://127.0.0.1:9/v1/actions".to_string(), None);

    let error = client
        .execute_action("mouse_click", &serde_json::json!({"action":"mouse_click","x":10}))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("Missing or invalid 'y' parameter"));
}

#[tokio::test]
async fn execute_action_posts_policy_metadata_trimmed_auth_and_structured_data() {
    let (endpoint, server) =
        sidecar_server(200, r#"{"success":true,"data":{"clicked":true,"count":2}}"#).await;
    let client = client(endpoint, Some("  test-token  "));

    let result = client
        .execute_action("mouse_click", &serde_json::json!({"action":"ignored","x":10,"y":20}))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(
        serde_json::from_str::<Value>(&result.output).unwrap(),
        serde_json::json!({"clicked":true,"count":2})
    );

    let request = server.await.unwrap();
    assert!(request.starts_with("POST /v1/actions "));
    assert!(request.to_ascii_lowercase().contains("authorization: bearer test-token"));
    let payload = serde_json::from_str::<Value>(request_body(&request)).unwrap();
    assert_eq!(payload["action"], "mouse_click");
    assert_eq!(payload["params"], serde_json::json!({"x":10,"y":20}));
    assert_eq!(payload["policy"]["allowed_domains"], serde_json::json!(["example.com"]));
    assert_eq!(payload["policy"]["window_allowlist"], serde_json::json!(["Test Browser"]));
    assert_eq!(payload["policy"]["max_coordinate_x"], 1_920);
    assert_eq!(payload["policy"]["max_coordinate_y"], 1_080);
    assert_eq!(payload["metadata"]["session_name"], "test-session");
    assert_eq!(payload["metadata"]["source"], "vibewindow.browser");
    assert_eq!(payload["metadata"]["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
// 输出路径解析必须发生在请求 sidecar 之前，确保路径逃逸不会产生网络副作用。
async fn execute_action_resolves_screen_capture_path_inside_workspace() {
    let workspace = TempDir::new().unwrap();
    let mut security = SecurityPolicy::default();
    security.workspace_dir = workspace.path().to_path_buf();
    let (endpoint, server) = sidecar_server(200, r#"{"success":true}"#).await;
    let client = ComputerUseClient::new(
        Arc::new(security),
        Vec::new(),
        None,
        super::super::config::ComputerUseConfig { endpoint, ..Default::default() },
    );

    let result = client
        .execute_action(
            "screen_capture",
            &serde_json::json!({"action":"screen_capture","path":"shots/capture.png"}),
        )
        .await
        .unwrap();

    assert!(result.success);
    let request = server.await.unwrap();
    let payload = serde_json::from_str::<Value>(request_body(&request)).unwrap();
    let expected = std::fs::canonicalize(workspace.path())
        .unwrap()
        .join("shots/capture.png")
        .to_string_lossy()
        .into_owned();
    assert_eq!(payload["params"].as_object().unwrap().len(), 1);
    assert_eq!(payload["params"]["path"].as_str(), Some(expected.as_str()));
}

#[tokio::test]
async fn execute_action_allows_screen_capture_without_path_and_builds_default_success() {
    let (endpoint, server) = sidecar_server(200, "{}").await;
    let client = client(endpoint, Some("   "));

    let result = client
        .execute_action("screen_capture", &serde_json::json!({"action":"screen_capture"}))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(
        serde_json::from_str::<Value>(&result.output).unwrap(),
        serde_json::json!({"backend":"computer_use","action":"screen_capture","ok":true})
    );
    let request = server.await.unwrap();
    assert!(!request.to_ascii_lowercase().contains("authorization:"));
    let payload = serde_json::from_str::<Value>(request_body(&request)).unwrap();
    assert_eq!(payload["params"], serde_json::json!({}));
}

#[tokio::test]
async fn screen_capture_forwards_valid_geometry() {
    let (endpoint, server) = sidecar_server(200, r#"{"success":true}"#).await;
    let geometry_client = client(endpoint, None);

    let result = geometry_client
        .execute_action(
            "screen_capture",
            &serde_json::json!({
                "action": "screen_capture",
                "x": 10,
                "y": 20,
                "width": 30,
                "height": 40,
            }),
        )
        .await
        .unwrap();

    assert!(result.success);
    let request = server.await.unwrap();
    let payload = serde_json::from_str::<Value>(request_body(&request)).unwrap();
    assert_eq!(payload["params"], serde_json::json!({"x": 10, "y": 20, "width": 30, "height": 40}));

    let (endpoint, server) = sidecar_server(200, r#"{"success":true}"#).await;
    let display_client = client(endpoint, None);
    let result = display_client
        .execute_action(
            "screen_capture",
            &serde_json::json!({"action": "screen_capture", "display_id": 2}),
        )
        .await
        .unwrap();

    assert!(result.success);
    let request = server.await.unwrap();
    let payload = serde_json::from_str::<Value>(request_body(&request)).unwrap();
    assert_eq!(payload["params"], serde_json::json!({"display_id": 2}));
}

#[tokio::test]
async fn execute_action_preserves_explicit_structured_error() {
    let (endpoint, server) =
        sidecar_server(400, r#"{"success":false,"error":"window denied"}"#).await;
    let client = client(endpoint, None);

    let result = client.execute_action("scroll", &serde_json::json!({})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("window denied"));
    server.await.unwrap();
}

#[tokio::test]
async fn execute_action_reports_success_false_without_error() {
    let (endpoint, server) = sidecar_server(200, r#"{"success":false}"#).await;
    let client = client(endpoint, None);

    let result = client.execute_action("scroll", &serde_json::json!({})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("computer-use sidecar returned success=false"));
    server.await.unwrap();
}

#[tokio::test]
async fn execute_action_reports_structured_http_failure_without_error() {
    let (endpoint, server) = sidecar_server(503, "{}").await;
    let client = client(endpoint, None);

    let result = client.execute_action("scroll", &serde_json::json!({})).await.unwrap();

    assert!(!result.success);
    assert_eq!(
        result.error.as_deref(),
        Some("computer-use sidecar request failed with status 503 Service Unavailable")
    );
    server.await.unwrap();
}

#[tokio::test]
async fn execute_action_returns_unstructured_success_body_verbatim() {
    let (endpoint, server) = sidecar_server(200, "plain sidecar output").await;
    let client = client(endpoint, None);

    let result = client.execute_action("scroll", &serde_json::json!({})).await.unwrap();

    assert!(result.success);
    assert_eq!(result.output, "plain sidecar output");
    assert!(result.error.is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn execute_action_reports_trimmed_unstructured_http_failure() {
    let (endpoint, server) = sidecar_server(503, "  sidecar unavailable  ").await;
    let client = client(endpoint, None);

    let result = client.execute_action("scroll", &serde_json::json!({})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(
        result.error.as_deref(),
        Some(
            "computer-use sidecar request failed with status 503 Service Unavailable: sidecar unavailable"
        )
    );
    server.await.unwrap();
}

#[tokio::test]
async fn execute_action_contextualizes_sidecar_disconnect() {
    let (endpoint, server) = disconnecting_server().await;
    let endpoint_for_assertion = endpoint.clone();
    let client = client(endpoint, None);

    let error = client.execute_action("scroll", &serde_json::json!({})).await.unwrap_err();

    assert!(
        error
            .to_string()
            .contains(&format!("Failed to call computer-use sidecar at {endpoint_for_assertion}"))
    );
    server.await.unwrap();
}

#[tokio::test]
async fn execute_action_contextualizes_response_body_read_failure() {
    let response = b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 8\r\nconnection: close\r\n\r\nabc"
        .to_vec();
    let (endpoint, server) = raw_server(response).await;
    let client = client(endpoint, None);

    let error = client.execute_action("scroll", &serde_json::json!({})).await.unwrap_err();

    assert!(error.to_string().contains("Failed to read computer-use sidecar response body"));
    server.await.unwrap();
}
