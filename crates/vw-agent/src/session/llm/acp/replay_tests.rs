use serde_json::{Value, json};

use super::*;

#[test]
fn build_request_prompt_extracts_latest_user_message_when_not_forcing_replay() {
    let prompt = build_request_prompt(
        &json!([
            { "role": "user", "content": " first request " },
            { "role": "assistant", "content": "answer" },
            { "role": "user", "content": "   " },
            {
                "role": "user",
                "content": [
                    { "type": "text", "text": " latest " },
                    { "type": "text", "text": "request " },
                    { "type": "image", "url": "ignored" }
                ]
            }
        ]),
        false,
        AcpReplayStrategy::Full,
        3,
    );

    assert_eq!(prompt, "latest request");
}

#[test]
fn build_request_prompt_delegates_to_replay_when_forced() {
    let prompt = build_request_prompt(
        &json!([
            { "role": "user", "content": "old" },
            { "role": "user", "content": "new" }
        ]),
        true,
        AcpReplayStrategy::Full,
        3,
    );

    assert!(prompt.contains("newly created ACP session"));
    assert!(prompt.contains("<conversation_history>"));
    assert!(prompt.contains("<current_user_request>\nnew\n</current_user_request>"));
}

#[test]
fn extract_prompt_returns_empty_for_malformed_or_missing_user_content() {
    assert_eq!(extract_prompt(&Value::Null), "");
    assert_eq!(
        extract_prompt(&json!([
            { "content": "missing role" },
            { "role": "assistant", "content": "answer" },
            { "role": "user" },
            { "role": "user", "content": 42 },
            {
                "role": "user",
                "content": [{ "type": "image", "url": "ignored" }, { "text": null }]
            }
        ])),
        ""
    );
}

#[test]
fn extract_prompt_skips_blank_latest_user_and_returns_previous_request() {
    assert_eq!(
        extract_prompt(&json!([
            { "role": "user", "content": " previous request " },
            { "role": "assistant", "content": "answer" },
            { "role": "user", "content": " \n\t " },
            { "role": "user", "content": [{ "text": "   " }] }
        ])),
        "previous request"
    );
}

#[test]
fn content_to_text_trims_strings_and_joins_text_blocks() {
    assert_eq!(content_to_text(&json!("  hello  ")), "hello");
    assert_eq!(
        content_to_text(&json!([
            { "text": " first " },
            { "text": "second" },
            { "type": "image", "url": "ignored" }
        ])),
        "first\nsecond"
    );
    assert_eq!(content_to_text(&json!({ "text": "ignored object" })), "");
    assert_eq!(
        content_to_text(&json!([
            { "text": "   " },
            { "text": " keep " },
            { "text": null },
            { "text": " tail " }
        ])),
        "keep\ntail"
    );
}

#[test]
fn preview_line_compacts_whitespace_and_truncates_on_char_boundaries() {
    assert_eq!(preview_line(" alpha\n\t beta  gamma ", 100), "alpha beta gamma");
    assert_eq!(preview_line("你好 世界 再见", 4), "你好 世...");
    assert_eq!(preview_line("short", 5), "short");
    assert_eq!(preview_line("nonempty", 0), "...");
    assert_eq!(preview_line("", 0), "");
}

#[test]
fn render_helpers_build_markdown_bullets_and_recent_digest() {
    assert_eq!(render_bullets(&[]), "");
    assert_eq!(render_bullets(&["one".to_string(), "two".to_string()]), "- one\n- two");

    let digest = render_turn_digest(
        &[
            ("user".to_string(), "first".to_string()),
            ("assistant".to_string(), "second".to_string()),
            ("user".to_string(), "third".to_string()),
        ],
        2,
    );

    assert_eq!(digest, "Turn 1 | assistant | second\nTurn 2 | user | third");
    assert_eq!(render_turn_digest(&[("user".to_string(), "ignored".to_string())], 0), "");
}

#[test]
fn parse_replay_strategy_is_case_insensitive_and_defaults_to_discard() {
    assert_eq!(parse_replay_strategy(&json!({})), AcpReplayStrategy::Discard);
    assert_eq!(
        parse_replay_strategy(&json!({ "acp_history_strategy": "discard" })),
        AcpReplayStrategy::Discard
    );
    assert_eq!(
        parse_replay_strategy(&json!({ "acp_history_strategy": " FULL " })),
        AcpReplayStrategy::Full
    );
    assert_eq!(
        parse_replay_strategy(&json!({ "acp_history_strategy": "recent" })),
        AcpReplayStrategy::Recent
    );
    assert_eq!(
        parse_replay_strategy(&json!({ "acp_history_strategy": "summary" })),
        AcpReplayStrategy::Summary
    );
    assert_eq!(
        parse_replay_strategy(&json!({ "acp_history_strategy": "unknown" })),
        AcpReplayStrategy::Discard
    );
    assert_eq!(
        parse_replay_strategy(&json!({ "acp_history_strategy": 1 })),
        AcpReplayStrategy::Discard
    );
}

