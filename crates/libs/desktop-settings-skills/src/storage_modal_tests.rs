use super::*;
fn fixture() -> (State, SkillStorageResponse) {
    let scope = SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() };
    let current = SkillStorageResponse { scope: scope.clone(), version: 1, root: "/old".into(), mode: SkillBindingMode::Auto };
    (State { scope: Some(scope), current: Some(current.clone()), pending: Some(SkillStorageLocation::Agents), ..Default::default() }, current)
}
#[test]
fn cp12_storage_modal_success() {
    let (mut state, mut current) = fixture();
    current.root = "/new".into();
    assert!(state.accept(Readback::Move(SkillStorageMoveResponse { storage: current, task_id: "task".into(), migrated: vec![], skipped: vec![], errors: vec![], switched: true })).is_ok());
    assert_eq!(state.current.unwrap().root, "/new");
    assert!(state.pending.is_none());
}
#[test]
fn cp12_storage_modal_rejects_without_side_effects() {
    let (mut state, mut current) = fixture();
    current.root = "/new".into();
    assert!(state.accept(Readback::Move(SkillStorageMoveResponse { storage: current, task_id: "task".into(), migrated: vec![], skipped: vec![], errors: vec!["failed".into()], switched: false })).is_err());
    assert_eq!(state.current.as_ref().unwrap().root, "/old");
    assert!(state.pending.is_some());
}
