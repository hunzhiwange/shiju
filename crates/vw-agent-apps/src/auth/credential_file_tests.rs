use super::credential_file::{AccountCredential, CredentialFileStore};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

fn private_dir(path: &Path) {
    fs::DirBuilder::new().mode(0o700).create(path).unwrap();
}

fn claude(value: &str) -> AccountCredential {
    AccountCredential::ClaudeToken(value.into())
}

fn credential_path(root: &Path) -> PathBuf {
    let mut path = root.to_owned();
    while path.is_dir() {
        let mut entries = fs::read_dir(&path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|entry| entry.extension().is_none());
        path = entries.next().unwrap();
        assert!(entries.next().is_none());
    }
    path
}

#[test]
fn auth6_file_store_commits_expected_scoped_result() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let owner = store.for_owner("member").expect("trusted owner must get a managed file scope");
    let tenant = TenantScope::new("p", "c").unwrap();
    let save = |store: &CredentialFileStore, value| {
        store.save(&tenant, "host", AppId::Claude, "account", &claude(value))
    };
    save(&owner, "first-fixture").unwrap();
    let path = credential_path(root.path());
    for parent in path.ancestors().skip(1).take_while(|parent| *parent != root.path()) {
        assert_eq!(fs::metadata(parent).unwrap().mode() & 0o777, 0o700);
    }
    let first = fs::File::open(&path).unwrap();
    save(&owner, "second-fixture").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"second-fixture");
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    assert_ne!(first.metadata().unwrap().ino(), fs::metadata(&path).unwrap().ino());
    use std::io::Read as _;
    let mut old = Vec::new();
    (&first).read_to_end(&mut old).unwrap();
    assert_eq!(old, b"first-fixture");
    std::thread::scope(|threads| {
        let writer = threads.spawn(|| {
            for _ in 0..16 {
                save(&owner, "third-fixture").unwrap();
                save(&owner, "second-fixture").unwrap();
            }
        });
        for _ in 0..96 {
            assert!(matches!(
                owner.read(&tenant, "host", AppId::Claude, "account"),
                Ok(AccountCredential::ClaudeToken(value))
                    if matches!(value.as_str(), "second-fixture" | "third-fixture")
            ));
        }
        writer.join().unwrap();
    });

    let reopened = CredentialFileStore::new(root.path()).unwrap().for_owner("member").unwrap();
    assert!(matches!(
        reopened.read(&tenant, "host", AppId::Claude, "account"),
        Ok(AccountCredential::ClaudeToken(value)) if value == "second-fixture"
    ));
    let other_owner = store.for_owner("another-member").unwrap();
    for (scope, host, app, id) in [
        (TenantScope::new("other", "c").unwrap(), "host", AppId::Claude, "account"),
        (TenantScope::new("p", "other").unwrap(), "host", AppId::Claude, "account"),
        (tenant.clone(), "other-host", AppId::Claude, "account"),
        (tenant.clone(), "host", AppId::Codex, "account"),
        (tenant.clone(), "host", AppId::Claude, "other-account"),
    ] {
        assert!(reopened.read(&scope, host, app, id).is_err());
    }
    assert!(other_owner.read(&tenant, "host", AppId::Claude, "account").is_err());
    assert!(store.read(&tenant, "host", AppId::Claude, "account").is_err());
    save(&other_owner, "other-owner-fixture").unwrap();
    assert!(matches!(
        reopened.read(&tenant, "host", AppId::Claude, "account"),
        Ok(AccountCredential::ClaudeToken(value)) if value == "second-fixture"
    ));
}

