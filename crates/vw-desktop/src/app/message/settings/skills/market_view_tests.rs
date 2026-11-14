use super::*;

fn pending() -> (App, u64) {
    let mut app = App::new().0;
    let _ = update(&mut app, Event::Open);
    app.skills_settings.market.busy = true;
    let generation = app.skills_settings.market.generation;
    (app, generation)
}

#[test]
fn cp12_market_view_success() {
    let (mut app, generation) = pending();
    let report = DiscoverSkillsResponse { version: 4, skills: vec![], failures: vec![
        SkillRepoFailure { owner: "owner".into(), name: "repo".into(), branch: "main".into(),
            error: DiscoveryError::Timeout },
    ] };
    let _ = update(&mut app, Event::Completed(generation, Ok(Results::Discovery(report.clone()))));
    let state = &app.skills_settings.market;
    assert!(!state.busy);
    assert!(state.error.is_none());
    assert!(matches!(&state.results, Some(Results::Discovery(actual)) if actual == &report));
}

#[test]
fn cp12_market_view_rejects_without_side_effects() {
    let (mut app, generation) = pending();
    let _ = update(&mut app, Event::Completed(generation.wrapping_sub(1), Err("late".into())));
    assert!(app.skills_settings.market.busy);
    assert!(app.skills_settings.market.error.is_none());
    let _ = update(&mut app, Event::Query("new".into()));
    let _ = update(&mut app, Event::Completed(generation, Err("late".into())));
    assert_eq!(app.skills_settings.market.query, "new");
    assert!(app.skills_settings.market.error.is_none());
    assert!(app.skills_settings.market.results.is_none());
}

#[test]
fn cp12_market_view_rejects_old_repository_revision() {
    let (mut app, generation) = pending();
    app.skills_settings.market.version = 8;
    let _ = update(&mut app, Event::Completed(generation, Ok(Results::Discovery(
        DiscoverSkillsResponse { version: 7, skills: vec![], failures: vec![] }))));
    assert!(app.skills_settings.market.results.is_none());
    assert!(app.skills_settings.market.error.is_some());
}

#[test]
fn cp12_market_view_close_invalidates_request() {
    let (mut app, generation) = pending();
    let _ = update(&mut app, Event::Close);
    let _ = update(&mut app, Event::Completed(generation, Err("late".into())));
    assert!(!app.skills_settings.market.open);
    assert!(app.skills_settings.market.error.is_none());
}
