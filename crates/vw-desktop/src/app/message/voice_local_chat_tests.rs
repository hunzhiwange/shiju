use super::*;
use crate::app::message::chat::{self, ChatMessage};
fn app() -> App {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo".into());
    app.active_session_id = Some("local-turn".into());
    app.active_session_view_state.base_ready = true;
    app.active_session_view_state.ui_preparing = false;
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = None;
    runtime.task_mode_enabled = false;
    runtime.workflow_mode_enabled = false;
    runtime.input_editor = iced::widget::text_editor::Content::new();
    let _ = start(
        &mut app,
        Preferences {
            engine: crate::app::voice::preferences::VoiceEngine::Local,
            ..Default::default()
        },
    );
    assert!(app.voice.local_conversation.is_some());
    app
}
fn recognized(app: &mut App, input: &str) {
    let state = app.voice.local_conversation.as_mut().unwrap();
    state.stage = Stage::Recognizing;
    let (call, round) = (state.call, state.round);
    let _ = update(app, Event::Recognized(call, round, Ok(input.into())));
}
#[test]
fn voice_task_11_local_uses_ordinary_request_and_final_completion_once() {
    let mut app = app();
    recognized(&mut app, "请回答一个普通问题");
    app.current_session_runtime_mut().draft = None;
    let request = app.current_session_runtime_ref().unwrap().active_agent_request.clone().unwrap();
    assert_eq!(request.query, "请回答一个普通问题");
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().stage, Stage::WaitingModel);
    assert!(app.voice.conversation.is_none());
    let _ = chat::update(
        &mut app,
        ChatMessage::AgentStreamDelta(
            request.id,
            "<think>秘密</think>你好。\n```rust\nsecret();\n```".into(),
        ),
    );
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().captions.len(), 1);
    let _ = chat::update(
        &mut app,
        ChatMessage::AgentStreamDone(request.id, Default::default(), None, None),
    );
    let state = app.voice.local_conversation.as_ref().unwrap();
    assert_eq!(state.stage, Stage::Speaking);
    assert_eq!(state.captions.last().unwrap().1, "你好。");
    let _ = chat::update(
        &mut app,
        ChatMessage::AgentStreamDone(request.id, Default::default(), None, None),
    );
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().captions.len(), 2);
}
#[test]
fn voice_task_11_local_rejects_commands_and_preserves_draft() {
    let mut app = app();
    recognized(&mut app, "/task 不执行");
    assert!(app.current_session_runtime_ref().unwrap().active_agent_request.is_none());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("保留草稿");
    recognized(&mut app, "不要覆盖");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "保留草稿");
    assert!(app.current_session_runtime_ref().unwrap().active_agent_request.is_none());
}
#[test]
fn voice_task_11_local_mute_end_and_switch_discard_late_results() {
    let mut app = app();
    let chat = app.voice.local_conversation.as_mut().unwrap();
    chat.stage = Stage::Recognizing;
    let (call, round, native) = (chat.call, chat.round, chat.native.clone());
    let _ = mute(&mut app);
    assert!(native.is_cancelled());
    let _ = update(&mut app, Event::Recognized(call, round, Ok("迟到的文字".into())));
    assert!(app.current_session_runtime_ref().unwrap().active_agent_request.is_none());
    app.active_session_id = Some("different".into());
    super::super::invalidate(&mut app);
    assert!(app.voice.local_conversation.is_none());
    let _ = update(&mut app, Event::Recognized(call, round, Ok("更迟的文字".into())));
    assert!(app.current_session_runtime_ref().is_none_or(|r| r.active_agent_request.is_none()));
}
#[test]
fn voice_task_11_local_interrupt_stops_only_owned_model_request() {
    let mut app = app();
    recognized(&mut app, "等待回答");
    let native = app.voice.local_conversation.as_ref().unwrap().native.clone();
    let _ = interrupt(&mut app);
    assert!(native.is_cancelled());
    assert!(app.voice.local_stop.is_some());
    let chat = app.voice.local_conversation.as_ref().unwrap();
    assert_eq!(chat.stage, Stage::Ready);
    assert!(chat.request.is_none());
    assert!(chat.captions.iter().all(|(assistant, _)| !assistant));
}

