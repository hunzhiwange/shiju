use super::{
    claude_status::{ClaudeStatusError, is_claude_plugin_applied},
    claude_toggle::apply_claude_plugin_config,
};
use crate::providers::live::{
    commit::{ExecutionHostId, ProviderTransactions},
    write_guard::WriteError,
};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt, symlink},
    path::PathBuf,
};
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
        self.home.path().join(".claude/config.json")
    }

    fn write(&self, bytes: &[u8]) {
        fs::create_dir_all(self.path().parent().unwrap()).unwrap();
        fs::write(self.path(), bytes).unwrap();
    }

    fn toggle(&self, enabled: bool) -> Result<bool, ClaudeStatusError> {
        apply_claude_plugin_config(
            self.home.path(), &self.tx, &self.tenant, "fixture-host", enabled,
        )
    }
}

#[test]
fn cp12_claude_toggle_success() {
    let f = Fixture::new();
    assert_eq!(f.toggle(false), Ok(false));
    assert!(!f.path().parent().unwrap().exists());
    assert_eq!(f.toggle(true), Ok(true));
    assert!(is_claude_plugin_applied(
        f.home.path(), &f.tx, &f.tenant, "fixture-host",
    ).unwrap());
    assert_eq!(fs::metadata(f.path()).unwrap().permissions().mode() & 0o777, 0o600);
    let managed = fs::read(f.path()).unwrap();
    assert_eq!(f.toggle(true), Ok(false));
    assert_eq!(fs::read(f.path()).unwrap(), managed);
    assert_eq!(f.toggle(false), Ok(true));
    let cleared = fs::read(f.path()).unwrap();
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&cleared).unwrap(), serde_json::json!({}));
    assert_eq!(f.toggle(false), Ok(false));
    assert_eq!(fs::read(f.path()).unwrap(), cleared);

    // Preserve user fields and their exact serialized representation.
    let user_fields = r#""nested" : {"token":"synthetic-secret","items":[1, 2]}, "flag":true"#;
    f.write(format!("{{{user_fields}}}\n").as_bytes());
    assert_eq!(f.toggle(true), Ok(true));
    assert!(fs::read_to_string(f.path()).unwrap().contains(user_fields));
    assert_eq!(f.toggle(false), Ok(true));
    assert!(fs::read_to_string(f.path()).unwrap().contains(user_fields));
    assert!(f.tx.pending(vw_api_types::agent_apps::AppId::Claude).unwrap().is_none());
}

#[test]
fn cp12_claude_toggle_rejects_without_side_effects() {
    let f = Fixture::new();
    for bytes in [b"{\"token\":".as_slice(), b"[]", b"null", b"", b"\xff"] {
        f.write(bytes);
        for enabled in [true, false] {
            assert_eq!(f.toggle(enabled), Err(ClaudeStatusError::InvalidConfig));
            assert_eq!(fs::read(f.path()).unwrap(), bytes);
        }
    }
    for bytes in [
        br#"{"primaryApiKey":"synthetic-secret","other":1}"#.as_slice(),
        br#"{"primaryApiKey":null}"#,
        br#"{"primaryApiKey":"ANY"}"#,
        br#"{"primaryApiKey":123}"#,
    ] {
        f.write(bytes);
        assert_eq!(f.toggle(false), Err(ClaudeStatusError::Target(WriteError::Conflict)));
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
    }
    let before = fs::read(f.path()).unwrap();
    assert_eq!(apply_claude_plugin_config(
        f.home.path(), &f.tx, &f.tenant, "remote-host", true,
    ), Err(ClaudeStatusError::Target(WriteError::InvalidResource)));
    assert_eq!(fs::read(f.path()).unwrap(), before);
}

#[test]
fn cp12_claude_toggle_uses_only_owned_selected_directory() {
    let f = Fixture::new();
    let selected = f.home.path().join("override");
    fs::DirBuilder::new().mode(0o700).create(&selected).unwrap();
    f.tx.set_claude_directory(f.home.path(), &f.tenant, selected.to_str()).unwrap();
    assert_eq!(f.toggle(true), Ok(true));
    assert!(!f.path().exists());
    let before = fs::read(selected.join("config.json")).unwrap();
    let other = TenantScope::new("fixture-platform", "other-company").unwrap();
    assert_eq!(apply_claude_plugin_config(
        f.home.path(), &f.tx, &other, "fixture-host", false,
    ), Err(ClaudeStatusError::Target(WriteError::Conflict)));
    assert_eq!(fs::read(selected.join("config.json")).unwrap(), before);
    fs::rename(&selected, f.home.path().join("original")).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&selected).unwrap();
    assert_eq!(f.toggle(true), Err(ClaudeStatusError::Target(WriteError::InvalidResource)));
    assert!(!selected.join("config.json").exists());
}

#[test]
fn cp12_claude_toggle_rejects_unsafe_files_and_permissions() {
    let f = Fixture::new();
    f.write(b"{}");
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("config.json");
    fs::write(&secret, br#"{"primaryApiKey":"synthetic-secret"}"#).unwrap();
    fs::remove_file(f.path()).unwrap();
    symlink(&secret, f.path()).unwrap();
    assert!(f.toggle(true).is_err());
    fs::remove_file(f.path()).unwrap();
    fs::create_dir(f.path()).unwrap();
    assert!(f.toggle(true).is_err());
    fs::remove_dir(f.path()).unwrap();
    f.write(&vec![b' '; 1024 * 1024 + 1]);
    assert!(f.toggle(true).is_err());
    f.write(b"{}");
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o400)).unwrap();
    assert!(f.toggle(true).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"{}");
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(f.path().parent().unwrap(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(f.toggle(true).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"{}");
    fs::set_permissions(f.path().parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::remove_dir_all(f.path().parent().unwrap()).unwrap();
    symlink(outside.path(), f.path().parent().unwrap()).unwrap();
    assert!(f.toggle(true).is_err());
    assert_eq!(fs::read(secret).unwrap(), br#"{"primaryApiKey":"synthetic-secret"}"#);
}
