use super::super::repo_modal::{State, Event, update};
use crate::update::test_support::{Harness, input};

#[test]
fn cp12_deeplink_success_handoff() {
    let mut app = Harness { state: State::default(), input: input() };
    app.input.directory = Some("/project".into());
    let _ = update(&mut app.state, &app.input, Event::Open);
    app.state.busy = false;
    let _ = update(&mut app.state, &app.input, Event::SourceLink("ccswitch://v1/import?resource=skill&repo=owner/name".into()));
    let _ = update(&mut app.state, &app.input, Event::PreviewSource);
    assert!(app.state.source_preview.is_some());
    assert!(!app.state.busy);
    assert!(app.state.snapshot.is_none());
    let _ = update(&mut app.state, &app.input, Event::CancelSource);
    assert!(app.state.source_preview.is_none());
}

#[test]
fn cp12_deeplink_rejects_without_side_effects_handoff() {
    let mut app = Harness { state: State::default(), input: input() };
    let _ = update(&mut app.state, &app.input, Event::ConfirmSource);
    assert!(!app.state.busy);
    assert!(app.state.snapshot.is_none());
}
