use super::*;
use sha2::{Digest, Sha256};

fn asset(kind: InstallerKind) -> DesktopInstaller {
    DesktopInstaller {
        name: format!("installer.{}", extension(kind)),
        target: String::new(),
        kind,
        url: String::new(),
        size: 8,
        sha256: hex::encode(Sha256::digest(b"verified")),
    }
}
fn fixture(kind: InstallerKind) -> (tempfile::TempDir, PreparedDesktopUpdate) {
    let root = tempfile::tempdir().unwrap();
    let directory = create_attempt(root.path(), "1.2.3").unwrap();
    let asset = asset(kind);
    let path = directory.path().join(&asset.name);
    std::fs::write(&path, b"verified").unwrap();
    let ready = finish_download(directory, path, &asset).unwrap();
    (root, ready)
}
#[test]
fn installer_update_six_platforms_use_expected_extensions() {
    for (os, kind) in [
        ("macos", InstallerKind::Dmg),
        ("windows", InstallerKind::Msi),
        ("linux", InstallerKind::Rpm),
    ] {
        for arch in ["x86_64", "aarch64"] {
            assert!(desktop_target(os, arch).unwrap().starts_with(arch));
            let (_root, ready) = fixture(kind);
            open_with(&ready, os, |path| {
                assert_eq!(path.extension().unwrap(), extension(kind));
                assert!(path.is_absolute());
                Ok(())
            })
            .unwrap();
        }
    }
    assert!(desktop_target("linux", "arm").is_err());
}
#[test]
fn installer_update_unicode_cache_and_retries_preserve_files() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("中文 cache");
    std::fs::create_dir(&cache).unwrap();
    let first = create_attempt(&cache, "1.2.3").unwrap();
    let second = create_attempt(&cache, "1.2.3").unwrap();
    assert_ne!(first.path(), second.path());
    assert!(first.path().starts_with(cache.canonicalize().unwrap().join("updates/v1.2.3")));
    let asset = asset(InstallerKind::Dmg);
    let path = first.path().join(&asset.name);
    std::fs::write(&path, b"verified").unwrap();
    let ready = finish_download(first, path.clone(), &asset).unwrap();
    drop(second);
    assert!(
        open_with(&ready, "macos", |_| anyhow::bail!("no handler"))
            .unwrap_err()
            .to_string()
            .contains(path.to_str().unwrap())
    );
    open_with(&ready, "macos", |_| Ok(())).unwrap();
    drop(ready);
    assert_eq!(std::fs::read(path).unwrap(), b"verified");
}
#[test]
fn installer_update_unverified_and_modified_files_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let directory = create_attempt(root.path(), "1.2.3").unwrap();
    let path = directory.path().join("installer.dmg");
    std::fs::write(&path, b"partial").unwrap();
    assert!(finish_download(directory, path.clone(), &asset(InstallerKind::Dmg)).is_err());
    assert!(!path.exists());
    let (_root, ready) = fixture(InstallerKind::Dmg);
    std::fs::write(ready.path(), b"tampered").unwrap();
    assert!(open_with(&ready, "macos", |_| panic!("must not open")).is_err());
}
#[test]
fn installer_update_missing_download_and_wrong_platform_never_open() {
    let root = tempfile::tempdir().unwrap();
    let directory = create_attempt(root.path(), "1.2.3").unwrap();
    let path = directory.path().join("installer.dmg");
    assert!(finish_download(directory, path, &asset(InstallerKind::Dmg)).is_err());
    let (_root, ready) = fixture(InstallerKind::Rpm);
    assert!(open_with(&ready, "windows", |_| panic!("must not open")).is_err());
    let message = open_with(&ready, "linux", |_| Ok(())).unwrap();
    assert!(message.contains("RPM"));
    assert!(!message.contains("升级完成"));
}
#[cfg(unix)]
#[test]
fn installer_update_symlink_escape_is_rejected() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), root.path().join("updates")).unwrap();
    assert!(create_attempt(root.path(), "1.2.3").is_err());
    let (_root, ready) = fixture(InstallerKind::Dmg);
    let external = outside.path().join("installer.dmg");
    std::fs::write(&external, b"verified").unwrap();
    std::fs::remove_file(ready.path()).unwrap();
    symlink(&external, ready.path()).unwrap();
    assert!(open_with(&ready, "macos", |_| panic!("must not open")).is_err());
}
