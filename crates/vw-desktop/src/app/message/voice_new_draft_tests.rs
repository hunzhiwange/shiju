#![cfg(target_os = "macos")]
use super::*;
use crate::app::message::ChatMessage;

fn new_draft() -> App {
    let (mut app, _) = App::new();
    app.project_path = Some("/voice-new-draft".into());
    app.project_id = Some("voice-project".into());
    app.active_session_id = None;
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = None;
    runtime.task_mode_enabled = false;
    runtime.workflow_mode_enabled = false;
    runtime.full_access_saving = false;
    runtime.full_access_save_failed = false;
    app
}
fn editor(app: &mut App, action: text_editor::Action) {
    let _ = app.update(Message::Chat(ChatMessage::InputEditorAction(action)));
}
#[cfg(target_os = "macos")]
#[test]
fn voice_task_07_new_draft_start_preserves_keyboard_editor_without_creating_session() {
    let mut app = new_draft();
    editor(&mut app, text_editor::Action::Edit(text_editor::Edit::Insert('前')));
    editor(&mut app, text_editor::Action::Edit(text_editor::Edit::Insert('后')));
    editor(&mut app, text_editor::Action::Move(text_editor::Motion::Left));
    let task = app.update(Message::Voice(Event::Start));
    assert_eq!(
        app.voice.draft.phase,
        Phase::Recording,
        "new draft was rejected before local capture preparation"
    );
    assert!(iced_runtime::task::into_stream(task).is_some());
    assert!(app.active_session_id.is_none());
    assert!(app.chat.is_empty());
    assert!(app.sessions.is_empty());
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前后");
    assert!(!app.current_session_runtime_ref().unwrap().is_requesting);
}