#[test]
fn parse_recent_count_clamps_to_supported_range() {
    assert_eq!(parse_recent_count(&json!({})), 3);
    assert_eq!(parse_recent_count(&json!({ "acp_history_recent_count": 0 })), 1);
    assert_eq!(parse_recent_count(&json!({ "acp_history_recent_count": 7 })), 7);
    assert_eq!(parse_recent_count(&json!({ "acp_history_recent_count": 99 })), 20);
    assert_eq!(parse_recent_count(&json!({ "acp_history_recent_count": "many" })), 3);
    assert_eq!(parse_recent_count(&json!({ "acp_history_recent_count": -1 })), 3);
    assert_eq!(parse_recent_count(&json!({ "acp_history_recent_count": 2.5 })), 3);
}

#[test]
fn build_replay_prompt_for_non_array_falls_back_to_latest_prompt_extraction() {
    assert_eq!(build_replay_prompt(&Value::Null, AcpReplayStrategy::Full, 3), "");
}

#[test]
fn build_replay_prompt_without_latest_user_returns_extract_prompt_fallback() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "system", "content": "rules" },
            { "role": "assistant", "content": "answer" }
        ]),
        AcpReplayStrategy::Full,
        3,
    );

    assert_eq!(prompt, "");
}

#[test]
fn full_replay_prompt_includes_system_and_old_history_but_not_current_twice() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "system", "content": "Follow repo rules." },
            { "role": "user", "content": "first request" },
            { "role": "assistant", "content": "first answer" },
            { "role": "user", "content": "current request" }
        ]),
        AcpReplayStrategy::Full,
        3,
    );

    assert!(prompt.contains("<system>\nFollow repo rules.\n</system>"));
    assert!(prompt.contains("<conversation_history>"));
    assert!(prompt.contains("[user]\nfirst request"));
    assert!(prompt.contains("[assistant]\nfirst answer"));
    assert!(prompt.contains("<current_user_request>\ncurrent request\n</current_user_request>"));
    assert_eq!(prompt.matches("current request").count(), 1);
}

