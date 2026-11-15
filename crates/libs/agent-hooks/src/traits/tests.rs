//! 钩子 trait 单元测试模块
//!
//! 本模块包含对 `HookHandler` trait 及相关功能的单元测试，验证钩子系统的核心行为：
//! - 钩子结果状态判断（取消/继续）
//! - 默认优先级机制
//! - 默认钩子行为（透传）
//!
//! ## 测试覆盖
//!
//! - `hook_result_is_cancel`: 测试 `HookResult::is_cancel()` 方法的正确性
//! - `hook_result_clone_preserves_variant_payloads`: 验证克隆保留继续值和取消原因
//! - `default_priority_is_zero`: 验证未覆盖 `priority()` 时的默认值为 0
//! - `default_modifying_hooks_pass_through_all_inputs`: 验证默认修改钩子透传各类输入
//! - `default_modifying_hooks_preserve_structured_payloads`: 验证结构化输入逐字段透传
//! - `wasm_hook_handler_bounds_accept_non_send_non_sync_types`: 验证 WASM 边界约束

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试钩子结构体
    ///
    /// 用于单元测试的简单钩子实现，包含可配置的名称和优先级。
    /// 实现了 `HookHandler` trait 以便在测试中验证钩子行为。
    struct TestHook {
        /// 钩子名称标识符
        name: String,
        /// 钩子执行优先级（数值越小优先级越高）
        priority: i32,
    }

    impl TestHook {
        /// 创建新的测试钩子实例
        ///
        /// # 参数
        ///
        /// - `name`: 钩子名称
        /// - `priority`: 钩子优先级
        ///
        /// # 返回值
        ///
        /// 返回配置好的 `TestHook` 实例
        fn new(name: &str, priority: i32) -> Self {
            Self { name: name.to_string(), priority }
        }
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl HookHandler for TestHook {
        /// 返回钩子名称
        fn name(&self) -> &str {
            &self.name
        }

        /// 返回钩子优先级
        fn priority(&self) -> i32 {
            self.priority
        }
    }

    /// 测试 `HookResult::is_cancel()` 方法的行为
    ///
    /// 验证：
    /// - `HookResult::Continue` 变体的 `is_cancel()` 应返回 `false`
    /// - `HookResult::Cancel` 变体的 `is_cancel()` 应返回 `true`
    #[test]
    fn hook_result_is_cancel() {
        // Continue 变体不应被判定为取消
        let ok: HookResult<String> = HookResult::Continue("hi".into());
        assert!(!ok.is_cancel());

        // Cancel 变体应被判定为取消
        let cancel: HookResult<String> = HookResult::Cancel("blocked".into());
        assert!(cancel.is_cancel());
    }

    /// 验证 `HookResult` 克隆后保留原变体及其完整载荷。
    #[test]
    fn hook_result_clone_preserves_variant_payloads() {
        let continued = HookResult::Continue(vec!["first".to_string(), "第二项".to_string()]);
        let cancelled: HookResult<Vec<String>> = HookResult::Cancel("策略拒绝".to_string());
        let continued_clone = continued.clone();
        let cancelled_clone = cancelled.clone();

        assert_eq!(expect_continue(continued), vec!["first".to_string(), "第二项".to_string()]);
        assert_eq!(
            expect_continue(continued_clone),
            vec!["first".to_string(), "第二项".to_string()]
        );
        assert!(cancelled.is_cancel());

        match cancelled_clone {
            HookResult::Cancel(reason) => assert_eq!(reason, "策略拒绝"),
            HookResult::Continue(_) => panic!("expected cloned result to remain cancelled"),
        }
    }

    /// 测试默认优先级为零
    ///
    /// 验证：当 `HookHandler` trait 实现未覆盖 `priority()` 方法时，
    /// 默认实现应返回 0。
    #[test]
    fn default_priority_is_zero() {
        /// 最小化钩子结构体（仅实现必需的 name 方法）
        struct MinimalHook;

        #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
        #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
        impl HookHandler for MinimalHook {
            fn name(&self) -> &str {
                "minimal"
            }
        }

        // 未覆盖 priority() 时应返回默认值 0
        assert_eq!(MinimalHook.priority(), 0);
    }

    fn expect_continue<T>(result: HookResult<T>) -> T {
        match result {
            HookResult::Continue(value) => value,
            HookResult::Cancel(reason) => panic!("unexpected cancel: {reason}"),
        }
    }

    fn channel_message(content: &str) -> ChannelMessage {
        ChannelMessage {
            instance_id: "default".to_string(),
            id: "msg-1".to_string(),
            sender: "sender".to_string(),
            reply_target: "reply".to_string(),
            content: content.to_string(),
            channel: "cli".to_string(),
            timestamp: 7,
            thread_ts: None,
        }
    }

    /// 测试默认空钩子是 no-op。
    ///
    /// 所有 void hook 默认实现都应可被安全调用，不产生 panic 或额外约束。
    #[tokio::test]
    async fn default_void_hooks_are_noops() {
        let hook = TestHook::new("test", 0);
        let messages = vec![ChatMessage::user("hello")];
        let response = ChatResponse {
            text: Some("ok".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        };
        let result = ToolResult { success: true, output: "done".to_string(), error: None };

        hook.on_gateway_start("localhost", 3000).await;
        hook.on_gateway_stop().await;
        hook.on_session_start("session", "telegram").await;
        hook.on_session_end("session", "telegram").await;
        hook.on_llm_input(&messages, "model").await;
        hook.on_llm_output(&response).await;
        hook.on_after_tool_call("shell", &result, std::time::Duration::from_millis(3)).await;
        hook.on_message_sent("cli", "user", "hello").await;
        hook.on_heartbeat_tick().await;

        assert_eq!(hook.name(), "test");
    }

    /// 测试默认修改钩子透传行为
    ///
    /// 验证：当 `HookHandler` 未覆盖修改型钩子方法时，
    /// 默认实现应透传输入（不修改、不取消）。
    #[tokio::test]
    async fn default_modifying_hooks_pass_through_all_inputs() {
        let hook = TestHook::new("test", 0);

        assert_eq!(
            expect_continue(hook.before_model_resolve("openai".into(), "gpt".into()).await),
            ("openai".to_string(), "gpt".to_string())
        );
        assert_eq!(expect_continue(hook.before_prompt_build("prompt".into()).await), "prompt");

        let (messages, model) =
            expect_continue(hook.before_llm_call(vec![ChatMessage::user("hi")], "m".into()).await);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hi");
        assert_eq!(model, "m");

        let (name, args) = expect_continue(
            hook.before_tool_call("shell".into(), serde_json::json!({"cmd": "ls"})).await,
        );
        assert_eq!(name, "shell");
        assert_eq!(args["cmd"], "ls");

        let received = expect_continue(hook.on_message_received(channel_message("body")).await);
        assert_eq!(received.content, "body");

        assert_eq!(
            expect_continue(
                hook.on_message_sending("cli".into(), "user".into(), "hello".into()).await
            ),
            ("cli".to_string(), "user".to_string(), "hello".to_string())
        );
    }

    /// 验证默认修改型钩子不会丢失结构化载荷中的字段、顺序或边界值。
    #[tokio::test]
    async fn default_modifying_hooks_preserve_structured_payloads() {
        let hook = TestHook::new("test", 0);
        let messages = vec![
            ChatMessage { role: "system".to_string(), content: String::new() },
            ChatMessage { role: "user".to_string(), content: "你好\0world".to_string() },
        ];
        let args = serde_json::json!({
            "array": [null, true, 42],
            "nested": {"text": "值"}
        });
        let message = ChannelMessage {
            instance_id: "default".to_string(),
            id: "message-id".to_string(),
            sender: "sender-id".to_string(),
            reply_target: "reply-id".to_string(),
            content: String::new(),
            channel: "channel-id".to_string(),
            timestamp: u64::MAX,
            thread_ts: Some("thread-id".to_string()),
        };

        let (passed_messages, model) =
            expect_continue(hook.before_llm_call(messages, "模型/版本".to_string()).await);
        assert_eq!(passed_messages.len(), 2);
        assert_eq!(passed_messages[0].role, "system");
        assert_eq!(passed_messages[0].content, "");
        assert_eq!(passed_messages[1].role, "user");
        assert_eq!(passed_messages[1].content, "你好\0world");
        assert_eq!(model, "模型/版本");

        let (tool_name, passed_args) =
            expect_continue(hook.before_tool_call("工具".to_string(), args.clone()).await);
        assert_eq!(tool_name, "工具");
        assert_eq!(passed_args, args);

        let passed_message = expect_continue(hook.on_message_received(message).await);
        assert_eq!(passed_message.id, "message-id");
        assert_eq!(passed_message.sender, "sender-id");
        assert_eq!(passed_message.reply_target, "reply-id");
        assert_eq!(passed_message.content, "");
        assert_eq!(passed_message.channel, "channel-id");
        assert_eq!(passed_message.timestamp, u64::MAX);
        assert_eq!(passed_message.thread_ts.as_deref(), Some("thread-id"));
    }

    /// WASM 平台的边界 trait 不应额外要求 `Send + Sync`。
    #[cfg(target_arch = "wasm32")]
    #[test]
    fn wasm_hook_handler_bounds_accept_non_send_non_sync_types() {
        use std::{cell::RefCell, rc::Rc};

        fn assert_hook_handler_bounds<T: HookHandlerBounds>() {}

        assert_hook_handler_bounds::<Rc<RefCell<()>>>();
    }
}
