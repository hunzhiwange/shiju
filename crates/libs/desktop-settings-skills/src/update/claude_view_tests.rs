use super::*;
use crate::update::test_support::{Harness, input};

// Message handling must not require a live service for stale or refused replies.
#[test]
fn cp12_claude_view_success() {
    let mut app = Harness { state: State::default(), input: input() };
    let _ = update(&mut app.state, &app.input, Event::Open);
    let s = &mut app.state;
    s.project = Some("project".into());
    app.input.project = Some("project".into());
    s.busy = true;
    let generation = s.generation;
    let scope = ClaudeConfigQuery { project_id: "project".into(), target_host_id: s.host.clone() };
    let _ = update(&mut app.state, &app.input, Event::Completed(generation, Ok((ClaudeConfigResponse {
        scope, status: ClaudeConfigStatus::Valid { managed: true, primary_api_key_present: true },
    }, Some("操作已保存".into())))));
    assert_eq!(app.state.status,
        Some(ClaudeConfigStatus::Valid { managed: true, primary_api_key_present: true }));
    assert!(!app.state.busy);
}

#[test]
fn cp12_claude_view_rejects_without_side_effects() {
    let mut app = Harness { state: State::default(), input: input() };
    let _ = update(&mut app.state, &app.input, Event::Open);
    let s = &mut app.state;
    s.status = Some(ClaudeConfigStatus::Missing);
    s.busy = true;
    let generation = s.generation;
    let _ = update(&mut app.state, &app.input, Event::Completed(generation.wrapping_sub(1), Err("late".into())));
    assert!(app.state.busy);
    let _ = update(&mut app.state, &app.input, Event::Completed(generation, Err("denied".into())));
    assert_eq!(app.state.status, Some(ClaudeConfigStatus::Missing));
    assert!(!app.state.busy);
}
