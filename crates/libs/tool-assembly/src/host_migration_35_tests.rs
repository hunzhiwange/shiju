use crate::{ToolResult, ToolSpec};
#[test]
fn tool_result_serde() {
    let result = ToolResult { success: true, output: "hello".into(), error: None };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: ToolResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.success);
    assert_eq!(parsed.output, "hello");
    assert!(parsed.error.is_none());
}
#[test]
fn tool_result_with_error_serde() {
    let result = ToolResult { success: false, output: String::new(), error: Some("boom".into()) };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: ToolResult = serde_json::from_str(&json).unwrap();
    assert!(!parsed.success);
    assert_eq!(parsed.error.as_deref(), Some("boom"));
}
#[test]
fn tool_spec_serde() {
    let spec = ToolSpec::new("test", "A test tool", serde_json::json!({"type": "object"}));
    let json = serde_json::to_string(&spec).unwrap();
    let parsed: ToolSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "test");
    assert_eq!(parsed.description, "A test tool");
}

#[path = "host_migration_35_registry.rs"]
mod registry;
