use vw_api_app_skills::inventory_api::SkillInventoryResponse;
use super::*;
use crate::update::test_support::{Harness, input};

#[test]
fn cp12_install_view_rejects_without_side_effects() {
    let mut app = Harness { state: State::default(), input: input() };
    // A late response cannot reopen a closed panel or populate its inventory.
    let _ = update(&mut app.state, &app.input, Event::Loaded(0, Ok(SkillInventoryResponse {
        scope: SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() },
        version: 7, skills: vec![],
    })));
    assert!(!app.state.open);
    assert!(app.state.inventory.is_none());
}

#[test]
fn cp12_install_view_success() {
    let mut app = Harness { state: State::default(), input: input() };
    app.state.open = true;
    app.state.generation = 4;
    let _ = update(&mut app.state, &app.input, Event::Close);
    assert!(!app.state.open);
    assert_eq!(app.state.generation, 5);
}
