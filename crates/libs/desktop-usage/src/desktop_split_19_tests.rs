//! 从宿主用量投影拆出的局部聚合行为，保留原断言。
use crate::data::UsageData;
use session_ui_types::{ChatMessage, ChatRole, ChatSessionStep, TokenUsage};
struct Input {
    usage: TokenUsage,
    active_session_id: Option<String>,
    chat: Vec<ChatMessage>,
    steps: Vec<ChatSessionStep>,
    updated_ms: u64,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            usage: TokenUsage::default(),
            active_session_id: None,
            chat: Vec::new(),
            steps: Vec::new(),
            updated_ms: 0,
        }
    }
}
fn project(input: &Input) -> UsageData {
    UsageData::from_parts(
        &input.usage,
        input.active_session_id.as_ref(),
        &input.chat,
        None,
        &[],
        &input.steps,
        input.updated_ms,
        config_ui::DesktopLocale::SimplifiedChinese,
    )
}
fn step(index: u32, usage: TokenUsage) -> ChatSessionStep {
    ChatSessionStep {
        index,
        turn_id: None,
        started_ms: 10,
        finished_ms: Some(20),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage,
        cost_usd: None,
        finish_reason: None,
        model: None,
        tool_timings: Vec::new(),
    }
}

fn message(role: ChatRole) -> ChatMessage {
    ChatMessage {
        role,
        content: "content".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn from_app_sums_total_usage_and_counts_message_roles() {
    let mut app = Input::default();
    app.usage.input_tokens = 10;
    app.usage.output_tokens = 20;
    app.usage.cached_tokens = 3;
    app.usage.reasoning_tokens = 7;
    app.chat = vec![
        message(ChatRole::User),
        message(ChatRole::Assistant),
        message(ChatRole::Assistant),
        message(ChatRole::System),
        message(ChatRole::Tool),
    ];

    let usage = project(&app);

    assert_eq!(usage.total_tokens, 40);
    assert_eq!(usage.message_count, 5);
    assert_eq!(usage.user_msgs, 1);
    assert_eq!(usage.assistant_msgs, 2);
    assert_eq!(usage.system_msgs, 1);
    assert_eq!(usage.tool_msgs, 1);
}

#[test]
fn from_app_defaults_without_active_session() {
    let app = Input::default();

    let usage = project(&app);

    assert!(usage.session.is_none());
    assert_eq!(usage.session_id, "暂无");
    assert_eq!(usage.session_title, "暂无");
    assert_eq!(usage.call_count, 0);
    assert_eq!(usage.last_step_input_tokens, 0);
    assert_eq!(usage.last_step_output_tokens, 0);
    assert_eq!(usage.last_step_cached_tokens, 0);
    assert_eq!(usage.last_step_reasoning_tokens, 0);
    assert_eq!(usage.last_step_total_tokens, 0);
}

#[test]
fn from_app_uses_active_view_updated_time_when_metadata_is_missing() {
    let mut app = Input::default();
    app.active_session_id = Some("missing-metadata".to_string());
    app.updated_ms = 9876;
    app.steps = vec![step(
        1,
        TokenUsage { input_tokens: 4, output_tokens: 5, cached_tokens: 6, reasoning_tokens: 7 },
    )];

    let usage = project(&app);
    let session = usage.session.as_ref().expect("active session should still be projected");

    assert_eq!(usage.session_id, "missing-metadata");
    assert_eq!(usage.session_title, "暂无");
    assert_eq!((session.created_ms, session.updated_ms), (0, 9876));
    assert_eq!(usage.last_step_input_tokens, 4);
    assert_eq!(usage.last_step_output_tokens, 5);
    assert_eq!(usage.last_step_cached_tokens, 6);
    assert_eq!(usage.last_step_reasoning_tokens, 7);
    assert_eq!(usage.last_step_total_tokens, 22);
}

#[test]
fn from_app_preserves_reported_usage_while_next_step_is_pending() {
    let mut app = Input::default();
    app.active_session_id = Some("session".to_string());
    app.steps = vec![
        step(
            1,
            TokenUsage {
                input_tokens: 300,
                output_tokens: 20,
                cached_tokens: 0,
                reasoning_tokens: 0,
            },
        ),
        step(2, TokenUsage::default()),
    ];
    app.steps[1].finished_ms = None;
    let usage = project(&app);
    assert_eq!(usage.last_step_input_tokens, 300);
    assert_eq!(usage.last_step_total_tokens, 320);
}
