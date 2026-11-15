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
    let ready = finish_download(directory, path, &asset, "1.2.3").unwrap();
    (root, ready)
}
#[test]
fn installer_update_six_platforms_use_expected_extensions() {
    for (os, kind) in [
        ("macos", InstallerKind::Dmg),
        ("windows", InstallerKind::Msi),
        ("linux", InstallerKind::Rpm),
        ("linux", InstallerKind::Deb),
    ] {
        for arch in ["x86_64", "aarch64"] {
            assert!(desktop_target(os, arch).unwrap().starts_with(arch));
            let (_root, ready) = fixture(kind);
            open_with(&ready, os, |path| {
                assert_eq!(path, ready.path());
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
    let ready = finish_download(first, path.clone(), &asset, "1.2.3").unwrap();
    drop(second);
    let error = open_with(&ready, "macos", |_| {
        Err(anyhow::anyhow!("no handler").context("helper launch failed"))
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("helper launch failed: no handler"));
    assert!(!error.contains("安装包已就绪"));
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
    assert!(finish_download(directory, path.clone(), &asset(InstallerKind::Dmg), "1.2.3").is_err());
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
    assert!(finish_download(directory, path, &asset(InstallerKind::Dmg), "1.2.3").is_err());
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

#[test]
fn installer_update_linux_authorization_failure_retains_verified_retry() {
    let (_root, ready) = fixture(InstallerKind::Rpm);
    let error = open_with(&ready, "linux", |path| {
        assert_eq!(path, ready.path());
        Err(anyhow::anyhow!("authorization unavailable"))
    })
    .unwrap_err();
    assert!(error.to_string().contains("authorization unavailable"));
    let message = open_with(&ready, "linux", |path| {
        assert!(path.is_file());
        assert_eq!(path, ready.path());
        Ok(())
    })
    .unwrap();
    assert!(message.contains("自动更新已准备就绪"));
    assert!(message.contains("正在退出"));
    // Recheck the package before handing it to the independent updater.
    std::fs::write(ready.path(), b"tampered").unwrap();
    assert!(open_with(&ready, "linux", |_| panic!("must not open")).is_err());
}

#[test]
fn installer_update_macos_retained_package_allows_retry() {
    let (_root, ready) = fixture(InstallerKind::Dmg);
    let message = open_with(&ready, "macos", |path| {
        assert_eq!(path, ready.path());
        Ok(())
    })
    .unwrap();
    assert!(message.contains("正在退出"));
}
