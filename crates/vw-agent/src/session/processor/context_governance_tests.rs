//! 验证上下文预算、历史裁剪、摘要请求与技能上下文恢复。

use super::*;
use crate::app::agent::provider::provider::{ModelLimit, resolve_model_limit};
use crate::app::agent::tools::ToolSpec;
use crate::session::session::{Message, Role, Session};
use serde_json::json;

fn limit(context: u64, input: Option<u64>, output: u64) -> ModelLimit {
    ModelLimit { context, input, output }
}

fn message(role: Role, content: &str) -> Message {
    Message { role, content: content.to_string() }
}

#[test]
fn context_compaction_never_removes_agents_protocol_reminder() {
    let reminder = json!({
        "role": "user",
        "content": "<system-reminder>Instructions from: /workspace/AGENTS.md\nFollow it.</system-reminder>"
    });
    let mut messages = vec![reminder.clone()];
    for index in 0..12 {
        messages.push(json!({
            "role": if index % 2 == 0 { "user" } else { "assistant" },
            "content": format!("history-{index}-{}", "x".repeat(240))
        }));
    }

    let result = compact_context_for_next_request(
        &mut messages,
        &["system".to_string()],
        &[],
        &limit(600, Some(600), 0),
        None,
    );

    let result = result.expect("history sizing should succeed");
    assert!(result.needs_internal_compaction);
    assert!(messages.contains(&reminder));
}

