//! HTTP 请求工具的 URL、方法、安全策略测试。
//!
//! 这些测试重点覆盖 SSRF 防护边界、域名 allowlist 归一化、只读/速率限制阻断，
//! 以及响应截断等不需要真实外网访问的本地逻辑。

use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::url_validation::{
    is_private_or_local_host, normalize_allowed_domains, normalize_domain,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn test_tool(allowed_domains: Vec<&str>) -> HttpRequestTool {
    // 测试工具默认处于受监督模式，便于只聚焦 URL allowlist 和方法校验。
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        ..SecurityPolicy::default()
    });
    HttpRequestTool::new(
        security,
        allowed_domains.into_iter().map(String::from).collect(),
        1_000_000,
        30,
        "test".to_string(),
    )
}

async fn spawn_http_server(
    status: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let body = body.to_string();
    let headers =
        headers.iter().map(|(name, value)| format!("{name}: {value}\r\n")).collect::<String>();
    let response =
        format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\n{headers}\r\n{body}", body.len());
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = Vec::new();
        let mut chunk = [0; 1024];
        loop {
            let read = stream.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
            if http_request_is_complete(&buffer) {
                break;
            }
        }
        stream.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&buffer).to_string()
    });

    (format!("http://{addr}"), handle)
}

fn http_request_is_complete(buffer: &[u8]) -> bool {
    let request = String::from_utf8_lossy(buffer);
    let Some(header_end) = request.find("\r\n\r\n") else {
        return false;
    };
    let content_length = request
        .lines()
        .find_map(|line| line.strip_prefix("content-length: "))
        .or_else(|| request.lines().find_map(|line| line.strip_prefix("Content-Length: ")))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    buffer.len() >= header_end + 4 + content_length
}

#[test]
fn normalize_domain_strips_scheme_path_and_case() {
    let got = normalize_domain("  HTTPS://Docs.Example.com/path ").unwrap();
    assert_eq!(got, "docs.example.com");
}

#[test]
fn normalize_allowed_domains_deduplicates() {
    let got = normalize_allowed_domains(vec![
        "example.com".into(),
        "EXAMPLE.COM".into(),
        "https://example.com/".into(),
    ]);
    assert_eq!(got, vec!["example.com".to_string()]);
}

#[test]
fn validate_accepts_exact_domain() {
    let tool = test_tool(vec!["example.com"]);
    let got = tool.validate_url("https://example.com/docs").unwrap();
    assert_eq!(got, "https://example.com/docs");
}

#[test]
fn validate_accepts_http() {
    let tool = test_tool(vec!["example.com"]);
    assert!(tool.validate_url("http://example.com").is_ok());
}

#[test]
fn validate_accepts_subdomain() {
    let tool = test_tool(vec!["example.com"]);
    assert!(tool.validate_url("https://api.example.com/v1").is_ok());
}

#[test]
fn validate_accepts_wildcard_allowlist_for_public_host() {
    let tool = test_tool(vec!["*"]);
    assert!(tool.validate_url("https://news.ycombinator.com").is_ok());
}

#[test]
fn validate_wildcard_allowlist_still_rejects_private_host() {
    let tool = test_tool(vec!["*"]);
    let err = tool.validate_url("https://localhost:8080").unwrap_err().to_string();
    assert!(err.contains("local/private"));
}

#[test]
fn validate_accepts_wildcard_subdomain_pattern() {
    let tool = test_tool(vec!["*.example.com"]);
    assert!(tool.validate_url("https://example.com").is_ok());
    assert!(tool.validate_url("https://sub.example.com").is_ok());
    assert!(tool.validate_url("https://other.com").is_err());
}

#[test]
fn validate_rejects_allowlist_miss() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_url("https://google.com").unwrap_err().to_string();
    assert!(err.contains("allowed_domains"));
}

#[test]
fn validate_rejects_localhost() {
    let tool = test_tool(vec!["localhost"]);
    let err = tool.validate_url("https://localhost:8080").unwrap_err().to_string();
    assert!(err.contains("local/private"));
}

#[test]
fn validate_rejects_private_ipv4() {
    let tool = test_tool(vec!["192.168.1.5"]);
    let err = tool.validate_url("https://192.168.1.5").unwrap_err().to_string();
    assert!(err.contains("local/private"));
}

