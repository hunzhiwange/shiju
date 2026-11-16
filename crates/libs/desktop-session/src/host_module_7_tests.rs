//! Project-session history commits exercise the real session state without desktop resources.
use crate::session::{
    active::ActiveSessionViewState, history_commit::HistoryState, loading::prepare_session_open,
};
use session_ui_types::{ChatMessage, ChatRole, TokenUsage};
fn message(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::User,
        content: content.into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
#[test]
fn prepared_session_open_moves_messages_into_session_state() {
    let mut chat = vec![message("old")];
    let mut ids = Vec::new();
    let mut usage = TokenUsage::default();
    let mut view = ActiveSessionViewState::<()>::default();
    let mut timeline = None;
    let prepared = prepare_session_open(
        "session".into(),
        vec![message("new")],
        vec![Some("message".into())],
        TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        42,
        Some("provider/model".into()),
        |_| (),
    );
    let commit = HistoryState {
        chat: &mut chat,
        ids: &mut ids,
        usage: &mut usage,
        view: &mut view,
        timeline: &mut timeline,
    }
    .install(prepared, 7);
    assert_eq!(chat[0].content, "new");
    assert_eq!(ids, vec![Some("message".into())]);
    assert_eq!(view.updated_ms, 42);
    assert_eq!(commit.session_model.as_deref(), Some("provider/model"));
    assert!(commit.base_ui_chunk.is_some());
    assert!(commit.seq_base.is_none());
}
#[test]
fn cached_history_error_restores_ids_and_keeps_usage() {
    let mut chat = Vec::new();
    let mut ids = Vec::new();
    let mut usage = TokenUsage { input_tokens: 9, ..Default::default() };
    let mut view = ActiveSessionViewState::<()>::default();
    let mut timeline = None;
    let shared = HistoryState {
        chat: &mut chat,
        ids: &mut ids,
        usage: &mut usage,
        view: &mut view,
        timeline: &mut timeline,
    }
    .restore_cached(vec![message("cached")], Some(Vec::new()), |_, _| ());
    assert_eq!(shared[0].content, "cached");
    assert_eq!(chat[0].content, "cached");
    assert_eq!(ids, vec![None]);
    assert_eq!(usage.input_tokens, 9);
    assert!(view.ui_preparing);
    assert!(!view.base_ready);
}
#[test]
fn empty_history_marks_base_ready_without_preparation() {
    let mut chat = Vec::new();
    let mut ids = Vec::new();
    let mut usage = TokenUsage::default();
    let mut view = ActiveSessionViewState::<()>::default();
    let mut timeline = None;
    assert!(
        !HistoryState {
            chat: &mut chat,
            ids: &mut ids,
            usage: &mut usage,
            view: &mut view,
            timeline: &mut timeline
        }
        .prepare_loaded_ui()
    );
    assert!(view.base_ready);
    assert!(!view.ui_preparing);
}

#[test]
fn history_prepend_uses_timeline_generation_and_updates_sequence_base() {
    use crate::session::chat_timeline::{
        ChatEntryRevision, ChatTimelineEntry, PagedChatTimeline, TimelinePage,
    };
    let entry = |seq, id: &str| {
        ChatTimelineEntry::persisted(
            "session",
            seq,
            Some(id.into()),
            ChatEntryRevision::INITIAL,
            message(id),
        )
    };
    let mut timeline = PagedChatTimeline::new("session", 7);
    timeline.merge_initial_page(TimelinePage {
        entries: vec![entry(10, "tail")],
        before_cursor: Some("before".into()),
        after_cursor: None,
        total_count: 2,
    });
    assert!(timeline.begin_prepend("before"));
    let mut timeline = Some(timeline);
    let mut chat = vec![message("tail")];
    let mut ids = vec![Some("tail".into())];
    let mut usage = TokenUsage::default();
    let mut view = ActiveSessionViewState::<()>::default();
    let mut state = HistoryState {
        chat: &mut chat,
        ids: &mut ids,
        usage: &mut usage,
        view: &mut view,
        timeline: &mut timeline,
    };
    let page = TimelinePage {
        entries: vec![entry(9, "head")],
        before_cursor: None,
        after_cursor: None,
        total_count: 2,
    };
    assert!(state.prepend(Some("other"), "session", true, 7, "before", Ok(page.clone())).is_none());
    let commit = state.prepend(Some("session"), "session", true, 7, "before", Ok(page)).unwrap();
    assert_eq!(commit.seq_base, 9);
    assert_eq!(commit.inserted, 1);
    assert_eq!(chat[0].content, "head");
    assert_eq!(ids, vec![Some("head".into()), Some("tail".into())]);
}
