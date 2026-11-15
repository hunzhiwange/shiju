//! 命令日志钩子测试模块
//!
//! 本模块包含 `CommandLoggerHook` 的单元测试，用于验证钩子是否正确记录工具调用信息。
//!
//! # 测试覆盖
//!
//! - 工具调用日志记录功能
//! - 日志条目格式验证（工具名称、执行时间、成功状态）

use super::*;

/// 命令日志钩子测试集
///
/// 包含验证命令日志钩子功能的测试用例。
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试工具调用日志记录功能
    ///
    /// # 测试目的
    ///
    /// 验证 `CommandLoggerHook` 是否正确记录工具调用的日志信息，包括：
    /// - 工具名称
    /// - 执行耗时
    /// - 执行结果状态
    ///
    /// # 测试步骤
    ///
    /// 1. 创建一个新的命令日志钩子实例
    /// 2. 构造一个成功的工具执行结果
    /// 3. 调用钩子的 `on_after_tool_call` 方法记录日志
    /// 4. 验证日志条目数量为 1
    /// 5. 验证日志条目包含工具名称 "shell"
    /// 6. 验证日志条目包含执行时间 "42ms"
    /// 7. 验证日志条目包含成功状态 "success=true"
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let hook = CommandLoggerHook::new();
    /// let result = ToolResult { success: true, output: "ok".into(), error: None };
    /// hook.on_after_tool_call("shell", &result, Duration::from_millis(42)).await;
    /// assert_eq!(hook.entries().len(), 1);
    /// ```
    #[tokio::test]
    async fn logs_tool_calls() {
        // 创建命令日志钩子实例
        let hook = CommandLoggerHook::new();

        // 构造一个成功的工具执行结果
        let result = ToolResult { success: true, output: "ok".into(), error: None };

        // 记录工具调用日志，工具名称为 "shell"，执行时间为 42 毫秒
        hook.on_after_tool_call("shell", &result, Duration::from_millis(42)).await;

        // 获取所有日志条目
        let entries = hook.entries();

        // 验证：应该只有一条日志记录
        assert_eq!(entries.len(), 1);

        assert_eq!(entries[0].tool_name, "shell");
        assert_eq!(entries[0].duration_ms, 42);
        assert!(entries[0].success);
    }

    #[tokio::test]
    async fn log_entry_uses_utc_timestamp_and_structured_fields() {
        let hook = CommandLoggerHook::new();
        let result = ToolResult { success: true, output: "ok".into(), error: None };

        hook.on_after_tool_call("shell", &result, Duration::from_millis(7)).await;

        let entries = hook.entries();
        assert_eq!(entries[0].tool_name, "shell");
        assert_eq!(entries[0].duration_ms, 7);
        assert!(entries[0].success);
        assert!(entries[0].timestamp <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn log_entry_omits_tool_result_output_and_error_details() {
        let hook = CommandLoggerHook::new();
        let result = ToolResult {
            success: false,
            output: "private-output-marker".into(),
            error: Some("private-error-marker".into()),
        };

        hook.on_after_tool_call("fetch", &result, Duration::from_millis(5)).await;

        let entries = hook.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tool_name, "fetch");
        assert_eq!(entries[0].duration_ms, 5);
        assert!(!entries[0].success);
        let serialized = serde_json::to_string(&entries[0]).expect("serialize audit entry");
        assert!(!serialized.contains("private-output-marker"));
        assert!(!serialized.contains("private-error-marker"));
    }

    #[tokio::test]
    async fn entries_returns_a_snapshot_independent_from_the_internal_log() {
        let hook = CommandLoggerHook::new();
        let result = ToolResult { success: true, output: String::new(), error: None };

        hook.on_after_tool_call("shell", &result, Duration::from_millis(1)).await;

        let mut snapshot = hook.entries();
        snapshot[0].tool_name.push_str("-changed");
        snapshot.clear();

        let persisted = hook.entries();
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].tool_name, "shell");
        assert_eq!(persisted[0].duration_ms, 1);
        assert!(persisted[0].success);
    }

    #[tokio::test]
    async fn concurrent_tool_calls_are_all_appended_to_the_log() {
        let hook = Arc::new(CommandLoggerHook::new());
        let first_hook = Arc::clone(&hook);
        let second_hook = Arc::clone(&hook);

        let first = tokio::spawn(async move {
            let result = ToolResult { success: true, output: String::new(), error: None };
            first_hook.on_after_tool_call("first", &result, Duration::from_millis(1)).await;
        });
        let second = tokio::spawn(async move {
            let result = ToolResult { success: false, output: String::new(), error: None };
            second_hook.on_after_tool_call("second", &result, Duration::from_millis(2)).await;
        });

        first.await.expect("first logging task should complete");
        second.await.expect("second logging task should complete");

        let entries = hook.entries();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| {
            entry.tool_name == "first" && entry.duration_ms == 1 && entry.success
        }));
        assert!(entries.iter().any(|entry| {
            entry.tool_name == "second" && entry.duration_ms == 2 && !entry.success
        }));
    }
}
