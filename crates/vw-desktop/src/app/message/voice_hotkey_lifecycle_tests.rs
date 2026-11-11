use super::*;

#[test]
fn voice_task_15_release_before_device_ready_is_pending_and_cancel_invalidates_start() {
    let mut app = App::new().0;
    app.active_session_id = Some("voice-hotkey-test".into());
    let scope = scope(&app).unwrap();
    let generation = app.voice.draft.start(scope, String::new()).unwrap();
    let _ = update(&mut app, Event::Finish);
    assert!(app.voice.finish_requested);
    assert_eq!(app.voice.draft.phase, Phase::Recording);
    let _ = update(&mut app, Event::Cancel);
    assert!(!app.voice.finish_requested);
    let _ = update(&mut app, Event::Started(generation, Err("late device failure".into())));
    assert_eq!(app.voice.draft.phase, Phase::Idle);
}

#[test]
fn voice_task_15_foreground_capture_ignores_batched_hotkey_start_and_release() {
    use crate::app::voice::hotkeys::Action;
    let mut app = App::new().0;
    app.active_session_id = Some("voice-hotkey-test".into());
    let scope = scope(&app).unwrap();
    let generation = app.voice.draft.start(scope, String::new()).unwrap();
    let _ = super::super::voice_hotkeys::dispatch(&mut app, vec![Action::Start, Action::Finish]);
    assert_eq!(app.voice.draft.generation, generation);
    assert_eq!(app.voice.draft.phase, Phase::Recording);
    assert!(!app.voice.finish_requested);
}
