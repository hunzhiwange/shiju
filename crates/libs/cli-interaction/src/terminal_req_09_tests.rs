use crate::app::{ActiveRunner, V3GatewayApp};
use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use cli_interaction_model::settings_panel::{SettingsPanel, SettingsValues};
use cli_interaction_model::vim::VimMode;
use cli_interaction_state::TuiState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::V3Action;

fn app() -> V3GatewayApp {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).unwrap();
    V3GatewayApp::new(
        GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace"))),
        TuiState::default(),
    )
}
fn key(app: &mut V3GatewayApp, code: KeyCode) -> V3Action {
    app.widget_mut().handle_key_event(KeyEvent::new(code, KeyModifiers::NONE))
}
fn enable(app: &mut V3GatewayApp) {
    ActiveRunner::new().execute_action(
        app,
        V3Action::Select { surface: "session_settings_vim".into(), item_id: "enabled".into() },
    );
}

#[test]
fn terminal_req_09_default_disabled() {
    assert!(!app().widget().is_vim_enabled());
}

#[test]
fn terminal_req_09_normal_commands_edit_unicode() {
    let mut app = app();
    app.widget_mut().replace_draft("中文🙂");
    enable(&mut app);
    key(&mut app, KeyCode::Char('0'));
    key(&mut app, KeyCode::Char('x'));
    assert_eq!(app.widget().draft(), "文🙂");
}

#[test]
fn terminal_req_09_escape_preserves_draft() {
    let mut app = app();
    app.widget_mut().replace_draft("中文🙂");
    enable(&mut app);
    key(&mut app, KeyCode::Char('i'));
    assert!(matches!(key(&mut app, KeyCode::Esc), V3Action::None));
    assert!(matches!(key(&mut app, KeyCode::Esc), V3Action::None));
    assert_eq!(app.widget().draft(), "中文🙂");
}

#[test]
fn terminal_req_09_cancel_picker_preserves_disabled() {
    let mut app = app();
    ActiveRunner::new().execute_action(
        &mut app,
        V3Action::Select { surface: "session_settings".into(), item_id: "vim".into() },
    );
    key(&mut app, KeyCode::Esc);
    assert!(!app.widget().is_vim_enabled());
}

#[test]
fn terminal_req_09_unknown_value_rejected() {
    assert!(crate::commands::route_selection("session_settings_vim", "unknown").is_err());
}

#[test]
fn terminal_req_09_panel_cancel_restores_preference() {
    let mut panel = SettingsPanel::new(SettingsValues::default());
    panel.pending.vim_mode = VimMode::Normal;
    panel.cancel();
    assert_eq!(panel.pending.vim_mode, VimMode::Disabled);
}

#[test]
fn terminal_req_09_normal_enter_submits() {
    let mut app = app();
    app.widget_mut().replace_draft("中文🙂");
    enable(&mut app);
    assert!(
        matches!(key(&mut app, KeyCode::Enter), V3Action::Submit { text, .. } if text == "中文🙂")
    );
}

#[test]
fn terminal_req_09_normal_modified_enter_inserts_newline() {
    let mut app = app();
    app.widget_mut().replace_draft("中文🙂");
    enable(&mut app);
    assert!(matches!(
        app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        V3Action::InsertNewline
    ));
    assert!(app.widget().draft().contains('\n'));
}
