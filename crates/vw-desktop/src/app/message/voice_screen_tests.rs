use super::*;
fn target() -> screen_context::Target {
    screen_context::Target { pid: 42, window: 7, label: "中性窗口".into() }
}
#[test]
fn voice_task_17_old_completion_cannot_cancel_new_share() {
    let (mut app, _) = App::new();
    app.voice.screen.set_enabled(true);
    let old = app.voice.screen.begin(target(), true).unwrap();
    let current = app.voice.screen.begin(target(), true).unwrap();
    let _ = update(&mut app, ScreenEvent::Ready(old, VoiceCounter::FIRST, Ok(vec![1])));
    assert!(current.is_live());
    assert!(app.voice.screen.owns(&current));
}
#[test]
fn voice_task_17_end_and_cancel_retire_queued_screen_without_sending() {
    let (mut app, _) = App::new();
    app.voice.screen.set_enabled(true);
    let ticket = app.voice.screen.begin(target(), true).unwrap();
    end_conversation(&mut app);
    assert!(!ticket.is_live());
    assert!(!app.voice.screen.enabled);
    let _ = update(&mut app, ScreenEvent::Ready(ticket, VoiceCounter::FIRST, Ok(vec![1])));
    assert!(app.files.is_empty());
    assert!(app.voice.conversation.is_none());
    assert!(!app.current_session_runtime_ref().is_some_and(|runtime| runtime.is_requesting));
}
