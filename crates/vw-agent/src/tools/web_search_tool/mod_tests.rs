//! 验证多提供方网页搜索的结果解析、规格生成及请求前参数校验。
//! Serper、Claude 兼容字段和用户代理错误均在网络分派之前完成断言。

use super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::sync::Arc;

fn tool(provider: &str, key: Option<&str>) -> WebSearchTool {
    WebSearchTool::new(
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        }),
        provider.to_string(),
        key.map(str::to_string),
        None,
        2,
        0,
        "ua".to_string(),
    )
}

#[test]
fn constructor_helpers_and_result_parsers_are_covered() {
    let tool = tool(" BRAVE ", Some("a, ,b"));
    assert_eq!(tool.provider, "brave");
    assert_eq!(tool.timeout_secs, 1);
    assert_eq!(tool.get_next_api_key().as_deref(), Some("a"));
    assert_eq!(tool.provider_label(), "Brave");
    assert_eq!(tool.resolve_serper_endpoint("bing"), "https://bing.serper.dev/search");

    let (provider, rows) = WebSearchTool::parse_formatted_results(
        "Search results for: q (via Brave)\n1. One\n   https://one.test\n   A\n   B",
    );
    assert_eq!(provider.as_deref(), Some("Brave"));
    assert_eq!(rows[0].snippet.as_deref(), Some("A B"));

    let ddg = tool.parse_duckduckgo_results(
        r#"<a class="result__a" href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com">Title</a><a class="result__snippet">Desc</a>"#,
        "q",
    ).unwrap();
    assert!(ddg.contains("https://example.com"));
    assert!(
        tool.parse_serper_results(&json!({"organic": []}), "q", "serper")
            .unwrap()
            .contains("No results")
    );
    assert!(tool.parse_serper_results(&json!({}), "q", "serper").is_err());
    assert_eq!(decode_ddg_redirect_url("x?uddg=https%3A%2F%2Fe.test&rut=1"), "https://e.test");
    assert_eq!(strip_tags("<b>x</b>"), "x");
}

#[test]
fn search_result_item_serialization_omits_absent_optional_fields() {
    let without_optional_fields =
        SearchResultItem { title: "Only title".to_string(), url: None, snippet: None };
    let with_optional_fields = SearchResultItem {
        title: "Complete".to_string(),
        url: Some("https://example.test".to_string()),
        snippet: Some("Summary".to_string()),
    };

    assert_eq!(
        serde_json::to_value(without_optional_fields).unwrap(),
        json!({"title": "Only title"})
    );
    assert_eq!(
        serde_json::to_value(with_optional_fields).unwrap(),
        json!({
            "title": "Complete",
            "url": "https://example.test",
            "snippet": "Summary"
        })
    );
}

#[test]
fn formatted_results_keep_first_url_and_append_later_lines_to_snippet() {
    let (provider, rows) = WebSearchTool::parse_formatted_results(
        "Search results for: q (via DuckDuckGo)\n\
         1. Result\n\
         http://first.test\n\
         https://second.test\n\
         final detail",
    );

    assert_eq!(provider.as_deref(), Some("DuckDuckGo"));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].url.as_deref(), Some("http://first.test"));
    assert_eq!(rows[0].snippet.as_deref(), Some("https://second.test final detail"));
}

#[test]
fn build_client_accepts_valid_and_rejects_invalid_user_agent() {
    assert!(tool("duckduckgo", None).build_client().is_ok());

    let invalid = WebSearchTool::new(
        Arc::new(SecurityPolicy::default()),
        "duckduckgo".to_string(),
        None,
        None,
        2,
        1,
        "invalid\nuser-agent".to_string(),
    );
    assert!(invalid.build_client().is_err());
}

#[test]
fn serper_parser_prefers_organic_results_and_omits_blank_description() {
    let output = tool("serper", Some("key"))
        .parse_serper_results(
            &json!({
                "organic": [{
                    "title": "Organic",
                    "link": "https://organic.test",
                    "snippet": "   "
                }],
                "results": [{
                    "title": "Fallback",
                    "url": "https://fallback.test",
                    "description": "fallback"
                }]
            }),
            "q",
            "serper",
        )
        .unwrap();

    assert!(output.contains("Organic"));
    assert!(output.contains("https://organic.test"));
    assert!(!output.contains("Fallback"));
    assert!(!output.ends_with("   "));
}

#[test]
fn args_schema_and_spec_cover_claude_surface() {
    let args: Args =
        serde_json::from_value(json!({"query": "rust", "numResults": 3, "lang": "en"})).unwrap();
    assert_eq!(args.query, "rust");
    assert_eq!(args.num, Some(3));
    assert_eq!(args.lr.as_deref(), Some("en"));

    let tool = tool("duckduckgo", None);
    assert!(tool.parameters_schema()["properties"]["numResults"].is_object());
    assert!(tool.spec().read_only);
}

#[tokio::test]
async fn execute_rejects_readonly_empty_unknown_and_missing_key_without_network() {
    let readonly = WebSearchTool::new(
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() }),
        "duckduckgo".into(),
        None,
        None,
        2,
        1,
        "ua".into(),
    );
    assert!(
        readonly
            .execute(json!({"query": "rust"}))
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("read-only")
    );
    assert!(tool("duckduckgo", None).execute(json!({"query": ""})).await.is_err());
    assert!(
        tool("unknown", None)
            .execute(json!({"query": "rust"}))
            .await
            .unwrap_err()
            .to_string()
            .contains("Unknown search provider")
    );
    assert!(
        tool("brave", None)
            .execute(json!({"query": "rust"}))
            .await
            .unwrap_err()
            .to_string()
            .contains("API key")
    );
}

#[tokio::test]
async fn execute_rejects_invalid_parameter_types_before_provider_dispatch() {
    let error =
        tool("duckduckgo", None).execute(json!({"query": 7, "num": "many"})).await.unwrap_err();

    assert!(error.to_string().contains("Missing or invalid parameters"));
}
