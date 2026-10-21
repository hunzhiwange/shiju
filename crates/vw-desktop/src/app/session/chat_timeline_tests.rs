use super::ChatRole;
use super::chat_timeline::{
    ChatEntryKey, ChatEntryRevision, ChatTimelineEntry, PageMergeOutcome, PagedChatTimeline,
    TimelinePage,
};
use crate::app::models::ChatMessage;

fn message(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::Assistant,
        content: content.to_owned(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn persisted(session_id: &str, seq_no: u64, id: Option<&str>) -> ChatTimelineEntry {
    ChatTimelineEntry::persisted(
        session_id,
        seq_no,
        id.map(str::to_owned),
        ChatEntryRevision::INITIAL,
        message(&format!("message-{seq_no}")),
    )
}

fn page(
    entries: Vec<ChatTimelineEntry>,
    before_cursor: Option<&str>,
    after_cursor: Option<&str>,
    total_count: usize,
) -> TimelinePage {
    TimelinePage {
        entries,
        before_cursor: before_cursor.map(str::to_owned),
        after_cursor: after_cursor.map(str::to_owned),
        total_count,
    }
}

#[test]
fn chat_entry_keys_prefer_id_and_fallback_to_session_sequence() {
    let cases = [
        (
            persisted("session-a", 8, Some("message-8")),
            ChatEntryKey::PersistedId("message-8".to_owned()),
        ),
        (
            persisted("session-a", 9, None),
            ChatEntryKey::PersistedSeq { session_id: "session-a".to_owned(), seq_no: 9 },
        ),
    ];

    for (entry, expected) in cases {
        assert_eq!(entry.key, expected);
    }
}

#[test]
fn paged_chat_timeline_prepends_without_changing_keys() {
    let mut timeline = PagedChatTimeline::new("session-a", 7);
    assert_eq!(
        timeline.merge_initial_page(page(
            vec![persisted("session-a", 3, Some("m3")), persisted("session-a", 4, None)],
            Some("before-3"),
            None,
            4,
        )),
        PageMergeOutcome::Applied { inserted: 2 }
    );
    let original = timeline.entries()[0].clone();

    assert!(timeline.begin_prepend("before-3"));
    assert_eq!(timeline.in_flight_before(), Some("before-3"));
    assert_eq!(
        timeline.merge_prepend_page(
            "session-a",
            7,
            "before-3",
            page(
                vec![persisted("session-a", 1, Some("m1")), persisted("session-a", 2, None)],
                None,
                Some("after-2"),
                4,
            ),
        ),
        PageMergeOutcome::Applied { inserted: 2 }
    );

    assert_eq!(timeline.loaded_seq_range(), Some((1, 4)));
    assert_eq!(timeline.total_count(), 4);
    assert_eq!(timeline.before_cursor(), None);
    assert_eq!(timeline.after_cursor(), Some("after-2"));
    assert_eq!(timeline.entries()[2].key, original.key);
    assert_eq!(timeline.entries()[2].revision, original.revision);
}

#[test]
fn paged_chat_timeline_merges_duplicate_page_idempotently() {
    let mut timeline = PagedChatTimeline::new("session-a", 1);
    let initial = page(
        vec![persisted("session-a", 2, Some("m2")), persisted("session-a", 3, None)],
        Some("before-2"),
        None,
        3,
    );
    timeline.merge_initial_page(initial.clone());
    assert_eq!(timeline.merge_initial_page(initial), PageMergeOutcome::NoProgress);
    assert_eq!(timeline.entries().len(), 2);
}

#[test]
fn paged_chat_timeline_stops_repeated_cursor() {
    let mut timeline = PagedChatTimeline::new("session-a", 1);
    timeline.merge_initial_page(page(vec![persisted("session-a", 2, None)], Some("same"), None, 2));
    assert!(timeline.begin_prepend("same"));
    assert_eq!(
        timeline.merge_prepend_page(
            "session-a",
            1,
            "same",
            page(Vec::new(), Some("same"), None, 2),
        ),
        PageMergeOutcome::NoProgress
    );
    assert!(!timeline.begin_prepend("same"));
    assert!(timeline.is_before_exhausted());
}

#[test]
fn paged_chat_timeline_overlays_active_entry_once() {
    let mut timeline = PagedChatTimeline::new("session-a", 3);
    timeline.merge_initial_page(page(vec![persisted("session-a", 1, Some("m1"))], None, None, 1));
    let key = timeline.set_active(9, message("first"));
    assert_eq!(timeline.visible_entries().len(), 2);
    assert_eq!(timeline.set_active(9, message("updated")), key);
    assert_eq!(timeline.visible_entries().len(), 2);
    assert_eq!(timeline.visible_entries()[1].message.content, "updated");

    let completed = timeline.complete_active(2, Some("m2".to_owned())).unwrap();
    assert_eq!(completed.key, ChatEntryKey::PersistedId("m2".to_owned()));
    assert_eq!(timeline.entries().len(), 2);
    assert_eq!(timeline.visible_entries().len(), 2);
    assert!(timeline.active_entry().is_none());
}

#[test]
fn chat_entry_rejects_stale_generation_or_revision() {
    let mut timeline = PagedChatTimeline::new("session-a", 4);
    timeline.merge_initial_page(page(vec![persisted("session-a", 1, Some("m1"))], None, None, 1));
    let entry = &timeline.entries()[0];
    assert!(timeline.accepts_entry_result("session-a", 4, &entry.key, entry.revision));
    assert!(!timeline.accepts_entry_result("session-b", 4, &entry.key, entry.revision));
    assert!(!timeline.accepts_entry_result("session-a", 3, &entry.key, entry.revision));
    assert!(!timeline.accepts_entry_result("session-a", 4, &entry.key, entry.revision.next()));
}

#[test]
fn paged_chat_timeline_rejects_out_of_order_page_response() {
    let mut timeline = PagedChatTimeline::new("session-a", 2);
    timeline.merge_initial_page(page(
        vec![persisted("session-a", 3, None)],
        Some("before-3"),
        None,
        3,
    ));
    assert!(timeline.begin_prepend("before-3"));
    assert_eq!(
        timeline.merge_prepend_page(
            "session-a",
            1,
            "before-3",
            page(vec![persisted("session-a", 2, None)], None, None, 3),
        ),
        PageMergeOutcome::RejectedStale
    );
    assert_eq!(timeline.entries().len(), 1);
}
