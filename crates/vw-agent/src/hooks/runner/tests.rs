//! HookRunner 测试模块
//!
//! 本模块提供 HookRunner 的单元测试，验证钩子系统的核心功能：
//! - 钩子注册与优先级排序
//! - 无返回值钩子的触发机制
//! - 修改型钩子的管道式处理
//! - 取消信号的传播行为

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    /// 计数钩子 - 用于追踪无返回值事件触发次数
    ///
    /// 该钩子在心跳事件触发时递增计数器，主要用于测试钩子是否被正确调用。
    struct CountingHook {
        /// 钩子名称
        name: String,
        /// 优先级（数值越大越先执行）
        priority: i32,
        /// 触发计数器（原子引用计数，支持跨线程共享）
        fire_count: Arc<AtomicU32>,
    }

    impl CountingHook {
        /// 创建新的计数钩子实例
        ///
        /// # 参数
        /// - `name`: 钩子名称标识
        /// - `priority`: 执行优先级
        ///
        /// # 返回值
        /// 返回元组 (钩子实例, 可共享的计数器引用)
        fn new(name: &str, priority: i32) -> (Self, Arc<AtomicU32>) {
            let count = Arc::new(AtomicU32::new(0));
            (Self { name: name.to_string(), priority, fire_count: count.clone() }, count)
        }
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl HookHandler for CountingHook {
        fn name(&self) -> &str {
            &self.name
        }
        fn priority(&self) -> i32 {
            self.priority
        }
        /// 心跳事件处理 - 递增触发计数
        async fn on_heartbeat_tick(&self) {
            self.fire_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// 大写转换钩子 - 将提示词转换为大写
    ///
    /// 该钩子实现修改型钩子行为，在提示词构建前将其转换为大写形式。
    struct UppercasePromptHook {
        /// 钩子名称
        name: String,
        /// 优先级
        priority: i32,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl HookHandler for UppercasePromptHook {
        fn name(&self) -> &str {
            &self.name
        }
        fn priority(&self) -> i32 {
            self.priority
        }
        /// 提示词构建前置处理 - 将提示词转为大写
        async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
            HookResult::Continue(prompt.to_uppercase())
        }
    }

    /// 取消钩子 - 阻止提示词构建流程
    ///
    /// 该钩子用于测试取消信号传播机制，任何经过此钩子的提示词构建都会被取消。
    struct CancelPromptHook {
        /// 钩子名称
        name: String,
        /// 优先级
        priority: i32,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl HookHandler for CancelPromptHook {
        fn name(&self) -> &str {
            &self.name
        }
        fn priority(&self) -> i32 {
            self.priority
        }
        /// 提示词构建前置处理 - 返回取消信号
        async fn before_prompt_build(&self, _prompt: String) -> HookResult<String> {
            HookResult::Cancel("blocked by policy".into())
        }
    }

    /// 后缀添加钩子 - 为提示词追加后缀
    ///
    /// 该钩子在提示词末尾添加指定后缀，用于测试修改型钩子的链式处理。
    struct SuffixPromptHook {
        /// 钩子名称
        name: String,
        /// 优先级
        priority: i32,
        /// 要追加的后缀内容
        suffix: String,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl HookHandler for SuffixPromptHook {
        fn name(&self) -> &str {
            &self.name
        }
        fn priority(&self) -> i32 {
            self.priority
        }
        /// 提示词构建前置处理 - 追加后缀
        async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
            HookResult::Continue(format!("{}{}", prompt, self.suffix))
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum EventMode {
        Pass,
        Modify,
        Cancel,
        Panic,
    }

    struct EventHook {
        name: String,
        priority: i32,
        mode: EventMode,
        events: Arc<Mutex<Vec<String>>>,
    }

    impl EventHook {
        fn with_log(
            name: &str,
            priority: i32,
            mode: EventMode,
            events: Arc<Mutex<Vec<String>>>,
        ) -> Self {
            Self { name: name.to_string(), priority, mode, events }
        }

        fn record(&self, event: impl Into<String>) {
            self.events.lock().expect("events lock").push(event.into());
        }

        fn finish<T>(&self, method: &str, value: T, modified: T) -> HookResult<T> {
            match self.mode {
                EventMode::Pass => HookResult::Continue(value),
                EventMode::Modify => HookResult::Continue(modified),
                EventMode::Cancel => HookResult::Cancel(format!("{}:{method}", self.name)),
                EventMode::Panic => panic!("{}:{method}:panic", self.name),
            }
        }
    }

    fn shared_events() -> Arc<Mutex<Vec<String>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn events_snapshot(events: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
        events.lock().expect("events lock").clone()
    }

    fn expect_continue<T>(result: HookResult<T>) -> T {
        match result {
            HookResult::Continue(value) => value,
            HookResult::Cancel(reason) => panic!("unexpected cancel: {reason}"),
        }
    }

    fn expect_cancel<T>(result: HookResult<T>) -> String {
        match result {
            HookResult::Continue(_) => panic!("unexpected continue"),
            HookResult::Cancel(reason) => reason,
        }
    }

    fn message(content: &str) -> ChannelMessage {
        ChannelMessage {
            instance_id: "default".to_string(),
            id: "msg-1".to_string(),
            sender: "user-1".to_string(),
            reply_target: "thread-1".to_string(),
            content: content.to_string(),
            channel: "cli".to_string(),
            timestamp: 123,
            thread_ts: Some("thread-ts".to_string()),
        }
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl HookHandler for EventHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        async fn on_gateway_start(&self, host: &str, port: u16) {
            self.record(format!("{}:gateway_start:{host}:{port}", self.name));
        }

        async fn on_gateway_stop(&self) {
            self.record(format!("{}:gateway_stop", self.name));
        }

        async fn on_session_start(&self, session_id: &str, channel: &str) {
            self.record(format!("{}:session_start:{session_id}:{channel}", self.name));
        }

        async fn on_session_end(&self, session_id: &str, channel: &str) {
            self.record(format!("{}:session_end:{session_id}:{channel}", self.name));
        }

        async fn on_llm_input(&self, messages: &[ChatMessage], model: &str) {
            self.record(format!("{}:llm_input:{}:{model}", self.name, messages.len()));
        }

        async fn on_llm_output(&self, response: &ChatResponse) {
            self.record(format!("{}:llm_output:{}", self.name, response.text_or_empty()));
        }

        async fn on_after_tool_call(&self, tool: &str, result: &ToolResult, duration: Duration) {
            self.record(format!(
                "{}:after_tool:{tool}:{}:{}:{}",
                self.name,
                result.success,
                result.output,
                duration.as_millis()
            ));
        }

        async fn on_message_sent(&self, channel: &str, recipient: &str, content: &str) {
            self.record(format!("{}:message_sent:{channel}:{recipient}:{content}", self.name));
        }

        async fn on_heartbeat_tick(&self) {
            self.record(format!("{}:heartbeat", self.name));
        }

        async fn before_model_resolve(
            &self,
            provider: String,
            model: String,
        ) -> HookResult<(String, String)> {
            self.record(format!("{}:before_model_resolve:{provider}:{model}", self.name));
            let modified = (format!("{provider}|{}", self.name), format!("{model}|{}", self.name));
            self.finish("before_model_resolve", (provider, model), modified)
        }

        async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
            self.record(format!("{}:before_prompt_build:{prompt}", self.name));
            let modified = format!("{prompt}|{}", self.name);
            self.finish("before_prompt_build", prompt, modified)
        }

        async fn before_llm_call(
            &self,
            messages: Vec<ChatMessage>,
            model: String,
        ) -> HookResult<(Vec<ChatMessage>, String)> {
            self.record(format!("{}:before_llm_call:{}:{model}", self.name, messages.len()));
            let mut modified_messages = messages.clone();
            modified_messages.push(ChatMessage::system(format!("hook:{}", self.name)));
            let modified = (modified_messages, format!("{model}|{}", self.name));
            self.finish("before_llm_call", (messages, model), modified)
        }

        async fn before_tool_call(
            &self,
            tool_name: String,
            args: Value,
        ) -> HookResult<(String, Value)> {
            self.record(format!("{}:before_tool_call:{tool_name}", self.name));
            let mut modified_args = args.clone();
            if let Some(map) = modified_args.as_object_mut() {
                map.insert("hook".to_string(), Value::String(self.name.clone()));
            }
            let modified = (format!("{tool_name}|{}", self.name), modified_args);
            self.finish("before_tool_call", (tool_name, args), modified)
        }

        async fn on_message_received(&self, message: ChannelMessage) -> HookResult<ChannelMessage> {
            self.record(format!("{}:on_message_received:{}", self.name, message.content));
            let mut modified = message.clone();
            modified.content = format!("{}|{}", modified.content, self.name);
            self.finish("on_message_received", message, modified)
        }

        async fn on_message_sending(
            &self,
            channel: String,
            recipient: String,
            content: String,
        ) -> HookResult<(String, String, String)> {
            self.record(format!(
                "{}:on_message_sending:{channel}:{recipient}:{content}",
                self.name
            ));
            let modified = (
                format!("{channel}|{}", self.name),
                format!("{recipient}|{}", self.name),
                format!("{content}|{}", self.name),
            );
            self.finish("on_message_sending", (channel, recipient, content), modified)
        }
    }

    /// 测试钩子注册与优先级排序
    ///
    /// 验证钩子按优先级从高到低排序：
    /// - 优先级 10 的钩子应排在第一位
    /// - 优先级 5 的钩子应排在第二位
    /// - 优先级 1 的钩子应排在第三位
    #[test]
    fn register_and_sort_by_priority() {
        let mut runner = HookRunner::new();

        // 创建三个不同优先级的钩子
        let (low, _) = CountingHook::new("low", 1);
        let (high, _) = CountingHook::new("high", 10);
        let (mid, _) = CountingHook::new("mid", 5);

        // 按随机顺序注册钩子
        runner.register(Box::new(low));
        runner.register(Box::new(high));
        runner.register(Box::new(mid));

        // 验证内部存储顺序：优先级从高到低
        let names: Vec<&str> = runner.handlers.iter().map(|h| h.name()).collect();
        assert_eq!(names, vec!["high", "mid", "low"]);
    }

    #[test]
    fn register_orders_priority_extremes_and_preserves_equal_priority_order() {
        let mut runner = HookRunner::new();

        let (first_equal, _) = CountingHook::new("first_equal", 0);
        let (lowest, _) = CountingHook::new("lowest", i32::MIN);
        let (highest, _) = CountingHook::new("highest", i32::MAX);
        let (second_equal, _) = CountingHook::new("second_equal", 0);

        runner.register(Box::new(first_equal));
        runner.register(Box::new(lowest));
        runner.register(Box::new(highest));
        runner.register(Box::new(second_equal));

        let names: Vec<&str> = runner.handlers.iter().map(|handler| handler.name()).collect();
        assert_eq!(names, vec!["highest", "first_equal", "second_equal", "lowest"]);
    }

    /// 测试无返回值钩子的全部触发机制
    ///
    /// 验证所有已注册钩子在心跳事件时都会被调用，
    /// 不受优先级影响（因为无返回值钩子不产生取消信号）。
    #[tokio::test]
    async fn void_hooks_fire_all_handlers() {
        let mut runner = HookRunner::new();

        // 注册两个计数钩子
        let (h1, c1) = CountingHook::new("hook_a", 0);
        let (h2, c2) = CountingHook::new("hook_b", 0);

        runner.register(Box::new(h1));
        runner.register(Box::new(h2));

        // 触发心跳事件
        runner.fire_heartbeat_tick().await;

        // 验证两个钩子的计数器都递增了
        assert_eq!(c1.load(Ordering::SeqCst), 1);
        assert_eq!(c2.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn empty_runner_accepts_all_void_dispatches() {
        let runner = HookRunner::new();
        let response = ChatResponse {
            text: None,
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        };
        let result = ToolResult { success: false, output: String::new(), error: None };

        runner.fire_gateway_start("127.0.0.1", 0).await;
        runner.fire_gateway_stop().await;
        runner.fire_session_start("", "").await;
        runner.fire_session_end("", "").await;
        runner.fire_llm_input(&[], "").await;
        runner.fire_llm_output(&response).await;
        runner.fire_after_tool_call("", &result, Duration::ZERO).await;
        runner.fire_message_sent("", "", "").await;
        runner.fire_heartbeat_tick().await;

        assert!(runner.handlers.is_empty());
    }

    /// 测试修改型钩子的取消传播
    ///
    /// 验证高优先级钩子返回 Cancel 时，
    /// 后续钩子不会被调用，整个管道立即终止。
    #[tokio::test]
    async fn modifying_hook_can_cancel() {
        let mut runner = HookRunner::new();

        // 注册高优先级的取消钩子（优先级 10）
        runner.register(Box::new(CancelPromptHook { name: "blocker".into(), priority: 10 }));
        // 注册低优先级的大写钩子（优先级 0）
        runner.register(Box::new(UppercasePromptHook { name: "upper".into(), priority: 0 }));

        // 执行提示词构建管道
        let result = runner.run_before_prompt_build("hello".into()).await;

        // 验证结果为取消状态
        assert!(result.is_cancel());
    }

    /// 测试修改型钩子的管道式数据传递
    ///
    /// 验证多个修改型钩子按优先级顺序执行，
    /// 前一个钩子的输出作为下一个钩子的输入。
    #[tokio::test]
    async fn modifying_hook_pipelines_data() {
        let mut runner = HookRunner::new();

        // 优先级 10 先执行：将 "hello" 转为 "HELLO"
        runner.register(Box::new(UppercasePromptHook { name: "upper".into(), priority: 10 }));

        // 优先级 0 后执行：将 "HELLO" 追加后缀变为 "HELLO_done"
        runner.register(Box::new(SuffixPromptHook {
            name: "suffix".into(),
            priority: 0,
            suffix: "_done".into(),
        }));

        // 执行管道处理
        match runner.run_before_prompt_build("hello".into()).await {
            HookResult::Continue(result) => assert_eq!(result, "HELLO_done"),
            HookResult::Cancel(_) => panic!("should not cancel"),
        }
    }

    #[tokio::test]
    async fn void_hooks_forward_all_event_arguments_to_handlers() {
        let events = shared_events();
        let mut runner = HookRunner::new();
        runner.register(Box::new(EventHook::with_log(
            "first",
            10,
            EventMode::Pass,
            events.clone(),
        )));
        runner.register(Box::new(EventHook::with_log(
            "second",
            0,
            EventMode::Pass,
            events.clone(),
        )));

        runner.fire_gateway_start("127.0.0.1", 8787).await;
        runner.fire_gateway_stop().await;
        runner.fire_session_start("session-1", "telegram").await;
        runner.fire_session_end("session-1", "telegram").await;
        runner.fire_llm_input(&[ChatMessage::user("hello")], "model-a").await;
        runner
            .fire_llm_output(&ChatResponse {
                text: Some("done".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            })
            .await;
        runner
            .fire_after_tool_call(
                "shell",
                &ToolResult { success: true, output: "ok".to_string(), error: None },
                Duration::from_millis(42),
            )
            .await;
        runner.fire_message_sent("slack", "user-1", "hi").await;
        runner.fire_heartbeat_tick().await;

        let mut events = events_snapshot(&events);
        events.sort();
        let mut expected = vec![
            "first:after_tool:shell:true:ok:42".to_string(),
            "first:gateway_start:127.0.0.1:8787".to_string(),
            "first:gateway_stop".to_string(),
            "first:heartbeat".to_string(),
            "first:llm_input:1:model-a".to_string(),
            "first:llm_output:done".to_string(),
            "first:message_sent:slack:user-1:hi".to_string(),
            "first:session_end:session-1:telegram".to_string(),
            "first:session_start:session-1:telegram".to_string(),
            "second:after_tool:shell:true:ok:42".to_string(),
            "second:gateway_start:127.0.0.1:8787".to_string(),
            "second:gateway_stop".to_string(),
            "second:heartbeat".to_string(),
            "second:llm_input:1:model-a".to_string(),
            "second:llm_output:done".to_string(),
            "second:message_sent:slack:user-1:hi".to_string(),
            "second:session_end:session-1:telegram".to_string(),
            "second:session_start:session-1:telegram".to_string(),
        ];
        expected.sort();
        assert_eq!(events, expected);
    }

    #[tokio::test]
    async fn empty_runner_returns_original_values_for_modifying_hooks() {
        let runner = HookRunner::new();

        assert_eq!(
            expect_continue(runner.run_before_model_resolve("openai".into(), "gpt".into()).await),
            ("openai".to_string(), "gpt".to_string())
        );
        assert_eq!(
            expect_continue(runner.run_before_prompt_build("prompt".into()).await),
            "prompt"
        );

        let (messages, model) = expect_continue(
            runner.run_before_llm_call(vec![ChatMessage::user("hi")], "m".into()).await,
        );
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hi");
        assert_eq!(model, "m");

        let (tool, args) =
            expect_continue(runner.run_before_tool_call("read".into(), json!({"path": "a"})).await);
        assert_eq!(tool, "read");
        assert_eq!(args["path"], "a");

        let received = expect_continue(runner.run_on_message_received(message("body")).await);
        assert_eq!(received.content, "body");

        assert_eq!(
            expect_continue(
                runner.run_on_message_sending("cli".into(), "user".into(), "hello".into()).await
            ),
            ("cli".to_string(), "user".to_string(), "hello".to_string())
        );
    }

    #[tokio::test]
    async fn modifying_hooks_pipeline_every_dispatcher_by_priority() {
        let events = shared_events();
        let mut runner = HookRunner::new();
        runner.register(Box::new(EventHook::with_log("low", 0, EventMode::Modify, events.clone())));
        runner.register(Box::new(EventHook::with_log(
            "high",
            10,
            EventMode::Modify,
            events.clone(),
        )));

        assert_eq!(
            expect_continue(runner.run_before_model_resolve("p".into(), "m".into()).await),
            ("p|high|low".to_string(), "m|high|low".to_string())
        );
        assert_eq!(
            expect_continue(runner.run_before_prompt_build("prompt".into()).await),
            "prompt|high|low"
        );

        let (messages, model) = expect_continue(
            runner.run_before_llm_call(vec![ChatMessage::user("hi")], "m".into()).await,
        );
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].content, "hook:high");
        assert_eq!(messages[2].content, "hook:low");
        assert_eq!(model, "m|high|low");

        let (tool, args) =
            expect_continue(runner.run_before_tool_call("read".into(), json!({"path": "a"})).await);
        assert_eq!(tool, "read|high|low");
        assert_eq!(args["hook"], "low");

        let received = expect_continue(runner.run_on_message_received(message("body")).await);
        assert_eq!(received.content, "body|high|low");

        assert_eq!(
            expect_continue(
                runner.run_on_message_sending("cli".into(), "user".into(), "hello".into()).await
            ),
            ("cli|high|low".to_string(), "user|high|low".to_string(), "hello|high|low".to_string())
        );

        let events = events_snapshot(&events);
        let high =
            events.iter().position(|event| event.starts_with("high:before_model_resolve")).unwrap();
        let low =
            events.iter().position(|event| event.starts_with("low:before_model_resolve")).unwrap();
        assert!(high < low);
    }

    #[tokio::test]
    async fn modifying_hooks_cancel_every_dispatcher_and_short_circuit() {
        let events = shared_events();
        let mut runner = HookRunner::new();
        runner.register(Box::new(EventHook::with_log("low", 0, EventMode::Modify, events.clone())));
        runner.register(Box::new(EventHook::with_log(
            "block",
            10,
            EventMode::Cancel,
            events.clone(),
        )));

        assert_eq!(
            expect_cancel(runner.run_before_model_resolve("p".into(), "m".into()).await),
            "block:before_model_resolve"
        );
        assert_eq!(
            expect_cancel(runner.run_before_prompt_build("prompt".into()).await),
            "block:before_prompt_build"
        );
        assert_eq!(
            expect_cancel(
                runner.run_before_llm_call(vec![ChatMessage::user("hi")], "m".into()).await
            ),
            "block:before_llm_call"
        );
        assert_eq!(
            expect_cancel(runner.run_before_tool_call("read".into(), json!({})).await),
            "block:before_tool_call"
        );
        assert_eq!(
            expect_cancel(runner.run_on_message_received(message("body")).await),
            "block:on_message_received"
        );
        assert_eq!(
            expect_cancel(
                runner.run_on_message_sending("cli".into(), "user".into(), "hello".into()).await
            ),
            "block:on_message_sending"
        );

        assert!(events_snapshot(&events).iter().all(|event| !event.starts_with("low:")));
    }

    #[tokio::test]
    async fn pre_execution_hooks_reject_panics_while_observers_keep_previous_values() {
        let events = shared_events();
        let mut runner = HookRunner::new();
        runner.register(Box::new(EventHook::with_log("low", 0, EventMode::Modify, events.clone())));
        runner.register(Box::new(EventHook::with_log(
            "panic",
            10,
            EventMode::Panic,
            events.clone(),
        )));

        assert_eq!(
            expect_continue(runner.run_before_model_resolve("p".into(), "m".into()).await),
            ("p|low".to_string(), "m|low".to_string())
        );
        assert_eq!(
            expect_continue(runner.run_before_prompt_build("prompt".into()).await),
            "prompt|low"
        );

        assert!(matches!(
            runner.run_before_llm_call(vec![ChatMessage::user("hi")], "m".into()).await,
            HookResult::Cancel(_)
        ));
        assert!(matches!(
            runner.run_before_tool_call("read".into(), json!({"path": "a"})).await,
            HookResult::Cancel(_)
        ));

        let received = expect_continue(runner.run_on_message_received(message("body")).await);
        assert_eq!(received.content, "body|low");

        assert_eq!(
            expect_continue(
                runner.run_on_message_sending("cli".into(), "user".into(), "hello".into()).await
            ),
            ("cli|low".to_string(), "user|low".to_string(), "hello|low".to_string())
        );

        let events = events_snapshot(&events);
        assert!(events.iter().any(|event| event.starts_with("panic:before_tool_call")));
        assert!(!events.iter().any(|event| event.starts_with("low:before_tool_call")));
    }
}
