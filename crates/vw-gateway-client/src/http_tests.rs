//! 验证 HTTP 请求上下文、响应解析、错误分类与日志安全边界。
//!
//! 异步和阻塞路径应产生一致请求头；认证信息与敏感正文不得进入日志或错误消息。

use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{Interest, Subscriber};
use tracing::{Event, Metadata};

use crate::client::test_support;
use crate::endpoint::{GatewayAuth, GatewayEndpoint};
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

#[derive(Clone, Default)]
struct CapturedEvents(Arc<Mutex<Vec<String>>>);

impl CapturedEvents {
    fn joined(&self) -> String {
        self.0.lock().expect("captured events").join("\n")
    }
}

impl Subscriber for CapturedEvents {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target() == "vw_gateway_client"
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        self.0.lock().expect("captured events").push(visitor.fields.join(" "));
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if metadata.target() == "vw_gateway_client" {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn clone_span(&self, id: &Id) -> Id {
        id.clone()
    }

    fn try_close(&self, _id: Id) -> bool {
        true
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
}

#[derive(Default)]
struct EventVisitor {
    fields: Vec<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }
}

#[test]
fn directory_query_skips_blank_values() {
    assert!(super::directory_query(None).is_empty());
    assert!(super::directory_query(Some("   ")).is_empty());
    assert_eq!(
        super::directory_query(Some("/tmp/project")),
        vec![("directory".to_string(), "/tmp/project".to_string())]
    );
}

#[test]
fn apply_request_context_keeps_auth_and_selected_tenant_headers() {
    let tenant_scope = TenantScope::new("platform-east", "company-blue").expect("tenant scope");
    let endpoint = GatewayEndpoint::new("127.0.0.1", 1234)
        .with_auth(GatewayAuth { skey: Some("skey-1".to_string()) })
        .with_tenant_scope(tenant_scope);
    let request =
        super::apply_request_context(reqwest::Client::new().get("http://example.test"), &endpoint)
            .build()
            .expect("request");

    assert_eq!(request.headers()["authorization"].to_str().unwrap(), "Bearer skey-1");
    assert_eq!(request.headers()[PLATFORM_ID_HEADER].to_str().unwrap(), "platform-east");
    assert_eq!(request.headers()[COMPANY_ID_HEADER].to_str().unwrap(), "company-blue");
    assert!(!request.headers().contains_key("x-skey"));
}

#[test]
fn apply_request_context_ignores_blank_skey_but_keeps_default_tenant() {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 1234)
        .with_auth(GatewayAuth { skey: Some(" ".to_string()) });
    let request =
        super::apply_request_context(reqwest::Client::new().get("http://example.test"), &endpoint)
            .build()
            .expect("request");

    assert!(!request.headers().contains_key("authorization"));
    assert!(!request.headers().contains_key("x-skey"));
    assert_eq!(request.headers()[PLATFORM_ID_HEADER], "10000");
    assert_eq!(request.headers()[COMPANY_ID_HEADER], "100000000");
}

#[test]
fn apply_request_context_without_credentials_leaves_only_tenant_headers() {
    let anonymous = GatewayEndpoint::new("127.0.0.1", 1234);
    let request =
        super::apply_request_context(reqwest::Client::new().get("http://example.test"), &anonymous)
            .build()
            .expect("request");
    assert!(!request.headers().contains_key("authorization"));
    assert!(!request.headers().contains_key("x-skey"));

    let empty_auth = GatewayEndpoint::new("127.0.0.1", 1234).with_auth(GatewayAuth::default());
    let request = super::apply_request_context(
        reqwest::Client::new().get("http://example.test"),
        &empty_auth,
    )
    .build()
    .expect("request");

    assert!(!request.headers().contains_key("authorization"));
    assert!(!request.headers().contains_key("x-skey"));
    assert_eq!(request.headers()[PLATFORM_ID_HEADER], "10000");
    assert_eq!(request.headers()[COMPANY_ID_HEADER], "100000000");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn apply_blocking_request_context_matches_async_headers() {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 1234)
        .with_auth(GatewayAuth { skey: Some("skey-2".to_string()) });
    let request = super::apply_blocking_request_context(
        reqwest::blocking::Client::new().get("http://example.test"),
        &endpoint,
    )
    .build()
    .expect("request");

    assert_eq!(request.headers()["authorization"].to_str().unwrap(), "Bearer skey-2");
    assert_eq!(request.headers()[PLATFORM_ID_HEADER], "10000");
    assert_eq!(request.headers()[COMPANY_ID_HEADER], "100000000");
    assert!(!request.headers().contains_key("x-skey"));
}

