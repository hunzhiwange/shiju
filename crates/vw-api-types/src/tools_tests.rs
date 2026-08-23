//! 验证工具规格、调用和结果 DTO 的默认值及省略规则。

use crate::id::ToolId;
use crate::tools::{ToolResultContentDto, ToolSpecDto, ToolUseDto};
use serde_json::json;

#[test]
fn tool_specs_default_to_strict_and_skip_empty_fields() {
    let spec: ToolSpecDto = serde_json::from_value(json!({
        "id": "shell",
        "display_name": "Shell",
        "description": "Run command",
        "input_schema": { "type": "object" }
    }))
    .expect("valid spec");
    assert!(spec.strict);
    assert!(spec.aliases.is_empty());

    let tool_use = ToolUseDto {
        id: "call-1".to_string(),
        tool_id: ToolId::from("shell"),
        arguments: serde_json::Value::Null,
    };
    assert_eq!(
        serde_json::to_value(tool_use).expect("serialize"),
        json!({ "id": "call-1", "tool_id": "shell" })
    );
    assert_eq!(
        serde_json::to_value(ToolResultContentDto::Text { text: "ok".to_string() })
            .expect("serialize"),
        json!({ "type": "text", "text": "ok" })
    );
}
