//! 运行时工具执行提示词测试。
//!
//! 本模块聚焦通道层构造工具可见性提示词时的策略边界：被运行时策略
//! 排除的工具必须从可调用清单中移除，同时保留面向模型的权威说明。
//! 这些测试避免提示词与真实工具执行能力发生漂移。

use super::*;

#[test]
fn build_runtime_tool_visibility_prompt_respects_excluded_snapshot() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockPriceTool), Box::new(MockEchoTool)];
    let excluded = vec!["mock_price".to_string()];

    // 非原生工具调用路径需要明确的文本协议，因此同时验证排除列表
    // 与工具使用协议都出现在最终提示词中。
    let non_native = build_runtime_tool_visibility_prompt(&tools, &excluded, false);
    assert!(non_native.contains("Runtime Tool Availability (Authoritative)"));
    assert!(non_native.contains("Excluded by runtime policy: mock_price"));
    assert!(non_native.contains("`mock_echo`"));
    assert!(!non_native.contains("**mock_price**:"));
    assert!(non_native.contains("## Tool Use Protocol"));

    // 原生 function-calling 由 provider 承载调用协议，提示词只需说明可见性，
    // 避免重复注入文本协议导致模型混用两套工具调用方式。
    let native = build_runtime_tool_visibility_prompt(&tools, &excluded, true);
    assert!(native.contains("Runtime Tool Availability (Authoritative)"));
    assert!(native.contains("native provider function-calling"));
    assert!(!native.contains("## Tool Use Protocol"));
}

/// 用于测试排除策略的价格工具替身。
///
/// 该工具故意提供完整的名称、描述、参数 schema 与执行结果，使提示词构造
/// 能覆盖真实工具对象的读取路径，而不依赖外部行情服务。
struct MockPriceTool;

#[async_trait::async_trait]
impl Tool for MockPriceTool {
    fn name(&self) -> &str {
        "mock_price"
    }

    fn description(&self) -> &str {
        "Return a mocked BTC price"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string" }
            },
            "required": ["symbol"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let symbol = args.get("symbol").and_then(serde_json::Value::as_str);
        if symbol != Some("BTC") {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("unexpected symbol".to_string()),
            });
        }

        Ok(ToolResult {
            success: true,
            output: r#"{"symbol":"BTC","price_usd":65000}"#.to_string(),
            error: None,
        })
    }
}

/// 用于测试保留可见工具的回显工具替身。
///
/// 与 `MockPriceTool` 成对出现，便于断言运行时策略只排除指定工具，
/// 不会误删同一批次中的其他可调用工具。
struct MockEchoTool;

#[async_trait::async_trait]
impl Tool for MockEchoTool {
    fn name(&self) -> &str {
        "mock_echo"
    }

    fn description(&self) -> &str {
        "Echo back the input text"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult {
            success: true,
            output: args
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            error: None,
        })
    }
}
