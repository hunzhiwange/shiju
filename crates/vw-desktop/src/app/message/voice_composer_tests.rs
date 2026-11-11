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
