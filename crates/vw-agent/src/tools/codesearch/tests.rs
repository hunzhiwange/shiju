//! 代码搜索工具的单元测试模块
//!
//! 本模块包含了对 `CodeSearchTool` 及其相关功能的测试用例。
//! 主要测试内容包括：
//! - 工具的 JSON Schema 定义是否正确暴露查询参数
//! - SSE（Server-Sent Events）格式文本的解析功能
//!
//! 这些测试确保代码搜索工具能够正确地与 MCP（Model Context Protocol）服务器交互，
//! 并准确解析返回的响应数据。

use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::sync::Arc;

fn tool_with_security(security: SecurityPolicy) -> CodeSearchTool {
    CodeSearchTool::new(Arc::new(security))
}

fn supervised_tool() -> CodeSearchTool {
    tool_with_security(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        ..SecurityPolicy::default()
    })
}

/// 解析 SSE 格式文本中的第一个文本内容
///
/// 这是一个测试辅助函数，直接调用 `codesearch` 模块中的 `parse_sse_first_text` 函数。
/// 该函数用于从 SSE 格式的响应数据中提取第一个文本类型的内容。
///
/// # 参数
///
/// * `s` - SSE 格式的字符串，通常包含 `data:` 前缀和 JSON 数据
///
/// # 返回值
///
/// * `Some(String)` - 如果成功提取到文本内容，返回包含该内容的字符串
/// * `None` - 如果解析失败或没有找到文本内容
fn parse_sse_first_text(s: &str) -> Option<String> {
    super::super::codesearch::parse_sse_first_text(s)
}

/// 测试 CodeSearchTool 的 JSON Schema 是否正确暴露 query 字段
///
/// 此测试验证以下内容：
/// 1. Schema 的类型为 "object"
/// 2. Schema 的 properties 中包含 "query" 字段
/// 3. "query" 字段本身是一个对象（包含类型、描述等信息）
///
/// 这确保了工具的接口定义符合规范，调用方能够正确识别和使用查询参数。
#[test]
fn schema_exposes_query() {
    // 获取 CodeSearchTool 的 JSON Schema 定义
    let schema = CodeSearchTool::schema();

    // 验证 schema 的根类型是 object
    assert_eq!(schema["type"], "object");

    // 验证 schema 包含 query 属性，且该属性是一个对象
    assert!(schema["properties"]["query"].is_object());
}

#[test]
fn schema_exposes_compatibility_query_and_token_fields() {
    let schema = CodeSearchTool::schema();

    assert_eq!(schema["properties"]["query"]["type"], "string");
    assert_eq!(schema["properties"]["information_request"]["type"], "string");
    assert_eq!(schema["properties"]["tokensNum"]["type"], "number");
    assert_eq!(schema["required"], json!([]));
}

#[test]
fn tool_metadata_identifies_codesearch_and_has_a_description() {
    let tool = supervised_tool();

    assert_eq!(tool.name(), "codesearch");
    assert!(!tool.description().trim().is_empty());
    assert_eq!(tool.parameters_schema(), CodeSearchTool::schema());
}

#[test]
fn args_deserialize_camel_case_token_count() {
    let args: super::Args = serde_json::from_value(json!({
        "query": "rust",
        "tokensNum": 1_234
    }))
    .unwrap();

    assert_eq!(args.query.as_deref(), Some("rust"));
    assert_eq!(args.tokens_num, Some(1_234));
}

#[test]
fn resolve_query_prefers_query_and_trims_it() {
    let args = super::Args {
        query: Some("  primary query  ".into()),
        information_request: Some("fallback query".into()),
        tokens_num: None,
    };

    assert_eq!(CodeSearchTool::resolve_query(&args), "primary query");
}

#[test]
fn resolve_query_falls_back_to_trimmed_information_request() {
    let args = super::Args {
        query: None,
        information_request: Some("  compatibility query  ".into()),
        tokens_num: None,
    };

    assert_eq!(CodeSearchTool::resolve_query(&args), "compatibility query");
}

#[test]
fn resolve_query_returns_empty_for_missing_or_blank_values() {
    let missing = super::Args { query: None, information_request: None, tokens_num: None };
    let blank = super::Args {
        query: Some(" \t\n ".into()),
        information_request: Some("fallback is not used when query is present".into()),
        tokens_num: None,
    };

    assert_eq!(CodeSearchTool::resolve_query(&missing), "");
    assert_eq!(CodeSearchTool::resolve_query(&blank), "");
}

#[test]
fn validate_exa_endpoint_accepts_the_fixed_https_allowlisted_url() {
    assert_eq!(supervised_tool().validate_exa_endpoint().unwrap(), super::EXA_MCP_URL);
}

