use super::{
    stage::FileChange,
    write_guard::{FilePrivacy, WriteError, lock_app},
};
use std::{
    ffi::OsStr,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
};
use vw_api_types::agent_apps::AppId;

#[test]
fn cc04_stage_failure_changes_no_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a"), b"old").unwrap();
    fs::write(dir.path().join("pointer"), b"provider-a").unwrap();
    fs::create_dir(dir.path().join("blocked")).unwrap();
    let mut guard = lock_app(AppId::Claude, dir.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    let changes = [
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Private, patch: &patch },
        FileChange { name: OsStr::new("blocked"), privacy: FilePrivacy::Private, patch: &patch },
    ];
    assert!(guard.stage(&changes).is_err());
    assert_eq!(fs::read(dir.path().join("a")).unwrap(), b"old");
    assert_eq!(fs::read(dir.path().join("pointer")).unwrap(), b"provider-a");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn cc04_stage_noop_keeps_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    let original = b"# retained\r\nkey = 'value'\r\n";
    fs::write(&path, original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let before = fs::metadata(&path).unwrap();
    let mut guard = lock_app(AppId::Codex, dir.path()).unwrap();
    let patch = |bytes: Option<&[u8]>| Ok(bytes.unwrap().to_vec());
    let changes =
        [FileChange { name: OsStr::new("config"), privacy: FilePrivacy::Private, patch: &patch }];
    let batch = guard.stage(&changes).unwrap();
    assert!(batch.is_empty());
    batch.publish().unwrap();
    let after = fs::metadata(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(
        (before.ino(), before.mode(), before.mtime(), before.mtime_nsec()),
        (after.ino(), after.mode(), after.mtime(), after.mtime_nsec())
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn cc04_stage_success_waits_for_explicit_publish_and_drop_cleans() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a"), b"old").unwrap();
    fs::set_permissions(dir.path().join("a"), fs::Permissions::from_mode(0o640)).unwrap();
    let mut guard = lock_app(AppId::Gemini, dir.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    let changes = [
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Shared, patch: &patch },
        FileChange { name: OsStr::new("b"), privacy: FilePrivacy::Private, patch: &patch },
    ];
    for publish in [false, true] {
        let batch = guard.stage(&changes).unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(fs::read(dir.path().join("a")).unwrap(), b"old");
        assert!(!dir.path().join("b").exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
        if publish {
            batch.publish().unwrap();
        } else {
            drop(batch);
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), if publish { 2 } else { 1 });
    }
    assert_eq!(fs::read(dir.path().join("a")).unwrap(), b"new");
    assert_eq!(fs::read(dir.path().join("b")).unwrap(), b"new");
    assert_eq!(fs::metadata(dir.path().join("a")).unwrap().mode() & 0o777, 0o640);
    assert_eq!(fs::metadata(dir.path().join("b")).unwrap().mode() & 0o777, 0o600);
}

#[test]
fn cc04_stage_parse_failure_and_staging_failure_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a"), b"old").unwrap();
    fs::write(dir.path().join("bad-json"), b"{").unwrap();
    let mut guard = lock_app(AppId::ClaudeDesktop, dir.path()).unwrap();
    let patch = |_: Option<&[u8]>| {
        // Every parser runs before any staging file is created.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        Ok(b"new".to_vec())
    };
    let parse = |bytes: Option<&[u8]>| {
        serde_json::from_slice::<serde_json::Value>(bytes.unwrap())
            .map(|_| b"{}".to_vec())
            .map_err(|_| WriteError::InvalidResource)
    };
    let changes = [
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Private, patch: &patch },
        FileChange { name: OsStr::new("bad-json"), privacy: FilePrivacy::Private, patch: &parse },
    ];
    assert!(guard.stage(&changes).is_err());
    assert_eq!(fs::read(dir.path().join("a")).unwrap(), b"old");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);

    // Simulate an external filesystem change after the second file was read.
    // First staging succeeds; the second sees a non-file destination and fails.
    let obstruct = |_: Option<&[u8]>| {
        fs::create_dir(dir.path().join("later")).unwrap();
        Ok(b"new".to_vec())
    };
    let changes = [
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Private, patch: &patch },
        FileChange { name: OsStr::new("later"), privacy: FilePrivacy::Private, patch: &obstruct },
    ];
    assert!(guard.stage(&changes).is_err());
    assert_eq!(fs::read(dir.path().join("a")).unwrap(), b"old");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
    assert!(dir.path().join("later").is_dir());
}

#[test]
fn cc04_stage_rejects_unsafe_and_duplicate_targets() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), b"outside").unwrap();
    symlink(outside.path().join("secret"), dir.path().join("link")).unwrap();
    let mut guard = lock_app(AppId::OpenCode, dir.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    for name in ["link", "../secret", "/secret", "", ".shiju-live-reserved", "a/b", "nul\0"] {
        let changes =
            [FileChange { name: OsStr::new(name), privacy: FilePrivacy::Private, patch: &patch }];
        assert!(guard.stage(&changes).is_err(), "{name:?}");
    }
    let changes = [
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Private, patch: &patch },
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Private, patch: &patch },
    ];
    assert!(guard.stage(&changes).is_err());
    let large = |_: Option<&[u8]>| Ok(vec![b'x'; 1024 * 1024 + 1]);
    assert!(
        guard
            .stage(&[FileChange {
                name: OsStr::new("large"),
                privacy: FilePrivacy::Private,
                patch: &large
            }])
            .is_err()
    );
    assert_eq!(fs::read(outside.path().join("secret")).unwrap(), b"outside");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
