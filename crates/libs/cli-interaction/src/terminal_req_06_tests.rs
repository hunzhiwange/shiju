use super::app::{ActiveRunner, V3GatewayApp};
use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use cli_interaction_state::TuiState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use std::sync::Arc;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

fn app(root: PathBuf) -> V3GatewayApp {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).unwrap();
    let runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(root.clone()).with_id(Some("paste-path".into())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("paste-path".into());
    state.project.workspace_root = Some(root);
    V3GatewayApp::new(runtime, state)
}

#[test]
fn terminal_req_06_pasted_image_reaches_request() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("图片 a.png");
    std::fs::write(&path, b"image-bytes").unwrap();
    let mut app = app(root.path().into());
    app.widget_mut().handle_paste(format!("\"{}\"", path.display()));
    assert_eq!(app.widget().attachments().len(), 1);
    assert_eq!(app.widget().draft(), "");
    let action =
        app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let command = app.handle_action(action).unwrap();
    let request = app.request_for_command(&command).unwrap();
    let content = request.messages.last().unwrap()["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "image");
    assert_eq!(content[0]["mimeType"], "image/png");
    assert_eq!(content[0]["data"], "aW1hZ2UtYnl0ZXM=");
}

#[test]
fn terminal_req_06_outside_workspace_restores_draft() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("outside.png");
    std::fs::write(&path, b"image").unwrap();
    let mut app = app(root.path().into());
    app.widget_mut().replace_draft("保留🙂");
    app.widget_mut().handle_paste(format!("'{}'", path.display()));
    let attachments = app.widget().attachments().to_vec();
    let action =
        app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(&mut app, action);
    assert_eq!(app.widget().draft(), "保留🙂");
    assert_eq!(app.widget().attachments(), attachments);
}

#[test]
#[cfg(unix)]
fn terminal_req_06_symlink_escape_rejected() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("outside.png");
    std::fs::write(&target, b"image").unwrap();
    let link = root.path().join("link.png");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let mut app = app(root.path().into());
    app.widget_mut().handle_paste(format!("'{}'", link.display()));
    let action =
        app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(app.handle_action(action).unwrap_err().to_string().contains("escapes workspace"));
}

#[test]
fn terminal_req_06_ordinary_text_has_no_attachment() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path().into());
    let text = "cat missing.png\n中文🙂";
    app.widget_mut().handle_paste(text.into());
    assert_eq!(app.widget().draft(), text);
    assert!(app.widget().attachments().is_empty());
}
