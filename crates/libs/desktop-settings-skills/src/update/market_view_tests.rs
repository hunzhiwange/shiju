use vw_api_app_skills::discovery_api::*;
use super::*;
use crate::update::test_support::{Harness, input};

fn pending() -> (Harness<State>, u64) {
    let mut app = Harness { state: State::default(), input: input() };
    let _ = update(&mut app.state, &app.input, Event::Open);
    app.state.busy = true;
    let generation = app.state.generation;
    (app, generation)
}

#[test]
fn cp12_market_view_success() {
    let (mut app, generation) = pending();
    let report = DiscoverSkillsResponse { version: 4, skills: vec![], failures: vec![
        SkillRepoFailure { owner: "owner".into(), name: "repo".into(), branch: "main".into(),
            error: DiscoveryError::Timeout },
    ] };
    let _ = update(&mut app.state, &app.input, Event::Completed(generation, Ok(Results::Discovery(report.clone()))));
    let state = &app.state;
    assert!(!state.busy);
    assert!(state.error.is_none());
    assert!(matches!(&state.results, Some(Results::Discovery(actual)) if actual == &report));
}

#[test]
fn cp12_market_view_rejects_without_side_effects() {
    let (mut app, generation) = pending();
    let _ = update(&mut app.state, &app.input, Event::Completed(generation.wrapping_sub(1), Err("late".into())));
    assert!(app.state.busy);
    assert!(app.state.error.is_none());
    let _ = update(&mut app.state, &app.input, Event::Query("new".into()));
    let _ = update(&mut app.state, &app.input, Event::Completed(generation, Err("late".into())));
    assert_eq!(app.state.query, "new");
    assert!(app.state.error.is_none());
    assert!(app.state.results.is_none());
}

#[test]
fn cp12_market_view_rejects_old_repository_revision() {
    let (mut app, generation) = pending();
    app.state.version = 8;
    let _ = update(&mut app.state, &app.input, Event::Completed(generation, Ok(Results::Discovery(
        DiscoverSkillsResponse { version: 7, skills: vec![], failures: vec![] }))));
    assert!(app.state.results.is_none());
    assert!(app.state.error.is_some());
}

#[test]
fn cp12_market_view_close_invalidates_request() {
    let (mut app, generation) = pending();
    let _ = update(&mut app.state, &app.input, Event::Close);
    let _ = update(&mut app.state, &app.input, Event::Completed(generation, Err("late".into())));
    assert!(!app.state.open);
    assert!(app.state.error.is_none());
}