#[test]
fn validate_rejects_whitespace() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_url("https://example.com/hello world").unwrap_err().to_string();
    assert!(err.contains("whitespace"));
}

#[test]
fn validate_rejects_userinfo() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_url("https://user@example.com").unwrap_err().to_string();
    assert!(err.contains("userinfo"));
}

#[test]
fn validate_requires_allowlist() {
    let security = Arc::new(SecurityPolicy::default());
    let tool = HttpRequestTool::new(security, vec![], 1_000_000, 30, "test".to_string());
    let err = tool.validate_url("https://example.com").unwrap_err().to_string();
    assert!(err.contains("allowed_domains"));
}

#[test]
fn new_normalizes_allowed_domains_before_validation() {
    let tool = test_tool(vec![" HTTPS://API.Example.COM/path "]);

    assert!(tool.validate_url("https://api.example.com/v1").is_ok());
}

#[test]
fn validate_accepts_valid_methods() {
    let tool = test_tool(vec!["example.com"]);
    assert!(tool.validate_method("GET").is_ok());
    assert!(tool.validate_method("POST").is_ok());
    assert!(tool.validate_method("PUT").is_ok());
    assert!(tool.validate_method("DELETE").is_ok());
    assert!(tool.validate_method("PATCH").is_ok());
    assert!(tool.validate_method("HEAD").is_ok());
    assert!(tool.validate_method("OPTIONS").is_ok());
}

#[test]
fn validate_rejects_invalid_method() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_method("INVALID").unwrap_err().to_string();
    assert!(err.contains("Unsupported HTTP method"));
}

#[test]
fn validate_method_is_case_insensitive_and_returns_exact_method() {
    let tool = test_tool(vec!["example.com"]);

    assert_eq!(tool.validate_method("get").unwrap(), reqwest::Method::GET);
    assert_eq!(tool.validate_method("pOsT").unwrap(), reqwest::Method::POST);
    assert_eq!(tool.validate_method("options").unwrap(), reqwest::Method::OPTIONS);
}

#[test]
fn validate_method_rejects_surrounding_whitespace() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_method(" GET ").unwrap_err().to_string();

    assert!(err.contains("Unsupported HTTP method:  GET "));
}

#[test]
fn blocks_multicast_ipv4() {
    assert!(is_private_or_local_host("224.0.0.1"));
    assert!(is_private_or_local_host("239.255.255.255"));
}

#[test]
fn blocks_broadcast() {
    assert!(is_private_or_local_host("255.255.255.255"));
}

#[test]
fn blocks_reserved_ipv4() {
    assert!(is_private_or_local_host("240.0.0.1"));
    assert!(is_private_or_local_host("250.1.2.3"));
}

#[test]
fn blocks_documentation_ranges() {
    assert!(is_private_or_local_host("192.0.2.1"));
    assert!(is_private_or_local_host("198.51.100.1"));
    assert!(is_private_or_local_host("203.0.113.1"));
}

#[test]
fn blocks_benchmarking_range() {
    assert!(is_private_or_local_host("198.18.0.1"));
    assert!(is_private_or_local_host("198.19.255.255"));
}

#[test]
fn blocks_ipv6_localhost() {
    assert!(is_private_or_local_host("::1"));
    assert!(is_private_or_local_host("[::1]"));
}

#[test]
fn blocks_ipv6_multicast() {
    assert!(is_private_or_local_host("ff02::1"));
}

#[test]
fn blocks_ipv6_link_local() {
    assert!(is_private_or_local_host("fe80::1"));
}

#[test]
fn blocks_ipv6_unique_local() {
    assert!(is_private_or_local_host("fd00::1"));
}

#[test]
fn blocks_ipv4_mapped_ipv6() {
    assert!(is_private_or_local_host("::ffff:127.0.0.1"));
    assert!(is_private_or_local_host("::ffff:192.168.1.1"));
    assert!(is_private_or_local_host("::ffff:10.0.0.1"));
}

#[test]
fn allows_public_ipv4() {
    assert!(!is_private_or_local_host("8.8.8.8"));
    assert!(!is_private_or_local_host("1.1.1.1"));
    assert!(!is_private_or_local_host("93.184.216.34"));
}

#[test]
fn blocks_ipv6_documentation_range() {
    assert!(is_private_or_local_host("2001:db8::1"));
}

