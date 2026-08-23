//! 会话视图缓存测试，覆盖快照移动、容量上限、最近最少使用淘汰和打开路径追踪。
//! 诊断事件只记录路径类别与原因，不得携带聊天正文等会话内容。

use super::view_cache::{
    SessionOpenUiCommit, SessionViewCache, SessionViewCacheEvictionReason,
    SessionViewCacheInsertResult, SessionViewSnapshot,
};
use crate::app::App;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::state::ChatPanelMode;

fn snapshot_with_message(content: String) -> SessionViewSnapshot {
    let (mut app, _task) = App::new();
    app.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    SessionViewSnapshot::take_from(&mut app)
}

#[test]
fn session_view_cache_moves_snapshot_without_reallocating_chat() {
    let content = "moved chat message".to_owned();
    let expected_pointer = content.as_ptr();
    let snapshot = snapshot_with_message(content);

    assert_eq!(snapshot.chat()[0].content.as_ptr(), expected_pointer);
}

#[test]
fn session_view_snapshot_keeps_chat_panel_mode_independent() {
    let (mut app, _task) = App::new();
    app.chat_panel_mode = ChatPanelMode::Trajectory;

    let snapshot = SessionViewSnapshot::take_from(&mut app);

    assert_eq!(app.chat_panel_mode, ChatPanelMode::Conversation);
    snapshot.restore_into(&mut app);
    assert_eq!(app.chat_panel_mode, ChatPanelMode::Trajectory);
}

#[test]
fn session_view_cache_evicts_lru_after_three_entries() {
    let mut cache = SessionViewCache::new();
    for session_id in ["a", "b", "c"] {
        let result =
            cache.insert(session_id.to_owned(), snapshot_with_message(session_id.to_owned()));
        assert!(matches!(result, SessionViewCacheInsertResult::Inserted { .. }));
    }
    assert!(cache.get("a").is_some());

    let result = cache.insert("d".to_owned(), snapshot_with_message("d".to_owned()));
    assert!(matches!(
        result,
        SessionViewCacheInsertResult::Inserted { ref evicted_session_ids }
            if evicted_session_ids == &["b"]
    ));
    assert!(cache.contains("a"));
    assert!(!cache.contains("b"));
    assert!(cache.contains("c"));
    assert!(cache.contains("d"));
}

#[test]
fn session_view_cache_rejects_snapshot_over_32_mib() {
    let mut cache = SessionViewCache::new();
    let retained = snapshot_with_message("retained".to_owned());
    assert!(matches!(
        cache.insert("retained".to_owned(), retained),
        SessionViewCacheInsertResult::Inserted { .. }
    ));

    let oversized = snapshot_with_message("x".repeat(32 * 1024 * 1024 + 1));
    let result = cache.insert("oversized".to_owned(), oversized);
    let SessionViewCacheInsertResult::RejectedOverCapacity { snapshot, estimated_bytes } = result
    else {
        panic!("oversized snapshot should be rejected");
    };

    assert!(estimated_bytes > 32 * 1024 * 1024);
    assert_eq!(snapshot.chat()[0].content.len(), 32 * 1024 * 1024 + 1);
    assert_eq!(cache.len(), 1);
    assert!(cache.contains("retained"));
    assert!(!cache.contains("oversized"));
}

#[test]
fn session_open_trace_distinguishes_hot_and_cold_paths() {
    let mut cache = SessionViewCache::new();
    let hot_generation = cache.begin_open("hot");
    let hot = cache
        .record_open_commit(
            "hot",
            hot_generation,
            SessionOpenUiCommit::HotRestore,
            12,
            web_time::Duration::ZERO,
            true,
        )
        .expect("hot restore should complete its trace");
    let cold_generation = cache.begin_open("cold");
    let cold = cache
        .record_open_commit(
            "cold",
            cold_generation,
            SessionOpenUiCommit::PreparedHistory,
            12,
            web_time::Duration::ZERO,
            true,
        )
        .expect("cold commit should complete its trace");

    assert_eq!(hot.source(), "view_cache");
    assert!(hot.cache_hit());
    assert_eq!(cold.source(), "remote_history");
    assert!(!cold.cache_hit());
}

#[test]
fn session_open_emits_single_ui_commit_summary() {
    let mut cache = SessionViewCache::new();
    let generation = cache.begin_open("session");
    assert!(
        cache
            .record_open_commit(
                "session",
                generation,
                SessionOpenUiCommit::PreparedHistory,
                4,
                web_time::Duration::ZERO,
                false,
            )
            .is_none()
    );

    let summary = cache
        .record_open_commit(
            "session",
            generation,
            SessionOpenUiCommit::SameContentRefresh,
            4,
            web_time::Duration::ZERO,
            true,
        )
        .expect("remote refresh should finalize the single summary");

    assert_eq!(summary.source(), "ui_store_then_remote_history");
    assert_eq!(summary.ui_commit_count(), 1);
}

#[test]
fn view_cache_eviction_trace_contains_reason_without_content() {
    let mut cache = SessionViewCache::new();
    for session_id in ["a", "b", "c"] {
        let _ = cache.insert(
            session_id.to_owned(),
            snapshot_with_message(format!("private message for {session_id}")),
        );
    }
    let _ =
        cache.insert("d".to_owned(), snapshot_with_message("another private message".to_owned()));
    let generation = cache.begin_open("opened");
    let summary = cache
        .record_open_commit(
            "opened",
            generation,
            SessionOpenUiCommit::HotRestore,
            1,
            web_time::Duration::ZERO,
            true,
        )
        .expect("hot restore should emit a summary");

    assert_eq!(summary.eviction_reason(), SessionViewCacheEvictionReason::EntryLimit);
    assert!(!format!("{summary:?}").contains("private message"));
}
