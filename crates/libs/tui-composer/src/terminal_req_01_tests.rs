use super::*;
use std::time::Duration;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_at(composer: &mut ChatComposer, text: &str, now: Instant) {
    for ch in text.chars() {
        assert_eq!(composer.handle_key_event_at(key(KeyCode::Char(ch)), now), V3Action::None);
    }
}

#[test]
fn terminal_req_01_single_ascii_flushes_when_due() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "a", now);
    assert_eq!(composer.text(), "");
    assert!(composer.flush_paste_burst_if_due(now + Duration::from_secs(1)));
    assert_eq!(composer.text(), "a");
}

#[test]
fn terminal_req_01_burst_enter_inserts_newline() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "ab", now);
    assert_eq!(composer.handle_key_event_at(key(KeyCode::Enter), now), V3Action::InsertNewline);
    type_at(&mut composer, "cd", now);
    composer.flush_paste_burst_if_due(now + Duration::from_secs(1));
    assert_eq!(composer.text(), "ab\ncd");
}

#[test]
fn terminal_req_01_enter_after_window_submits() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "ab", now);
    assert_eq!(
        composer.handle_key_event_at(key(KeyCode::Enter), now + Duration::from_secs(1)),
        V3Action::Submit { text: "ab".into(), attachments: vec![] }
    );
}

#[test]
fn terminal_req_01_explicit_paste_preserves_pending_prefix() {
    let mut composer = ChatComposer::new();
    type_at(&mut composer, "a", Instant::now());
    composer.handle_paste("中文🙂".into());
    assert_eq!(composer.text(), "a中文🙂");
}

#[test]
fn terminal_req_01_unicode_is_immediate_and_mixed_burst_keeps_order() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "中", now);
    assert_eq!(composer.text(), "中");
    type_at(&mut composer, "a🙂", now);
    composer.flush_paste_burst_if_due(now + Duration::from_secs(1));
    assert_eq!(composer.text(), "中a🙂");
}

#[test]
fn terminal_req_01_arrow_flushes_before_moving_cursor() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "ab", now);
    composer.handle_key_event_at(key(KeyCode::Left), now);
    composer.handle_paste("🙂".into());
    assert_eq!(composer.text(), "a🙂b");
}

#[test]
fn terminal_req_01_shortcut_flushes_without_losing_text() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "ab", now);
    assert_eq!(
        composer.handle_key_event_at(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL), now),
        V3Action::OpenExternalEditor
    );
    assert_eq!(composer.text(), "ab");
}

#[test]
fn terminal_req_01_cancel_discards_pending_text_and_window() {
    for event in [key(KeyCode::Esc), KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)] {
        let mut composer = ChatComposer::new();
        let now = Instant::now();
        type_at(&mut composer, "ab", now);
        assert_eq!(composer.handle_key_event_at(event, now), V3Action::Cancel);
        assert!(!composer.flush_paste_burst_if_due(now + Duration::from_secs(1)));
        assert_eq!(composer.handle_key_event_at(key(KeyCode::Enter), now), V3Action::None);
        assert_eq!(composer.text(), "");
    }
}

#[test]
fn terminal_req_01_replacing_draft_drops_stale_pending_text() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "ab", now);
    composer.restore_submission("restored🙂", vec![]);
    assert!(!composer.flush_paste_burst_if_due(now + Duration::from_secs(1)));
    assert_eq!(composer.text(), "restored🙂");
}

#[test]
fn terminal_req_01_release_does_not_flush_or_append() {
    let mut composer = ChatComposer::new();
    let now = Instant::now();
    type_at(&mut composer, "a", now);
    let mut release = key(KeyCode::Char('b'));
    release.kind = KeyEventKind::Release;
    composer.handle_key_event_at(release, now);
    composer.flush_paste_burst_if_due(now + Duration::from_secs(1));
    assert_eq!(composer.text(), "a");
}