#[test]
fn allows_public_ipv6() {
    assert!(!is_private_or_local_host("2607:f8b0:4004:800::200e"));
}

#[test]
fn blocks_shared_address_space() {
    assert!(is_private_or_local_host("100.64.0.1"));
    assert!(is_private_or_local_host("100.127.255.255"));
    assert!(!is_private_or_local_host("100.63.0.1"));
    assert!(!is_private_or_local_host("100.128.0.1"));
}

#[tokio::test]
async fn execute_blocks_readonly_mode() {
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = HttpRequestTool::new(
        security,
        vec!["example.com".into()],
        1_000_000,
        30,
        "test".to_string(),
    );
    let result = tool.execute(json!({"url": "https://example.com"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("read-only"));
}

#[tokio::test]
async fn execute_blocks_when_rate_limited() {
    let security =
        Arc::new(SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() });
    let tool = HttpRequestTool::new(
        security,
        vec!["example.com".into()],
        1_000_000,
        30,
        "test".to_string(),
    );
    let result = tool.execute(json!({"url": "https://example.com"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("rate limit"));
}

#[test]
fn truncate_response_within_limit() {
    let tool = test_tool(vec!["example.com"]);
    let text = "hello world";
    assert_eq!(tool.truncate_response(text), "hello world");
}

#[test]
fn truncate_response_over_limit() {
    let tool = HttpRequestTool::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".into()],
        10,
        30,
        "test".to_string(),
    );
    let text = "hello world this is long";
    let truncated = tool.truncate_response(text);
    assert!(truncated.len() <= 10 + 60);
    assert!(truncated.contains("[Response truncated"));
}

#[test]
fn parse_headers_preserves_original_values() {
    let tool = test_tool(vec!["example.com"]);
    let headers = json!({
        "Authorization": "Bearer secret",
        "Content-Type": "application/json",
        "X-API-Key": "my-key"
    });
    let parsed = tool.parse_headers(&headers);
    assert_eq!(parsed.len(), 3);
    assert!(parsed.iter().any(|(k, v)| k == "Authorization" && v == "Bearer secret"));
    assert!(parsed.iter().any(|(k, v)| k == "X-API-Key" && v == "my-key"));
    assert!(parsed.iter().any(|(k, v)| k == "Content-Type" && v == "application/json"));
}

#[test]
fn parse_headers_keeps_only_string_values() {
    let tool = test_tool(vec!["example.com"]);
    let parsed = tool.parse_headers(&json!({
        "X-String": "kept",
        "X-Number": 42,
        "X-Bool": true,
        "X-Null": null,
        "X-Array": ["ignored"]
    }));

    assert_eq!(parsed, vec![("X-String".to_string(), "kept".to_string())]);
}

#[test]
fn parse_headers_returns_empty_for_non_object_values() {
    let tool = test_tool(vec!["example.com"]);

    assert!(tool.parse_headers(&json!(["X-Test", "value"])).is_empty());
    assert!(tool.parse_headers(&serde_json::Value::Null).is_empty());
}

#[test]
fn redact_headers_for_display_redacts_sensitive() {
    let headers = vec![
        ("Authorization".into(), "Bearer secret".into()),
        ("Content-Type".into(), "application/json".into()),
        ("X-API-Key".into(), "my-key".into()),
        ("X-Secret-Token".into(), "tok-123".into()),
    ];
    let redacted = HttpRequestTool::redact_headers_for_display(&headers);
    assert_eq!(redacted.len(), 4);
    assert!(redacted.iter().any(|(k, v)| k == "Authorization" && v == "***REDACTED***"));
    assert!(redacted.iter().any(|(k, v)| k == "X-API-Key" && v == "***REDACTED***"));
    assert!(redacted.iter().any(|(k, v)| k == "X-Secret-Token" && v == "***REDACTED***"));
    assert!(redacted.iter().any(|(k, v)| k == "Content-Type" && v == "application/json"));
}

#[test]
fn redact_headers_does_not_alter_original() {
    let headers = vec![("Authorization".into(), "Bearer real-token".into())];
    let _ = HttpRequestTool::redact_headers_for_display(&headers);
    assert_eq!(headers[0].1, "Bearer real-token");
}

#[test]
fn redact_headers_recognizes_apikey_without_hyphen_case_insensitively() {
    let headers = vec![
        ("X-aPiKeY".into(), "secret-value".into()),
        ("X-Correlation-Id".into(), "request-123".into()),
    ];

    let redacted = HttpRequestTool::redact_headers_for_display(&headers);

    assert_eq!(redacted[0], ("X-aPiKeY".into(), "***REDACTED***".into()));
    assert_eq!(redacted[1], ("X-Correlation-Id".into(), "request-123".into()));
}

#[test]
fn truncate_response_preserves_utf8_character_boundaries() {
    let tool = HttpRequestTool::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".into()],
        1,
        30,
        "test".to_string(),
    );

    let truncated = tool.truncate_response("éclair");

    assert!(truncated.starts_with("é\n\n... [Response truncated"));
}

#[test]
fn truncate_response_keeps_text_at_exact_byte_limit() {
    let tool = HttpRequestTool::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".into()],
        2,
        30,
        "test".to_string(),
    );

    assert_eq!(tool.truncate_response("é"), "é");
}

