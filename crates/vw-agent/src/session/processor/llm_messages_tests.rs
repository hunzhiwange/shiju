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
        None,
    );
    let next =
        super::assemble_session_protocol_messages(&history, &instructions, &invoked, listing, None);

    for messages in [&first, &next] {
        assert_eq!(messages[0], invoked[0]);
        assert_eq!(
            messages[1]["content"],
            "<system-reminder>Available skills: rustcodegraph</system-reminder>"
        );
        assert!(messages[2]["content"].as_str().is_some_and(|content| {
            content.contains("AGENTS.md loaded for this request")
                && content.contains("Follow the protocol.")
        }));
        assert_eq!(messages[2]["role"], "system");
        assert_eq!(messages[3], history[0]);
    }
}

#[test]
fn assemble_session_protocol_messages_tracks_context_sources() {
    let user_source = json!({
        "kind": "user",
        "rpcId": "desktop-request-42",
        "clientTimeZone": "Asia/Shanghai",
    });
    let mut history = vec![json!({"role": "user", "content": "current question"})];
    super::annotate_current_user_source(&mut history, user_source.clone());
    let listing = Some(super::sourced_protocol_message(
        json!({
            "role": "user",
            "content": "<system-reminder>Available skills: rustcodegraph</system-reminder>"
        }),
        super::context_message_source("skill_catalog", &["rustcodegraph".to_string()]),
    ));

    let messages = super::assemble_session_protocol_messages_with_sources(
        &history,
        &[],
        &[],
        listing,
        None,
    );

    assert_eq!(
        messages.message_sources[0],
        json!({
            "kind": "context",
            "form": "skill_catalog",
            "entries": ["rustcodegraph"],
        })
    );
    assert_eq!(messages.message_sources[1], user_source);
    assert!(messages[0].get("_vw_source").is_none());
    assert!(messages[0].get("source").is_none());
    assert!(messages[1].get("_vw_source").is_none());
    assert!(messages[1].get("source").is_none());
}

#[test]
fn session_protocol_places_agents_immediately_before_current_user_turn() {
    let instructions = vec![crate::session::instruction::Resolved {
        filepath: "/workspace/AGENTS.md".to_string(),
        content: "Instructions from: /workspace/AGENTS.md\nCURRENT_PROTOCOL".to_string(),
        sources: Vec::new(),
        diagnostics: Vec::new(),
    }];
    let history = vec![
        json!({"role": "user", "content": "old question"}),
        json!({"role": "assistant", "content": "old answer"}),
        json!({"role": "user", "content": "current question"}),
    ];

    let messages = super::assemble_session_protocol_messages(
        &history,
        &instructions,
        &[json!({"role": "user", "content": "<system-reminder>Skill active</system-reminder>"})],
        Some(
            json!({"role": "user", "content": "<system-reminder>Skill listing</system-reminder>"}),
        ),
        super::default_tool_reminder(&["file_read".to_string()]),
    );

    assert_eq!(&messages[..2], &history[..2]);
    assert_eq!(messages[2]["content"], "<system-reminder>Skill active</system-reminder>");
    assert_eq!(messages[3]["content"], "<system-reminder>Skill listing</system-reminder>");
    assert!(messages[4]["content"].as_str().is_some_and(|content| {
        content.contains("file_read") && content.contains("not a restriction")
    }));
    assert!(messages[5]["content"].as_str().is_some_and(|content| {
        content.contains("AGENTS.md") && content.contains("CURRENT_PROTOCOL")
    }));
    assert_eq!(messages[5]["role"], "system");
    assert_eq!(messages[6], history[2]);
}

#[test]
fn default_tool_reminder_is_adjacent_to_current_user_turn_and_not_a_whitelist() {
    let history = vec![json!({"role": "user", "content": "current question"})];
    let default_tools = vec!["file_read".to_string(), "shell".to_string()];

    let messages = super::assemble_session_protocol_messages(
        &history,
        &[],
        &[],
        None,
        super::default_tool_reminder(&default_tools),
    );

    assert_eq!(messages.len(), 2);
    assert!(messages[0]["content"].as_str().is_some_and(|content| {
        content.contains("file_read, shell") && content.contains("not a restriction")
    }));
    assert_eq!(messages[1], history[0]);
}
