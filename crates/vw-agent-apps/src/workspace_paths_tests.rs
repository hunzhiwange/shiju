use super::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory};
use std::{fs, os::unix::fs::symlink};

#[test]
fn paths_reject_symlink_escape_parent_and_file() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), home.path().join("linked")).unwrap();
    assert!(
        WorkspaceDirectory::open(
            "host",
            "host",
            home.path(),
            &home.path().join("linked/workspace"),
            WorkspaceArea::OpenClawWorkspace,
            true
        )
        .is_err()
    );
    let root = home.path().join("workspace");
    let dir = WorkspaceDirectory::open(
        "host",
        "host",
        home.path(),
        &root,
        WorkspaceArea::OpenClawMemory,
        true,
    )
    .unwrap()
    .unwrap();
    fs::write(outside.path().join("secret"), "untouched").unwrap();
    symlink(outside.path().join("secret"), root.join("memory/2026-11-14.md")).unwrap();
    assert_eq!(dir.read("2026-11-14.md"), Err(PathError::UnsafeEntry));
    assert_eq!(dir.write("2026-11-14.md", "changed"), Err(PathError::UnsafeEntry));
    assert_eq!(dir.delete("2026-11-14.md"), Err(PathError::UnsafeEntry));
    assert_eq!(fs::read_to_string(outside.path().join("secret")).unwrap(), "untouched");
}

#[test]
fn paths_reject_symlink_escape_invalid_paths_and_hosts() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    assert!(matches!(
        WorkspaceDirectory::open(
            "other",
            "host",
            home.path(),
            &root,
            WorkspaceArea::OpenClawWorkspace,
            true
        ),
        Err(PathError::UnauthorizedHost)
    ));
    assert!(!root.exists());
    assert!(
        WorkspaceDirectory::open(
            "host",
            "host",
            home.path(),
            &home.path().join("../escape"),
            WorkspaceArea::OpenClawWorkspace,
            true
        )
        .is_err()
    );
    let dir = WorkspaceDirectory::open(
        "host",
        "host",
        home.path(),
        &root,
        WorkspaceArea::OpenClawWorkspace,
        true,
    )
    .unwrap()
    .unwrap();
    for name in ["/tmp/file", "../file", "nested/file", "", ".", "..", "nested\\file"] {
        assert_eq!(dir.read(name), Err(PathError::InvalidPath));
        assert_eq!(dir.write(name, "data"), Err(PathError::InvalidPath));
        assert_eq!(dir.delete(name), Err(PathError::InvalidPath));
    }
}

#[test]
fn paths_reject_symlink_escape_missing_directories_and_io() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("hermes");
    assert!(
        WorkspaceDirectory::open(
            "host",
            "host",
            home.path(),
            &root,
            WorkspaceArea::HermesMemories,
            false
        )
        .unwrap()
        .is_none()
    );
    assert!(!root.exists());
    let dir = WorkspaceDirectory::open(
        "host",
        "host",
        home.path(),
        &root,
        WorkspaceArea::HermesMemories,
        true,
    )
    .unwrap()
    .unwrap();
    assert_eq!(dir.read("MEMORY.md").unwrap(), None);
    dir.write("MEMORY.md", "中文记忆").unwrap();
    assert_eq!(dir.read("MEMORY.md").unwrap().as_deref(), Some("中文记忆"));
    dir.delete("MEMORY.md").unwrap();
    dir.delete("MEMORY.md").unwrap();
    fs::create_dir(root.join("memories/directory")).unwrap();
    assert_eq!(dir.read("directory"), Err(PathError::UnsafeEntry));
}
