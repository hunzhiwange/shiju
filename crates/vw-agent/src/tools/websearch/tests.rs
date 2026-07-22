//! 网络搜索工具集成测试模块
//!
//! 本模块包含针对网络搜索工具（WebSearch Tool）的集成测试用例。
//! 主要测试内容包括：
//! - 网络搜索工具的基本功能
//! - 搜索结果的处理和解析
//! - 错误处理和边界情况
//!
//! 通过 `use super::*` 导入父模块的工具定义和测试辅助函数，
//! 以便在本模块中编写具体的测试用例。

use super::*;

use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::sync::Arc;

fn new_tool() -> WebSearchTool {
    WebSearchTool::new(
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        }),
        "exa".to_string(),
        None,
        None,
        5,
        15,
        "test".to_string(),
    )
}

#[test]
fn parameters_schema_exposes_claude_compat_fields() {
    let tool = new_tool();
    let schema = tool.parameters_schema();

    assert!(schema["properties"]["query"].is_object());
    assert!(schema["properties"]["num"].is_object());
    assert!(schema["properties"]["numResults"].is_object());
    assert!(schema["properties"]["lr"].is_object());
}

#[test]
fn args_accept_claude_compat_fields() {
    let args: super::Args = serde_json::from_value(json!({
        "query": "rust",
        "num": 4,
        "lr": "lang_en"
    }))
    .unwrap();

    assert_eq!(args.query.as_deref(), Some("rust"));
    assert_eq!(args.num_results, Some(4));
    assert_eq!(args.lr.as_deref(), Some("lang_en"));
}

#[test]
fn parameters_schema_describes_the_complete_input_contract() {
    let schema = WebSearchTool::schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["query"]));
    assert_eq!(schema["properties"]["query"]["type"], "string");
    assert_eq!(schema["properties"]["num"]["type"], "number");
    assert_eq!(schema["properties"]["numResults"]["type"], "number");
    assert_eq!(schema["properties"]["lr"]["type"], "string");
    assert_eq!(schema["properties"]["livecrawl"]["enum"], json!(["fallback", "preferred"]));
    assert_eq!(schema["properties"]["type"]["enum"], json!(["auto", "fast", "deep"]));
    assert_eq!(schema["properties"]["contextMaxCharacters"]["type"], "number");
}

