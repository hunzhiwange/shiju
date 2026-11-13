use super::super::{ScreenEvent, screen};
use super::*;

#[test]
fn voice_task_17_local_sharing_is_explicit_and_disabled_by_default() {
    let mut app = super::tests::app();
    assert!(!app.voice.screen.enabled);
    let _ = screen::update(&mut app, ScreenEvent::Offer);
    assert!(!app.voice.screen.selecting);
    let _ = screen::update(&mut app, ScreenEvent::Enable(true));
    assert!(app.voice.screen.enabled, "Local must support explicit single-frame sharing");
    let _ = screen::update(&mut app, ScreenEvent::Offer);
    assert!(app.voice.screen.selecting);
    let _ = screen::update(&mut app, ScreenEvent::Cancel);
    assert!(!app.voice.screen.selecting);
    assert!(app.current_session_runtime_ref().unwrap().active_agent_request.is_none());
}

fn image_app() -> App {
    use crate::app::state::{ModelSummary, ProviderModelsSummary};
    let mut app = super::tests::app();
    let runtime = app.current_session_runtime_mut();
    runtime.auto_model = false;
    runtime.model = "fixture/vision".into();
    runtime.full_access_enabled = false;
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: "fixture".into(),
        name: "测试".into(),
        models: vec![ModelSummary {
            id: "vision".into(),
            name: "图像模型".into(),
            enabled: true,
            toolcall: true,
            attachment: true,
            context_limit: 8192,
            detail: serde_json::json!({"capabilities":{"input":{"image":true}}}),
        }],
    }];
    app
}
fn frame(app: &mut App) -> (crate::app::voice::screen_context::Ticket, screen::Owner, Vec<u8>) {
    use crate::app::voice::screen_context::{self, Target};
    let owner = screen::Owner::Local(screen_call(app).unwrap());
    app.voice.screen.set_enabled(true);
    let ticket = app
        .voice
        .screen
        .begin(Target { pid: 42, window: 7, label: "中性窗口".into() }, true)
        .unwrap();
    let png = screen_context::encode(2, 2, vec![128; 16]).unwrap();
    (ticket, owner, png)
}
#[test]
fn voice_task_17_local_frame_uses_current_model_history_and_permissions_once() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let mut app = image_app();
    app.chat.push(StoredMessage {
        role: ChatRole::User,
        content: "已有问题".into(),
        model: None,
        think_timing: vec![],
        processing_timing: None,
    });
    app.files = vec!["keep-private.png".into()];
    app.current_session_runtime_mut().tool_selector.toggle_manual_tool("read");
    let history = app.chat.clone();
    let (ticket, owner, png) = frame(&mut app);
    let _ = screen::receive(&mut app, ticket.clone(), owner, Ok(png.clone()), true);
    let request = app.current_session_runtime_ref().unwrap().active_agent_request.clone().unwrap();
    assert_eq!(request.session, "local-turn");
    assert_eq!(request.model.as_deref(), Some("fixture/vision"));
    assert_eq!(request.history, history);
    assert_ne!(request.full_access_enabled, Some(true));
    assert!(request.query.contains("不可信上下文"));
    assert_eq!(request.query.matches("[IMAGE:").count(), 1);
    assert!(
        request.query.contains(&format!("[IMAGE:data:image/png;base64,{}]", STANDARD.encode(&png)))
    );
    assert!(!request.query.contains("keep-private"));
    assert!(request.query.contains("工具：read"));
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().request, Some(request.id));
    assert_eq!(app.files, ["keep-private.png"]);
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().stage, Stage::WaitingModel);
    assert!(!app.voice.local_conversation.as_ref().unwrap().captions[0].1.contains("base64"));
    let _ = screen::receive(&mut app, ticket, owner, Ok(png), true);
    assert_eq!(
        app.current_session_runtime_ref().unwrap().active_agent_request.as_ref().unwrap().id,
        request.id
    );
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().captions.len(), 1);
    app.current_session_runtime_mut().draft = None;
    let _ = crate::app::message::chat::update(
        &mut app,
        crate::app::message::chat::ChatMessage::AgentStreamDelta(
            request.id,
            "这是一个窗口。".into(),
        ),
    );
    let _ = crate::app::message::chat::update(
        &mut app,
        crate::app::message::chat::ChatMessage::AgentStreamDone(
            request.id,
            Default::default(),
            None,
            None,
        ),
    );
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().stage, Stage::Speaking);
    assert_eq!(
        app.voice.local_conversation.as_ref().unwrap().captions.last().unwrap().1,
        "这是一个窗口。"
    );
}
#[test]
fn voice_task_17_local_cancel_disable_revoke_end_and_switch_discard_frames() {
    for action in 0..6 {
        let mut app = image_app();
        let (ticket, owner, png) = frame(&mut app);
        match action {
            0 => {
                let _ = screen::update(&mut app, ScreenEvent::Cancel);
            }
            1 => {
                let _ = screen::update(&mut app, ScreenEvent::Enable(false));
            }
            2 => {} // Permission is the OS boundary replaced below, not a fabricated image service.
            3 => super::super::end_conversation(&mut app),
            4 => {
                app.active_session_id = Some("other-session".into());
                super::super::invalidate(&mut app);
            }
            5 => {
                let _ = interrupt(&mut app);
            }
            _ => unreachable!(),
        }
        let _ = screen::receive(&mut app, ticket.clone(), owner, Ok(png), action != 2);
        assert!(!ticket.is_live(), "capture must retire for action {action}");
        assert!(app.current_session_runtime_ref().is_none_or(|r| r.active_agent_request.is_none()));
        assert!(app.voice.local_conversation.as_ref().is_none_or(|c| c.captions.is_empty()));
        assert!(app.files.is_empty());
    }
}
#[test]
fn voice_task_17_local_changed_model_busy_draft_invalid_png_and_old_call_send_nothing() {
    for action in 0..8 {
        let mut app = image_app();
        let (ticket, owner, mut png) = frame(&mut app);
        match action {
            0 => app.current_session_runtime_mut().auto_model = true,
            1 => app.current_session_runtime_mut().model = "fixture/text-only".into(),
            2 => app.voice.local_conversation.as_mut().unwrap().stage = Stage::Recognizing,
            3 => {
                app.current_session_runtime_mut().input_editor =
                    iced::widget::text_editor::Content::with_text("保留草稿")
            }
            4 => png = vec![1, 2, 3],
            5 => app.voice.local_conversation.as_mut().unwrap().call = rand::random(),
            6 => app.model_settings.providers[0].models[0].enabled = false,
            7 => app.current_session_runtime_mut().acp_options_pending = true,
            _ => unreachable!(),
        }
        let _ = screen::receive(&mut app, ticket, owner, Ok(png), true);
        assert!(
            app.current_session_runtime_ref().unwrap().active_agent_request.is_none(),
            "action {action}"
        );
        if action == 3 {
            assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "保留草稿");
        }
        assert!(app.voice.local_conversation.as_ref().unwrap().captions.is_empty());
    }
}