fn recognize(app: &mut App) -> u64 {
    let task = app.update(Message::Voice(Event::Start));
    assert_eq!(app.voice.draft.phase, Phase::Recording);
    drop(task); // Do not poll physical capture/permission IO in this App wiring test.
    let generation = app.voice.draft.generation;
    // Replace only physical capture + OS recognition with its completed boundary event.
    app.voice.draft.phase = Phase::Transcribing;
    generation
}
fn complete(app: &mut App, generation: u64, text: &str) {
    let _ = app.update(Message::Voice(Event::Completed(generation, Ok(text.into()))));
}
fn keyboard(app: &mut App, text: &str) {
    for c in text.chars() {
        editor(app, text_editor::Action::Edit(text_editor::Edit::Insert(c)));
    }
}
#[test]
fn voice_task_07_new_draft_fills_original_cursor_once_without_sending() {
    let mut app = new_draft();
    keyboard(&mut app, "前后");
    editor(&mut app, text_editor::Action::Move(text_editor::Motion::Left));
    let generation = recognize(&mut app);
    complete(&mut app, generation, "识别");
    complete(&mut app, generation, "重复");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前识别后");
    assert_eq!(app.input_editor.text(), "前识别后");
    assert!(app.active_session_id.is_none());
    assert!(app.chat.is_empty());
    assert!(!app.current_session_runtime_ref().unwrap().is_requesting);
}
#[test]
fn voice_task_07_new_draft_first_send_uses_original_session_creation_chain() {
    let mut app = new_draft();
    let generation = recognize(&mut app);
    complete(&mut app, generation, "首条听写");
    assert!(app.active_session_id.is_none());
    let task = app.update(Message::Chat(ChatMessage::SendPressed));
    assert!(app.active_session_id.is_some());
    assert!(iced_runtime::task::into_stream(task).is_some());
    assert_eq!(app.chat[0].role, crate::app::models::ChatRole::User);
    assert_eq!(app.chat[0].content, "首条听写");
    let runtime = app.current_session_runtime_ref().unwrap();
    assert!(runtime.input_editor.text().is_empty());
    assert_eq!(runtime.active_agent_request.as_ref().unwrap().query, "首条听写");
}
#[test]
fn voice_task_07_new_draft_first_send_invalidates_inflight_dictation() {
    let mut app = new_draft();
    keyboard(&mut app, "键盘首条");
    let generation = recognize(&mut app);
    let token = app.voice.native_cancel.clone().unwrap();
    let _ = app.update(Message::Chat(ChatMessage::SendPressed));
    assert!(app.active_session_id.is_some());
    assert!(token.is_cancelled());
    complete(&mut app, generation, "迟到识别");
    let runtime = app.current_session_runtime_ref().unwrap();
    assert!(runtime.input_editor.text().is_empty());
    assert_eq!(runtime.active_agent_request.as_ref().unwrap().query, "键盘首条");
    assert!(runtime.is_requesting, "dictation cancellation stopped the normal first send");
}
#[test]
fn voice_task_07_new_draft_cancel_rejects_late_recognition() {
    let mut app = new_draft();
    keyboard(&mut app, "键盘草稿");
    let generation = recognize(&mut app);
    let token = app.voice.native_cancel.clone().unwrap();
    let _ = app.update(Message::Voice(Event::Cancel));
    assert!(token.is_cancelled());
    complete(&mut app, generation, "迟到");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "键盘草稿");
    assert!(app.voice.draft.pending.is_none());
}
#[test]
fn voice_task_07_new_draft_edit_then_undo_requires_explicit_apply() {
    let mut app = new_draft();
    keyboard(&mut app, "原稿");
    let generation = recognize(&mut app);
    keyboard(&mut app, "改");
    editor(&mut app, text_editor::Action::Edit(text_editor::Edit::Backspace));
    complete(&mut app, generation, "识别");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "原稿");
    assert_eq!(app.voice.draft.pending.as_deref(), Some("识别"));
    let _ = app.update(Message::Voice(Event::Apply));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "原稿识别");
    assert_eq!(app.input_editor.text(), "原稿识别");
    assert!(app.active_session_id.is_none());
}
#[test]
fn voice_task_07_new_draft_reopen_same_project_rejects_old_generation() {
    let mut app = new_draft();
    let generation = recognize(&mut app);
    let original = scope(&app).unwrap();
    let token = app.voice.native_cancel.clone().unwrap();
    let _ = app.update(Message::Project(
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPicked {
            project_path: "/voice-new-draft".into(),
            directory: "/voice-new-draft".into(),
        },
    ));
    assert!(app.active_session_id.is_none());
    assert!(original != scope(&app).unwrap());
    assert!(token.is_cancelled());
    keyboard(&mut app, "另一个草稿");
    let next = recognize(&mut app);
    assert!(next > generation);
    complete(&mut app, generation, "旧回包");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "另一个草稿");
    complete(&mut app, next, "新结果");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "另一个草稿新结果");
}
#[test]
fn voice_task_07_new_draft_session_navigation_cancels_before_async_history_load() {
    let mut app = new_draft();
    keyboard(&mut app, "原稿");
    let generation = recognize(&mut app);
    let token = app.voice.native_cancel.clone().unwrap();
    let _ = app.update(Message::Project(
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/voice-new-draft".into(),
            "existing-session".into(),
        ),
    ));
    assert!(token.is_cancelled());
    complete(&mut app, generation, "迟到");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "原稿");
    assert!(app.voice.draft.scope.is_none());
}
#[test]
fn voice_task_07_new_draft_project_switch_cancels_and_preserves_current_editor() {
    let mut app = new_draft();
    let generation = recognize(&mut app);
    let token = app.voice.native_cancel.clone().unwrap();
    let _ = app.update(Message::Project(
        crate::app::message::project::ProjectMessage::OpenRecentPressed(
            "/another-voice-project".into(),
        ),
    ));
    assert!(token.is_cancelled());
    let current = app.current_session_runtime_mut().input_editor.text();
    complete(&mut app, generation, "迟到");
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), current);
}
#[tokio::test]
async fn voice_task_07_new_draft_cloud_requires_real_session_before_gateway() {
    let cloud = crate::app::voice::preferences::Preferences {
        engine: crate::app::voice::preferences::VoiceEngine::Cloud,
        ..Default::default()
    };
    let result =
        dictation::select_route(&cloud, GatewayEndpoint::new("unreachable.invalid", 9), None, 1)
            .await;
    assert!(result.err().unwrap().contains("已建立的聊天会话"));
    let local = crate::app::voice::preferences::Preferences {
        engine: crate::app::voice::preferences::VoiceEngine::Local,
        ..Default::default()
    };
    assert!(matches!(
        dictation::select_route(&local, GatewayEndpoint::new("unreachable.invalid", 9), None, 1)
            .await,
        Ok(dictation::Route::Local { .. })
    ));
}
