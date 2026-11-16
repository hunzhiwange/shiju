//! sanitize 模块测试
//!
//! 本模块包含对 `sanitize_gateway_response` 函数的单元测试，验证网关响应的
//! 清理和净化功能。主要测试场景包括：
//!
//! - 移除工具调用标签（tool call tags）
//! - 移除孤立的工具 JSON 片段
//! - 确保响应文本的清洁性和安全性
//!
//! 这些测试确保代理系统在处理外部响应时能够正确过滤和清理潜在的
//! 工具注入或恶意标签。

use async_trait::async_trait;
use tool_types::Tool;

/// 模拟调度工具
///
/// 用于测试目的的 Mock 工具实现，模拟一个调度（schedule）工具的行为。
/// 该工具提供一个简单的 `create` 操作接口，用于验证工具调用的清理逻辑。
struct MockScheduleTool;

/// 实现 Tool trait
///
/// 为 `MockScheduleTool` 实现 `Tool` trait，提供测试所需的最小功能集。
/// 该实现完全异步兼容，支持 WASM 和原生平台。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for MockScheduleTool {
    /// 返回工具名称
    ///
    /// # 返回值
    ///
    /// 返回固定的工具名称 `"schedule"`
    fn name(&self) -> &str {
        "schedule"
    }

    /// 返回工具描述
    ///
    /// # 返回值
    ///
    /// 返回工具的简短描述文本
    fn description(&self) -> &str {
        "Mock schedule tool"
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义工具接受的参数结构，包含一个必需的 `action` 字符串字段。
    ///
    /// # 返回值
    ///
    /// 返回 JSON Schema 对象，描述参数的结构和约束
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string" }
            },
            "required": ["action"]
        })
    }

    /// 执行工具操作
    ///
    /// 模拟工具执行，总是返回成功结果。这是一个测试替身（test double），
    /// 不执行实际的调度逻辑。
    ///
    /// # 参数
    ///
    /// * `_args` - 工具参数（在此 Mock 实现中被忽略）
    ///
    /// # 返回值
    ///
    /// 返回一个成功的 `ToolResult`，包含输出 `"ok"`
    async fn execute(
        &self,
        _args: serde_json::Value,
    ) -> anyhow::Result<tool_types::ToolResult> {
        Ok(tool_types::ToolResult {
            success: true,
            output: "ok".to_string(),
            error: None,
        })
    }
}

/// 测试移除工具调用标签
///
/// 验证 `sanitize_gateway_response` 函数能够正确移除被 `ฦ` 标签包裹的
/// 工具调用 JSON 片段。这是防止工具注入攻击的关键测试。
///
/// # 测试场景
///
/// 输入文本包含被 `ฦ` 标签包裹的 JSON 工具调用：
/// ```text
/// Before
/// ฦ
/// {"name":"schedule","arguments":{"action":"create"}}
/// ฦ
/// After
/// ```
///
/// # 期望结果
///
/// - 输出应为 `"Before\nAfter"`（移除了工具调用部分）
/// - 输出不应包含 `ฦ` 标签
/// - 输出不应包含 `"name":"schedule"` 等工具调用内容
#[test]
fn sanitize_gateway_response_removes_tool_call_tags() {
    // 构造包含工具调用标签的测试输入
    let input = r#"Before
     ฦ
    {"name":"schedule","arguments":{"action":"create"}}
    ฦ
    After"#;

    // 调用清理函数（不提供工具列表，仅测试标签移除）
    let result = super::sanitize_gateway_response(input, &[]);

    // 标准化输出：移除空行，保留有内容的行
    let normalized =
        result.lines().filter(|line| !line.trim().is_empty()).collect::<Vec<_>>().join("\n");

    // 断言：清理后的文本应仅包含 Before 和 After
    assert_eq!(normalized, "Before\nAfter");

    // 断言：不应包含工具调用标签
    assert!(!result.contains("ฦ"));

    // 断言：不应包含工具调用的 JSON 内容
    assert!(!result.contains("\"name\":\"schedule\""));
}

/// 测试移除孤立的工具 JSON 片段
///
/// 验证 `sanitize_gateway_response` 函数能够识别并移除未被标签包裹的
/// 工具调用 JSON 片段。当工具列表中存在对应的工具定义时，函数应能
/// 匹配并清理这些片段。
///
/// # 测试场景
///
/// 输入文本包含多个 JSON 片段和普通文本：
/// ```text
/// {"name":"schedule","parameters":{"action":"create"}}
/// {"result":{"status":"scheduled"}}
/// Reminder set successfully.
/// ```
///
/// # 期望结果
///
/// - 输出应为 `"Reminder set successfully."`（移除了所有 JSON 片段）
/// - 输出不应包含 `"name":"schedule"`
/// - 输出不应包含 `"result"` 字段
#[test]
fn sanitize_gateway_response_removes_isolated_tool_json_artifacts() {
    // 创建工具列表，包含 MockScheduleTool 用于匹配和识别工具调用
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockScheduleTool)];

    // 构造包含孤立 JSON 片段的测试输入
    let input = r#"{"name":"schedule","parameters":{"action":"create"}}
    {"result":{"status":"scheduled"}}
    Reminder set successfully."#;

    // 调用清理函数，提供工具列表以匹配已知的工具调用格式
    let result = super::sanitize_gateway_response(input, &tools);

    // 断言：清理后应仅保留普通文本部分
    assert_eq!(result, "Reminder set successfully.");

    // 断言：不应包含工具调用的 JSON 内容
    assert!(!result.contains("\"name\":\"schedule\""));

    // 断言：不应包含结果 JSON 片段
    assert!(!result.contains("\"result\""));
}
