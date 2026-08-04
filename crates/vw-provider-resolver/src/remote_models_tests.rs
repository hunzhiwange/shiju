use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::json;

use crate::provider::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, ProviderSource,
};
use crate::remote_models::{
    DiscoveryLimits, discover_openai_compatible_models,
    discover_openai_compatible_models_with_limits, merge_remote_models,
};

struct TestServer {
    api_base: String,
    request: Receiver<String>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("test server should stop");
        }
    }
}

fn serve_once(status: &str, headers: &[(&str, &str)], body: &str, delay: Duration) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener.local_addr().expect("test listener should have an address");
    let (request_tx, request_rx) = mpsc::channel();
    let response_headers =
        headers.iter().map(|(name, value)| format!("{name}: {value}\r\n")).collect::<String>();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{response_headers}Connection: close\r\n\r\n{body}",
        body.len()
    );

    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept one request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout should be configured");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let read = stream.read(&mut chunk).expect("request should be readable");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        request_tx
            .send(String::from_utf8(request).expect("request should be UTF-8"))
            .expect("request should be observed");
        std::thread::sleep(delay);
        let _ = stream.write_all(response.as_bytes());
    });

    TestServer { api_base: format!("http://{address}"), request: request_rx, thread: Some(thread) }
}

fn test_model(id: &str, status: &str) -> Model {
    Model {
        id: id.to_string(),
        provider_id: "test-provider".to_string(),
        api: ApiInfo {
            id: id.to_string(),
            url: "https://static.example/v1".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: format!("Static {id}"),
        family: Some("static-family".to_string()),
        capabilities: Capabilities {
            temperature: true,
            reasoning: true,
            attachment: true,
            toolcall: true,
            input: CapabilityIO { text: true, audio: false, image: true, video: false, pdf: false },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Bool(false),
        },
        cost: ModelCost {
            input: 1.0,
            output: 2.0,
            cache: ModelCostCache { read: 0.1, write: 0.2 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 32_000, input: Some(16_000), output: 8_000 },
        status: status.to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    }
}

fn test_provider() -> Info {
    Info {
        id: "test-provider".to_string(),
        name: "Test Provider".to_string(),
        source: ProviderSource::Config,
        env: vec![],
        key: Some("secret-token".to_string()),
        options: HashMap::new(),
        models: HashMap::from([
            ("explicit-disabled".to_string(), test_model("explicit-disabled", "disabled")),
            ("static-model".to_string(), test_model("static-model", "disabled")),
        ]),
    }
}

#[tokio::test]
async fn connected_provider_discovers_openai_compatible_models() {
    let server = serve_once(
        "200 OK",
        &[("Content-Type", "application/json")],
        r#"{"data":[{"id":"zeta"},{"id":" alpha "},{"id":"alpha"},{"id":""},{"name":"ignored"}]}"#,
        Duration::ZERO,
    );

    let models = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect("model discovery should succeed");
    let request = server
        .request
        .recv_timeout(Duration::from_secs(2))
        .expect("request should reach local server");

    assert_eq!(models, vec!["alpha", "zeta"]);
    assert!(request.starts_with("GET /models HTTP/1.1\r\n"));
    assert!(
        request.lines().any(|line| line.eq_ignore_ascii_case("authorization: Bearer secret-token"))
    );
}

#[tokio::test]
async fn remote_models_reports_http_status_without_returning_models() {
    let server = serve_once("404 Not Found", &[], "missing", Duration::ZERO);

    let error = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect_err("404 should not produce discovered models");

    assert_eq!(error.category(), "http_status");
}

#[tokio::test]
async fn remote_models_reports_invalid_json_without_returning_models() {
    let server = serve_once("200 OK", &[], "{not-json", Duration::ZERO);

    let error = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect_err("invalid JSON should not produce discovered models");

    assert_eq!(error.category(), "invalid_response");
}

#[tokio::test]
async fn remote_models_empty_data_leaves_existing_models_unchanged() {
    let server = serve_once("200 OK", &[], r#"{"data":[]}"#, Duration::ZERO);
    let mut provider = test_provider();
    let before = provider.models.keys().cloned().collect::<HashSet<_>>();

    let remote_ids = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect("empty data is a valid discovery response");
    let changed = merge_remote_models(&mut provider, &server.api_base, &HashSet::new(), remote_ids);

    assert_eq!(changed, 0);
    assert_eq!(provider.models.keys().cloned().collect::<HashSet<_>>(), before);
}

#[tokio::test]
async fn remote_models_rejects_oversized_response() {
    let body = json!({"data": [{"id": "x".repeat(256)}]}).to_string();
    let server = serve_once("200 OK", &[], &body, Duration::ZERO);

    let error = discover_openai_compatible_models_with_limits(
        &server.api_base,
        "secret-token",
        DiscoveryLimits::new(Duration::from_secs(1), 64),
    )
    .await
    .expect_err("oversized response should be rejected");

    assert_eq!(error.category(), "response_too_large");
}

#[tokio::test]
async fn remote_models_rejects_even_same_origin_redirects() {
    let server = serve_once("302 Found", &[("Location", "/other-models")], "", Duration::ZERO);

    let error = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect_err("redirect should not be followed");
    let request = server
        .request
        .recv_timeout(Duration::from_secs(2))
        .expect("original request should reach local server");

    assert_eq!(error.category(), "redirect");
    assert!(request.starts_with("GET /models HTTP/1.1\r\n"));
}

#[tokio::test]
async fn remote_models_reports_timeout_without_returning_models() {
    let server =
        serve_once("200 OK", &[], r#"{"data":[{"id":"too-late"}]}"#, Duration::from_millis(150));

    let error = discover_openai_compatible_models_with_limits(
        &server.api_base,
        "secret-token",
        DiscoveryLimits::new(Duration::from_millis(25), 1024),
    )
    .await
    .expect_err("slow response should time out");

    assert_eq!(error.category(), "timeout");
}

#[test]
fn remote_models_merge_preserves_explicit_status_and_static_metadata() {
    let mut provider = test_provider();
    let explicit = HashSet::from(["explicit-disabled".to_string()]);

    let merged = merge_remote_models(
        &mut provider,
        "https://connected.example/v1",
        &explicit,
        vec!["remote-new".to_string(), "static-model".to_string(), "explicit-disabled".to_string()],
    );

    assert_eq!(merged, 2);
    assert_eq!(provider.models["explicit-disabled"].status, "disabled");
    assert_eq!(provider.models["static-model"].status, "active");
    assert_eq!(provider.models["static-model"].name, "Static static-model");
    assert!(provider.models["static-model"].capabilities.reasoning);
    let discovered = &provider.models["remote-new"];
    assert_eq!(discovered.status, "active");
    assert_eq!(discovered.api.url, "https://connected.example/v1");
    assert!(discovered.capabilities.input.text);
    assert!(discovered.capabilities.output.text);
    assert!(!discovered.capabilities.attachment);
    assert_eq!(discovered.cost.input, 0.0);
}
