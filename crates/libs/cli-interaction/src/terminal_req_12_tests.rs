//! 跨终端事件、真实组合器、附件校验和 Gateway 请求的验收资产。
//! 平台剪贴板与网络不在本组测试内。
use std::path::PathBuf;
use std::sync::Arc;

use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use cli_interaction_model::PermissionMode;
use cli_interaction_state::TuiState;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3Attachment, V3AttachmentKind};

use crate::app::{ActiveRunner, V3GatewayApp, apply_ready_terminal_events};

fn app(root: PathBuf) -> V3GatewayApp {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).unwrap();
    let runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(root.clone()).with_id(Some("matrix-session".into())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("matrix-session".into());
    state.project.workspace_root = Some(root);
    state.session_options.permission_mode = PermissionMode::Plan;
    state.session_options.set_full_access(false);
    V3GatewayApp::new(runtime, state)
}

fn paste(app: &mut V3GatewayApp, text: &str) {
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    apply_ready_terminal_events(app, &mut runner, Event::Paste(text.into()), || Ok(None)).unwrap();
    assert!(!runner.has_active_stream(), "paste must never start a request");
    assert!(app.state().messages.is_empty());
}

fn submit(app: &mut V3GatewayApp) -> V3Action {
    app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
}

fn image() -> V3Attachment {
    V3Attachment {
        kind: V3AttachmentKind::Image,
        label: "clipboard.png".into(),
        path: None,
        media_type: Some("image/png".into()),
        text: None,
        bytes: Some(b"encoded-png".to_vec()),
    }
}

#[test]
fn terminal_req_12_paste_waits_for_manual_submit_and_keeps_permissions() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path().into());
    paste(&mut app, "中文🙂\n第二行");
    assert_eq!(app.widget().draft(), "中文🙂\n第二行");
    let action = submit(&mut app);
    let command = app.handle_action(action).unwrap();
    let request = app.request_for_command(&command).unwrap();
    assert_eq!(request.messages.last().unwrap()["content"], "中文🙂\n第二行");
    assert_eq!(request.allowed_tools, Some(vec!["todowrite".into()]));
    let options = request.options.unwrap();
    assert_eq!(options["full_access"], false);
    assert_eq!(options["acp_permission_mode"], "approve-reads");
}

#[test]
fn terminal_req_12_long_paste_and_image_reach_structured_request() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path().into());
    let payload = format!("  {}\n", "中文🙂".repeat(1001));
    paste(&mut app, &payload);
    assert!(app.widget().draft().contains("[Pasted text:"));
    app.widget_mut().add_attachment(image());
    let action = submit(&mut app);
    let command = app.handle_action(action).unwrap();
    let request = app.request_for_command(&command).unwrap();
    let blocks = request.messages.last().unwrap()["content"].as_array().unwrap();
    assert!(blocks.iter().any(|block| block["type"] == "text" && block["text"] == payload));
    assert!(blocks.iter().any(|block| block["type"] == "image"
        && block["mimeType"] == "image/png"
        && block["data"] == "ZW5jb2RlZC1wbmc="));
    assert!(!serde_json::to_string(&request.messages).unwrap().contains("[Pasted text:"));
}

#[test]
fn terminal_req_12_attachment_error_restores_rich_draft() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path().into());
    let payload = "中🙂".repeat(1001);
    paste(&mut app, &payload);
    let display = app.widget().draft().to_owned();
    let mut attachment = image();
    attachment.path = Some(root.path().join("missing.png").display().to_string());
    attachment.bytes = None;
    app.widget_mut().add_attachment(attachment.clone());
    let action = submit(&mut app);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(&mut app, action);
    assert!(!runner.has_active_stream());
    assert_eq!(app.widget().draft(), display);
    assert_eq!(app.widget().attachments(), [attachment]);
    assert!(matches!(submit(&mut app), V3Action::Submit { text, .. } if text == payload));
}

#[test]
fn terminal_req_12_rich_queue_is_fifo_and_drained_once() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path().into());
    app.widget_mut().handle_v3_event(vw_tui::V3Event::TurnStarted {
        turn_id: "queue-turn".into(),
    });
    app.widget_mut().queue_input("first".into(), Vec::new());
    let payload = "中🙂".repeat(1001);
    paste(&mut app, &payload);
    app.widget_mut().add_attachment(image());
    let action = app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let V3Action::Queue { text, attachments } = action else { panic!("queue action expected") };
    assert_eq!(text, payload);
    app.widget_mut().queue_input(text, attachments);
    assert_eq!(app.widget_mut().pop_queued_input(), Some(("first".into(), Vec::new())));
    let (text, attachments) = app.widget_mut().pop_queued_input().unwrap();
    assert_eq!(attachments, [image()]);
    let command = app.handle_action(V3Action::Submit { text, attachments }).unwrap();
    let request = app.request_for_command(&command).unwrap();
    let blocks = request.messages.last().unwrap()["content"].as_array().unwrap();
    assert!(blocks.iter().any(|block| block["text"] == payload));
    assert_eq!(app.widget_mut().pop_queued_input(), None);
}

#[test]
fn terminal_req_12_overlay_paste_and_cancel_keep_background_draft() {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(root.path().into());
    app.widget_mut().replace_draft("后台🙂");
    app.widget_mut().show_selection_overlay("matrix", "选择", vec![]);
    paste(&mut app, "覆盖层\n输入");
    let action = app.widget_mut().handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!matches!(action, V3Action::Submit { .. }));
    assert_eq!(app.widget().draft(), "后台🙂");
    assert!(app.state().messages.is_empty());
}

#[test]
fn terminal_req_12_workspace_escape_does_not_start_request() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("图片.png");
    std::fs::write(&path, b"png").unwrap();
    let mut app = app(root.path().into());
    app.widget_mut().replace_draft("保留🙂");
    paste(&mut app, &format!("'{}'", path.display()));
    let attachments = app.widget().attachments().to_vec();
    let action = submit(&mut app);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(&mut app, action);
    assert!(!runner.has_active_stream());
    assert_eq!(app.widget().draft(), "保留🙂");
    assert_eq!(app.widget().attachments(), attachments);
}
