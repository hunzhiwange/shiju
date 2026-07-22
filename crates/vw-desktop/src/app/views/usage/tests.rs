//! 用量视图的数据与展示行为测试，覆盖聚合结果和边界输入。

use super::data::UsageData;
use crate::app::App;
use crate::app::models::{ChatMessage, ChatRole, ChatSessionStep, TokenUsage};
use serde_json::json;

fn child_count(element: &iced::Element<'_, crate::app::Message>) -> usize {
    element.as_widget().children().len()
}

#[test]
fn usage_data_counts_tool_messages_separately() {
    let (mut app, _task) = App::new();
    app.chat = vec![
        ChatMessage { role: ChatRole::User, content: "user".to_string(), think_timing: Vec::new() },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "assistant".to_string(),
            think_timing: Vec::new(),
        },
        ChatMessage {
            role: ChatRole::Tool,
            content: "tool grep\n{\"status\":\"completed\",\"output\":\"2 matches\"}\n".to_string(),
            think_timing: Vec::new(),
        },
        ChatMessage {
            role: ChatRole::System,
            content: "system".to_string(),
            think_timing: Vec::new(),
        },
    ];

    let usage = UsageData::from_app(&app);

    assert_eq!(usage.message_count, 4);
    assert_eq!(usage.user_msgs, 1);
    assert_eq!(usage.assistant_msgs, 1);
    assert_eq!(usage.system_msgs, 1);
    assert_eq!(usage.tool_msgs, 1);
}

#[test]
fn usage_view_constructs_from_app_state() {
    let (mut app, _task) = App::new();
    app.chat = vec![ChatMessage {
        role: ChatRole::User,
        content: "统计一下".to_string(),
        think_timing: Vec::new(),
    }];

    let element = super::view(&app);

    assert!(child_count(&element) > 0);
}

#[test]
fn usage_data_exposes_active_steps_and_raw_calls() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("session-usage".to_string());
    app.active_session_view_state.calls = vec![json!({"prompt": "system", "messages": []})];
    app.active_session_view_state.steps = vec![ChatSessionStep {
        index: 1,
        started_ms: 10,
        finished_ms: Some(20),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage {
            input_tokens: 11,
            output_tokens: 12,
            cached_tokens: 3,
            reasoning_tokens: 4,
        },
        cost_usd: Some(0.1),
        finish_reason: Some("stop".to_string()),
        model: Some("model-a".to_string()),
    }];

    let usage = UsageData::from_app(&app);

    assert_eq!(usage.call_count, 1);
    assert_eq!(usage.last_step_total_tokens, 30);
    assert_eq!(usage.session.as_ref().map(|session| session.steps.len()), Some(1));
    assert_eq!(usage.session.as_ref().map(|session| session.calls.len()), Some(1));
}
