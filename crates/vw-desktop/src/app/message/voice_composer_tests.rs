use super::*;
fn app_with_draft() -> App {
    let (mut app, _) = App::new();
    app.active_session_id = Some("voice-original".into());
    let editor = &mut app.current_session_runtime_mut().input_editor;
    *editor = text_editor::Content::with_text("前后");
    editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentStart));
    editor.perform(text_editor::Action::Move(text_editor::Motion::Right));
    app.voice.cursor = Some(editor.cursor());
    let scope = scope(&app).unwrap();
    app.voice.draft.start(scope, "前后".into()).unwrap();
    app.voice.draft.phase = Phase::Transcribing;
    app
}
#[test]
fn voice_task_07_composer_inserts_at_original_cursor_without_sending() {
    let mut app = app_with_draft();
    let generation = app.voice.draft.generation;
    let _ = update(&mut app, Event::Completed(generation, Ok("听写".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前听写后");
    assert!(!app.current_session_runtime_ref().unwrap().is_requesting);
    let _ = update(&mut app, Event::Completed(generation, Ok("重复".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前听写后");
}
#[test]
fn voice_task_07_composer_switch_and_cancel_reject_late_reply() {
    let mut app = app_with_draft();
    let generation = app.voice.draft.generation;
    app.active_session_id = Some("other".into());
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("另一草稿");
    let _ = update(&mut app, Event::Completed(generation, Ok("迟到".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "另一草稿");
    assert_eq!(app.voice.draft.phase, Phase::Idle);
    app.active_session_id = Some("voice-original".into());
    let _ = update(&mut app, Event::Completed(generation, Ok("迟到".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前后");
}
#[test]
fn voice_task_07_composer_preserves_edit_then_applies_explicitly() {
    let mut app = app_with_draft();
    let generation = app.voice.draft.generation;
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("编辑后");
    let _ = update(&mut app, Event::Completed(generation, Ok("听写".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "编辑后");
    assert_eq!(app.voice.draft.pending.as_deref(), Some("听写"));
    let _ = update(&mut app, Event::Apply);
    assert!(app.current_session_runtime_ref().unwrap().input_editor.text().contains("听写"));
    assert!(app.voice.draft.pending.is_none());
}
#[test]
fn voice_task_07_escape_invalidates_before_late_completion() {
    let mut app = app_with_draft();
    let generation = app.voice.draft.generation;
    let _ = app.update(Message::View(crate::app::message::ViewMessage::GlobalKeyPressed(
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        iced::keyboard::Modifiers::empty(),
    )));
    assert_eq!(app.voice.draft.phase, Phase::Idle);
    let _ = update(&mut app, Event::Completed(generation, Ok("迟到".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前后");
}
#[test]
fn voice_task_14_recordings_cancel_and_failure_discard_audio() {
    let mut app = app_with_draft();
    app.voice.retain_requested = true;
    app.voice.audio = Some(vec![1, 2]);
    let generation = app.voice.draft.generation;
    let _ = update(&mut app, Event::Cancel);
    assert!(app.voice.audio.is_none());
    assert!(!app.voice.retain_requested);
    let _ = update(&mut app, Event::Transcribed(generation, Ok(("迟到".into(), vec![1, 2]))));
    assert!(app.voice.audio.is_none());
    assert_eq!(app.voice.draft.phase, Phase::Idle);
    let mut app = app_with_draft();
    app.voice.audio = Some(vec![1, 2]);
    app.voice.retain_requested = true;
    let generation = app.voice.draft.generation;
    let _ = update(&mut app, Event::Completed(generation, Err("服务失败".into())));
    assert!(app.voice.audio.is_none());
    assert!(!app.voice.retain_requested);
}

#[test]
fn voice_task_16_external_failure_preserves_result_without_touching_chat_or_sending() {
    let mut app = app_with_draft();
    let generation = app.voice.draft.generation;
    app.voice.insertion.target = Some(Err("焦点已变化".into()));
    let _ = update(&mut app, Event::Completed(generation, Ok("你好，\n外部目标！".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前后");
    assert!(!app.current_session_runtime_ref().unwrap().is_requesting);
    assert_eq!(app.voice.insertion.pending.as_deref(), Some("你好，\n外部目标！"));
    assert_eq!(app.voice.draft.phase, Phase::Idle);
    let _ = update(&mut app, Event::Completed(generation, Ok("重复结果".into())));
    assert_eq!(app.voice.insertion.pending.as_deref(), Some("你好，\n外部目标！"));
    let _ = update(&mut app, Event::Onboarding(OnboardingEvent::Discard));
    assert!(app.voice.insertion.pending.is_none());
}
#[test]
fn voice_task_16_external_cancel_and_service_error_reject_late_results() {
    for event in [Event::Cancel, Event::Completed(1, Err("服务失败".into()))] {
        let mut app = app_with_draft();
        let generation = app.voice.draft.generation;
        assert_eq!(generation, 1);
        app.voice.insertion.target = Some(Err("辅助功能权限不足".into()));
        let _ = update(&mut app, event);
        assert!(app.voice.insertion.target.is_none());
        let _ = update(&mut app, Event::Completed(generation, Ok("迟到".into())));
        assert!(app.voice.insertion.pending.is_none());
        assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "前后");
    }
}

#[test]
fn voice_task_16_external_pending_remains_scoped_and_prevents_overwrite() {
    let mut app = app_with_draft();
    let generation = app.voice.draft.generation;
    app.voice.insertion.target = Some(Err("权限不足".into()));
    let _ = update(&mut app, Event::Completed(generation, Ok("待复制".into())));
    assert!(app.voice.draft.scope.is_some());
    let _ = update(&mut app, Event::Start);
    assert_eq!(app.voice.draft.generation, generation);
    assert_eq!(app.voice.insertion.pending.as_deref(), Some("待复制"));
    app.active_session_id = Some("other-session".into());
    invalidate(&mut app);
    assert!(app.voice.insertion.pending.is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn voice_task_11_local_start_never_opens_cloud_conversation() {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo".into());
    app.active_session_id = Some("local-voice-route".into());
    app.current_session_runtime_mut();
    assert_eq!(
        crate::app::voice::preferences::Preferences::load().unwrap().engine,
        crate::app::voice::preferences::VoiceEngine::Local
    );
    let _ = update(&mut app, Event::ChatStart);
    assert!(app.voice.conversation.is_none(), "Local opened a cloud audio conversation");
    assert!(app.voice.local_conversation.is_some());
}
