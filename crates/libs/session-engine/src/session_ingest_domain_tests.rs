use crate::session_memory::{Role, Session};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

// 每个用例使用独立会话键，避免全局会话状态让去重断言相互污染。
fn unique_session(prefix: &str) -> String {
    format!("{prefix}-{}-{}", std::process::id(), NEXT_SESSION.fetch_add(1, Ordering::Relaxed))
}

#[test]
fn push_user_dedup_skips_empty_and_duplicate_trailing_user_message() {
    let mut session = Session::new(unique_session("ingest-dedup"));

    super::push_user_dedup(&mut session, "   ".to_string());
    assert!(session.messages.is_empty());

    super::push_user_dedup(&mut session, "hello".to_string());
    super::push_user_dedup(&mut session, " hello ".to_string());

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "hello");
}

#[test]
fn push_user_dedup_allows_same_text_after_non_user_message() {
    let mut session = Session::new(unique_session("ingest-dedup-after-tool"));

    super::push_user_dedup(&mut session, "hello".to_string());
    session.push(Role::Tool, "tool output".to_string());
    super::push_user_dedup(&mut session, "hello".to_string());

    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[2].role, Role::User);
}

#[test]
fn push_user_dedup_preserves_original_nonempty_content() {
    let mut session = Session::new(unique_session("ingest-preserve-user-content"));

    super::push_user_dedup(&mut session, "  hello\n".to_string());

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "  hello\n");
}
