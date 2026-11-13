use super::*;

fn recording() -> App {
    let (mut app, _) = App::new();
    let scope = scope(&app).unwrap();
    app.current_session_runtime_mut();
    app.voice.draft.start(scope, String::new()).unwrap();
    app
}
#[test]
fn voice_ui_dictation_preparing_stop_is_safe() {
    let mut app = recording();
    let _ = app.update(Message::Voice(Event::Finish));
    let _ = app.update(Message::Voice(Event::Finish));
    assert!(app.voice.finish_requested);
    assert_eq!(app.voice.draft.phase, Phase::Recording);
    let _ = app.update(Message::Voice(Event::Cancel));
    assert!(!app.voice.finish_requested);
}
#[test]
fn voice_ui_dictation_stop_transcribes_once() {
    let mut app = recording();
    // Substitute the physical device boundary; the transcription Task is never polled.
    app.voice.device = Some(Arc::new(Mutex::new(None)));
    app.voice.route = Some(dictation::Route::Local { language: None });
    let generation = app.voice.draft.generation;
    let first = app.update(Message::Voice(Event::Finish));
    assert!(iced_runtime::task::into_stream(first).is_some());
    assert_eq!(app.voice.draft.phase, Phase::Transcribing);
    let _ = app.update(Message::Voice(Event::Finish));
    assert_eq!(app.voice.draft.phase, Phase::Transcribing);
    assert!(app.voice.device.is_none());
    assert!(app.voice.route.is_none());
    assert!(iced_runtime::task::into_stream(update(&mut app, Event::Finish)).is_none());
    let _ = app.update(Message::Voice(Event::Completed(generation, Ok("听写".into()))));
    let _ = app.update(Message::Voice(Event::Completed(generation, Ok("重复".into()))));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "听写");
    assert!(app.chat.is_empty());
}
#[test]
fn voice_ui_dictation_cancel_rejects_late_result() {
    let mut app = recording();
    let generation = app.voice.draft.generation;
    app.voice.waveform.push(1, 1.0);
    let _ = app.update(Message::Voice(Event::Cancel));
    let _ = app.update(Message::Voice(Event::Completed(generation, Ok("迟到".into()))));
    assert!(app.current_session_runtime_ref().unwrap().input_editor.text().is_empty());
}
#[test]
fn voice_ui_dictation_conflict_requires_apply() {
    let mut app = recording();
    app.voice.draft.phase = Phase::Transcribing;
    let generation = app.voice.draft.generation;
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("修改");
    let _ = app.update(Message::Voice(Event::Completed(generation, Ok("结果".into()))));
    assert_eq!(app.voice.draft.pending.as_deref(), Some("结果"));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "修改");
    let _ = app.update(Message::Voice(Event::Apply));
    assert!(app.current_session_runtime_ref().unwrap().input_editor.text().contains("结果"));
    assert!(app.chat.is_empty());
}
