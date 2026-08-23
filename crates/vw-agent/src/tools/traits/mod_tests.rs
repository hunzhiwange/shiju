//! 验证工具 trait 的默认调用桥接、结果辅助方法与安全渲染回退。
//! 失败 fixture 确认旧式错误会转换为结构化失败，而不会丢失模型可见信息。

use super::*;
use async_trait::async_trait;
use serde_json::{Value, json};

struct FailingTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for FailingTool {
    fn name(&self) -> &str {
        "file_write"
    }
    fn description(&self) -> &str {
        "write"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }
    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: false, output: "out".into(), error: Some("err".into()) })
    }
}

#[test]
fn builders_dto_and_result_helpers_cover_structured_paths() {
    let spec = ToolSpec::new("id", "desc", json!({"type": "object"}))
        .with_display_name("Display")
        .with_aliases(vec!["a"])
        .with_read_only(true)
        .with_destructive(true)
        .with_concurrency_safe(true)
        .with_requires_user_interaction(true)
        .with_strict(false);
    let dto = spec.to_dto();
    assert_eq!(dto.id.0, "id");
    assert_eq!(dto.display_name, "Display");
    assert_eq!(dto.aliases, vec!["a"]);
    assert!(
        dto.read_only && dto.destructive && dto.concurrency_safe && dto.requires_user_interaction
    );
    assert!(!dto.strict);

    let hint = ToolRenderHint::titled("Title");
    assert_eq!(hint.to_dto().title.as_deref(), Some("Title"));

    let ok = ToolCallResult::from_legacy_result(ToolResult {
        success: true,
        output: "ok".into(),
        error: None,
    });
    assert!(ok.is_success());
    assert_eq!(ok.model_text(), "ok");
    assert!(ok.error_text().is_none());

    let bad = ToolCallResult::from_legacy_result(ToolResult {
        success: false,
        output: "out".into(),
        error: Some("bad".into()),
    });
    assert!(!bad.is_success());
    assert_eq!(bad.default_model_result(), json!("bad"));
    assert_eq!(bad.error_text().as_deref(), Some("bad"));
}

#[tokio::test]
async fn default_trait_call_maps_legacy_failure_and_default_flags() {
    let tool = FailingTool;
    let result = tool.call(json!({})).await.unwrap();
    assert!(!result.is_success());
    assert_eq!(result.model_result, json!("err"));
    assert!(tool.spec().destructive);

    assert_eq!(default_tool_aliases("web_fetch"), vec!["webfetch".to_string()]);
    assert!(default_tool_read_only("web_search_tool"));
    assert!(default_tool_destructive("todowrite"));
    assert!(default_tool_concurrency_safe("file_read"));
    assert!(default_tool_requires_user_interaction("question"));
}

#[test]
fn dto_content_falls_back_to_model_result_then_structured_data() {
    let model_result = ToolCallResult { model_result: json!({"answer": 42}), ..Default::default() };
    let dto = model_result.to_dto();
    assert_eq!(dto.content.len(), 1);
    assert!(matches!(
        &dto.content[0],
        ToolResultContentDto::Json { value } if value == &json!({"answer": 42})
    ));

    let null_output =
        ToolCallResult { data: json!({"output": null, "details": "kept"}), ..Default::default() };
    let dto = null_output.to_dto();
    assert_eq!(dto.content.len(), 1);
    assert!(matches!(
        &dto.content[0],
        ToolResultContentDto::Json { value }
            if value == &json!({"output": null, "details": "kept"})
    ));
}

#[test]
fn result_helpers_cover_data_fallbacks_and_error_shapes() {
    let output = ToolCallResult { data: json!({"output": "from-data"}), ..Default::default() };
    assert_eq!(output.default_model_result(), json!("from-data"));

    let error = ToolCallResult { data: json!({"error": {"code": 7}}), ..Default::default() };
    assert!(!error.is_success());
    assert_eq!(error.default_model_result(), json!({"code": 7}));
    assert_eq!(error.error_text(), Some("{\n  \"code\": 7\n}".to_string()));

    let explicit_success = ToolCallResult { data: json!({"success": true}), ..Default::default() };
    assert!(explicit_success.is_success());

    let empty_error = ToolCallResult { data: json!({"error": "  "}), ..Default::default() };
    assert!(empty_error.is_success());
    assert_eq!(empty_error.error_text().as_deref(), Some("  "));

    let no_data = ToolCallResult::default();
    assert_eq!(no_data.default_model_result(), json!(""));
    assert_eq!(no_data.model_text(), "");
}

#[test]
fn render_helpers_use_hint_values_and_safe_fallbacks() {
    let hinted = ToolCallResult {
        render_hint: Some(ToolRenderHint {
            title: Some("Result".to_string()),
            metadata: json!({"kind": "preview"}),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(hinted.render_title("Fallback"), "Result");
    assert_eq!(hinted.render_metadata(), json!({"kind": "preview"}));

    let null_metadata =
        ToolCallResult { render_hint: Some(ToolRenderHint::default()), ..Default::default() };
    assert_eq!(null_metadata.render_title("Fallback"), "Fallback");
    assert_eq!(null_metadata.render_metadata(), json!({}));
    assert_eq!(ToolCallResult::default().render_metadata(), json!({}));
}
