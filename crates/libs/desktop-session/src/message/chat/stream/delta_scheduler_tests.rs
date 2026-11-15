use super::delta_scheduler::{
    DeltaScheduler, ScheduledDelta, StreamDelta, StreamItemKey, TerminalEvent,
};

const ACTIVE_FRAME_MS: u64 = 16;

fn key() -> StreamItemKey {
    StreamItemKey::new("session-1", "assistant-1")
}

#[test]
fn stream_delta_scheduler_emits_first_delta_immediately() {
    let mut scheduler = DeltaScheduler::default();
    scheduler.push(key(), StreamDelta::Text("你好🙂abc".into()), 100);

    assert_eq!(scheduler.drain(&key(), 100, true), vec![ScheduledDelta::Text("你好🙂abc".into())]);
}

#[test]
fn stream_delta_scheduler_targets_24_chars_per_frame() {
    let mut scheduler = DeltaScheduler::default();
    let content = "界".repeat(60);
    scheduler.push(key(), StreamDelta::Text(content.clone()), 0);

    assert_eq!(scheduler.drain(&key(), 0, true), vec![ScheduledDelta::Text("界".repeat(24))]);
    assert!(scheduler.drain(&key(), 15, true).is_empty());
    assert_eq!(
        scheduler.drain(&key(), ACTIVE_FRAME_MS, true),
        vec![ScheduledDelta::Text("界".repeat(24))]
    );
    assert_eq!(
        scheduler.drain(&key(), ACTIVE_FRAME_MS * 2, true),
        vec![ScheduledDelta::Text("界".repeat(12))]
    );
}

#[test]
fn stream_delta_scheduler_throttles_background_items() {
    let mut scheduler = DeltaScheduler::default();
    scheduler.push(key(), StreamDelta::Text("a".repeat(72)), 0);

    assert_eq!(scheduler.drain(&key(), 0, false), vec![ScheduledDelta::Text("a".repeat(24))]);
    assert!(scheduler.drain(&key(), 99, false).is_empty());
    assert_eq!(scheduler.drain(&key(), 100, false), vec![ScheduledDelta::Text("a".repeat(24))]);
}

#[test]
fn stream_delta_scheduler_caps_backlog_catchup() {
    let mut scheduler = DeltaScheduler::default();
    scheduler.push(key(), StreamDelta::Text("x".repeat(500)), 0);

    assert_eq!(scheduler.drain(&key(), 0, true), vec![ScheduledDelta::Text("x".repeat(24))]);
    assert_eq!(
        scheduler.drain(&key(), 10_000, true),
        vec![ScheduledDelta::Text("x".repeat(24 * 8))]
    );
}

#[test]
fn stream_delta_scheduler_drains_before_terminal() {
    let mut scheduler = DeltaScheduler::default();
    scheduler.push(key(), StreamDelta::Text("🙂".repeat(30)), 0);
    scheduler.push(key(), StreamDelta::Terminal(TerminalEvent::Completed), 1);

    assert_eq!(scheduler.drain(&key(), 0, true), vec![ScheduledDelta::Text("🙂".repeat(24))]);
    assert_eq!(
        scheduler.drain(&key(), ACTIVE_FRAME_MS, true),
        vec![
            ScheduledDelta::Text("🙂".repeat(6)),
            ScheduledDelta::Terminal(TerminalEvent::Completed)
        ]
    );
}

#[test]
fn stream_delta_scheduler_preserves_cancel_and_error_order() {
    let mut scheduler = DeltaScheduler::default();
    scheduler.push(key(), StreamDelta::Text("pending".into()), 0);
    scheduler.push(key(), StreamDelta::Terminal(TerminalEvent::Cancelled), 1);
    scheduler.push(key(), StreamDelta::Terminal(TerminalEvent::Error("network".into())), 2);

    assert_eq!(
        scheduler.drain(&key(), 0, true),
        vec![
            ScheduledDelta::Text("pending".into()),
            ScheduledDelta::Terminal(TerminalEvent::Cancelled),
            ScheduledDelta::Terminal(TerminalEvent::Error("network".into())),
        ]
    );
}

#[test]
fn stream_delta_scheduler_keeps_replacements_separate() {
    let mut scheduler = DeltaScheduler::default();
    scheduler.push(key(), StreamDelta::Text("before".into()), 0);
    scheduler.push(
        key(),
        StreamDelta::Replacement { target: "tool-1".into(), content: "running".into() },
        1,
    );
    scheduler.push(key(), StreamDelta::Text("after".into()), 2);

    assert_eq!(
        scheduler.drain(&key(), 0, true),
        vec![
            ScheduledDelta::Text("before".into()),
            ScheduledDelta::Replacement { target: "tool-1".into(), content: "running".into() },
            ScheduledDelta::Text("after".into()),
        ]
    );
}

#[test]
fn stream_delta_scheduler_large_backlog_is_lossless_and_bounded() {
    let mut scheduler = DeltaScheduler::default();
    let content = "a中🙂".repeat(1_000);
    scheduler.push(key(), StreamDelta::Text(content.clone()), 0);

    let mut emitted = String::new();
    let mut now = 0;
    while scheduler.has_pending(&key()) {
        for event in scheduler.drain(&key(), now, true) {
            if let ScheduledDelta::Text(text) = event {
                assert!(text.chars().count() <= 24 * 8);
                emitted.push_str(&text);
            }
        }
        now += 500;
    }

    assert_eq!(emitted, content);
}
