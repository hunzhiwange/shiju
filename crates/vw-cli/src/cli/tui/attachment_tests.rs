use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3Attachment, V3AttachmentKind};

use super::app::{ActiveRunner, V3GatewayApp};
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

fn test_app(root: PathBuf) -> V3GatewayApp {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).expect("client");
    let runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(root.clone()).with_id(Some("attachment-session".to_string())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("attachment-session".to_string());
    state.project.workspace_root = Some(root);
    V3GatewayApp::new(runtime, state)
}

fn text_attachment(path: PathBuf) -> V3Attachment {
    V3Attachment {
        kind: V3AttachmentKind::File,
        label: "notes.md".to_string(),
        path: Some(path.display().to_string()),
        media_type: Some("text/markdown".to_string()),
        text: None,
    }
}

#[test]
fn attachments_reach_gateway_as_structured_content() {
    let temp = tempfile::tempdir().expect("temp workspace");
    let path = temp.path().join("notes.md");
    std::fs::write(&path, "attachment body").expect("fixture");
    let mut app = test_app(temp.path().to_path_buf());

    let command = app
        .handle_action(V3Action::Submit {
            text: "inspect".to_string(),
            attachments: vec![text_attachment(path)],
        })
        .expect("valid attachment submission");
    let request = app.request_for_command(&command).expect("gateway request");
    let content = request.messages.last().expect("current user message")["content"]
        .as_array()
        .expect("structured content blocks");

    assert_eq!(content[0], serde_json::json!({"type": "text", "text": "inspect"}));
    assert_eq!(content[1]["type"], "resource");
    assert_eq!(content[1]["resource"]["text"], "attachment body");
    assert!(
        content[1]["resource"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.starts_with("file://"))
    );
}

#[test]
fn attachment_failure_restores_the_composer_draft() {
    let temp = tempfile::tempdir().expect("temp workspace");
    let attachment = text_attachment(temp.path().join("missing.md"));
    let mut app = test_app(temp.path().to_path_buf());
    app.widget_mut().replace_draft("keep this draft");
    app.widget_mut().add_attachment(attachment.clone());
    let action = app.widget_mut().handle_key_event(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    ));
    assert_eq!(app.widget().draft(), "");

    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(&mut app, action);

    assert_eq!(app.widget().draft(), "keep this draft");
    assert_eq!(app.widget().attachments(), [attachment]);
}

