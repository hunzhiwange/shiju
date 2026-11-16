use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use cli_interaction_state::TuiState;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::V3Action;

use crate::app::{ActiveRunner, V3GatewayApp, apply_ready_terminal_events};

fn app() -> V3GatewayApp {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).unwrap();
    let runtime = GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")));
    V3GatewayApp::new(runtime, TuiState::default())
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn batch(app: &mut V3GatewayApp, runner: &mut ActiveRunner, events: Vec<Event>) {
    let mut events = events.into_iter();
    apply_ready_terminal_events(app, runner, events.next().unwrap(), || Ok(events.next())).unwrap();
}

fn flush(app: &mut V3GatewayApp) {
    app.widget_mut().flush_paste_burst_if_due(Instant::now() + Duration::from_secs(1));
}

#[test]
fn terminal_req_02_idle_single_key_flushes_without_more_events() {
    let mut app = app();
    batch(&mut app, &mut ActiveRunner::new(), vec![key(KeyCode::Char('a'))]);
    let deadline = app.widget().next_paste_flush_deadline().expect("pending key deadline");
    assert!(app.widget_mut().flush_paste_burst_if_due(deadline));
    assert_eq!(app.widget().draft(), "a");
    assert!(app.widget().next_paste_flush_deadline().is_none());
}

#[test]
fn terminal_req_02_multiline_burst_does_not_start_request() {
    let mut app = app();
    let mut runner = ActiveRunner::new();
    batch(&mut app, &mut runner, vec![key(KeyCode::Char('a')), key(KeyCode::Char('b')),
        key(KeyCode::Enter), key(KeyCode::Char('c'))]);
    flush(&mut app);
    assert_eq!(app.widget().draft(), "ab\nc");
    assert!(!runner.has_active_stream());
    // A cursor action ends the burst classification before the explicit submit.
    app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        V3Action::Submit { text, .. } if text == "ab\nc"));
}

#[test]
fn terminal_req_02_bracketed_unicode_paste_stays_local() {
    let mut app = app();
    let mut runner = ActiveRunner::new();
    batch(&mut app, &mut runner, vec![Event::Paste("中文🙂\n第二行".into())]);
    flush(&mut app);
    assert_eq!(app.widget().draft(), "中文🙂\n第二行");
    assert!(!runner.has_active_stream());
}

#[test]
fn terminal_req_02_cancel_discards_pending_input() {
    let mut app = app();
    batch(&mut app, &mut ActiveRunner::new(), vec![key(KeyCode::Char('a')), key(KeyCode::Esc)]);
    flush(&mut app);
    assert_eq!(app.widget().draft(), "");
    assert!(app.widget().next_paste_flush_deadline().is_none());
}

#[test]
fn terminal_req_02_pager_does_not_receive_background_paste() {
    let mut app = app();
    let mut runner = ActiveRunner::new();
    runner.execute_action(&mut app, V3Action::ShowTranscript);
    assert!(runner.has_transcript_pager());
    batch(&mut app, &mut runner, vec![Event::Paste("中文🙂".into()), key(KeyCode::Char('x'))]);
    assert_eq!(app.widget().draft(), "");
}

#[test]
fn terminal_req_02_overlay_suspends_background_flush() {
    let mut app = app();
    batch(&mut app, &mut ActiveRunner::new(), vec![key(KeyCode::Char('a'))]);
    app.widget_mut().show_selection_overlay("test", "选择", vec![]);
    assert!(app.widget().next_paste_flush_deadline().is_none());
    assert!(!app.widget_mut().flush_paste_burst_if_due(Instant::now() + Duration::from_secs(1)));
    app.widget_mut().handle_paste("搜索🙂".into());
    assert_eq!(app.widget().draft(), "");
    app.widget_mut().clear_overlay();
    flush(&mut app);
    assert_eq!(app.widget().draft(), "a");
}

#[test]
fn terminal_req_02_read_error_preserves_pending_input() {
    let mut app = app();
    let error = apply_ready_terminal_events(&mut app, &mut ActiveRunner::new(), key(KeyCode::Char('a')),
        || Err(std::io::Error::other("read failed"))).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    flush(&mut app);
    assert_eq!(app.widget().draft(), "a");
}