#[test]
fn full_replay_prompt_preserves_system_order_and_ignores_attachment_only_blocks() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "system", "content": "first constraint" },
            {
                "role": "system",
                "content": [
                    { "type": "image", "url": "ignored" },
                    { "type": "text", "text": "second constraint" }
                ]
            },
            { "role": "user", "content": "current" }
        ]),
        AcpReplayStrategy::Full,
        3,
    );

    assert!(prompt.contains("<system>\nfirst constraint\n\nsecond constraint\n</system>"));
    assert!(!prompt.contains("ignored"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
}

#[test]
fn full_replay_prompt_with_only_current_request_omits_optional_sections() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "system", "content": "   " },
            { "role": "assistant", "content": { "attachment": "ignored" } },
            { "role": "user", "content": "current" }
        ]),
        AcpReplayStrategy::Full,
        3,
    );

    assert!(prompt.contains("newly created ACP session"));
    assert!(!prompt.contains("<system>"));
    assert!(!prompt.contains("<conversation_history>"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
}

#[test]
fn discard_replay_prompt_keeps_only_system_and_current_request() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "system", "content": "rules" },
            { "role": "user", "content": "old" },
            { "role": "assistant", "content": "answer" },
            { "role": "user", "content": "current" }
        ]),
        AcpReplayStrategy::Discard,
        3,
    );

    assert!(prompt.contains("<system>\nrules\n</system>"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
    assert!(!prompt.contains("old"));
    assert!(!prompt.contains("<recent_messages>"));
    assert!(!prompt.contains("<conversation_history>"));
}

#[test]
fn recent_replay_prompt_includes_only_the_requested_recent_history() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "user", "content": "oldest" },
            { "role": "assistant", "content": "middle" },
            { "role": "user", "content": "newest old" },
            { "role": "user", "content": "current" }
        ]),
        AcpReplayStrategy::Recent,
        2,
    );

    assert!(prompt.contains("<recent_messages>"));
    assert!(!prompt.contains("oldest"));
    assert!(prompt.contains("[assistant]\nmiddle"));
    assert!(prompt.contains("[user]\nnewest old"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
}

#[test]
fn recent_replay_prompt_omits_empty_recent_section() {
    let prompt = build_replay_prompt(
        &json!([{ "role": "user", "content": "current" }]),
        AcpReplayStrategy::Recent,
        2,
    );

    assert!(!prompt.contains("<recent_messages>"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
}

#[test]
fn recent_replay_prompt_with_zero_limit_omits_existing_history() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "user", "content": "old" },
            { "role": "assistant", "content": "answer" },
            { "role": "user", "content": "current" }
        ]),
        AcpReplayStrategy::Recent,
        0,
    );

    assert!(!prompt.contains("<recent_messages>"));
    assert!(!prompt.contains("old"));
    assert!(!prompt.contains("answer"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
}

#[test]
fn summary_replay_prompt_builds_deterministic_snapshot_and_recent_messages() {
    let long_goal = "user goal ".repeat(40);
    let long_answer = "assistant progress ".repeat(40);
    let prompt = build_replay_prompt(
        &json!([
            { "role": "system", "content": "System constraint A" },
            { "role": "system", "content": "System constraint B" },
            { "role": "user", "content": long_goal },
            { "role": "assistant", "content": long_answer },
            { "role": "tool", "content": "tool observation" },
            { "role": "user", "content": "current" }
        ]),
        AcpReplayStrategy::Summary,
        1,
    );

    assert!(prompt.contains("<conversation_summary>"));
    assert!(prompt.contains("Session Snapshot v2"));
    assert!(prompt.contains("Messages in local history: 4"));
    assert!(prompt.contains("<system_constraints>"));
    assert!(prompt.contains("<recent_user_goals>"));
    assert!(prompt.contains("<assistant_progress>"));
    assert!(prompt.contains("<turn_digest>"));
    assert!(prompt.contains("..."));
    assert!(prompt.contains("<recent_messages>"));
    assert!(prompt.contains("[assistant]"));
    assert!(prompt.contains("[tool]\ntool observation"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
}

#[test]
fn summary_replay_prompt_omits_summary_when_there_is_no_old_history() {
    let prompt = build_replay_prompt(
        &json!([{ "role": "user", "content": "current" }]),
        AcpReplayStrategy::Summary,
        3,
    );

    assert!(!prompt.contains("<conversation_summary>"));
    assert!(!prompt.contains("<recent_messages>"));
    assert!(prompt.contains("<current_user_request>\ncurrent\n</current_user_request>"));
}

#[test]
fn summary_replay_prompt_omits_role_specific_sections_for_tool_only_history() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "tool", "content": "observation" },
            { "role": "user", "content": "current" }
        ]),
        AcpReplayStrategy::Summary,
        3,
    );

    assert!(prompt.contains("<conversation_summary>"));
    assert!(!prompt.contains("<system_constraints>"));
    assert!(!prompt.contains("<recent_user_goals>"));
    assert!(!prompt.contains("<assistant_progress>"));
    assert!(prompt.contains("Turn 1 | tool | observation"));
    assert!(prompt.contains("[tool]\nobservation"));
}

#[test]
fn summary_replay_prompt_caps_snapshot_lists_and_digest() {
    let mut messages = (1..=5)
        .map(|index| json!({ "role": "system", "content": format!("constraint {index}") }))
        .collect::<Vec<_>>();
    messages
        .extend((1..=6).map(|index| json!({ "role": "user", "content": format!("goal {index}") })));
    messages.extend(
        (1..=6).map(|index| json!({ "role": "assistant", "content": format!("progress {index}") })),
    );
    messages.push(json!({ "role": "user", "content": "current" }));

    let prompt = build_replay_prompt(&Value::Array(messages), AcpReplayStrategy::Summary, 0);

    assert!(prompt.contains("Messages in local history: 13"));
    for index in 1..=4 {
        assert!(prompt.contains(&format!("- constraint {index}")));
    }
    assert!(!prompt.contains("- constraint 5"));
    assert!(!prompt.contains("- goal 1"));
    assert!(prompt.contains("- goal 2"));
    assert!(prompt.contains("- goal 6"));
    assert!(!prompt.contains("- progress 1"));
    assert!(prompt.contains("- progress 2"));
    assert!(prompt.contains("- progress 6"));
    assert!(prompt.contains("Turn 1 | user | goal 5"));
    assert!(prompt.contains("Turn 8 | assistant | progress 6"));
}
