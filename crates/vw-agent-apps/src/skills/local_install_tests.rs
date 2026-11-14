use std::{fs, os::unix::fs::symlink};
use super::local_install::{LocalInstall, LocalInstallError};

#[test]
fn cp12_local_install_success() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("selected");
    let ssot = fixture.path().join("ssot");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&ssot).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: Example\ndescription: Local fixture\n---\nInstructions").unwrap();
    fs::create_dir(source.join("assets")).unwrap();
    fs::write(source.join("assets/data"), b"content").unwrap();
    symlink("assets/data", source.join("alias")).unwrap();
    // Merely copying a hook must never execute it.
    fs::write(source.join("hook.sh"), "exit 99\n").unwrap();
    let mut installation = LocalInstall::prepare(&ssot, &source, "fixture").unwrap();
    assert_eq!(installation.skill().name, "Example");
    assert_eq!(installation.skill().description.as_deref(), Some("Local fixture"));
    assert!(!ssot.join("fixture").exists());
    installation.publish().unwrap();
    let skill = installation.finish();
    assert_eq!(skill.source, fs::canonicalize(source).unwrap());
    assert_eq!(fs::read(ssot.join("fixture/alias")).unwrap(), b"content");
    assert!(!fs::symlink_metadata(ssot.join("fixture/alias")).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_dir(&ssot).unwrap().count(), 1);
}

#[test]
fn cp12_local_install_rejects_without_side_effects() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("source");
    let ssot = fixture.path().join("ssot");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&ssot).unwrap();
    fs::write(source.join("SKILL.md"), "# Example").unwrap();
    for name in ["", "..", "../escape", "nested/skill", "bad\\name"] {
        assert!(LocalInstall::prepare(&ssot, &source, name).is_err());
    }
    symlink("../outside", source.join("escape")).unwrap();
    fs::write(fixture.path().join("outside"), b"private").unwrap();
    assert!(matches!(LocalInstall::prepare(&ssot, &source, "fixture"), Err(LocalInstallError::UnsafeSource)));
    fs::remove_file(source.join("escape")).unwrap();
    symlink(".", source.join("cycle")).unwrap();
    assert!(LocalInstall::prepare(&ssot, &source, "fixture").is_err());
    fs::remove_file(source.join("cycle")).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: [broken\n---\n").unwrap();
    assert!(LocalInstall::prepare(&ssot, &source, "fixture").is_err());
    assert_eq!(fs::read_dir(&ssot).unwrap().count(), 0);
    fs::write(source.join("SKILL.md"), "# Example").unwrap();
    let mut installation = LocalInstall::prepare(&ssot, &source, "fixture").unwrap();
    installation.publish().unwrap();
    installation.rollback().unwrap();
    drop(installation);
    assert_eq!(fs::read_dir(&ssot).unwrap().count(), 0);
    fs::create_dir(ssot.join("fixture")).unwrap();
    fs::write(ssot.join("fixture/external"), b"untouched").unwrap();
    assert!(matches!(LocalInstall::prepare(&ssot, &source, "fixture"), Err(LocalInstallError::Conflict)));
    assert_eq!(fs::read(ssot.join("fixture/external")).unwrap(), b"untouched");
}

#[test]
fn cp12_local_install_concurrent_destination_is_preserved() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("source");
    let ssot = fixture.path().join("ssot");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&ssot).unwrap();
    fs::write(source.join("SKILL.md"), "# Example").unwrap();
    let mut installation = LocalInstall::prepare(&ssot, &source, "fixture").unwrap();
    symlink("missing", ssot.join("fixture")).unwrap();
    assert!(matches!(installation.publish(), Err(LocalInstallError::Conflict)));
    drop(installation);
    assert_eq!(fs::read_link(ssot.join("fixture")).unwrap(), std::path::Path::new("missing"));
    assert_eq!(fs::read_dir(&ssot).unwrap().count(), 1);
}
