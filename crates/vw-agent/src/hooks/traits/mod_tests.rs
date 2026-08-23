// 验证钩子事件与结果类型的默认行为和对象安全调用。

// before 钩子的 Continue 与 Cancel 分支分别校验，默认优先级保持可预测。
use super::*;

struct DefaultHook;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HookHandler for DefaultHook {
    fn name(&self) -> &str {
        "default"
    }
}

struct GuardHook;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HookHandler for GuardHook {
    fn name(&self) -> &str {
        "guard"
    }

    fn priority(&self) -> i32 {
        i32::MIN
    }

    async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
        HookResult::Continue(format!("{prompt}!"))
    }

    async fn before_tool_call(&self, name: String, args: Value) -> HookResult<(String, Value)> {
        if name == "blocked" {
            HookResult::Cancel("blocked by test".to_string())
        } else {
            HookResult::Continue((name, args))
        }
    }
}

fn channel_message(content: &str, thread_ts: Option<&str>) -> ChannelMessage {
    ChannelMessage {
        id: "msg-1".to_string(),
        sender: "sender".to_string(),
        reply_target: "reply".to_string(),
        content: content.to_string(),
        channel: "cli".to_string(),
        timestamp: u64::MAX,
        thread_ts: thread_ts.map(str::to_string),
    }
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

#[cfg(not(target_arch = "wasm32"))]
fn assert_hook_bounds<T: HookHandlerBounds>() {}

#[test]
fn hook_result_reports_cancel_state_and_preserves_reason() {
    let ok = HookResult::Continue(String::new());
    let cancel: HookResult<String> = HookResult::Cancel("no".to_string());

    assert!(!ok.is_cancel());
    assert!(cancel.is_cancel());
    assert_eq!(expect_cancel(cancel.clone()), "no");
    assert!(format!("{cancel:?}").contains("no"));
}

#[test]
fn default_hook_identity_priority_and_native_bounds_are_stable() {
    let hook = DefaultHook;

    assert_eq!(hook.name(), "default");
    assert_eq!(hook.priority(), 0);

    #[cfg(not(target_arch = "wasm32"))]
    assert_hook_bounds::<DefaultHook>();
}

#[tokio::test]
async fn default_void_hooks_accept_boundary_inputs_without_effects() {
    let hook = DefaultHook;
    let messages = [ChatMessage::system("")];
    let response = ChatResponse {
        text: None,
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: Some(String::new()),
    };
    let result = ToolResult { success: false, output: String::new(), error: Some(String::new()) };

    hook.on_gateway_start("", u16::MAX).await;
    hook.on_gateway_stop().await;
    hook.on_session_start("", "").await;
    hook.on_session_end("", "").await;
    hook.on_llm_input(&messages, "").await;
    hook.on_llm_output(&response).await;
    hook.on_after_tool_call("", &result, Duration::ZERO).await;
    hook.on_message_sent("", "", "").await;
    hook.on_heartbeat_tick().await;

    assert_eq!(hook.name(), "default");
}

#[tokio::test]
async fn default_modifying_hooks_pass_through_boundary_values() {
    let hook = DefaultHook;
    let args = serde_json::json!({"nested": [{"n": 1}], "empty": ""});
    let message = channel_message("", Some("thread"));
    let chat_messages = vec![ChatMessage::user("hi"), ChatMessage::assistant("")];

    assert_eq!(
        expect_continue(hook.before_model_resolve(String::new(), String::new()).await),
        (String::new(), String::new())
    );
    assert_eq!(expect_continue(hook.before_prompt_build(String::new()).await), String::new());

    let (messages, model) =
        expect_continue(hook.before_llm_call(chat_messages.clone(), String::new()).await);
    assert_eq!(messages.len(), chat_messages.len());
    assert_eq!(messages[0].role, "user");
    assert_eq!(model, String::new());

    let (name, passed_args) =
        expect_continue(hook.before_tool_call(String::new(), args.clone()).await);
    assert_eq!(name, String::new());
    assert_eq!(passed_args, args);

    let received = expect_continue(hook.on_message_received(message).await);
    assert_eq!(received.timestamp, u64::MAX);
    assert_eq!(received.thread_ts.as_deref(), Some("thread"));

    assert_eq!(
        expect_continue(hook.on_message_sending(String::new(), String::new(), String::new()).await),
        (String::new(), String::new(), String::new())
    );
}

#[tokio::test]
async fn custom_hook_can_modify_or_cancel_modifying_hooks() {
    let hook = GuardHook;

    assert_eq!(hook.name(), "guard");
    assert_eq!(hook.priority(), i32::MIN);
    assert_eq!(expect_continue(hook.before_prompt_build("prompt".to_string()).await), "prompt!");

    let args = serde_json::json!({"ok": true});
    assert_eq!(
        expect_continue(hook.before_tool_call("allowed".to_string(), args.clone()).await),
        ("allowed".to_string(), args)
    );
    assert_eq!(
        expect_cancel(hook.before_tool_call("blocked".to_string(), serde_json::json!({})).await),
        "blocked by test"
    );
}
