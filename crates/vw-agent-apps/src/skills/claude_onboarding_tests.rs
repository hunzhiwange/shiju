use super::{
    claude_onboarding::{apply_claude_onboarding_skip, clear_claude_onboarding_skip},
    claude_status::ClaudeStatusError,
};
use crate::providers::live::{
    commit::{ExecutionHostId, ProviderTransactions},
    write_guard::WriteError,
};
use std::{fs, os::unix::fs::{DirBuilderExt, PermissionsExt, symlink}, path::PathBuf};
use vw_api_types::id::TenantScope;

struct Fixture {
    home: tempfile::TempDir,
    tx: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = home.path().join("private");
        fs::DirBuilder::new().mode(0o700).create(&private).unwrap();
        Self {
            tx: ProviderTransactions::new(&private, ExecutionHostId::new("fixture-host").unwrap())
                .unwrap(),
            home,
            tenant: TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        }
    }

    fn path(&self) -> PathBuf {
        self.home.path().join(".claude.json")
    }

    fn update(&self, enabled: bool) -> Result<bool, ClaudeStatusError> {
        let action = if enabled { apply_claude_onboarding_skip } else { clear_claude_onboarding_skip };
        action(self.home.path(), &self.tx, &self.tenant, "fixture-host")
    }
}

#[test]
fn cp12_claude_onboarding_success() {
    let f = Fixture::new();
    assert_eq!(f.update(false), Ok(false));
    assert!(!f.path().exists());
    assert_eq!(f.update(true), Ok(true));
    assert_eq!(fs::metadata(f.path()).unwrap().permissions().mode() & 0o777, 0o600);
    let before = fs::read(f.path()).unwrap();
    assert_eq!(f.update(true), Ok(false));
    assert_eq!(fs::read(f.path()).unwrap(), before);
    assert!(!f.home.path().join(".claude").exists());

    let preserved = r#""oauthAccount" : {"accountUuid":"fixture"}, "mcpServers":{"keep":{}}, "primaryApiKey":"synthetic-secret""#;
    for previous in ["false", "null", "1", "\"value\""] {
        fs::write(f.path(), format!("{{{preserved},\"hasCompletedOnboarding\":{previous}}}\n")).unwrap();
        assert_eq!(f.update(true), Ok(true));
        let content = fs::read_to_string(f.path()).unwrap();
        assert!(content.contains(preserved));
        assert_eq!(serde_json::from_str::<serde_json::Value>(&content).unwrap()["hasCompletedOnboarding"], true);
        assert_eq!(f.update(false), Ok(true));
        let cleared = fs::read_to_string(f.path()).unwrap();
        assert!(cleared.contains(preserved));
        assert!(serde_json::from_str::<serde_json::Value>(&cleared).unwrap().get("hasCompletedOnboarding").is_none());
        assert_eq!(f.update(false), Ok(false));
        assert_eq!(fs::read_to_string(f.path()).unwrap(), cleared);

        fs::write(f.path(), format!("{{{preserved},\"hasCompletedOnboarding\":{previous}}}")).unwrap();
        assert_eq!(f.update(false), Ok(true));
        assert!(fs::read_to_string(f.path()).unwrap().contains(preserved));
    }
    assert!(f.tx.pending(vw_api_types::agent_apps::AppId::Claude).unwrap().is_none());
}

#[test]
fn cp12_claude_onboarding_rejects_without_side_effects() {
    let f = Fixture::new();
    for before in [b"{".as_slice(), b"[]", b"null", b"", b"\xff", b"{\"a\":1,}"] {
        fs::write(f.path(), before).unwrap();
        for enabled in [true, false] {
            assert_eq!(f.update(enabled), Err(ClaudeStatusError::InvalidConfig));
            assert_eq!(fs::read(f.path()).unwrap(), before);
        }
    }
    fs::write(f.path(), b"{}").unwrap();
    assert_eq!(apply_claude_onboarding_skip(
        f.home.path(), &f.tx, &f.tenant, "remote-host",
    ), Err(ClaudeStatusError::Target(WriteError::InvalidResource)));
    assert_eq!(fs::read(f.path()).unwrap(), b"{}");
}

#[test]
fn cp12_claude_onboarding_uses_owned_override_only() {
    let f = Fixture::new();
    let selected = f.home.path().join("override");
    fs::DirBuilder::new().mode(0o700).create(&selected).unwrap();
    f.tx.set_claude_directory(f.home.path(), &f.tenant, selected.to_str()).unwrap();
    assert_eq!(f.update(true), Ok(true));
    assert!(!f.path().exists());
    assert!(!selected.join("config.json").exists());
    let path = selected.join(".claude.json");
    let before = fs::read(&path).unwrap();
    let other = TenantScope::new("fixture-platform", "other-company").unwrap();
    assert_eq!(clear_claude_onboarding_skip(
        f.home.path(), &f.tx, &other, "fixture-host",
    ), Err(ClaudeStatusError::Target(WriteError::Conflict)));
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::rename(&selected, f.home.path().join("original")).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&selected).unwrap();
    assert!(f.update(true).is_err());
    assert!(!path.exists());
}

#[test]
fn cp12_claude_onboarding_refuses_unsafe_files() {
    let f = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("profile.json");
    fs::write(&target, b"{}").unwrap();
    symlink(&target, f.path()).unwrap();
    for enabled in [true, false] {
        assert!(f.update(enabled).is_err());
    }
    assert_eq!(fs::read(target).unwrap(), b"{}");
    fs::remove_file(f.path()).unwrap();
    fs::create_dir(f.path()).unwrap();
    assert!(f.update(true).is_err());
    fs::remove_dir(f.path()).unwrap();
    fs::write(f.path(), vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(f.update(true).is_err());
    fs::write(f.path(), b"{}").unwrap();
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o400)).unwrap();
    assert!(f.update(true).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"{}");
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(f.home.path(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(f.update(true).is_err());
    fs::set_permissions(f.home.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(fs::read(f.path()).unwrap(), b"{}");
}
