//! Deferred workflow regression cases; authored but not run during implementation.
//!
//! Static wiring inventory (no runtime acceptance evidence):
//! - Search: 3b71bc76f, transcript pager `/` + Enter, loaded rendered history only.
//! - Copy: 82bbd480d, `/copy [code N]`, canonical completed answer text;
//!   clipboard availability and SSH/OSC52 depend on the terminal and platform.
//! - Export: 25a63d00d, `/export path`, loaded canonical messages only;
//!   attachments omitted, secure publication currently Unix-only.
//! - Message picker: 5f5ff2ebf, `/fork`, `/reset`, `/compact`; canonical IDs
//!   remain authoritative. Existing session_command_tests cover confirmation
//!   cancellation and invalid IDs without duplicating their Gateway fixtures.
//! - Resume: 1ec8e3f2d, `/resume`, worker -> host result -> owned picker;
//!   existing workflow_req_05 cases cover stale results/cancellation/errors.
//!   No backend pagination or cross-source search is claimed.
//! - Native selection: d9c5dceb0, `/raw`, event loop -> frozen snapshot ->
//!   terminal lifecycle restoration. Emulator selection and physical terminal
//!   restoration still require later platform acceptance.
//!
//! All effective test counts and platform acceptance results remain unknown.
use std::path::Path;
use std::sync::Arc;

use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use cli_interaction_state::TuiState;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::V3Action;

use crate::app::{ActiveRunner, V3GatewayApp, apply_ready_terminal_events};

fn app(root: &Path) -> V3GatewayApp {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).unwrap(),
        GatewaySessionSeed::new(root.to_path_buf()),
    );
    V3GatewayApp::new(runtime, TuiState::default())
}

fn runner() -> ActiveRunner {
    ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)))
}

fn submit(command: &str) -> V3Action {
    V3Action::Submit { text: command.into(), attachments: Vec::new() }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn workflow_req_07_empty_copy_reports_error_without_creating_turn() {
    let mut app = app(Path::new("/workspace"));
    let before = app.state().clone();
    let mut runner = runner();
    runner.execute_action(&mut app, submit("/copy"));
    assert_eq!(app.state(), &before);
    assert!(!runner.has_active_stream());
    assert!(app.transcript_lines(80, true).iter().any(|line| !line.to_string().is_empty()));
}

#[test]
fn workflow_req_07_export_prompt_does_not_submit_chat() {
    let mut app = app(Path::new("/workspace"));
    let before = app.state().clone();
    let mut runner = runner();
    runner.execute_action(&mut app, submit("/export"));
    assert_eq!(app.widget().draft(), "/export ");
    assert_eq!(app.state(), &before);
    assert!(!runner.has_active_stream());
}

#[test]
fn workflow_req_07_export_escape_reports_error_without_mutating_session() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path());
    let before = app.state().clone();
    let mut runner = runner();
    runner.execute_action(&mut app, submit("/export ../escape.md"));
    assert_eq!(app.state(), &before);
    assert!(!runner.has_active_stream());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn workflow_req_07_unicode_export_routes_to_file_and_feedback() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path());
    let before = app.state().clone();
    let mut runner = runner();
    runner.execute_action(&mut app, submit("/export 会话 结果.md"));
    assert!(std::fs::read_to_string(root.path().join("会话 结果.md")).unwrap().contains("未加载历史未导出"));
    assert_eq!(app.state(), &before);
    assert!(!runner.has_active_stream());
    assert!(app.transcript_lines(120, true).iter().any(|line| line.to_string().contains("已导出")));
}

#[test]
fn workflow_req_07_pager_search_enter_does_not_submit_draft() {
    let mut app = app(Path::new("/workspace"));
    app.widget_mut().replace_draft("保留草稿🙂");
    let before = app.state().clone();
    let mut runner = runner();
    runner.execute_action(&mut app, V3Action::ShowTranscript);
    let mut events = [key(KeyCode::Char('中')), key(KeyCode::Enter), key(KeyCode::Esc)].into_iter();
    apply_ready_terminal_events(&mut app, &mut runner, key(KeyCode::Char('/')), || Ok(events.next())).unwrap();
    assert_eq!(app.widget().draft(), "保留草稿🙂");
    assert_eq!(app.state(), &before);
    assert!(!runner.has_active_stream());
}

#[test]
fn workflow_req_07_raw_entry_yields_before_reading_more_input() {
    let mut app = app(Path::new("/workspace"));
    app.widget_mut().replace_draft("/raw");
    let before = app.state().clone();
    let mut runner = runner();
    let outcome = apply_ready_terminal_events(&mut app, &mut runner, key(KeyCode::Enter), || {
        panic!("native selection must own the next event")
    }).unwrap();
    assert_eq!(outcome.processed, 1);
    assert_eq!(app.state(), &before);
    assert!(!runner.has_active_stream());
}
