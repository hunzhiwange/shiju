use super::*;

fn scope() -> SkillReposQuery {
    SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() }
}

#[test]
fn cp12_updates_view_success() {
    let mut state = State { scope: Some(scope()), ..Default::default() };
    state.finished(Ok(Outcome::Uninstalled("example".into(), UninstallSkillResponse {
        scope: scope(), version: 2, backup_id: "backup".into(), backup_path: "/host/backup".into(),
        preserved_paths: vec!["/host/external".into()], cleanup_incomplete: true,
    })));
    assert!(state.inventory.is_none());
    assert!(state.updates.is_none());
    assert_eq!(state.backup.as_ref().unwrap().preserved_paths.len(), 1);
    let outcome = state.outcome.clone();
    state.loaded(Err("offline".into()), Err("offline".into()));
    assert_eq!(state.outcome, outcome);
    assert!(state.backup.is_some());
    assert!(state.error.is_some());
}

#[test]
fn cp12_updates_view_rejects_without_side_effects() {
    let mut state = State { scope: Some(scope()), ..Default::default() };
    let mut wrong_scope = scope();
    wrong_scope.target_host_id = "other-host".into();
    state.loaded(Ok(SkillInventoryResponse { scope: wrong_scope, version: 99, skills: vec![] }), Err("offline".into()));
    assert!(state.inventory.is_none());
    assert!(state.updates.is_none());
    assert!(state.backup.is_none());
    assert!(state.outcome.is_none());
    assert!(state.error.is_some());
}
