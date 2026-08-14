use crate::app::App;
use crate::app::state::UsageStepRawModal;

#[test]
fn view_returns_none_when_modal_closed() {
    let (app, _task) = App::new();
    assert!(super::view(&app).is_none());
}

#[test]
fn view_builds_when_modal_open() {
    let (mut app, _task) = App::new();
    app.usage_step_raw_modal =
        Some(UsageStepRawModal { step_index: 3, content: r#"{"messages":[]}"#.to_string() });

    assert!(super::view(&app).is_some());
}
