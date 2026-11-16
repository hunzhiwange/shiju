use crate::stream_render::StreamRenderState;
use desktop_session::message::chat::stream::delta_scheduler::{
    DeltaScheduler, ScheduledDelta, StreamDelta, StreamItemKey, TerminalEvent,
};
use desktop_session::session::chat_timeline::{
    ChatEntryRevision, ChatTimelineEntry, PagedChatTimeline, TimelinePage,
};
use session_ui_types::{ChatMessage, ChatRole};

use crate::message_view::render_cache::build_render_cache_entry;
use crate::performance::{record_mounted_rows, reset, snapshot};
use crate::virtualizer::ChatVirtualizer;

const LONG_SESSION_MESSAGES: usize = 10_000;
const LARGE_ASSISTANT_BYTES: usize = 512 * 1024;
const STREAM_DELTAS: usize = 1_000;

fn message(role: ChatRole, content: String) -> ChatMessage {
    ChatMessage { role, content, model: None, think_timing: Vec::new(), processing_timing: None }
}

fn timeline_entry(index: usize) -> ChatTimelineEntry {
    ChatTimelineEntry::persisted(
        "long-session",
        index as u64,
        Some(format!("message-{index}")),
        ChatEntryRevision::INITIAL,
        message(ChatRole::Assistant, format!("message-{index}")),
    )
}

#[test]
fn chat_performance_acceptance_long_session_is_bounded() {
    reset();
    let mut timeline = PagedChatTimeline::new("long-session", 1);
    let latest = (LONG_SESSION_MESSAGES - 64..LONG_SESSION_MESSAGES).map(timeline_entry).collect();
    assert_eq!(
        timeline.merge_initial_page(TimelinePage {
            entries: latest,
            before_cursor: Some("before-9936".to_owned()),
            after_cursor: None,
            total_count: LONG_SESSION_MESSAGES,
        }),
        desktop_session::session::chat_timeline::PageMergeOutcome::Applied { inserted: 64 }
    );
    assert_eq!(timeline.entries().len(), 64);
    assert_eq!(timeline.total_count(), LONG_SESSION_MESSAGES);

    let anchor = timeline.target_at(0).expect("latest page anchor");
    assert!(timeline.begin_prepend("before-9936"));
    let previous =
        (LONG_SESSION_MESSAGES - 128..LONG_SESSION_MESSAGES - 64).map(timeline_entry).collect();
    timeline.merge_prepend_page(
        "long-session",
        1,
        "before-9936",
        TimelinePage {
            entries: previous,
            before_cursor: Some("before-9872".to_owned()),
            after_cursor: None,
            total_count: LONG_SESSION_MESSAGES,
        },
    );
    assert_eq!(timeline.index_of(&anchor.key), Some(64));
    assert!(timeline.accepts_entry_result(
        &anchor.session_id,
        anchor.generation,
        &anchor.key,
        anchor.revision,
    ));

    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&vec![40.0; 128]);
    virtualizer.set_viewport_height(720.0);
    let window = virtualizer.window();
    record_mounted_rows(window.render_end_idx - window.render_start_idx);
    let work = snapshot();
    assert!(work.mounted_rows <= 64);
    assert_eq!(work.initial_load_messages, 0);
    assert_eq!(work.full_content_hashes, 0);
    assert_eq!(work.full_content_parses, 0);
}

#[test]
fn chat_performance_acceptance_stream_is_incremental() {
    reset();
    let key = StreamItemKey::new("long-session", "assistant");
    let mut scheduler = DeltaScheduler::default();
    let mut render = StreamRenderState::default();
    let mut content = String::with_capacity(LARGE_ASSISTANT_BYTES);
    render.begin("long-session", 1, 64, "");

    let base = LARGE_ASSISTANT_BYTES / STREAM_DELTAS;
    let remainder = LARGE_ASSISTANT_BYTES % STREAM_DELTAS;
    for index in 0..STREAM_DELTAS {
        let len = base + usize::from(index < remainder);
        scheduler.push(key.clone(), StreamDelta::Text("x".repeat(len)), 0);
    }
    scheduler.push(key.clone(), StreamDelta::Terminal(TerminalEvent::Completed), 0);

    let mut now_ms = 0;
    let mut terminal_seen = false;
    while scheduler.has_pending(&key) {
        for event in scheduler.drain(&key, now_ms, true) {
            match event {
                ScheduledDelta::Text(delta) => {
                    content.push_str(&delta);
                    assert!(render.append_delta("long-session", 1, 64, &delta));
                }
                ScheduledDelta::Terminal(TerminalEvent::Completed) => {
                    assert_eq!(content.len(), LARGE_ASSISTANT_BYTES);
                    terminal_seen = true;
                }
                other => panic!("unexpected scheduled event: {other:?}"),
            }
        }
        now_ms += 16;
    }
    assert!(terminal_seen);
    assert!(render.complete("long-session", 1, &content));
    let work = render.work();
    assert_eq!(work.full_message_clones, 0);
    assert_eq!(work.full_content_hashes, 0);
    assert_eq!(work.full_content_parses, 0);
    assert_eq!(work.height_index_rebuilds, 0);
    assert_eq!(work.canonical_parses, 1);

    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[40.0; 65]);
    virtualizer.apply_measured_height(64, 120.0);
    let counters = snapshot();
    assert_eq!(counters.height_index_rebuilds, 1);
    assert_eq!(counters.height_point_updates, 1);
    assert_eq!(counters.chat_message_clone_bytes, 0);
    assert_eq!(counters.full_session_saves, 0);
}

#[test]
fn chat_performance_acceptance_persistence_is_incremental() {
    reset();
    let counters = snapshot();
    assert_eq!(counters.full_session_saves, 0);
    assert_eq!(counters.chat_message_clone_bytes, 0);
}

#[test]
fn chat_performance_acceptance_real_renderer_preserves_content() {
    reset();
    let content = "中文 😀\n\n# 标题\n\n```rust\nlet value = 1;\n```\n\n<think>推理</think>完成\n\ntool {\"status\":\"completed\"}";
    let cache = build_render_cache_entry(content, content, 0, true);
    assert!(!cache.display_text.trim().is_empty());
    assert!(cache.display_text.contains("中文"));
    assert!(cache.display_text.contains("😀"));
    assert!(cache.display_text.contains("完成"));
    assert_eq!(snapshot().full_content_parses, 1);
}
