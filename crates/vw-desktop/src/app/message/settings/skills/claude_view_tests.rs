use super::*;

// Message handling must not require a live service for stale or refused replies.
#[test]
fn cp12_claude_view_success() {
    let mut app = App::new().0;
    let _ = update(&mut app, Event::Open);
    let s = &mut app.skills_settings.claude_config;
    s.project = Some("project".into());
    app.project_id = Some("project".into());
    s.busy = true;
    let generation = s.generation;
    let scope = ClaudeConfigQuery { project_id: "project".into(), target_host_id: s.host.clone() };
    let _ = update(&mut app, Event::Completed(generation, Ok((ClaudeConfigResponse {
        scope, status: ClaudeConfigStatus::Valid { managed: true, primary_api_key_present: true },
    }, Some("操作已保存".into())))));
    assert_eq!(app.skills_settings.claude_config.status,
        Some(ClaudeConfigStatus::Valid { managed: true, primary_api_key_present: true }));
    assert!(!app.skills_settings.claude_config.busy);
}

#[test]
fn cp12_claude_view_rejects_without_side_effects() {
    let mut app = App::new().0;
    let _ = update(&mut app, Event::Open);
    let s = &mut app.skills_settings.claude_config;
    s.status = Some(ClaudeConfigStatus::Missing);
    s.busy = true;
    let generation = s.generation;
    let _ = update(&mut app, Event::Completed(generation.wrapping_sub(1), Err("late".into())));
    assert!(app.skills_settings.claude_config.busy);
    let _ = update(&mut app, Event::Completed(generation, Err("denied".into())));
    assert_eq!(app.skills_settings.claude_config.status, Some(ClaudeConfigStatus::Missing));
    assert!(!app.skills_settings.claude_config.busy);
}
