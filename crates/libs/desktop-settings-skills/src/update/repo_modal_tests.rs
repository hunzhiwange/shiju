use super::*;
use crate::update::test_support::{Harness, input};

#[test]
fn cp12_repo_modal_success() {
    let mut app = Harness { state: State::default(), input: input() };
    let _ = update(&mut app.state, &app.input, Event::Open);
    let state = &mut app.state;
    state.busy = true;
    let generation = state.generation;
    let response = SkillReposResponse {
        scope: SkillReposQuery { project_id: "project".into(), target_host_id: state.host.clone() },
        version: 4,
        repos: vec![SkillRepoEntry { owner: "owner".into(), name: "repo".into(), branch: String::new(), enabled: false }],
    };
    state.identity = "owner/repo".into();
    let _ = update(&mut app.state, &app.input, Event::Completed { generation, added: true, result: Ok(response) });
    let state = &app.state;
    assert_eq!(state.snapshot.as_ref().unwrap().version, 4);
    assert!(!state.snapshot.as_ref().unwrap().repos[0].enabled);
    assert!(state.identity.is_empty());
    assert!(!state.busy);
}

#[test]
fn cp12_repo_modal_rejects_without_side_effects() {
    let mut app = Harness { state: State::default(), input: input() };
    let _ = update(&mut app.state, &app.input, Event::Open);
    let state = &mut app.state;
    state.busy = true;
    state.identity = "owner/repo".into();
    let generation = state.generation;
    let _ = update(&mut app.state, &app.input, Event::Completed { generation: generation.wrapping_sub(1), added: true, result: Err("late".into()) });
    assert!(app.state.busy);
    assert_eq!(app.state.identity, "owner/repo");
    let _ = update(&mut app.state, &app.input, Event::Completed { generation, added: true, result: Err("rejected".into()) });
    let state = &app.state;
    assert!(state.snapshot.is_none());
    assert_eq!(state.identity, "owner/repo");
    assert_eq!(state.error.as_deref(), Some("rejected"));
    assert!(!state.busy);
}