#[test]
fn parameters_schema_requires_url_and_defaults_optional_fields() {
    let tool = test_tool(vec!["example.com"]);
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["url"]));
    assert_eq!(schema["properties"]["url"]["type"], "string");
    assert_eq!(schema["properties"]["method"]["default"], "GET");
    assert_eq!(schema["properties"]["headers"]["default"], json!({}));
    assert_eq!(schema["properties"]["body"]["type"], "string");
}

#[test]
fn description_advertises_supported_methods_and_safety_constraints() {
    let tool = test_tool(vec!["example.com"]);
    let description = tool.description();

    assert!(description.contains("GET、POST、PUT、DELETE、PATCH、HEAD、OPTIONS"));
    assert!(description.contains("allowlisted domains"));
    assert!(description.contains("no local/private hosts"));
}

#[tokio::test]
async fn execute_rejects_missing_url_parameter_before_request() {
    let tool = test_tool(vec!["example.com"]);

    let err = tool.execute(json!({})).await.unwrap_err().to_string();

    assert_eq!(err, "Missing 'url' parameter");
}

#[tokio::test]
async fn execute_rejects_non_string_url_parameter_before_request() {
    let tool = test_tool(vec!["example.com"]);

    let err = tool.execute(json!({"url": 42})).await.unwrap_err().to_string();

    assert_eq!(err, "Missing 'url' parameter");
}

