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

        // 验证：日志条目应包含工具名称
        assert!(entries[0].contains("shell"));

        // 验证：日志条目应包含执行时间
        assert!(entries[0].contains("42ms"));

        // 验证：日志条目应包含成功状态
        assert!(entries[0].contains("success=true"));
    }

    #[tokio::test]
    async fn log_entry_uses_hh_mm_ss_timestamp_and_exact_payload_format() {
        let hook = CommandLoggerHook::new();
        let result = ToolResult { success: true, output: "ok".into(), error: None };

        hook.on_after_tool_call("shell", &result, Duration::from_millis(7)).await;

        let entries = hook.entries();
        let (timestamp, payload) = entries[0]
            .strip_prefix('[')
            .and_then(|entry| entry.split_once("] "))
            .expect("log entry should contain a bracketed timestamp");
        assert!(chrono::NaiveTime::parse_from_str(timestamp, "%H:%M:%S").is_ok());
        assert_eq!(payload, "shell (7ms) success=true");
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
        assert!(entries[0].ends_with("fetch (5ms) success=false"));
        assert!(!entries[0].contains("private-output-marker"));
        assert!(!entries[0].contains("private-error-marker"));
    }

    #[tokio::test]
    async fn entries_returns_a_snapshot_independent_from_the_internal_log() {
        let hook = CommandLoggerHook::new();
        let result = ToolResult { success: true, output: String::new(), error: None };

        hook.on_after_tool_call("shell", &result, Duration::from_millis(1)).await;

        let mut snapshot = hook.entries();
        snapshot[0].push_str("-changed");
        snapshot.clear();

        let persisted = hook.entries();
        assert_eq!(persisted.len(), 1);
        assert!(persisted[0].ends_with("shell (1ms) success=true"));
        assert!(!persisted[0].contains("-changed"));
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
        assert!(entries.iter().any(|entry| entry.ends_with("first (1ms) success=true")));
        assert!(entries.iter().any(|entry| entry.ends_with("second (2ms) success=false")));
    }
}
