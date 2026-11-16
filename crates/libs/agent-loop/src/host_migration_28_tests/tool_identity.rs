use crate::core::tool_execution::history_tool_call_id;

#[test]
fn history_tool_call_id_uses_native_fallback_only_when_needed() {
    assert_eq!(
        history_tool_call_id(Some("call-testcov-0090"), true, 2, 3).as_deref(),
        Some("call-testcov-0090")
    );
    assert_eq!(history_tool_call_id(None, true, 2, 3).as_deref(), Some("fallback_2_3"));
    assert!(history_tool_call_id(None, false, 2, 3).is_none());
}
