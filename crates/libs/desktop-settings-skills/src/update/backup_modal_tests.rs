use super::*;

fn scope() -> SkillReposQuery {
    SkillReposQuery { project_id: "project".into(), target_host_id: "local".into() }
}
fn state() -> State {
    State {
        scope: Some(scope()), selected: Some("backup-1".into()), version: Some(2),
        backups: vec![SkillBackup { backup_id: "backup-1".into(), directory: "tool".into(), name: "Tool".into(), created_at: 1 }],
        ..Default::default()
    }
}
#[test]
fn cp12_backup_modal_success() {
    let mut s = state();
    s.accept(Readback::Restored(RestoreSkillBackupResponse {
        scope: scope(), backup_id: "backup-1".into(), directory: "tool".into(), version: 3,
    })).unwrap();
    assert_eq!(s.version, Some(3));
    assert_eq!(s.backups.len(), 1, "restore preserves backup");
    s.accept(Readback::Deleted(DeleteSkillBackupResponse { scope: scope(), backup_id: "backup-1".into() })).unwrap();
    assert!(s.backups.is_empty());
    assert!(s.selected.is_none());
    // A subsequent read failure never reintroduces a successfully deleted item.
    s.error = Some("read failed".into());
    assert!(s.backups.is_empty());
}
#[test]
fn cp12_backup_modal_rejects_without_side_effects() {
    let mut s = state();
    let mut other = scope();
    other.target_host_id = "other".into();
    assert!(s.accept(Readback::Deleted(DeleteSkillBackupResponse { scope: other, backup_id: "backup-1".into() })).is_err());
    assert!(s.accept(Readback::Restored(RestoreSkillBackupResponse { scope: scope(), backup_id: "unexpected".into(), directory: "tool".into(), version: 9 })).is_err());
    assert_eq!(s.selected.as_deref(), Some("backup-1"));
    assert_eq!(s.backups.len(), 1);
    assert_eq!(s.version, Some(2));
}