#[tokio::test]
async fn parse_json_response_decodes_success_and_reports_decode_failures() {
    let server = test_support::server_raw(vec![
        (200, r#"{"ok":true}"#.to_string()),
        (200, "not-json".to_string()),
    ]);
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    let client = reqwest::Client::new();

    let response = client.get(format!("{}/ok", server.base_url())).send().await.unwrap();
    let value: Value = super::parse_json_response("GET", &endpoint, "/ok", response).await.unwrap();
    assert_eq!(value, json!({"ok": true}));
    let response = client.get(format!("{}/bad-json", server.base_url())).send().await.unwrap();
    let error = super::parse_json_response::<Value>("GET", &endpoint, "/bad-json", response)
        .await
        .unwrap_err();
    assert!(!error.trim().is_empty());

    assert_eq!(server.take_request().path, "/ok");
    assert_eq!(server.take_request().path, "/bad-json");
    server.join();
}

#[tokio::test]
async fn parse_json_response_delegates_non_success_status_to_gateway_error() {
    let server = test_support::server_raw(vec![(429, "retry later".to_string())]);
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    let response = reqwest::Client::new()
        .get(format!("{}/limited", server.base_url()))
        .send()
        .await
        .expect("response");

    let error = super::parse_json_response::<Value>("GET", &endpoint, "/limited", response)
        .await
        .expect_err("non-success response should fail");

    assert_eq!(error, "gateway request failed: 429 Too Many Requests retry later");
    assert_eq!(server.take_request().path, "/limited");
    server.join();
}

#[tokio::test]
async fn response_error_includes_status_and_body_when_present() {
    let server = test_support::server(vec![(500, json!({"error": "bad"}))]);
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    let response =
        reqwest::Client::new().get(format!("{}/failure", server.base_url())).send().await.unwrap();

    let error = super::response_error("GET", &endpoint, "/failure", response).await;

    assert!(error.contains("500 Internal Server Error"));
    assert!(error.contains("\"bad\""));
    assert_eq!(server.take_request().path, "/failure");
    server.join();
}

#[tokio::test(flavor = "current_thread")]
async fn gateway_http_logs_and_errors_redact_sensitive_payloads() {
    const REQUEST_TOKEN: &str = "request-token-sentinel-9fb3";
    const REQUEST_SKEY: &str = "request-skey-sentinel-c381";
    const REQUEST_PASSWORD: &str = "request-password-sentinel-819a";
    const RESPONSE_CREDENTIAL: &str = "response-credential-sentinel-a491";
    const RESPONSE_API_KEY: &str = "response-api-key-sentinel-d212";

    let captured = CapturedEvents::default();
    let _guard = tracing::subscriber::set_default(captured.clone());
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    super::log_request(
        "POST",
        &endpoint,
        "/v1/sensitive",
        &[("token".to_string(), REQUEST_TOKEN.to_string())],
        Some(&json!({
            "skey": REQUEST_SKEY,
            "nested": {"password": REQUEST_PASSWORD},
            "payload": "x".repeat(4_096),
        })),
    );

    let server = test_support::server(vec![(
        400,
        json!({
            "error": "invalid gateway input",
            "credential": RESPONSE_CREDENTIAL,
            "api_key": RESPONSE_API_KEY,
            "details": "y".repeat(4_096),
        }),
    )]);
    let response = reqwest::Client::new()
        .get(format!("{}/failure", server.base_url()))
        .send()
        .await
        .expect("response");
    let error = super::response_error("POST", &endpoint, "/v1/sensitive", response).await;
    let logs = captured.joined();

    for secret in
        [REQUEST_TOKEN, REQUEST_SKEY, REQUEST_PASSWORD, RESPONSE_CREDENTIAL, RESPONSE_API_KEY]
    {
        assert!(!logs.contains(secret), "secret leaked into logs: {secret}");
        assert!(!error.contains(secret), "secret leaked into error: {secret}");
    }
    assert!(error.contains("400 Bad Request"));
    assert!(error.contains("invalid gateway input"));
    assert!(logs.len() < 2_048, "logs should contain only bounded summaries");
    assert!(error.len() < 1_024, "returned errors should be bounded");

    assert_eq!(server.take_request().path, "/failure");
    server.join();
}

#[tokio::test]
async fn response_error_omits_body_segment_when_response_is_empty() {
    let server = test_support::server_raw(vec![(204, String::new())]);
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    let response =
        reqwest::Client::new().get(format!("{}/empty", server.base_url())).send().await.unwrap();

    let error = super::response_error("GET", &endpoint, "/empty", response).await;

    assert_eq!(error, "gateway request failed: 204 No Content");
    assert_eq!(server.take_request().path, "/empty");
    server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn response_error_blocking_omits_body_segment_when_response_is_empty() {
    let server = test_support::server_raw(vec![(204, String::new())]);
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    let response = reqwest::blocking::Client::new()
        .get(format!("{}/empty", server.base_url()))
        .send()
        .unwrap();

    let error = super::response_error_blocking("GET", &endpoint, "/empty", response);

    assert_eq!(error, "gateway request failed: 204 No Content");
    assert_eq!(server.take_request().path, "/empty");
    server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn response_error_blocking_includes_status_and_body_when_present() {
    let server = test_support::server(vec![(404, json!({"error": "missing"}))]);
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    let response = reqwest::blocking::Client::new()
        .get(format!("{}/missing", server.base_url()))
        .send()
        .unwrap();

    let error = super::response_error_blocking("GET", &endpoint, "/missing", response);

    assert!(error.contains("404 Not Found"));
    assert!(error.contains("\"missing\""));
    assert_eq!(server.take_request().path, "/missing");
    server.join();
}

#[tokio::test]
async fn transport_error_returns_original_reqwest_message() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind unused port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    let endpoint = GatewayEndpoint::new("127.0.0.1", port);
    let error = reqwest::Client::new()
        .get(format!("{}/closed", endpoint.base_url()))
        .send()
        .await
        .expect_err("closed port should fail");

    let message = super::transport_error("GET", &endpoint, "/closed", error);

    assert!(!message.trim().is_empty());
    assert!(super::is_gateway_unavailable_error(&message));
}

#[test]
fn gateway_unavailable_error_classifies_connection_failures_only() {
    assert!(super::is_gateway_unavailable_error("error sending request for url"));
    assert!(super::is_gateway_unavailable_error("connection refused"));
    assert!(super::is_gateway_unavailable_error("request timed out"));
    assert!(!super::is_gateway_unavailable_error("gateway request failed: 500"));
    assert!(!super::is_gateway_unavailable_error("invalid api key"));
}

#[test]
fn log_helpers_accept_empty_and_non_empty_request_metadata() {
    let endpoint = GatewayEndpoint::new(" ", 8080);

    super::log_request::<Value>("GET", &endpoint, "/v1/plain", &[], None);
    super::log_request(
        "POST",
        &endpoint,
        "/v1/json",
        &[("q".to_string(), "1".to_string())],
        Some(&json!({"x": 1})),
    );
    super::log_request_succeeded("GET", &endpoint, "/v1/git/command");
    super::log_request_succeeded("GET", &endpoint, "/v1/other");
}

#[tokio::test(flavor = "current_thread")]
async fn message_delta_conflict_remains_an_error_without_generic_failure_log() {
    let captured = CapturedEvents::default();
    let _guard = tracing::subscriber::set_default(captured.clone());
    let endpoint = GatewayEndpoint::new("127.0.0.1", 0);
    let server = test_support::server(vec![(409, json!({"error": "stale session message delta"}))]);
    let response = reqwest::Client::new()
        .patch(format!("{}/v1/session/test/ui/messages", server.base_url()))
        .send()
        .await
        .expect("response");
    let error =
        super::response_error("PATCH", &endpoint, "/v1/session/test/ui/messages", response).await;
    assert!(error.contains("409 Conflict"));
    let logs = captured.joined();
    assert!(logs.contains("gateway session message version conflict"));
    assert!(!logs.contains("gateway request returned non-success status"));
    assert_eq!(server.take_request().path, "/v1/session/test/ui/messages");
    server.join();
}