#[test]
fn voice_task_11_local_preserves_model_history_and_permission_policy() {
    let mut ordinary_app = app();
    let mut app = app();
    app.chat = vec![
        StoredMessage {
            role: ChatRole::User,
            content: "已有问题".into(),
            model: None,
            think_timing: vec![],
            processing_timing: None,
        },
        StoredMessage {
            role: ChatRole::Assistant,
            content: "已有回答".into(),
            model: None,
            think_timing: vec![],
            processing_timing: None,
        },
    ];
    let history = app.chat.clone();
    ordinary_app.chat = history.clone();
    for current in [&mut ordinary_app, &mut app] {
        let runtime = current.current_session_runtime_mut();
        runtime.model = "voice-test-selected-model".into();
        runtime.auto_model = false;
        runtime.full_access_initialized = true;
        runtime.full_access_overridden = true;
        runtime.full_access_enabled = false;
    }
    let _ = chat::update(
        &mut ordinary_app,
        ChatMessage::SendToSession {
            session_id: "local-turn".into(), input: "保持原会话".into()
        },
    );
    let ordinary =
        ordinary_app.current_session_runtime_ref().unwrap().active_agent_request.as_ref().unwrap();
    recognized(&mut app, "  保持原会话  ");
    let request = app.current_session_runtime_ref().unwrap().active_agent_request.as_ref().unwrap();
    assert_eq!(request.query, "保持原会话");
    assert_eq!(request.history, history);
    assert_eq!(request.model.as_deref(), Some("voice-test-selected-model"));
    // Existing sessions defer to the saved gateway policy, just like typed chat.
    assert_eq!(request.full_access_enabled, ordinary.full_access_enabled);
    assert_ne!(request.full_access_enabled, Some(true));
    assert_eq!(request.allowed_tools, ordinary.allowed_tools);
    assert_eq!(request.require_agent_tool, ordinary.require_agent_tool);
    assert!(!app.current_session_runtime_ref().unwrap().full_access_enabled);
    assert_eq!(app.active_session_id.as_deref(), Some("local-turn"));
}
#[test]
fn voice_task_11_local_screen_share_rejects_before_permission_or_capture() {
    let mut app = app();
    let _ =
        super::super::update(&mut app, VoiceEvent::Screen(super::super::ScreenEvent::Permission));
    assert_eq!(
        app.active_toast.as_ref().unwrap().message,
        "本机逐轮语音暂不支持屏幕分享，请使用普通聊天附件。"
    );
    assert!(!app.voice.screen.enabled);
    assert!(!app.voice.screen.busy());
    assert!(app.voice.local_conversation.is_some());
    assert!(app.voice.conversation.is_none());
}

#[test]
fn voice_task_19_full_app_routes_local_turn_and_discards_completion_after_end() {
    let mut app = app();
    let state = app.voice.local_conversation.as_mut().unwrap();
    state.stage = Stage::Recognizing;
    let (call, round) = (state.call, state.round);
    let _ = app.update(Message::Voice(VoiceEvent::Local(Event::Recognized(
        call,
        round,
        Ok("使用当前普通聊天模型".into()),
    ))));
    let request = app.current_session_runtime_ref().unwrap().active_agent_request.clone().unwrap();
    assert_eq!(request.query, "使用当前普通聊天模型");
    assert!(app.voice.conversation.is_none());
    app.current_session_runtime_mut().draft = None;
    let _ = app.update(Message::Chat(ChatMessage::AgentStreamDelta(
        request.id,
        "<think>不朗读</think>本机回答。\n```rust\nnot_spoken();\n```".into(),
    )));
    assert_eq!(app.voice.local_conversation.as_ref().unwrap().captions.len(), 1);
    let _ = app.update(Message::Chat(ChatMessage::AgentStreamDone(
        request.id,
        Default::default(),
        None,
        None,
    )));
    let state = app.voice.local_conversation.as_ref().unwrap();
    assert_eq!(state.stage, Stage::Speaking);
    assert_eq!(state.captions.last().unwrap().1, "本机回答。");
    let native = state.native.clone();
    let _ = app.update(Message::Voice(VoiceEvent::ChatEnd));
    assert!(native.is_cancelled());
    let _ = app.update(Message::Voice(VoiceEvent::Local(Event::Spoken(call, round, Ok(())))));
    assert!(app.voice.local_conversation.is_none());
    assert!(app.voice.conversation.is_none());
}
