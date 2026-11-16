use acp_types::{AcpSessionOptions, DEFAULT_EVENT_MAX_SEGMENTS};
use message_types as message;
use super::*;
fn assistant_message_text(error: Error) -> String {
    match error {
        Error::Api(message::AssistantError::Unknown { message }) => message,
        other => panic!("expected unknown assistant error, got {other:?}"),
    }
}

#[test]
fn clone_non_empty_tools_trims_entries_and_drops_empty_values() {
    assert_eq!(
        clone_non_empty_tools(&[" shell ".to_string(), "".to_string(), " edit".to_string()]),
        Some(vec!["shell".to_string(), "edit".to_string()])
    );
    assert_eq!(clone_non_empty_tools(&[" ".to_string(), "\t".to_string()]), None);
}

#[test]
fn clone_non_empty_tools_preserves_order_and_duplicates() {
    assert_eq!(
        clone_non_empty_tools(&[" edit ".to_string(), "shell".to_string(), "edit".to_string(),]),
        Some(vec!["edit".to_string(), "shell".to_string(), "edit".to_string()])
    );
}

#[test]
fn normalize_session_name_rejects_blank_names() {
    assert_eq!(normalize_session_name(Some("  worktree  ")).as_deref(), Some("worktree"));
    assert_eq!(normalize_session_name(Some("   ")), None);
    assert_eq!(normalize_session_name(None), None);
}

#[test]
fn session_event_log_for_record_uses_bounded_defaults() {
    let log = session_event_log_for_record("session-1");

    assert_eq!(log.segment_count, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(log.max_segments, DEFAULT_EVENT_MAX_SEGMENTS);
    assert!(log.max_segment_bytes > 0);
}

#[test]
fn session_state_options_omits_empty_state_and_filters_allowed_tools() {
    assert_eq!(session_state_options(None), None);
    assert_eq!(session_state_options(Some(&AcpSessionOptions::default())), None);

    let state = session_state_options(Some(&AcpSessionOptions {
        model: Some("gpt-5".to_string()),
        allowed_tools: Some(vec![" shell ".to_string(), " ".to_string(), "edit".to_string()]),
        max_turns: Some(4),
    }))
    .expect("non-empty options should produce state");

    assert_eq!(state.model.as_deref(), Some("gpt-5"));
    assert_eq!(state.allowed_tools, Some(vec!["shell".to_string(), "edit".to_string()]));
    assert_eq!(state.max_turns, Some(4));
}

#[test]
fn session_state_options_keeps_model_or_turns_without_tools() {
    let model_only = session_state_options(Some(&AcpSessionOptions {
        model: Some("gpt-5-mini".to_string()),
        allowed_tools: None,
        max_turns: None,
    }))
    .expect("model should be state");
    assert_eq!(model_only.model.as_deref(), Some("gpt-5-mini"));
    assert_eq!(model_only.allowed_tools, None);

    let turns_only = session_state_options(Some(&AcpSessionOptions {
        model: None,
        allowed_tools: Some(vec![" ".to_string()]),
        max_turns: Some(2),
    }))
    .expect("turns should be state even when tools are blank");
    assert_eq!(turns_only.allowed_tools, None);
    assert_eq!(turns_only.max_turns, Some(2));
}

#[test]
// 显式空工具集与未提供工具状态语义不同，序列化时必须保留。
fn session_state_options_keeps_explicit_empty_tool_state() {
    let state = session_state_options(Some(&AcpSessionOptions {
        model: None,
        allowed_tools: Some(Vec::new()),
        max_turns: None,
    }))
    .expect("an explicitly supplied tool option creates session state");

    assert_eq!(state.model, None);
    assert_eq!(state.allowed_tools, None);
    assert_eq!(state.max_turns, None);
}

#[test]
fn missing_session_error_includes_search_boundary_and_create_command() {
    let named =
        assistant_message_text(missing_session_error("/repo", "claude-code", Some("session-a")));
    assert!(named.contains("searched up to /repo"));
    assert!(named.contains("vwacp claude-code sessions new --name session-a"));

    let unnamed = assistant_message_text(missing_session_error("/repo", "claude-code", None));
    assert!(unnamed.contains("vwacp claude-code sessions new"));
    assert!(!unnamed.contains("--name"));
}

#[test]
fn missing_session_error_treats_blank_name_as_unnamed() {
    let message =
        assistant_message_text(missing_session_error("/repo", "claude-code", Some(" \n\t ")));

    assert!(message.contains("vwacp claude-code sessions new"));
    assert!(!message.contains("--name"));
}
