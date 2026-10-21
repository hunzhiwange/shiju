use std::time::Instant;

use crate::app::message::chat::stream::delta_scheduler::{
    DeltaScheduler, ScheduledDelta, StreamDelta, StreamItemKey, TerminalEvent,
};
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::session::chat_timeline::{
    ChatEntryRevision, ChatTimelineEntry, PagedChatTimeline, TimelinePage,
};
use crate::app::session::stream_render::StreamRenderState;

use super::message_view::{build_render_cache_entry, hash_chat_content};
use super::performance::{
    ChatPerformanceSnapshot, record_chat_message_clone, record_full_session_save,
    record_mounted_rows, reset, snapshot,
};
use super::virtualizer::ChatVirtualizer;

const LONG_SESSION_MESSAGES: usize = 10_000;
const LARGE_ASSISTANT_BYTES: usize = 512 * 1024;
const STREAM_DELTAS: usize = 1_000;

fn message(role: ChatRole, content: String) -> ChatMessage {
    ChatMessage { role, content, model: None, think_timing: Vec::new(), processing_timing: None }
}

fn long_session_fixture() -> Vec<ChatMessage> {
    (0..LONG_SESSION_MESSAGES)
        .map(|index| {
            let role = if index % 2 == 0 { ChatRole::User } else { ChatRole::Assistant };
            message(role, format!("message-{index}"))
        })
        .collect()
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
        crate::app::session::chat_timeline::PageMergeOutcome::Applied { inserted: 64 }
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

#[test]
fn chat_performance_contract_reports_all_hot_path_counters() {
    reset();

    let messages = long_session_fixture();
    let expected_bytes = messages.iter().map(|message| message.content.len()).sum::<usize>();
    let _prepared = crate::app::session::prepare_session_open(
        "long-session".to_string(),
        messages,
        vec![None; LONG_SESSION_MESSAGES],
        Default::default(),
        Vec::new(),
        Vec::new(),
        1,
        None,
        false,
    );

    let mut virtualizer =
        ChatVirtualizer::from_estimated_heights(&vec![40.0; LONG_SESSION_MESSAGES]);
    virtualizer.set_viewport_height(720.0);
    virtualizer.apply_measured_height(0, 80.0);
    let window = virtualizer.window();
    record_mounted_rows(window.render_end_idx - window.render_start_idx);

    let large_assistant = message(ChatRole::Assistant, "x".repeat(LARGE_ASSISTANT_BYTES));
    record_chat_message_clone(large_assistant.content.len());
    let _cloned = large_assistant.clone();
    let _cache = build_render_cache_entry(
        &large_assistant.content,
        &large_assistant.content,
        hash_chat_content(&large_assistant.content),
        false,
    );
    record_full_session_save();

    let actual = snapshot();
    assert_eq!(actual.initial_load_messages, LONG_SESSION_MESSAGES);
    assert_eq!(actual.initial_load_bytes, expected_bytes);
    assert_eq!(actual.chat_message_clone_bytes, LARGE_ASSISTANT_BYTES);
    assert!(actual.full_content_hashes > 0);
    assert!(actual.full_content_hash_bytes >= LARGE_ASSISTANT_BYTES);
    assert!(actual.full_content_parses > 0);
    assert!(actual.full_content_parse_bytes >= LARGE_ASSISTANT_BYTES);
    assert_eq!(actual.height_index_rebuilds, 1);
    assert_eq!(actual.height_point_updates, 1);
    assert!(actual.mounted_rows > 0 && actual.mounted_rows < LONG_SESSION_MESSAGES);
    assert_eq!(actual.full_session_saves, 1);

    reset();
    assert_eq!(snapshot(), ChatPerformanceSnapshot::default());
}

#[test]
#[ignore = "manual long-session workload probe"]
fn chat_performance_probe_long_session() {
    reset();
    let started = Instant::now();
    let messages = long_session_fixture();
    let fixture_elapsed = started.elapsed();

    let load_started = Instant::now();
    let _prepared = crate::app::session::prepare_session_open(
        "long-session-probe".to_string(),
        messages,
        vec![None; LONG_SESSION_MESSAGES],
        Default::default(),
        Vec::new(),
        Vec::new(),
        1,
        None,
        false,
    );
    let load_elapsed = load_started.elapsed();

    let mut stream_chat = Vec::new();
    let delta = "x".repeat(LARGE_ASSISTANT_BYTES / STREAM_DELTAS);
    let stream_started = Instant::now();
    for _ in 0..STREAM_DELTAS {
        let index = crate::app::message::chat::stream::append_stream_delta_for_performance_probe(
            &mut stream_chat,
            delta.clone(),
        );
        let assistant = &stream_chat[index];
        record_chat_message_clone(assistant.content.len());
        let cloned = assistant.clone();
        let _cache = build_render_cache_entry(
            &cloned.content,
            &cloned.content,
            hash_chat_content(&cloned.content),
            false,
        );
    }
    let stream_elapsed = stream_started.elapsed();

    let mut virtualizer = ChatVirtualizer::default();
    let height_started = Instant::now();
    for index in 0..STREAM_DELTAS {
        virtualizer.sync_len(LONG_SESSION_MESSAGES, 40.0);
        virtualizer.apply_measured_height(index, 41.0);
    }
    virtualizer.set_viewport_height(720.0);
    let window = virtualizer.window();
    record_mounted_rows(window.render_end_idx - window.render_start_idx);
    let height_elapsed = height_started.elapsed();
    record_full_session_save();

    let before = snapshot();

    reset();
    let optimized_load_started = Instant::now();
    let bounded_messages = (LONG_SESSION_MESSAGES - 64..LONG_SESSION_MESSAGES)
        .map(|index| message(ChatRole::Assistant, format!("message-{index}")))
        .collect();
    let _prepared = crate::app::session::prepare_session_open(
        "long-session-probe-optimized".to_string(),
        bounded_messages,
        vec![None; 64],
        Default::default(),
        Vec::new(),
        Vec::new(),
        1,
        None,
        false,
    );
    let optimized_load_elapsed = optimized_load_started.elapsed();

    let mut optimized_content = String::with_capacity(LARGE_ASSISTANT_BYTES);
    let mut optimized_render = StreamRenderState::default();
    optimized_render.begin("long-session-probe-optimized", 1, 64, "");
    let optimized_stream_started = Instant::now();
    for index in 0..STREAM_DELTAS {
        let len = LARGE_ASSISTANT_BYTES / STREAM_DELTAS
            + usize::from(index < LARGE_ASSISTANT_BYTES % STREAM_DELTAS);
        let delta = "x".repeat(len);
        optimized_content.push_str(&delta);
        assert!(optimized_render.append_delta("long-session-probe-optimized", 1, 64, &delta,));
    }
    assert!(optimized_render.complete("long-session-probe-optimized", 1, &optimized_content,));
    let optimized_stream_elapsed = optimized_stream_started.elapsed();
    let after = snapshot();

    eprintln!(
        "chat_performance_probe before_fixture={fixture_elapsed:?} before_load={load_elapsed:?} before_stream={stream_elapsed:?} before_height={height_elapsed:?} before={before:?} after_load={optimized_load_elapsed:?} after_stream={optimized_stream_elapsed:?} after={after:?}"
    );
}
