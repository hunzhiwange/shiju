use super::*;
use crate::session::session::{Message, Role, Session};
use serde_json::json;

fn message(role: Role, content: &str) -> Message {
    Message { role, content: content.to_string() }
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

    let mut unchanged = Session {
        id: "unchanged".to_string(),
        messages: vec![message(Role::User, "user")],
    };
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
