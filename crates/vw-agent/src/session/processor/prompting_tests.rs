//! 验证提示拼装的角色顺序、字节预算与 UTF-8 截断边界。

use crate::app::agent::session::session::{Role, Session};

#[test]
fn build_prompt_with_empty_session_contains_only_system_prompt_separator() {
    let session = Session::new("prompting-tests-empty".to_string());
    let system_prompt = crate::session::prompt::system(None, None);

    let prompt = super::build_prompt(&session, None, None, None);

    assert_eq!(prompt, format!("{system_prompt}\n\n"));
}

fn session_with(messages: &[(Role, &str)]) -> Session {
    let mut session = Session::new("prompting-tests".to_string());
    for (role, content) in messages {
        session.push(*role, (*content).to_string());
    }
    session
}

#[test]
fn build_prompt_preserves_history_order_and_role_labels() {
    let session = session_with(&[
        (Role::User, "hello"),
        (Role::Assistant, "hi there"),
        (Role::System, "system note"),
        (Role::Tool, "tool output"),
    ]);

    let prompt = super::build_prompt(&session, Some("test-model"), Some("/tmp/project"), None);

    let user_pos = prompt.find("user: hello\n").expect("user message should be present");
    let assistant_pos =
        prompt.find("assistant: hi there\n").expect("assistant message should be present");
    let system_pos =
        prompt.find("system: system note\n").expect("system message should be present");
    let tool_pos = prompt.find("tool: tool output\n").expect("tool message should be present");

    assert!(user_pos < assistant_pos);
    assert!(assistant_pos < system_pos);
    assert!(system_pos < tool_pos);
}

#[test]
fn build_prompt_appends_trimmed_extra_assistant_text() {
    let session = session_with(&[(Role::User, "question")]);

    let prompt = super::build_prompt(&session, None, None, Some("  partial answer  \n"));

    assert!(prompt.ends_with("assistant: partial answer\n"));
}

#[test]
fn build_prompt_ignores_blank_extra_assistant_text() {
    let session = session_with(&[(Role::User, "question")]);

    let prompt = super::build_prompt(&session, None, None, Some("   \n"));

    assert!(!prompt.ends_with("assistant: \n"));
    assert!(!prompt.contains("assistant:    "));
}

#[test]
// 工具输出按字节预算截断时回退到合法字符边界。
fn build_prompt_truncates_tool_messages_on_utf8_boundary() {
    let repeated = "界".repeat(2_000);
    let session = session_with(&[(Role::Tool, repeated.as_str())]);

    let prompt = super::build_prompt(&session, None, None, None);

    assert!(prompt.contains(&format!("tool: {}\n", "界".repeat(1_024))));
    assert!(!prompt.contains(&"界".repeat(1_025)));
}

#[test]
fn build_prompt_backs_up_when_tool_limit_splits_utf8_character() {
    let content = format!("a{}", "界".repeat(2_000));
    let session = session_with(&[(Role::Tool, content.as_str())]);

    let prompt = super::build_prompt(&session, None, None, None);

    assert!(prompt.contains(&format!("tool: a{}\n", "界".repeat(1_023))));
    assert!(!prompt.contains(&format!("tool: a{}", "界".repeat(1_024))));
}

#[test]
fn build_prompt_keeps_tool_message_at_exact_byte_limit() {
    let content = "x".repeat(3 * 1_024);
    let session = session_with(&[(Role::Tool, content.as_str())]);

    let prompt = super::build_prompt(&session, None, None, None);

    assert!(prompt.ends_with(&format!("tool: {content}\n")));
}

#[test]
fn build_prompt_stops_before_oversized_older_history() {
    let mut session = Session::new("prompting-tests-budget".to_string());
    session.push(Role::User, format!("old {}", "x".repeat(130 * 1024)));
    session.push(Role::Assistant, "new answer".to_string());

    let prompt = super::build_prompt(&session, None, None, None);

    assert!(prompt.contains("assistant: new answer\n"));
    assert!(!prompt.contains("old "));
}

#[test]
// 最新消息自身超限时也不跳过它去拼接更旧历史，保持时间语义。
fn build_prompt_stops_at_oversized_newest_message() {
    let mut session = Session::new("prompting-tests-newest-budget".to_string());
    session.push(Role::User, "older question".to_string());
    session.push(Role::Assistant, "x".repeat(130 * 1_024));
    let empty_session = Session::new("prompting-tests-newest-budget-empty".to_string());
    let baseline = super::build_prompt(&empty_session, None, None, None);

    let prompt = super::build_prompt(&session, None, None, None);

    assert_eq!(prompt, baseline);
}

#[test]
fn build_prompt_accepts_exact_total_budget_and_rejects_one_byte_more() {
    const MAX_PROMPT_BYTES: usize = 120 * 1_024;
    const USER_CHUNK_OVERHEAD: usize = "user: \n".len();

    let empty_session = Session::new("prompting-tests-empty-budget".to_string());
    let baseline = super::build_prompt(&empty_session, None, None, None);
    let exact_content_len = MAX_PROMPT_BYTES
        .checked_sub(baseline.len() + USER_CHUNK_OVERHEAD)
        .expect("system prompt should leave room for one user message");

    let exact_content = "x".repeat(exact_content_len);
    let exact_session = session_with(&[(Role::User, exact_content.as_str())]);
    let exact_prompt = super::build_prompt(&exact_session, None, None, None);
    assert_eq!(exact_prompt.len(), MAX_PROMPT_BYTES);
    assert!(exact_prompt.ends_with(&format!("user: {exact_content}\n")));

    let over_content = "x".repeat(exact_content_len + 1);
    let over_session = session_with(&[(Role::User, over_content.as_str())]);
    let over_prompt = super::build_prompt(&over_session, None, None, None);
    assert_eq!(over_prompt, baseline);
}

#[test]
fn build_prompt_adds_missing_newline_for_message_chunks() {
    let session = session_with(&[(Role::User, "no trailing newline")]);

    let prompt = super::build_prompt(&session, None, None, None);

    assert!(prompt.contains("user: no trailing newline\n"));
}

#[test]
fn build_prompt_keeps_existing_message_newline_without_double_required_suffix() {
    let session = session_with(&[(Role::Assistant, "already has newline\n")]);

    let prompt = super::build_prompt(&session, None, None, None);

    assert!(prompt.contains("assistant: already has newline\n"));
    assert!(!prompt.contains("assistant: already has newline\n\nassistant:"));
}
