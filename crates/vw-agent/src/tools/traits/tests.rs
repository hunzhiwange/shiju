//! 工具 trait 测试模块
//!
//! 本模块提供了针对 `Tool` trait 的单元测试，验证工具的行为契约：
//! - 工具规格（名称、描述、参数 schema）的正确性
//! - 工具执行结果的预期输出
//! - `ToolResult` 结构体的序列化/反序列化一致性
//!
//! 使用 `DummyTool` 作为测试替身，提供确定性行为以简化测试验证。

use super::super::*;
use async_trait::async_trait;
use serde_json::json;
use vw_api_tools::PermissionRequestDto;

/// 测试用虚拟工具
///
/// 一个确定性的测试工具实现，用于验证 `Tool` trait 的行为契约。
/// 该工具接收一个 `value` 参数并将其原样返回，便于测试结果验证。
struct DummyTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for DummyTool {
    /// 返回工具名称
    ///
    /// # 返回值
    /// 固定返回 `"dummy_tool"` 作为工具标识符
    fn name(&self) -> &str {
        "dummy_tool"
    }

    /// 返回工具描述
    ///
    /// # 返回值
    /// 固定返回 `"A deterministic test tool"` 作为工具描述文本
    fn description(&self) -> &str {
        "A deterministic test tool"
    }

    /// 返回工具参数 schema
    ///
    /// # 返回值
    /// JSON Schema 定义，包含一个 `value` 字符串类型参数
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        })
    }

    /// 执行工具逻辑
    ///
    /// 从参数中提取 `value` 字段并返回，若参数缺失则返回空字符串。
    ///
    /// # 参数
    /// - `args`: 工具执行参数，预期包含 `value` 字符串字段
    ///
    /// # 返回值
    /// - `Ok(ToolResult)`: 成功结果，`output` 为参数中的 `value` 值或空字符串
    ///
    /// # 示例
    /// ```ignore
    /// let result = tool.execute(json!({ "value": "test" })).await?;
    /// assert_eq!(result.output, "test");
    /// ```
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult {
            success: true,
            // 从 args 中获取 "value" 字段，若不存在或非字符串则返回默认值
            output: args
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            error: None,
        })
    }
}

/// 测试工具规格的元数据和 schema 正确性
///
/// 验证 `Tool::spec()` 方法正确聚合工具的名称、描述和参数 schema
#[test]
fn spec_uses_tool_metadata_and_schema() {
    let tool = DummyTool;
    // 通过 spec() 获取完整的工具规格
    let spec = tool.spec();

    // 验证工具元数据正确映射到 ToolSpec 结构
    assert_eq!(spec.name, "dummy_tool");
    assert_eq!(spec.description, "A deterministic test tool");
    // 验证参数 schema 的结构完整性
    assert_eq!(spec.parameters["type"], "object");
    assert_eq!(spec.parameters["properties"]["value"]["type"], "string");
}

/// 测试工具执行返回预期输出
///
/// 验证 `Tool::execute()` 方法正确处理输入参数并返回预期的执行结果
#[tokio::test]
async fn execute_returns_expected_output() {
    let tool = DummyTool;
    // 执行工具，传入测试参数
    let result = tool.execute(serde_json::json!({ "value": "hello-tool" })).await.unwrap();

    // 验证执行成功且输出与输入参数一致
    assert!(result.success);
    assert_eq!(result.output, "hello-tool");
    // 验证无错误信息
    assert!(result.error.is_none());
}

/// 测试 ToolResult 序列化/反序列化往返一致性
///
/// 验证 `ToolResult` 结构体能够正确序列化为 JSON 并从 JSON 反序列化还原，
/// 确保错误状态的字段在序列化过程中不会丢失
#[test]
fn tool_result_serialization_roundtrip() {
    // 构造包含错误状态的 ToolResult
    let result = ToolResult { success: false, output: String::new(), error: Some("boom".into()) };

    // 序列化为 JSON 字符串
    let json = serde_json::to_string(&result).unwrap();
    // 从 JSON 反序列化还原
    let parsed: ToolResult = serde_json::from_str(&json).unwrap();

    // 验证反序列化后的数据与原始数据一致
    assert!(!parsed.success);
    assert_eq!(parsed.error.as_deref(), Some("boom"));
}

/// 测试 ToolSpec / ToolCallResult 到共享 DTO 的映射。
#[test]
fn tool_v2_dto_conversion_uses_structured_fields() {
    let tool = DummyTool;
    let spec = tool.spec();
    let dto = spec.to_dto();

    assert_eq!(dto.id.0, "dummy_tool");
    assert_eq!(dto.display_name, "dummy_tool");
    assert_eq!(dto.input_schema["properties"]["value"]["type"], "string");

    let result = ToolCallResult::from_legacy_result(ToolResult {
        success: true,
        output: "hello-dto".to_string(),
        error: None,
    });
    let dto = result.to_dto_with_meta(Some("dummy_tool"), Some("call-1"));

    assert_eq!(dto.tool_id.as_ref().map(|id| id.0.as_str()), Some("dummy_tool"));
    assert_eq!(dto.tool_use_id.as_deref(), Some("call-1"));
    assert_eq!(dto.success, Some(true));
    assert_eq!(dto.model_result, json!("hello-dto"));
}

#[test]
fn tool_result_dto_preserves_permission_request() {
    let mut result = ToolCallResult::from_legacy_result(ToolResult {
        success: false,
        output: String::new(),
        error: Some("Approval required".to_string()),
    });
    result.permission_request = Some(PermissionRequestDto {
        reason: "Approval required for file write".to_string(),
        warning: Some("High-risk path".to_string()),
        updated_input: Some(json!({"path": "src/main.rs"})),
    });

    let dto = result.to_dto_with_meta(Some("file_write"), Some("call-2"));

    let permission_request = dto.permission_request.expect("permission request should exist");
    assert_eq!(permission_request.reason, "Approval required for file write");
    assert_eq!(permission_request.warning.as_deref(), Some("High-risk path"));
    assert_eq!(permission_request.updated_input, Some(json!({"path": "src/main.rs"})));
}
