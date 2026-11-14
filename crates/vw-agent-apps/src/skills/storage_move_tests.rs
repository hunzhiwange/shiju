use super::storage_move::StorageMove;
use std::fs;

#[test]
fn cp12_storage_move_success() {
    let fixture = tempfile::tempdir().unwrap();
    let old = fixture.path().join("old");
    let new = fixture.path().join("new");
    fs::create_dir_all(old.join("sample")).unwrap();
    fs::create_dir(&new).unwrap();
    fs::write(old.join("sample/SKILL.md"), "# Sample").unwrap();
    let migration = StorageMove::prepare(&old, &new, &["sample".into()], &[]).unwrap();
    assert!(migration.result.errors.is_empty());
    assert!(migration.finish().switched);
    assert_eq!(fs::read(new.join("sample/SKILL.md")).unwrap(), b"# Sample");
    assert!(old.join("sample/SKILL.md").exists());
}

#[test]
fn cp12_storage_move_rejects_without_side_effects() {
    let fixture = tempfile::tempdir().unwrap();
    let old = fixture.path().join("old");
    let new = fixture.path().join("new");
    fs::create_dir_all(old.join("sample")).unwrap();
    fs::create_dir_all(new.join("sample")).unwrap();
    fs::write(old.join("sample/SKILL.md"), "old").unwrap();
    fs::write(new.join("sample/SKILL.md"), "external").unwrap();
    let migration = StorageMove::prepare(&old, &new, &["sample".into()], &[]).unwrap();
    assert_eq!(migration.result.errors.len(), 1);
    drop(migration);
    assert_eq!(fs::read(new.join("sample/SKILL.md")).unwrap(), b"external");
    assert_eq!(fs::read(old.join("sample/SKILL.md")).unwrap(), b"old");
}