#[test]
fn tool_metadata_marks_websearch_as_strict_read_only_and_concurrency_safe() {
    let tool = new_tool();
    let spec = tool.spec();

    assert_eq!(tool.name(), "web_search_tool");
    assert!(!tool.description().trim().is_empty());
    assert_eq!(tool.parameters_schema(), WebSearchTool::schema());
    assert_eq!(spec.id, crate::app::agent::tools::WEB_SEARCH_TOOL_ID);
    assert_eq!(spec.display_name, crate::app::agent::tools::WEB_SEARCH_TOOL_ID);
    assert_eq!(
        spec.aliases,
        vec![
            crate::app::agent::tools::WEB_SEARCH_TOOL_ALIAS.to_string(),
            "websearch".to_string(),
            "web_search".to_string(),
        ]
    );
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[test]
fn args_accept_every_result_count_alias() {
    let cases = [
        json!({"query": "rust", "numResults": 1}),
        json!({"query": "rust", "num": 2}),
        json!({"query": "rust", "num_results": 3}),
        json!({"query": "rust", "max_results": 4}),
    ];

    for (value, expected) in cases.into_iter().zip(1..=4) {
        let args: super::Args = serde_json::from_value(value).unwrap();
        assert_eq!(args.num_results, Some(expected));
    }
}

#[test]
fn args_accept_every_language_alias_and_context_limit() {
    for (field, expected) in [("lr", "en"), ("language", "zh"), ("lang", "ja")] {
        let mut value = json!({
            "query": "rust",
            "type": "fast",
            "livecrawl": "preferred",
            "contextMaxCharacters": 2048,
        });
        value[field] = json!(expected);

        let args: super::Args = serde_json::from_value(value).unwrap();
        assert_eq!(args.lr.as_deref(), Some(expected));
        assert_eq!(args.search_type.as_deref(), Some("fast"));
        assert_eq!(args.livecrawl.as_deref(), Some("preferred"));
        assert_eq!(args.context_max_characters, Some(2048));
    }
}

#[test]
fn parse_sse_first_text_ignores_non_data_and_malformed_lines() {
    let response = concat!(
        "event: message\n",
        "data:{\"result\":{\"content\":[{\"text\":\"missing space\"}]}}\n",
        "data: {not-json}\n",
        "data: {\"result\":{\"content\":[{\"text\":\"found\"}]}}",
    );

    assert_eq!(WebSearchTool::parse_sse_first_text(response).as_deref(), Some("found"));
}

#[test]
fn parse_sse_first_text_skips_empty_text_before_later_result() {
    let response = concat!(
        "data: {\"result\":{\"content\":[{\"text\":\"\"}]}}\n",
        "data: {\"result\":{\"content\":[{\"text\":\"later\"}]}}",
    );

    assert_eq!(WebSearchTool::parse_sse_first_text(response).as_deref(), Some("later"));
}

#[test]
fn parse_sse_first_text_rejects_missing_or_non_string_first_content_text() {
    let responses = [
        "",
        "data: {}",
        "data: {\"result\":{}}",
        "data: {\"result\":{\"content\":[]}}",
        "data: {\"result\":{\"content\":[{}]}}",
        "data: {\"result\":{\"content\":[{\"text\":7}]}}",
        "data: {\"result\":{\"content\":[{\"type\":\"image\"},{\"text\":\"second\"}]}}",
    ];

    for response in responses {
        assert_eq!(WebSearchTool::parse_sse_first_text(response), None, "response: {response}");
    }
}

#[test]
fn parse_numbered_results_rejects_empty_or_unrecognized_headers() {
    assert!(WebSearchTool::parse_numbered_results("").is_empty());
    assert!(WebSearchTool::parse_numbered_results("Results for: rust\n1. Rust").is_empty());
}

#[test]
fn parse_numbered_results_extracts_multiple_results_and_multiline_snippets() {
    let results = WebSearchTool::parse_numbered_results(
        "Search results for: rust\n\
         ignored preamble\n\
         1. First result\n\
         https://first.test\n\
         first snippet line\n\
         second snippet line\n\
         2. Second result\n\
         second snippet",
    );

    assert_eq!(
        results,
        vec![
            json!({
                "title": "First result",
                "url": "https://first.test",
                "snippet": "first snippet line second snippet line",
            }),
            json!({
                "title": "Second result",
                "url": null,
                "snippet": "second snippet",
            }),
        ]
    );
}

#[test]
fn parse_numbered_results_keeps_only_the_first_url_as_url() {
    let results = WebSearchTool::parse_numbered_results(
        "Search results for: rust\n1. Result\nhttps://first.test\nhttps://second.test",
    );

    assert_eq!(results[0]["url"], "https://first.test");
    assert_eq!(results[0]["snippet"], "https://second.test");
}

#[test]
fn parse_numbered_results_ignores_empty_titles_and_preserves_non_numeric_lines() {
    let results = WebSearchTool::parse_numbered_results(
        "Search results for: rust\n1.   \n2. Valid title\nlabel. remains snippet",
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "Valid title");
    assert_eq!(results[0]["url"], json!(null));
    assert_eq!(results[0]["snippet"], "label. remains snippet");
}

#[tokio::test]
async fn execute_rejects_non_object_parameters_before_network() {
    let error = new_tool().execute(json!([])).await.err().expect("array args must fail");

    assert!(error.to_string().contains("Missing or invalid parameters"));
}

#[tokio::test]
async fn execute_rejects_missing_query_before_network() {
    let result = new_tool().execute(json!({})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("query cannot be empty"));
}

#[tokio::test]
async fn execute_blocks_exhausted_action_budget_before_network() {
    let tool = WebSearchTool::new(
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            max_actions_per_hour: 0,
            ..SecurityPolicy::default()
        }),
        "exa".to_string(),
        None,
        None,
        5,
        15,
        "test".to_string(),
    );

    let result = tool.execute(json!({"query": "rust"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(
        result.error.as_deref(),
        Some("Rate limit exceeded: too many actions in the last hour")
    );
}

#[tokio::test]
async fn call_enriches_failed_search_with_query_language_and_render_hint() {
    let result = new_tool()
        .call(json!({"query": "  rust  ", "type": "unsupported", "lr": "lang_en"}))
        .await
        .unwrap();
    let render_hint = result.render_hint.as_ref().expect("failed calls have a render hint");

    assert_eq!(result.data["success"], false);
    assert_eq!(result.data["error"], "type must be 'auto', 'fast', or 'deep'");
    assert_eq!(result.model_result, "type must be 'auto', 'fast', or 'deep'");
    assert_eq!(render_hint.title.as_deref(), Some(crate::app::agent::tools::WEB_SEARCH_TOOL_ID));
    assert_eq!(render_hint.kind.as_deref(), Some("web_search"));
    assert_eq!(render_hint.summary.as_deref(), Some("Web search failed for rust"));
    assert_eq!(render_hint.metadata["provider"], "Exa");
    assert_eq!(render_hint.metadata["query"], "rust");
    assert_eq!(render_hint.metadata["lr"], "lang_en");
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(false));
}

#[tokio::test]
async fn call_uses_generic_failure_summary_for_blank_query() {
    let result = new_tool().call(json!({"query": " \t "})).await.unwrap();
    let render_hint = result.render_hint.expect("failed calls have a render hint");

    assert_eq!(render_hint.summary.as_deref(), Some("Web search failed"));
    assert_eq!(render_hint.metadata["query"], "");
}
