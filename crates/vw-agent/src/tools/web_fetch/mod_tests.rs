//! 验证网页抓取工具的配置、内容转换及发起网络请求前的输入拒绝。
//! 内容格式与密钥轮换使用纯辅助路径验证，测试不依赖外部网络。

use super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::sync::Arc;

fn tool(provider: &str) -> WebFetchTool {
    WebFetchTool::new(
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        }),
        provider.to_string(),
        Some("a, ,b".to_string()),
        None,
        vec!["example.com".to_string()],
        vec!["blocked.example.com".to_string()],
        3,
        0,
        "ua".to_string(),
    )
}

#[test]
fn constructor_validation_timeout_truncation_and_conversion_helpers() {
    let tool = tool("");
    assert_eq!(tool.provider, "fast_html2md");
    assert_eq!(tool.api_keys, vec!["a", "b"]);
    assert_eq!(tool.get_next_api_key().as_deref(), Some("a"));
    assert_eq!(tool.get_next_api_key().as_deref(), Some("b"));
    assert_eq!(tool.validate_url("http://example.com/path").unwrap(), "http://example.com/path");
    assert!(tool.validate_url("https://blocked.example.com").is_err());
    assert_eq!(tool.effective_timeout_secs(None), DEFAULT_TIMEOUT_SECS);
    assert_eq!(tool.effective_timeout_secs(Some(999)), MAX_TIMEOUT_SECS);
    assert!(tool.truncate_response("你好世界").contains("Response truncated"));
    assert!(WebFetchTool::select_accept_header(Format::Html).contains("text/html"));
    assert!(WebFetchTool::markdown_to_text("# Title\n`code`").contains("Title"));
    assert_eq!(tool.convert_http_output("{}", "application/json", Format::Markdown).unwrap(), "{}");
    assert_eq!(
        tool.convert_http_output("<h1>x</h1>", "text/html", Format::Html).unwrap(),
        "<h1>x</h1>"
    );
}

#[test]
fn api_key_rotation_handles_empty_configuration_and_wraps() {
    let without_keys = WebFetchTool::new(
        Arc::new(SecurityPolicy::default()),
        "fast_html2md".to_string(),
        None,
        None,
        vec!["example.com".to_string()],
        vec![],
        100,
        30,
        "ua".to_string(),
    );
    assert_eq!(without_keys.get_next_api_key(), None);

    let rotating = tool("fast_html2md");
    assert_eq!(rotating.get_next_api_key().as_deref(), Some("a"));
    assert_eq!(rotating.get_next_api_key().as_deref(), Some("b"));
    assert_eq!(rotating.get_next_api_key().as_deref(), Some("a"));
}

#[test]
fn non_html_http_output_is_preserved_for_every_requested_format() {
    let tool = tool("unknown");
    let body = "plain <content> that must stay unchanged";

    for format in [Format::Text, Format::Markdown, Format::Html] {
        assert_eq!(
            tool.convert_http_output(body, "text/plain; charset=utf-8", format).unwrap(),
            body
        );
    }
}

#[test]
fn schema_spec_and_unknown_provider_are_covered() {
    let tool = tool("unknown");
    assert!(
        tool.convert_html_to_output("<p>x</p>")
            .unwrap_err()
            .to_string()
            .contains("Unknown web_fetch provider")
    );

    let spec = tool.spec();
    assert_eq!(spec.id, crate::app::agent::tools::WEB_FETCH_TOOL_ID);
    assert!(spec.read_only);
    assert!(spec.concurrency_safe);
    assert!(tool.parameters_schema()["properties"]["href"].is_object());
}

#[tokio::test]
async fn execute_rejects_invalid_inputs_before_network() {
    let readonly = WebFetchTool::new(
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() }),
        "fast_html2md".to_string(),
        None,
        None,
        vec!["example.com".into()],
        vec![],
        100,
        30,
        "ua".into(),
    );
    assert!(
        readonly
            .execute(json!({"url": "https://example.com"}))
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("read-only")
    );

    assert!(
        tool("fast_html2md")
            .execute(json!({"url": "  "}))
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("Missing 'url'")
    );
    assert!(
        tool("fast_html2md")
            .execute(json!({"url": "https://other.com"}))
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("not in web_fetch.allowed_domains")
    );
    let tavily_without_key = WebFetchTool::new(
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        }),
        "tavily".to_string(),
        None,
        None,
        vec!["example.com".into()],
        vec![],
        100,
        30,
        "ua".into(),
    );
    assert!(
        tavily_without_key
            .execute(json!({"url": "https://example.com"}))
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("api_key")
    );
}