#[test]
fn auth6_file_store_rejects_invalid_or_stale_input() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let save = |id, value| store.save(&tenant, "host", AppId::Claude, id, &claude(value));
    assert!(save("../outside", "fixture").is_err());
    assert!(store.save(&tenant, "../host", AppId::Claude, "account", &claude("fixture")).is_err());
    assert_eq!(
        store.save(&tenant, "host", AppId::Agy, "account", &claude("fixture")),
        Err("agy official accounts require an isolated OS keychain execution host".into())
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    save("account", "old-fixture").unwrap();
    let path = credential_path(root.path());
    let external = tempfile::tempdir().unwrap();
    let outside = external.path().join("outside");
    let backup = external.path().join("backup");
    let linked_root = external.path().join("linked-root");
    symlink(root.path(), &linked_root).unwrap();
    assert!(CredentialFileStore::new(&linked_root).is_err());
    let legacy_alias = external.path().join("alias-store");
    private_dir(&legacy_alias);
    let alias_store = CredentialFileStore::new(&legacy_alias).unwrap();
    alias_store.save(&tenant, "host", AppId::Claude, "aaa", &claude("old-fixture")).unwrap();
    // "aaa" and "aaG" encode to YWFh and YWFH in the existing launch layout.
    assert!(alias_store.read(&tenant, "host", AppId::Claude, "aaG").is_err());
    #[cfg(target_os = "macos")]
    if legacy_alias.join("cA/Yw/aG9zdA/Y2xhdWRl/YWFH").exists() {
        assert!(
            alias_store.save(&tenant, "host", AppId::Claude, "aaG", &claude("fixture")).is_err()
        );
    }
    fs::rename(&path, &backup).unwrap();
    symlink(&outside, &path).unwrap();
    assert!(save("account", "new-fixture").is_err(), "dangling symlink must be refused");
    assert!(fs::symlink_metadata(&path).unwrap().is_symlink());
    assert!(!outside.exists());
    fs::remove_file(&path).unwrap();
    fs::hard_link(&backup, &path).unwrap();
    assert!(save("account", "new-fixture").is_err(), "shared inode must be refused");
    fs::remove_file(&path).unwrap();
    fs::rename(&backup, &path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(store.read(&tenant, "host", AppId::Claude, "account").is_err());
    assert!(save("account", "new-fixture").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"old-fixture");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

    let parent = path.parent().unwrap();
    #[cfg(target_os = "macos")]
    {
        let chmod = |mode, acl| {
            assert!(
                std::process::Command::new("/bin/chmod")
                    .args([mode, acl])
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        chmod("+a", "everyone allow read");
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        assert!(store.read(&tenant, "host", AppId::Claude, "account").is_err());
        assert!(save("account", "new-fixture").is_err());
        chmod("-a", "everyone allow read");
        assert!(store.read(&tenant, "host", AppId::Claude, "account").is_ok());

        let chflags = |flag| {
            assert!(
                std::process::Command::new("/usr/bin/chflags")
                    .arg(flag)
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        let entries = fs::read_dir(parent).unwrap().count();
        let before = fs::metadata(&path).unwrap().ino();
        chflags("uchg");
        let failed = save("account", "new-fixture");
        chflags("nouchg");
        assert_eq!(failed, Err("official credential storage unavailable".into()));
        assert_eq!(fs::metadata(&path).unwrap().ino(), before);
        assert_eq!(fs::read(&path).unwrap(), b"old-fixture");
        assert_eq!(fs::read_dir(parent).unwrap().count(), entries);
    }
    fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(save("account", "new-fixture").is_err());
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).unwrap();
    let lease = store.lease(&tenant, "host", AppId::Claude, "account", false).unwrap();
    let moved = external.path().join("moved");
    fs::rename(parent, &moved).unwrap();
    symlink(&moved, parent).unwrap();
    assert!(store.read(&tenant, "host", AppId::Claude, "account").is_err());
    assert!(lease.read().is_err());
    assert!(lease.save(&claude("new-fixture")).is_err());
    fs::remove_file(parent).unwrap();
    private_dir(parent);
    assert!(lease.save(&claude("new-fixture")).is_err(), "a lease must pin the original slot");
    assert_eq!(fs::read(moved.join(path.file_name().unwrap())).unwrap(), b"old-fixture");
    assert_eq!(fs::read_dir(parent).unwrap().count(), 0);
}
