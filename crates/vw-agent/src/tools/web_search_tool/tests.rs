//! 验证 Web 搜索工具的参数、结果解析和 API key 轮换行为。
//! 测试保持在工具边界内，避免真实网络调用影响安全策略与可重复性。

use std::sync::Arc;

use super::super::traits::Tool;
use super::super::web_search_tool::{WebSearchTool, decode_ddg_redirect_url, strip_tags};
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::{WEB_SEARCH_TOOL_ALIAS, WEB_SEARCH_TOOL_ID};
use serde_json::json;

fn new_tool(provider: &str, api_key: Option<&str>) -> WebSearchTool {
    new_tool_with_options(provider, api_key, None, 5)
}

fn new_tool_with_options(
    provider: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    max_results: usize,
) -> WebSearchTool {
    WebSearchTool::new(
        test_security(),
        provider.to_string(),
        api_key.map(ToString::to_string),
        api_url.map(ToString::to_string),
        max_results,
        15,
        "test".to_string(),
    )
}

fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy { autonomy: AutonomyLevel::Supervised, ..SecurityPolicy::default() })
}

#[test]
fn test_tool_name() {
    let tool = new_tool("duckduckgo", None);
    assert_eq!(tool.name(), "web_search_tool");
}

#[test]
fn test_tool_description() {
    let tool = new_tool("duckduckgo", None);
    assert!(
        tool.description().contains("网络搜索") || tool.description().contains("Search the web")
    );
}

#[test]
fn test_parameters_schema() {
    let tool = new_tool("duckduckgo", None);
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["query"].is_object());
    assert!(schema["properties"]["num"].is_object());
    assert!(schema["properties"]["numResults"].is_object());
    assert!(schema["properties"]["lr"].is_object());
}

#[test]
fn test_tool_spec_uses_claude_surface() {
    let tool = new_tool("duckduckgo", None);
    let spec = tool.spec();

    assert_eq!(spec.id, WEB_SEARCH_TOOL_ID);
    assert!(spec.aliases.iter().any(|alias| alias == WEB_SEARCH_TOOL_ALIAS));
}

#[test]
fn test_strip_tags() {
    let html = "<b>Hello</b> <i>World</i>";
    assert_eq!(strip_tags(html), "Hello World");
}

#[test]
fn test_parse_duckduckgo_results_empty() {
    let tool = new_tool("duckduckgo", None);
    let result = tool.parse_duckduckgo_results("<html>No results here</html>", "test").unwrap();
    assert!(result.contains("No results found"));
}

#[test]
fn test_parse_duckduckgo_results_with_data() {
    let tool = new_tool("duckduckgo", None);
    let html = r#"
            <a class="result__a" href="https://example.com">Example Title</a>
            <a class="result__snippet">This is a description</a>
        "#;
    let result = tool.parse_duckduckgo_results(html, "test").unwrap();
    assert!(result.contains("Example Title"));
    assert!(result.contains("https://example.com"));
}

#[test]
fn test_parse_duckduckgo_results_decodes_redirect_url() {
    let tool = new_tool("duckduckgo", None);
    let html = r#"
            <a class="result__a" href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpath%3Fa%3D1&amp;rut=test">Example Title</a>
            <a class="result__snippet">This is a description</a>
        "#;
    let result = tool.parse_duckduckgo_results(html, "test").unwrap();
    assert!(result.contains("https://example.com/path?a=1"));
    assert!(!result.contains("rut=test"));
}

#[test]
fn test_constructor_clamps_web_search_limits() {
    let tool = WebSearchTool::new(
        test_security(),
        "duckduckgo".to_string(),
        None,
        None,
        0,
        0,
        "test".to_string(),
    );
    let html = r#"
            <a class="result__a" href="https://example.com">Example Title</a>
            <a class="result__snippet">This is a description</a>
        "#;
    let result = tool.parse_duckduckgo_results(html, "test").unwrap();
    assert!(result.contains("Example Title"));
}

#[test]
fn test_args_accept_claude_compat_fields() {
    let args: super::super::web_search_tool::Args = serde_json::from_value(json!({
        "query": "rust",
        "num": 3,
        "lr": "lang_en"
    }))
    .unwrap();

    assert_eq!(args.query, "rust");
    assert_eq!(args.num, Some(3));
    assert_eq!(args.lr.as_deref(), Some("lang_en"));
}