#[tokio::test]
async fn execute_returns_failed_result_for_rejected_url() {
    let tool = test_tool(vec!["example.com"]);

    let result = tool.execute(json!({"url": "ftp://example.com"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    let error = result.error.unwrap();
    assert!(error.contains("http://") || error.contains("https://"));
}

#[tokio::test]
async fn execute_returns_failed_result_for_unsupported_method() {
    let tool = test_tool(vec!["example.com"]);

    let result =
        tool.execute(json!({"url": "https://example.com", "method": "TRACE"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert!(result.error.unwrap().contains("Unsupported HTTP method: TRACE"));
}

#[test]
fn ssrf_octal_loopback_not_parsed_as_ip() {
    assert!(!is_private_or_local_host("0177.0.0.1"));
}

#[test]
fn ssrf_hex_loopback_not_parsed_as_ip() {
    assert!(!is_private_or_local_host("0x7f000001"));
}

#[test]
fn ssrf_decimal_loopback_not_parsed_as_ip() {
    assert!(!is_private_or_local_host("2130706433"));
}

#[test]
fn ssrf_zero_padded_loopback_not_parsed_as_ip() {
    assert!(!is_private_or_local_host("127.000.000.001"));
}

#[test]
fn ssrf_alternate_notations_rejected_by_validate_url() {
    let tool = test_tool(vec!["example.com"]);
    for notation in
        ["http://0177.0.0.1", "http://0x7f000001", "http://2130706433", "http://127.000.000.001"]
    {
        let err = tool.validate_url(notation).unwrap_err().to_string();
        assert!(
            err.contains("allowed_domains"),
            "Expected allowlist rejection for {notation}, got: {err}"
        );
    }
}

#[test]
fn redirect_policy_is_none() {
    let tool = test_tool(vec!["example.com"]);
    assert_eq!(tool.name(), "http_request");
}

#[test]
fn ssrf_blocks_loopback_127_range() {
    assert!(is_private_or_local_host("127.0.0.1"));
    assert!(is_private_or_local_host("127.0.0.2"));
    assert!(is_private_or_local_host("127.255.255.255"));
}

#[test]
fn ssrf_blocks_rfc1918_10_range() {
    assert!(is_private_or_local_host("10.0.0.1"));
    assert!(is_private_or_local_host("10.255.255.255"));
}

#[test]
fn ssrf_blocks_rfc1918_172_range() {
    assert!(is_private_or_local_host("172.16.0.1"));
    assert!(is_private_or_local_host("172.31.255.255"));
}

#[test]
fn ssrf_blocks_unspecified_address() {
    assert!(is_private_or_local_host("0.0.0.0"));
}

#[test]
fn ssrf_blocks_dot_localhost_subdomain() {
    assert!(is_private_or_local_host("evil.localhost"));
    assert!(is_private_or_local_host("a.b.localhost"));
}

#[test]
fn ssrf_blocks_dot_local_tld() {
    assert!(is_private_or_local_host("service.local"));
}

#[test]
fn ssrf_ipv6_unspecified() {
    assert!(is_private_or_local_host("::"));
}

#[test]
fn validate_rejects_ftp_scheme() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_url("ftp://example.com").unwrap_err().to_string();
    assert!(err.contains("http://") || err.contains("https://"));
}

#[test]
fn validate_rejects_empty_url() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_url("").unwrap_err().to_string();
    assert!(err.contains("empty"));
}

#[test]
fn validate_rejects_ipv6_host() {
    let tool = test_tool(vec!["example.com"]);
    let err = tool.validate_url("http://[::1]:8080/path").unwrap_err().to_string();
    assert!(err.contains("IPv6"));
}

#[test]
fn truncate_response_with_zero_limit_returns_only_truncation_notice() {
    let tool = HttpRequestTool::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".into()],
        0,
        30,
        "test".to_string(),
    );

    assert_eq!(
        tool.truncate_response("payload"),
        "\n\n... [Response truncated due to size limit] ..."
    );
}

#[tokio::test]
async fn execute_uses_default_method_before_rejecting_disallowed_domain() {
    let tool = test_tool(vec!["example.com"]);

    let result = tool
        .execute(json!({
            "url": "https://not-allowed.example.net/path",
            "headers": null,
            "body": 42
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert!(result.error.as_deref().unwrap().contains("allowed_domains"));
}

#[tokio::test]
async fn execute_request_sends_headers_body_and_formats_success_response() {
    let tool = HttpRequestTool::new(
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        }),
        vec!["example.com".into()],
        1_000_000,
        0,
        "unit-agent".to_string(),
    );
    let (base_url, request) = spawn_http_server(
        "200 OK",
        &[("Set-Cookie", "session=secret"), ("X-Trace", "abc")],
        "ok-body",
    )
    .await;

    let response = tool
        .execute_request(
            &format!("{base_url}/submit"),
            reqwest::Method::POST,
            vec![("X-Request".into(), "value".into())],
            Some("payload"),
        )
        .await
        .unwrap();
    let result = tool.format_response(response).await;
    let captured_request = request.await.unwrap();

    assert!(result.success);
    assert_eq!(result.error, None);
    assert!(result.output.contains("Status: 200 OK"));
    assert!(result.output.contains("set-cookie: ***REDACTED***"));
    assert!(result.output.contains("x-trace: \"x-trace\""));
    assert!(result.output.contains("Response Body:\nok-body"));
    assert!(captured_request.contains("POST /submit HTTP/1.1"));
    assert!(captured_request.contains("x-request: value"));
    assert!(captured_request.contains("user-agent: unit-agent"));
    assert!(captured_request.ends_with("\r\n\r\npayload"));
}

#[tokio::test]
async fn format_response_marks_server_error_statuses_as_failed_results() {
    let tool = test_tool(vec!["example.com"]);
    let (base_url, request) =
        spawn_http_server("503 Service Unavailable", &[("X-Error", "true")], "unavailable").await;

    let response =
        tool.execute_request(&base_url, reqwest::Method::GET, Vec::new(), None).await.unwrap();
    let result = tool.format_response(response).await;
    let captured_request = request.await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("HTTP 503"));
    assert!(result.output.contains("Status: 503 Service Unavailable"));
    assert!(result.output.contains("Response Body:\nunavailable"));
    assert!(captured_request.contains("GET / HTTP/1.1"));
}
