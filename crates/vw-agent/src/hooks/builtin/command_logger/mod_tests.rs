// 验证命令日志钩子的事件筛选、字段整理与敏感信息保护。

// 审计存储容量有限，达到上限时必须淘汰最旧记录而非最新结果。
use super::*;

#[test]
fn new_hook_has_expected_identity_priority_and_empty_log() {
    let hook = CommandLoggerHook::new();

    assert_eq!(hook.name(), "command-logger");
    assert_eq!(hook.priority(), -50);
    assert!(hook.entries().is_empty());
}

#[tokio::test]
async fn on_after_tool_call_records_success_and_failure_entries_in_order() {
    let hook = CommandLoggerHook::new();
    let success = ToolResult { success: true, output: "ok".into(), error: None };
    let failure =
        ToolResult { success: false, output: String::new(), error: Some("failed".into()) };

    hook.on_after_tool_call("shell", &success, Duration::from_millis(42)).await;
    hook.on_after_tool_call("git", &failure, Duration::from_micros(1_500)).await;

    let entries = hook.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].tool_name, "shell");
    assert_eq!(entries[0].duration_ms, 42);
    assert!(entries[0].success);
    assert_eq!(entries[1].tool_name, "git");
    assert_eq!(entries[1].duration_ms, 1);
    assert!(!entries[1].success);
}

#[tokio::test]
async fn on_after_tool_call_records_zero_duration_without_dropping_tool_name() {
    let hook = CommandLoggerHook::new();
    let result = ToolResult::default();

    hook.on_after_tool_call("noop", &result, Duration::ZERO).await;

    let entries = hook.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].tool_name, "noop");
    assert_eq!(entries[0].duration_ms, 0);
}

#[tokio::test]
async fn command_audit_store_evicts_oldest_entries() {
    let store = CommandAuditStore::new(2);
    let hook = CommandLoggerHook::with_store(store.clone());

    for tool in ["first", "second", "third"] {
        hook.on_after_tool_call(tool, &ToolResult::default(), Duration::from_millis(1)).await;
    }

    let entries = store.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].tool_name, "second");
    assert_eq!(entries[1].tool_name, "third");
}