#[tokio::test]
async fn test_execute_missing_query() {
    let tool = new_tool("duckduckgo", None);
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_empty_query() {
    let tool = new_tool("duckduckgo", None);
    let result = tool.execute(json!({"query": ""})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_brave_without_api_key() {
    let tool = new_tool("brave", None);
    let result = tool.execute(json!({"query": "test"})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("API key"));
}

#[tokio::test]
async fn test_execute_firecrawl_without_api_key() {
    let tool = new_tool("firecrawl", None);
    let result = tool.execute(json!({"query": "test"})).await;
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    if cfg!(feature = "firecrawl") {
        assert!(error.contains("api_key"));
    } else {
        assert!(error.contains("requires Cargo feature 'firecrawl'"));
    }
}

#[tokio::test]
async fn test_execute_blocked_in_read_only_mode() {
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = WebSearchTool::new(
        security,
        "duckduckgo".to_string(),
        None,
        None,
        5,
        15,
        "test".to_string(),
    );
    let result = tool.execute(json!({"query": "rust"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("read-only"));
}

#[tokio::test]
async fn test_execute_tavily_without_api_key() {
    let tool = new_tool("tavily", None);
    let result = tool.execute(json!({"query": "test"})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("api_key"));
}

#[test]
fn test_parse_serper_results_with_data() {
    let tool = new_tool("serper", Some("key"));
    let parsed = json!({
        "organic": [
            {
                "title": "Example Title",
                "link": "https://example.com",
                "snippet": "Example snippet"
            }
        ]
    });

    let result = tool.parse_serper_results(&parsed, "test", "serper").unwrap();
    assert!(result.contains("via Serper"));
    assert!(result.contains("Example Title"));
    assert!(result.contains("https://example.com"));
    assert!(result.contains("Example snippet"));
}

#[test]
fn test_parse_serper_results_supports_bing_label() {
    let tool = new_tool("bing", Some("key"));
    let parsed = json!({
        "organic": [
            {
                "title": "Bing Title",
                "link": "https://bing.example.com",
                "snippet": "Bing snippet"
            }
        ]
    });

    let result = tool.parse_serper_results(&parsed, "test", "bing").unwrap();
    assert!(result.contains("via Bing"));
}

#[tokio::test]
async fn test_execute_serper_without_api_key() {
    let tool = new_tool("serper", None);
    let result = tool.execute(json!({"query": "test"})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("api_key"));
}

#[tokio::test]
async fn test_execute_google_without_api_key() {
    let tool = new_tool("google", None);
    let result = tool.execute(json!({"query": "test"})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("api_key"));
}

#[tokio::test]
async fn test_execute_bing_without_api_key() {
    let tool = new_tool("bing", None);
    let result = tool.execute(json!({"query": "test"})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("api_key"));
}

#[test]
fn test_multiple_api_keys_parsing() {
    let tool = new_tool("tavily", Some("key1,key2,key3"));
    assert_eq!(tool.api_keys.len(), 3);
    assert_eq!(tool.api_keys[0], "key1");
    assert_eq!(tool.api_keys[1], "key2");
    assert_eq!(tool.api_keys[2], "key3");
}

#[test]
fn test_multiple_api_keys_with_spaces() {
    let tool = new_tool("tavily", Some("key1, key2 , key3"));
    assert_eq!(tool.api_keys.len(), 3);
    assert_eq!(tool.api_keys[0], "key1");
    assert_eq!(tool.api_keys[1], "key2");
    assert_eq!(tool.api_keys[2], "key3");
}

#[test]
fn test_round_robin_api_key_selection() {
    let tool = new_tool("tavily", Some("key1,key2,key3"));

    assert_eq!(tool.get_next_api_key().unwrap(), "key1");
    assert_eq!(tool.get_next_api_key().unwrap(), "key2");
    assert_eq!(tool.get_next_api_key().unwrap(), "key3");
    assert_eq!(tool.get_next_api_key().unwrap(), "key1");
}

#[test]
fn test_empty_api_key_returns_none() {
    let tool = new_tool("tavily", None);
    assert!(tool.get_next_api_key().is_none());
}

#[test]
fn test_single_api_key_works() {
    let tool = new_tool("tavily", Some("single-key"));
    assert_eq!(tool.api_keys.len(), 1);
    assert_eq!(tool.get_next_api_key().unwrap(), "single-key");
}

#[test]
fn with_max_results_clamps_override_and_shares_key_rotation() {
    let tool = new_tool("tavily", Some("key1,key2"));
    let upper = tool.with_max_results(99);
    let lower = tool.with_max_results(0);

    assert_eq!(tool.max_results, 5);
    assert_eq!(upper.max_results, 10);
    assert_eq!(lower.max_results, 1);
    assert_eq!(upper.get_next_api_key().as_deref(), Some("key1"));
    assert_eq!(tool.get_next_api_key().as_deref(), Some("key2"));
    assert_eq!(lower.get_next_api_key().as_deref(), Some("key1"));
}

#[test]
fn resolve_serper_endpoint_normalizes_custom_and_default_urls() {
    let default = new_tool("google", Some("key"));
    let blank = new_tool_with_options("google", Some("key"), Some("  "), 5);
    let custom =
        new_tool_with_options("google", Some("key"), Some(" https://search.example.test/api/ "), 5);
    let complete =
        new_tool_with_options("google", Some("key"), Some("https://search.example.test/search"), 5);

    assert_eq!(default.resolve_serper_endpoint("google"), "https://google.serper.dev/search");
    assert_eq!(blank.resolve_serper_endpoint("google"), "https://google.serper.dev/search");
    assert_eq!(custom.resolve_serper_endpoint("google"), "https://search.example.test/api/search");
    assert_eq!(complete.resolve_serper_endpoint("google"), "https://search.example.test/search");
}

#[test]
fn provider_label_maps_aliases_and_preserves_unknown_provider() {
    for (provider, expected) in [
        ("ddg", "DuckDuckGo"),
        ("duckduckgo", "DuckDuckGo"),
        ("brave", "Brave"),
        ("serper", "Serper"),
        ("google", "Google"),
        ("bing", "Bing"),
        ("firecrawl", "Firecrawl"),
        ("tavily", "Tavily"),
        ("custom", "custom"),
    ] {
        assert_eq!(new_tool(provider, None).provider_label(), expected);
    }
}

#[test]
fn formatted_results_handle_empty_no_result_and_non_row_outputs() {
    let (provider, results) = WebSearchTool::parse_formatted_results("");
    assert_eq!(provider, None);
    assert!(results.is_empty());

    let (provider, results) =
        WebSearchTool::parse_formatted_results("No results found for: rust\nignored");
    assert_eq!(provider, None);
    assert!(results.is_empty());

    let (provider, results) = WebSearchTool::parse_formatted_results(
        "Search results for: rust\nnot a numbered row\n. missing number\n1. \n",
    );
    assert_eq!(provider, None);
    assert!(results.is_empty());
}

#[test]
fn formatted_results_preserve_optional_fields_and_ignore_leading_noise() {
    let (provider, results) = WebSearchTool::parse_formatted_results(
        "Search results for: rust (via Google)\nignored before first row\n1. First\nfirst snippet\nsecond snippet\n2. Second\nhttps://second.test",
    );

    assert_eq!(provider.as_deref(), Some("Google"));
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].title, "First");
    assert_eq!(results[0].url, None);
    assert_eq!(results[0].snippet.as_deref(), Some("first snippet second snippet"));
    assert_eq!(results[1].title, "Second");
    assert_eq!(results[1].url.as_deref(), Some("https://second.test"));
    assert_eq!(results[1].snippet, None);
}

#[test]
fn parse_duckduckgo_results_caps_rows_and_omits_empty_snippet() {
    let tool = new_tool("duckduckgo", None).with_max_results(1);
    let html = r#"
        <a class="result__a" href="https://one.test">First <b>Title</b></a>
        <a class="result__a" href="https://two.test">Second Title</a>
        <a class="result__snippet"></a>
        <a class="result__snippet">Second snippet</a>
    "#;

    let output = tool.parse_duckduckgo_results(html, "rust").unwrap();

    assert_eq!(
        output,
        "Search results for: rust (via DuckDuckGo)\n1. First Title\n   https://one.test"
    );
}

#[test]
fn decode_ddg_redirect_url_preserves_plain_and_invalid_utf8_urls() {
    let plain = "https://example.test/path";
    let invalid = "https://duckduckgo.com/l/?uddg=%FF&rut=1";

    assert_eq!(decode_ddg_redirect_url(plain), plain);
    assert_eq!(decode_ddg_redirect_url(invalid), invalid);
}

#[test]
fn parse_brave_results_rejects_missing_array_and_reports_empty_results() {
    let tool = new_tool("brave", Some("key"));

    let error = tool.parse_brave_results(&json!({"web": {}}), "rust").unwrap_err();
    assert_eq!(error.to_string(), "Invalid Brave API response");
    assert_eq!(
        tool.parse_brave_results(&json!({"web": {"results": []}}), "rust").unwrap(),
        "No results found for: rust"
    );
}

#[test]
fn parse_brave_results_caps_rows_and_uses_missing_field_defaults() {
    let tool = new_tool("brave", Some("key")).with_max_results(2);
    let response = json!({
        "web": {
            "results": [
                {
                    "title": "First",
                    "url": "https://first.test",
                    "description": "First description"
                },
                {},
                {"title": "Excluded"}
            ]
        }
    });

    let output = tool.parse_brave_results(&response, "rust").unwrap();

    assert!(output.contains("Search results for: rust (via Brave)"));
    assert!(output.contains("1. First"));
    assert!(output.contains("https://first.test"));
    assert!(output.contains("First description"));
    assert!(output.contains("2. No title"));
    assert!(!output.contains("Excluded"));
}

#[test]
fn parse_serper_results_accepts_fallback_fields_and_google_label() {
    let tool = new_tool("google", Some("key"));
    let response = json!({
        "results": [
            {
                "title": "Fallback result",
                "url": "https://fallback.test",
                "description": "  Fallback description  "
            },
            {}
        ]
    });

    let output = tool.parse_serper_results(&response, "rust", "google").unwrap();

    assert!(output.contains("Search results for: rust (via Google)"));
    assert!(output.contains("Fallback result"));
    assert!(output.contains("https://fallback.test"));
    assert!(output.contains("   Fallback description"));
    assert!(!output.contains("Fallback description  "));
    assert!(output.contains("2. No title"));
}

#[test]
fn args_accept_snake_case_and_max_results_aliases() {
    let snake_case: super::super::web_search_tool::Args = serde_json::from_value(json!({
        "query": "rust",
        "num_results": 4,
        "language": "zh"
    }))
    .unwrap();
    let max_results: super::super::web_search_tool::Args = serde_json::from_value(json!({
        "query": "rust",
        "max_results": 9
    }))
    .unwrap();

    assert_eq!(snake_case.num, Some(4));
    assert_eq!(snake_case.lr.as_deref(), Some("zh"));
    assert_eq!(max_results.num, Some(9));
}

#[tokio::test]
async fn execute_rate_limit_rejection_precedes_network_work() {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        max_actions_per_hour: 0,
        ..SecurityPolicy::default()
    });
    let tool = WebSearchTool::new(
        security,
        "duckduckgo".to_string(),
        None,
        None,
        5,
        15,
        "test".to_string(),
    );

    let result = tool.execute(json!({"query": "rust"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Action blocked: rate limit exceeded"));
}

#[tokio::test]
async fn call_read_only_failure_includes_render_metadata_without_network() {
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool =
        WebSearchTool::new(security, " GoOgLe ".to_string(), None, None, 5, 15, "test".to_string());

    let result = tool
        .call(json!({
            "query": "  rust async  ",
            "numResults": 3,
            "lang": "en"
        }))
        .await
        .unwrap();

    assert!(!result.is_success());
    assert_eq!(result.model_result, json!("Action blocked: autonomy is read-only"));
    let hint = result.render_hint.expect("read-only rejection should include a render hint");
    assert_eq!(hint.summary.as_deref(), Some("Web search failed for rust async"));
    assert_eq!(hint.metadata["query"], "rust async");
    assert_eq!(hint.metadata["provider"], "Google");
    assert_eq!(hint.metadata["requested_num"], 3);
    assert_eq!(hint.metadata["lr"], "en");
}
