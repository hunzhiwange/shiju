use super::claude_status::{
    ClaudeConfigProjection, ClaudeConfigStatus, ClaudeStatusError, get_claude_plugin_status,
    is_claude_plugin_applied, read_claude_plugin_config,
};
use crate::providers::live::{
    commit::{ExecutionHostId, ProviderTransactions},
    write_guard::WriteError,
};
use std::{fs, os::unix::fs::{DirBuilderExt, symlink}, path::PathBuf};
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
    fn status(&self) -> Result<ClaudeConfigStatus, ClaudeStatusError> {
        get_claude_plugin_status(self.home.path(), &self.tx, &self.tenant, "fixture-host")
    }
}

#[test]
fn cp12_claude_status_success() {
    let f = Fixture::new();
    assert_eq!(f.status(), Ok(ClaudeConfigStatus::Missing));
    assert!(!f.path().parent().unwrap().exists());
    for (bytes, present, managed) in [
        (br#"{}"#.as_slice(), false, false),
        (br#"{"primaryApiKey":"any","nested":{"token":"synthetic-secret"}}"#, true, true),
        (br#"{"primaryApiKey":"synthetic-secret"}"#, true, false),
        (br#"{"primaryApiKey":null}"#, true, false),
        (br#"{"primaryApiKey":"ANY"}"#, true, false),
    ] {
        f.write(bytes);
        let expected = ClaudeConfigProjection { primary_api_key_present: present, managed };
        assert_eq!(f.status(), Ok(ClaudeConfigStatus::Valid(expected)));
        assert_eq!(read_claude_plugin_config(f.home.path(), &f.tx, &f.tenant, "fixture-host"), Ok(Some(expected)));
        assert_eq!(is_claude_plugin_applied(f.home.path(), &f.tx, &f.tenant, "fixture-host"), Ok(managed));
        let public = serde_json::to_string(&f.status().unwrap()).unwrap();
        assert!(!public.contains("synthetic-secret"));
        assert!(!public.contains("nested"));
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
    }
}

#[test]
fn cp12_claude_status_invalid_documents_are_explicit() {
    let f = Fixture::new();
    for bytes in [b"{\"synthetic-secret\":".as_slice(), b"[]", b"null", b"\xff"] {
        f.write(bytes);
        assert_eq!(f.status(), Ok(ClaudeConfigStatus::Invalid));
        assert_eq!(read_claude_plugin_config(f.home.path(), &f.tx, &f.tenant, "fixture-host"), Err(ClaudeStatusError::InvalidConfig));
        assert_eq!(is_claude_plugin_applied(f.home.path(), &f.tx, &f.tenant, "fixture-host"), Err(ClaudeStatusError::InvalidConfig));
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
    }
}

#[test]
fn cp12_claude_status_rejects_without_side_effects() {
    let f = Fixture::new();
    let selected = f.home.path().join("override");
    fs::DirBuilder::new().mode(0o700).create(&selected).unwrap();
    fs::write(selected.join("config.json"), br#"{"primaryApiKey":"any"}"#).unwrap();
    f.tx.set_claude_directory(f.home.path(), &f.tenant, selected.to_str()).unwrap();
    assert_eq!(f.status(), Ok(ClaudeConfigStatus::Valid(ClaudeConfigProjection {
        primary_api_key_present: true, managed: true,
    })));
    let other = TenantScope::new("fixture-platform", "other-company").unwrap();
    assert_eq!(get_claude_plugin_status(f.home.path(), &f.tx, &other, "fixture-host"), Err(ClaudeStatusError::Target(WriteError::Conflict)));
    assert_eq!(get_claude_plugin_status(f.home.path(), &f.tx, &f.tenant, "remote-host"), Err(ClaudeStatusError::Target(WriteError::InvalidResource)));
    fs::rename(&selected, f.home.path().join("original")).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&selected).unwrap();
    assert_eq!(f.status(), Err(ClaudeStatusError::Target(WriteError::InvalidResource)));
    assert!(!f.path().exists());
    assert!(!selected.join("config.json").exists());
}

#[test]
fn cp12_claude_status_links_special_files_and_large_files_are_rejected() {
    let f = Fixture::new();
    f.write(b"{}");
    fs::remove_file(f.path()).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("config.json");
    fs::write(&secret, br#"{"primaryApiKey":"synthetic-secret"}"#).unwrap();
    symlink(&secret, f.path()).unwrap();
    assert!(f.status().is_err());
    fs::remove_file(f.path()).unwrap();
    fs::create_dir(f.path()).unwrap();
    assert!(f.status().is_err());
    fs::remove_dir(f.path()).unwrap();
    f.write(&vec![b' '; 1024 * 1024 + 1]);
    assert!(f.status().is_err());
    fs::remove_dir_all(f.path().parent().unwrap()).unwrap();
    symlink(outside.path(), f.path().parent().unwrap()).unwrap();
    assert!(f.status().is_err());
    assert_eq!(fs::read(secret).unwrap(), br#"{"primaryApiKey":"synthetic-secret"}"#);
}
