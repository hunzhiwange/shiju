use super::*;
fn app() -> App {
    let (mut app, _) = App::new();
    app.session_state.active_session_id = Some("voice-archive".into());
    app.navigation_panels.show_system_settings = true;
    app.navigation_panels.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("原草稿");
    app
}
#[test]
fn voice_task_14_recordings_retry_requires_explicit_apply_and_rejects_late_reply() {
    let mut app = app();
    let scope = super::super::voice::scope(&app).unwrap();
    app.voice.recordings.scope = Some(scope);
    app.voice.recordings.generation = 7;
    app.voice.recordings.busy = true;
    let _ = update(&mut app, Event::Retried(7, Ok("识别结果".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "原草稿");
    assert_eq!(app.voice.recordings.result.as_deref(), Some("识别结果"));
    let _ = update(&mut app, Event::Apply);
    assert!(app.current_session_runtime_ref().unwrap().input_editor.text().contains("识别结果"));
    assert!(!app.current_session_runtime_ref().unwrap().request_lifecycle.is_requesting);
    let _ = update(&mut app, Event::Retried(7, Ok("迟到".into())));
    assert!(app.voice.recordings.result.is_none());
}
#[test]
fn voice_task_14_recordings_session_switch_discards_retry_result() {
    let mut app = app();
    let scope = super::super::voice::scope(&app).unwrap();
    app.voice.recordings.scope = Some(scope);
    app.voice.recordings.generation = 7;
    app.voice.recordings.busy = true;
    app.session_state.active_session_id = Some("other".into());
    let _ = update(&mut app, Event::Retried(7, Ok("旧结果".into())));
    assert!(app.voice.recordings.result.is_none());
    assert!(!app.voice.recordings.busy);
}