#[test]
fn history_trim_preserves_system_and_recent_messages() {
    let mut session = Session {
        id: "context-governance".to_string(),
        messages: vec![
            message(Role::System, "root rules"),
            message(Role::User, "old user"),
            message(Role::Assistant, "old answer"),
            message(Role::System, "runtime notice"),
            message(Role::User, "recent user"),
            message(Role::Tool, "recent tool"),
            message(Role::Assistant, "recent answer"),
        ],
    };

    let removed = trim_session_history(&mut session, 3);

    assert_eq!(removed, 2);
    assert_eq!(
        session
            .messages
            .iter()
            .map(|item| (item.role.clone(), item.content.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (Role::System, "root rules"),
            (Role::System, "runtime notice"),
            (Role::User, "recent user"),
            (Role::Tool, "recent tool"),
            (Role::Assistant, "recent answer"),
        ]
    );
}

#[test]
fn history_trim_handles_zero_budget_and_in_range_history() {
    let mut zero = Session {
        id: "zero".to_string(),
        messages: vec![
            message(Role::System, "rules"),
            message(Role::User, "user"),
            message(Role::Assistant, "answer"),
        ],
    };
    assert_eq!(trim_session_history(&mut zero, 0), 2);
    assert_eq!(zero.messages.len(), 1);
    assert_eq!(zero.messages[0].role, Role::System);
    assert_eq!(zero.messages[0].content, "rules");

    let mut unchanged =
        Session { id: "unchanged".to_string(), messages: vec![message(Role::User, "user")] };
    assert_eq!(trim_session_history(&mut unchanged, 2), 0);
    assert_eq!(unchanged.messages.len(), 1);
    assert_eq!(unchanged.messages[0].role, Role::User);
    assert_eq!(unchanged.messages[0].content, "user");
}

#[test]
fn stale_tool_results_clear_content_but_keep_protocol_fields() {
    let mut messages = vec![
        json!({
            "role": "assistant",
            "tool_calls": [{"id": "call-old", "name": "read"}]
        }),
        json!({
            "role": "tool",
            "tool_call_id": "call-old",
            "name": "read",
            "content": "large old output"
        }),
        json!({"role": "assistant", "content": "continuing"}),
        json!({
            "role": "tool",
            "tool_call_id": "call-new",
            "name": "bash",
            "content": "recent output"
        }),
    ];

    let cleared = clear_stale_tool_results(&mut messages, 1);

    assert_eq!(cleared, 1);
    assert_eq!(messages[1]["content"], STALE_TOOL_RESULT_PLACEHOLDER);
    assert_eq!(messages[1]["tool_call_id"], "call-old");
    assert_eq!(messages[1]["name"], "read");
    assert_eq!(messages[3]["content"], "recent output");
    assert_eq!(messages[0]["tool_calls"][0]["id"], "call-old");
}

#[test]
fn stale_tool_result_clearing_is_idempotent_and_ignores_other_roles() {
    let mut messages = vec![
        json!({"role": "user", "content": "keep user"}),
        json!({"role": "tool", "tool_call_id": "one", "content": "clear me"}),
        json!({"role": "assistant", "content": "keep assistant"}),
    ];

    assert_eq!(clear_stale_tool_results(&mut messages, 0), 1);
    assert_eq!(clear_stale_tool_results(&mut messages, 0), 0);
    assert_eq!(messages[0]["content"], "keep user");
    assert_eq!(messages[2]["content"], "keep assistant");
}

#[test]
fn auto_compaction_uses_an_inclusive_ninety_percent_boundary() {
    let model_limit = limit(1_000, Some(1_000), 100);

    assert!(!should_compact(899, &model_limit));
    assert!(should_compact(900, &model_limit));
    assert!(should_compact(901, &model_limit));
}

#[test]
fn context_governance_requests_internal_summary_instead_of_discarding_history() {
    let mut messages = vec![json!({"role": "system", "content": "rules"})];
    for index in 0..12 {
        messages.push(json!({
            "role": if index % 2 == 0 { "user" } else { "assistant" },
            "content": format!("history-{index}-{}", "x".repeat(240))
        }));
    }
    let original = messages.clone();

    let decision = context_governance_decision(
        &messages,
        &["rules".to_string()],
        &[],
        &limit(600, Some(600), 0),
        None,
    );

    assert!(decision.needs_internal_compaction);
    assert_eq!(messages, original, "sizing must not discard historical messages");
}

#[test]
fn effective_input_window_prefers_input_and_handles_invalid_output_space() {
    assert_eq!(effective_input_window(&limit(100_000, Some(80_000), 20_000)), 80_000);
    assert_eq!(effective_input_window(&limit(100_000, None, 10_000)), 90_000);
    assert_eq!(effective_input_window(&limit(1_000, None, 2_000)), 0);

    let defaults = resolve_model_limit(None, None, None);
    assert_eq!(defaults.context, 200_000);
    assert_eq!(defaults.input, Some(200_000));
    assert_eq!(defaults.output, 4_096);
}

#[test]
fn context_usage_prefers_positive_real_input_usage_and_estimates_missing_usage() {
    let messages = vec![json!({"role": "user", "content": "small"})];
    let system = vec!["rules".to_string()];
    let tools = vec![ToolSpec::new("search", "search", json!({"type": "object"}))];
    let model_limit = limit(1_000, Some(1_000), 100);

    assert_eq!(context_usage_tokens(&messages, &system, &tools, Some(900)), 900);
    assert!(context_usage_tokens(&messages, &system, &tools, Some(0)) > 0);
    assert!(context_usage_tokens(&messages, &system, &tools, Some(-1)) > 0);
    assert!(should_compact(
        context_usage_tokens(&messages, &system, &tools, Some(900)),
        &model_limit
    ));
}

#[test]
fn context_usage_does_not_count_base64_image_body_as_text_tokens() {
    let image_payload = "a".repeat(4 * 1024 * 1024);
    let mut messages = vec![json!({
        "role": "user",
        "content": format!("inspect [IMAGE:data:image/jpeg;base64,{image_payload}]")
    })];
    let system = vec!["rules".to_string()];
    let tools = Vec::<ToolSpec>::new();
    let model_limit = limit(1_000_000, Some(200_000), 4_096);

    let estimated = context_usage_tokens(&messages, &system, &tools, None);

    assert!(estimated < 200_000);
    let result =
        compact_context_for_next_request(&mut messages, &system, &tools, &model_limit, None)
            .expect("a vision request should be sized without mutation");
    assert!(!result.needs_internal_compaction);
}

#[test]
fn compaction_requests_internal_summary_without_clearing_protocol_history() {
    let mut messages = vec![json!({"role": "system", "content": "system rules"})];
    for index in 0..9 {
        let call_id = format!("old-call-{index}");
        messages.push(json!({
            "role": "assistant",
            "tool_calls": [{"id": call_id, "type": "function"}]
        }));
        messages.push(json!({
            "role": "tool",
            "tool_call_id": call_id,
            "content": if index == 0 { "old tool output ".repeat(80) } else { "recent".to_string() }
        }));
    }
    messages.push(json!({"role": "user", "content": "recent question"}));
    messages.push(json!({"role": "assistant", "content": "recent answer"}));

    let result = compact_context_for_next_request(
        &mut messages,
        &["system rules".to_string()],
        &[],
        &limit(500, Some(500), 0),
        Some(490),
    )
    .expect("oversized history should produce a compaction decision");

    assert!(result.needs_internal_compaction);
    assert!(messages.iter().any(|message| message["role"] == "system"));
    assert!(messages.iter().any(|message| message["role"] == "user"));
    assert!(
        messages
            .iter()
            .any(|message| message["role"] == "assistant" && message["tool_calls"].is_array())
    );
    let cleared = messages
        .iter()
        .find(|message| message["tool_call_id"] == "old-call-0")
        .expect("old tool pair should remain");
    assert_eq!(cleared["content"], "old tool output ".repeat(80));
}

#[test]
fn compaction_requests_summary_when_system_alone_is_too_large() {
    let mut messages = vec![json!({"role": "user", "content": "question"})];
    let result = compact_context_for_next_request(
        &mut messages,
        &["x".repeat(1_000)],
        &[],
        &limit(100, Some(100), 0),
        None,
    )
    .expect("sizing should not discard an oversized system prompt");

    assert!(result.needs_internal_compaction);
}

#[test]
fn invoked_skill_context_restore_replaces_stale_messages_and_preserves_updates() {
    let replay = crate::app::agent::skills::InvokedSkillReplay {
        stable_id: "file:///skills/review/SKILL.md".to_string(),
        content: "<system-reminder>current skill body</system-reminder>".to_string(),
        allowed_tools: vec!["read".to_string()],
        model: Some("provider/model".to_string()),
        effort: Some("high".to_string()),
    };
    let mut messages = vec![
        json!({"role": "user", "content": "old user"}),
        json!({
            "role": "user",
            "content": "stale skill body",
            "vw_invoked_skill_source": "file:///skills/review/SKILL.md"
        }),
    ];

    let updates = restore_invoked_skill_context(&mut messages, &[replay.clone()]);

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["content"], replay.content);
    assert_eq!(messages[1]["content"], "old user");
    assert_eq!(updates[0].key, "skill.name");
    assert_eq!(updates[1].key, "skill.allowed_tools");
    assert_eq!(updates[2].key, "skill.model");
    assert_eq!(updates[3].key, "skill.effort");
}
