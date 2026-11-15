use crate::app::voice::local::Cancellation;
#[test]
fn voice_task_07_local_context_change_cancels_native_and_rejects_completion() {
    use crate::app::App;
    use crate::app::message::voice::{self, Event};
    use crate::app::message::voice_draft::Phase;
    let mut app = App::new().0;
    app.active_session_id = Some("local-original".into());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("原草稿");
    let scope = voice::scope(&app).unwrap();
    let generation = app.voice.draft.start(scope, "原草稿".into()).unwrap();
    app.voice.draft.phase = Phase::Transcribing;
    let token = Cancellation::default();
    app.voice.native_cancel = Some(token.clone());
    app.active_session_id = Some("other".into());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("其它草稿");
    voice::invalidate(&mut app);
    assert!(token.is_cancelled());
    let _task = voice::update(&mut app, Event::Completed(generation, Ok("迟到识别".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "其它草稿");
    assert!(!app.current_session_runtime_ref().unwrap().request_lifecycle.is_requesting);
}
