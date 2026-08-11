use super::*;
use crate::session::session::{Message, Role, Session};
use serde_json::json;

fn message(role: Role, content: &str) -> Message {
    Message { role, content: content.to_string() }
}

fn session_with_roles() -> Session {
    Session {
        id: "session-1".to_string(),
        messages: vec![
            message(Role::System, "system prompt"),
            message(Role::User, "user turn"),
            message(Role::Assistant, "assistant turn"),
            message(Role::Tool, "tool result"),
        ],
    }
}

#[test]
fn session_message_to_llm_message_maps_all_roles() {
    assert_eq!(
        session_message_to_llm_message(&message(Role::User, "hello")),
        Some(json!({"role": "user", "content": "hello"}))
    );
    assert_eq!(
        session_message_to_llm_message(&message(Role::Assistant, "hi")),
        Some(json!({"role": "assistant", "content": "hi"}))
    );
    assert_eq!(
        session_message_to_llm_message(&message(Role::System, "rules")),
        Some(json!({"role": "system", "content": "rules"}))
    );
    assert_eq!(
        session_message_to_llm_message(&message(Role::Tool, "tool output")),
        Some(json!({"role": "assistant", "content": "tool output"}))
    );
}

#[test]
fn session_messages_to_llm_messages_preserves_order_and_content() {
    let converted = session_messages_to_llm_messages(&session_with_roles());

    assert_eq!(
        converted,
        vec![
            json!({"role": "system", "content": "system prompt"}),
            json!({"role": "user", "content": "user turn"}),
            json!({"role": "assistant", "content": "assistant turn"}),
            json!({"role": "assistant", "content": "tool result"}),
        ]
    );
}

#[test]
fn extend_llm_messages_from_session_range_appends_tail_only() {
    let session = session_with_roles();
    let mut llm_messages = vec![json!({"role": "system", "content": "existing"})];

    extend_llm_messages_from_session_range(&mut llm_messages, &session, 2);

    assert_eq!(
        llm_messages,
        vec![
            json!({"role": "system", "content": "existing"}),
            json!({"role": "assistant", "content": "assistant turn"}),
            json!({"role": "assistant", "content": "tool result"}),
        ]
    );
}

#[test]
fn extend_llm_messages_from_session_range_allows_start_past_end() {
    let session = session_with_roles();
    let mut llm_messages = vec![json!({"role": "user", "content": "existing"})];

    extend_llm_messages_from_session_range(&mut llm_messages, &session, usize::MAX);

    assert_eq!(llm_messages, vec![json!({"role": "user", "content": "existing"})]);
}

#[test]
fn session_protocol_injects_agents_and_skills_on_every_desktop_step() {
    let instructions = vec![crate::session::instruction::Resolved {
        filepath: "/workspace/AGENTS.md".to_string(),
        content: "Instructions from: /workspace/AGENTS.md\nFollow the protocol.".to_string(),
        sources: Vec::new(),
        diagnostics: Vec::new(),
    }];
    let history = vec![json!({"role": "user", "content": "real history"})];
    let invoked = vec![
        json!({"role": "user", "content": "<system-reminder>Skill is active.</system-reminder>"}),
    ];
    let listing = Some(
        json!({"role": "user", "content": "<system-reminder>Available skills: rustcodegraph</system-reminder>"}),
    );

    let first = super::assemble_session_protocol_messages(
        &history,
        &instructions,
        &invoked,
        listing.clone(),
    );
    let next =
        super::assemble_session_protocol_messages(&history, &instructions, &invoked, listing);

    for messages in [&first, &next] {
        assert!(
            messages[0]["content"]
                .as_str()
                .is_some_and(|content| content.contains("Follow the protocol."))
        );
        assert_eq!(messages[1], invoked[0]);
        assert_eq!(
            messages[2]["content"],
            "<system-reminder>Available skills: rustcodegraph</system-reminder>"
        );
        assert_eq!(messages[3], history[0]);
    }
}
