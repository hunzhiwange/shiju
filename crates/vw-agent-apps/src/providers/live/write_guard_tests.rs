use super::write_guard::{FilePrivacy, lock_app};
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt, sync::mpsc, time::Duration};
use vw_api_types::agent_apps::AppId;

#[test]
fn cc04_write_guard_serializes_same_app() {
    let directory = tempfile::tempdir().unwrap();
    let mut first = lock_app(AppId::Claude, directory.path()).unwrap();
    let (started, starting) = mpsc::channel();
    let (entered, entering) = mpsc::channel();
    let path = directory.path().to_owned();
    let worker = std::thread::spawn(move || {
        started.send(()).unwrap();
        let mut second = lock_app(AppId::Claude, &path).unwrap();
        second.write(OsStr::new("settings.json"), b"second", FilePrivacy::Private).unwrap();
        entered.send(()).unwrap();
    });
    starting.recv().unwrap();
    let waiting = entering.recv_timeout(Duration::from_millis(100)).is_err();
    first.write(OsStr::new("settings.json"), b"first", FilePrivacy::Private).unwrap();
    drop(first);
    worker.join().unwrap();
    assert!(waiting, "second concurrent writer must wait until the first releases");
    assert_eq!(fs::read(directory.path().join("settings.json")).unwrap(), b"second");
}

#[test]
fn cc04_write_guard_private_files_are_0600() {
    let directory = tempfile::tempdir().unwrap();
    let mut guard = lock_app(AppId::Codex, directory.path()).unwrap();
    let path = directory.path().join("auth.json");
    fs::write(&path, b"old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    guard.write(OsStr::new("auth.json"), b"fixture-secret", FilePrivacy::Private).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(fs::read(&path).unwrap(), b"fixture-secret");
    guard.write(OsStr::new("new.json"), b"new", FilePrivacy::Private).unwrap();
    assert_eq!(
        fs::metadata(directory.path().join("new.json")).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn cc04_write_guard_shared_modes_and_rejections_preserve_files() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let mut guard = lock_app(AppId::Gemini, directory.path()).unwrap();
    for mode in [0o400, 0o600, 0o640, 0o644] {
        let path = directory.path().join("shared.json");
        fs::write(&path, b"before").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        guard.write(OsStr::new("shared.json"), b"after", FilePrivacy::Shared).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, mode);
        assert_eq!(fs::read(&path).unwrap(), b"after");
        // Restore owner-write for the next fixture, independently of replacement.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    guard.write(OsStr::new("new-shared"), b"new", FilePrivacy::Shared).unwrap();
    assert_eq!(
        fs::metadata(directory.path().join("new-shared")).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret");
    fs::write(&secret, b"untouched").unwrap();
    symlink(&secret, directory.path().join("link")).unwrap();
    fs::create_dir(directory.path().join("folder")).unwrap();
    assert!(
        fs::read_dir(directory.path()).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .as_encoded_bytes()
            .starts_with(b".shiju-live-"))
    );
    let count = fs::read_dir(directory.path()).unwrap().count();
    for name in ["", ".", "..", "../secret", "/tmp/secret", "a/b", "a/", "a\0b", "link", "folder"] {
        assert!(
            guard.write(OsStr::new(name), b"rejected", FilePrivacy::Private).is_err(),
            "{name:?}"
        );
    }
    assert_eq!(fs::read(&secret).unwrap(), b"untouched");
    assert_eq!(fs::read(directory.path().join("shared.json")).unwrap(), b"after");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), count, "no temporary residue");
    assert!(fs::symlink_metadata(directory.path().join("link")).unwrap().file_type().is_symlink());
    assert!(lock_app(AppId::Gemini, &directory.path().join("missing")).is_err());
    symlink(directory.path(), outside.path().join("directory-link")).unwrap();
    assert!(lock_app(AppId::Gemini, &outside.path().join("directory-link")).is_err());
}

#[test]
fn cc04_write_guard_physical_resource_and_app_ownership_serialize() {
    // Distinct app IDs still conflict on the same physical directory, reached via an alias.
    let directory = tempfile::tempdir().unwrap();
    let first = lock_app(AppId::Claude, directory.path()).unwrap();
    let alias = directory.path().join(".");
    let (started, starting) = mpsc::channel();
    let (entered, entering) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        started.send(()).unwrap();
        let mut guard = lock_app(AppId::Codex, &alias).unwrap();
        guard.write(OsStr::new("shared"), b"serialized", FilePrivacy::Private).unwrap();
        entered.send(()).unwrap();
    });
    starting.recv().unwrap();
    let waiting = entering.recv_timeout(Duration::from_millis(100)).is_err();
    drop(first);
    worker.join().unwrap();
    assert!(waiting, "resource ownership must serialize even across app identities");
    assert_eq!(fs::read(directory.path().join("shared")).unwrap(), b"serialized");

    // Same application conflicts across directories; unrelated applications/resources progress.
    let other = tempfile::tempdir().unwrap();
    let first = lock_app(AppId::Claude, directory.path()).unwrap();
    let unrelated = lock_app(AppId::Gemini, other.path()).unwrap();
    drop(unrelated);
    let path = other.path().to_owned();
    let (entered, entering) = mpsc::channel();
    let (started, starting) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        started.send(()).unwrap();
        let _guard = lock_app(AppId::Claude, &path).unwrap();
        entered.send(()).unwrap();
    });
    starting.recv().unwrap();
    let waiting = entering.recv_timeout(Duration::from_millis(100)).is_err();
    drop(first);
    worker.join().unwrap();
    assert!(waiting, "application lock belongs to the execution host, not one directory");
}

#[test]
fn cc04_write_guard_directory_anchor_and_unwind_release() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("native");
    fs::create_dir(&directory).unwrap();
    let moved = root.path().join("moved");
    let result = std::panic::catch_unwind(|| {
        let mut guard = lock_app(AppId::Codex, &directory).unwrap();
        fs::rename(&directory, &moved).unwrap();
        fs::create_dir(&directory).unwrap();
        guard.write(OsStr::new("auth.json"), b"anchored", FilePrivacy::Private).unwrap();
        panic!("exercise RAII release");
    });
    assert!(result.is_err());
    assert_eq!(fs::read(moved.join("auth.json")).unwrap(), b"anchored");
    assert!(!directory.join("auth.json").exists());
    let mut guard = lock_app(AppId::Codex, &moved).unwrap();
    guard.write(OsStr::new("auth.json"), b"released", FilePrivacy::Private).unwrap();
    assert_eq!(fs::read(moved.join("auth.json")).unwrap(), b"released");
}