/// 测试 parse_sse_first_text 函数能否正确提取 SSE 响应中的文本内容
///
/// 此测试验证函数能够：
/// 1. 正确识别以 "data:" 开头的 SSE 行
/// 2. 解析其中的 JSON 数据结构
/// 3. 从嵌套的 content 数组中提取第一个文本类型的内容
///
/// 测试数据模拟了一个标准的 MCP 服务器 SSE 响应格式：
/// - 外层是 JSON-RPC 2.0 格式
/// - result.content 是一个数组，包含多个内容项
/// - 每个内容项有 type 和 text 字段
#[test]
fn parse_sse_first_text_extracts_content_text() {
    // 构造一个标准的 SSE 格式响应，包含 JSON-RPC 2.0 结构
    let s = r#"data: {"jsonrpc":"2.0","result":{"content":[{"type":"text","text":"hello"}]}}"#;

    // 验证能够正确提取出 "hello" 文本
    assert_eq!(parse_sse_first_text(s).as_deref(), Some("hello"));
}

/// 测试 parse_sse_first_text 函数能够正确忽略非 data 行
///
/// 此测试验证函数在遇到不符合 "data:" 前缀格式的行时，
/// 能够正确返回 None，而不是尝试解析或产生错误。
///
/// SSE 协议中可能包含各种类型的行（如 event:、id:、空行等），
/// 此测试确保解析器只处理包含 "data:" 前缀的行。
#[test]
fn parse_sse_first_text_ignores_non_data_lines() {
    // 构造一个不包含 "data:" 前缀的 SSE 行
    let s = "event: message\n\n";

    // 验证函数返回 None，表示没有可解析的文本内容
    assert_eq!(parse_sse_first_text(s), None);
}

#[test]
fn parse_sse_first_text_skips_empty_text_before_later_data() {
    let s = concat!(
        "data: {\"result\":{\"content\":[{\"text\":\"\"}]}}\n",
        "data: {\"result\":{\"content\":[{\"text\":\"later\"}]}}"
    );

    assert_eq!(parse_sse_first_text(s).as_deref(), Some("later"));
}

#[test]
fn parse_sse_first_text_returns_none_for_malformed_json() {
    assert_eq!(parse_sse_first_text("data: {not-json}"), None);
}

#[test]
fn parse_sse_first_text_returns_none_when_nested_text_is_missing_or_invalid() {
    let responses = [
        r#"data: {}"#,
        r#"data: {"result":{}}"#,
        r#"data: {"result":{"content":[]}}"#,
        r#"data: {"result":{"content":[{}]}}"#,
        r#"data: {"result":{"content":[{"text":7}]}}"#,
    ];

    for response in responses {
        assert_eq!(parse_sse_first_text(response), None, "response: {response}");
    }
}

#[test]
fn parse_sse_first_text_only_uses_the_first_content_item() {
    let s = r#"data: {"result":{"content":[{"type":"image"},{"text":"second"}]}}"#;

    assert_eq!(parse_sse_first_text(s), None);
}

#[test]
fn parse_sse_first_text_requires_space_after_data_prefix() {
    let s = r#"data:{"result":{"content":[{"text":"ignored"}]}}"#;

    assert_eq!(parse_sse_first_text(s), None);
}

#[tokio::test]
async fn execute_rejects_invalid_parameters_before_network() {
    let error = supervised_tool().execute(json!([])).await.err().expect("array args must fail");

    assert!(error.to_string().contains("Missing or invalid parameters"));
}

#[tokio::test]
async fn execute_rejects_negative_token_count_before_network() {
    let error = supervised_tool()
        .execute(json!({"query": "rust", "tokensNum": -1}))
        .await
        .err()
        .expect("negative token count must not deserialize as u32");

    assert!(error.to_string().contains("Missing or invalid parameters"));
    assert!(error.to_string().contains("-1"));
}

#[tokio::test]
async fn execute_returns_missing_query_for_absent_or_blank_queries() {
    for args in [json!({}), json!({"query": "  "}), json!({"information_request": "\t"})] {
        let result = supervised_tool().execute(args).await.unwrap();

        assert!(!result.success);
        assert!(result.output.is_empty());
        assert_eq!(result.error.as_deref(), Some("Missing query"));
    }
}

#[tokio::test]
async fn execute_blocks_read_only_policy_before_network() {
    let tool = tool_with_security(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        ..SecurityPolicy::default()
    });

    let result = tool.execute(json!({"query": "rust"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Action blocked: autonomy is read-only"));
}

#[tokio::test]
async fn execute_blocks_exhausted_action_budget_before_network() {
    let tool = tool_with_security(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        max_actions_per_hour: 0,
        ..SecurityPolicy::default()
    });

    let result = tool.execute(json!({"query": "rust"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(
        result.error.as_deref(),
        Some("Rate limit exceeded: too many actions in the last hour")
    );
}
