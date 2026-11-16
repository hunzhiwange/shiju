use crate::helpers::{self, HelperHost};
struct PreviewHost;
impl HelperHost for PreviewHost {
    fn specs(_: Option<&str>) -> Vec<tool_types::ToolSpec> { Vec::new() }
    fn sanitize_tool_input(name: &str, arguments: &str) -> String {
        crate::tool_output::sanitize_tool_input(name, arguments)
    }
}
fn tool_call_preview(name: &str, arguments: &str) -> String {
    helpers::tool_call_preview::<PreviewHost>(name, arguments)
}
#[test]
fn tool_call_preview_sanitizes_large_or_sensitive_arguments() {
    let preview = tool_call_preview(
        "file_write",
        r#"{"path":"/tmp/example","content":"full file content should be omitted"}"#,
    );

    assert!(preview.starts_with("file_write("));
    assert!(preview.contains("<omitted"));
    assert!(!preview.contains("full file content should be omitted"));
}

#[test]
fn tool_call_preview_handles_empty_and_short_plain_arguments() {
    assert_eq!(tool_call_preview("file_read", "  "), "file_read()");
    assert_eq!(tool_call_preview("search", "  plain search query  "), "search(plain search query)");
}

#[test]
fn tool_call_preview_truncates_sanitized_arguments_at_character_boundary() {
    let preview = tool_call_preview("search", &"界".repeat(121));

    assert_eq!(preview, format!("search({}...)", "界".repeat(120)));
}
