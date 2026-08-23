//! 验证兼容网页搜索工具的端点、参数解析及无网络前置拒绝路径。
//! 最大结果数与超时配置经过边界化，非法调用不得到达提供方客户端。

use super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::sync::Arc;

fn tool(max_results: usize, timeout_secs: u64) -> WebSearchTool {
    WebSearchTool::new(
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        }),
        "exa".into(),
        None,
        None,
        max_results,
        timeout_secs,
        "ua".into(),
    )
}

#[test]
fn constructor_schema_endpoint_and_parsers_are_covered() {
    let low = tool(0, 0);
    assert_eq!(low.default_num_results, 1);
    assert_eq!(low.timeout_ms, DEFAULT_TIMEOUT_MS);
    assert_eq!(tool(99, 2).default_num_results, 10);
    assert_eq!(tool(5, 1).validate_exa_endpoint().unwrap(), EXA_MCP_URL);
    assert!(WebSearchTool::schema()["properties"]["livecrawl"].is_object());

    let sse = "data: {bad}\ndata: {\"result\":{\"content\":[{\"text\":\"hello\"}]}}";
    assert_eq!(WebSearchTool::parse_sse_first_text(sse).as_deref(), Some("hello"));

    let rows = WebSearchTool::parse_numbered_results(
        "Search results for: q\n1. One\n   https://one.test\n   A\n   B",
    );
    assert_eq!(rows[0]["snippet"], "A B");
}

#[test]
fn args_and_spec_cover_compat_fields() {
    let args: Args = serde_json::from_value(json!({
        "query": "rust",
        "max_results": 4,
        "livecrawl": "preferred",
        "type": "deep",
        "contextMaxCharacters": 500,
        "language": "en"
    }))
    .unwrap();
    assert_eq!(args.query.as_deref(), Some("rust"));
    assert_eq!(args.num_results, Some(4));
    assert_eq!(args.lr.as_deref(), Some("en"));
    assert!(tool(5, 1).spec().read_only);
}

#[tokio::test]
async fn execute_rejects_invalid_arguments_and_readonly_before_network() {
    assert_eq!(
        tool(5, 1).execute(json!({"query": " "})).await.unwrap().error.as_deref(),
        Some("query cannot be empty")
    );
    assert!(
        tool(5, 1)
            .execute(json!({"query": "rust", "livecrawl": "bad"}))
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("livecrawl")
    );
    assert!(
        tool(5, 1)
            .execute(json!({"query": "rust", "type": "bad"}))
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("type")
    );

    let readonly = WebSearchTool::new(
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() }),
        "exa".into(),
        None,
        None,
        5,
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
}

#[tokio::test]
async fn call_propagates_non_object_parameter_errors_before_network() {
    let error =
        tool(5, 1).call(json!([])).await.expect_err("array parameters must fail before dispatch");

    assert!(error.to_string().contains("Missing or invalid parameters"));
}
