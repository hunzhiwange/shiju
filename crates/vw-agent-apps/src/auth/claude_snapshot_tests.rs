use super::claude_keychain::{ClaudeKeychain, KeychainError, MacOsKeychain};
use crate::providers::{
    claude::ClaudeFile,
    claude_login::LoginHost,
    live::{
        commit::{AppState, ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    },
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

// Map the default slot to a unique directory-derived service, so the real macOS
// adapter never reads or writes the user's live Claude entry.
pub(super) struct Keychain {
    pub(super) live: PathBuf,
    pub(super) target: PathBuf,
    pub(super) parked: PathBuf,
}
impl ClaudeKeychain for Keychain {
    fn authorize_home(&self, home: &Path) -> Result<(), KeychainError> {
        if self.live.parent() == Some(home) { Ok(()) } else { Err(KeychainError) }
    }
    fn read(&self, directory: Option<&Path>) -> Result<Option<Value>, KeychainError> {
        MacOsKeychain.read(Some(directory.unwrap_or(&self.live)))
    }
    fn write(&self, directory: Option<&Path>, value: Option<&Value>) -> Result<(), KeychainError> {
        MacOsKeychain.write(Some(directory.unwrap_or(&self.live)), value)
    }
}
impl Drop for Keychain {
    fn drop(&mut self) {
        let _ = self.write(None, None);
        let _ = self.write(Some(&self.target), None);
        let _ = self.write(Some(&self.parked), None);
    }
}

struct Fixture {
    home: tempfile::TempDir,
    keychain: Keychain,
    tx: ProviderTransactions,
    tenant: TenantScope,
}
impl Fixture {
    fn new(shared: &str) -> Self {
        let home = tempfile::tempdir().unwrap();
        let live = home.path().join(".claude");
        let target = home.path().join("managed");
        let parked = home.path().join("parked");
        let private = home.path().join("private");
        for dir in [&live, &target, &parked, &private] {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new().mode(0o700).create(dir).unwrap();
        }
        let keychain = Keychain { live, target, parked };
        assert!(keychain.read(None).unwrap().is_none());
        assert!(keychain.read(Some(&keychain.target)).unwrap().is_none());
        let before = json!({
            "claudeAiOauth":{"refreshToken":"synthetic-before"},
            "organizationUuid":"fixture-org",
            "mcpOAuth":{"shared":shared},
            "trustedDeviceToken":"synthetic-obsolete"
        });
        assert!(keychain.write(None, Some(&before)).is_ok());
        fs::write(
            home.path().join(".claude.json"),
            br#"{"oauthAccount":{"accountUuid":"before","organizationUuid":"fixture-org"},"mcpServers":{"kept":true},"modelAccessCache":{"obsolete":true}}"#,
        ).unwrap();
        fs::write(home.path().join(".claude/settings.json"), b"{}").unwrap();
        let tx = ProviderTransactions::new(&private, ExecutionHostId::new("fixture-host").unwrap())
            .unwrap();
        Self {
            home,
            keychain,
            tx,
            tenant: TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        }
    }
    fn target(&self, owner: &str, refresh: &str) {
        let value = json!({
            "claudeAiOauth":{"refreshToken":refresh},
            "organizationUuid":"fixture-org",
            "mcpOAuth":{"foreign":true}
        });
        assert!(self.keychain.write(Some(&self.keychain.target), Some(&value)).is_ok());
        fs::write(
            self.keychain.target.join(".claude.json"),
            json!({"oauthAccount":{"accountUuid":owner,"organizationUuid":"fixture-org"},"mcpServers":{"foreign":true}}).to_string(),
        ).unwrap();
    }
    fn profile(&self) -> PathBuf {
        self.home.path().join(".claude.json")
    }
    fn live_has(&self, refresh: &str, shared: &str, owner: &str) -> bool {
        let credential = self.keychain.read(None).unwrap().unwrap();
        let profile: Value = serde_json::from_slice(&fs::read(self.profile()).unwrap()).unwrap();
        credential.pointer("/claudeAiOauth/refreshToken").and_then(Value::as_str) == Some(refresh)
            && credential.pointer("/mcpOAuth/shared").and_then(Value::as_str) == Some(shared)
            && credential.get("trustedDeviceToken").is_none()
            && credential.pointer("/mcpOAuth/foreign").is_none()
            && profile.pointer("/oauthAccount/accountUuid").and_then(Value::as_str) == Some(owner)
            && profile["mcpServers"] == json!({"kept":true})
            && profile.get("modelAccessCache").is_none()
    }
}

#[test]
fn auth6_claude_switch_commits_expected_scoped_result() {
    let f = Fixture::new("shared-live");
    f.target("target", "synthetic-target");
    {
        let host = LoginHost::open(f.home.path()).unwrap();
        let op = host
            .begin(&f.tx, &f.tenant, &f.keychain.target, &f.keychain, |previous| {
                assert_eq!(previous.identity().unwrap().as_deref(), Some("before:fixture-org"));
                previous.archive(&f.keychain.parked, &f.keychain)
            })
            .unwrap();
        assert!(op.is_some());
        ClaudeFile::open(f.home.path(), false)
            .unwrap()
            .apply_with_login(
                &f.tx,
                &f.tenant,
                &AppState::default(),
                None,
                &json!({"env":{}}),
                Some("bound".into()),
                || true,
                op,
            )
            .unwrap();
        host.settle(&f.tx, &f.keychain).unwrap();
        assert!(!host.has_journal().unwrap());
        assert!(f.live_has("synthetic-target", "shared-live", "target"));
    }
    f.target("target", "synthetic-fresh");
    let host = LoginHost::open(f.home.path()).unwrap();
    assert_eq!(
        host.replace_live(&f.tx, &f.tenant, &f.keychain.target, &f.keychain),
        Ok(true),
        "a re-login must replace the old token only for the live account",
    );
    assert!(!host.has_journal().unwrap());
    assert!(f.live_has("synthetic-fresh", "shared-live", "target"));
    let parked = f.keychain.read(Some(&f.keychain.parked)).unwrap().unwrap();
    assert!(
        parked.pointer("/claudeAiOauth/refreshToken").and_then(Value::as_str)
            == Some("synthetic-before")
    );
    assert!(parked.get("mcpOAuth").is_none());
    assert_eq!(f.tx.state(AppId::Claude).unwrap().pointer.unwrap().id, "bound");
    f.keychain.write(None, None).unwrap();
    f.keychain.write(Some(&f.keychain.target), None).unwrap();
    assert!(f.keychain.read(None).unwrap().is_none());
    assert!(f.keychain.read(Some(&f.keychain.target)).unwrap().is_none());
}

#[test]
fn auth6_claude_switch_rejects_invalid_or_stale_input() {
    let shared = "s".repeat(1350);
    let f = Fixture::new(&shared);
    f.target("target", "synthetic-target");
    let before = f.keychain.read(None).unwrap();
    let profile = fs::read(f.profile()).unwrap();
    let host = LoginHost::open(f.home.path()).unwrap();
    assert_eq!(
        host.begin(&f.tx, &f.tenant, &f.keychain.target, &f.keychain, |_| {
            f.target("target", "synthetic-rotated-during-archive");
            Ok(())
        })
        .err(),
        Some(WriteError::Conflict),
        "a stale target must be refused before the live login is changed",
    );
    assert!(f.keychain.read(None).unwrap() == before);
    assert_eq!(fs::read(f.profile()).unwrap(), profile);
    assert!(!host.has_journal().unwrap());
    drop(host);

    // Both entries individually fit the adapter's stdin limit. The merged
    // live entry is too large and the real adapter must reject it.
    f.target("target", &"t".repeat(1000));
    let host = LoginHost::open(f.home.path()).unwrap();
    assert_eq!(
        host.begin(&f.tx, &f.tenant, &f.keychain.target, &f.keychain, |_| Ok(())).err(),
        Some(WriteError::Io),
    );
    assert!(f.keychain.read(None).unwrap() == before);
    assert_eq!(fs::read(f.profile()).unwrap(), profile);
    assert!(!host.has_journal().unwrap());
    assert!(f.tx.state(AppId::Claude).unwrap().pointer.is_none());
    drop(host);

    let host = LoginHost::open(f.home.path()).unwrap();
    assert_eq!(host.replace_live(&f.tx, &f.tenant, &f.keychain.target, &f.keychain), Ok(false));
    assert!(f.keychain.read(None).unwrap() == before);
    assert_eq!(fs::read(f.profile()).unwrap(), profile);
    assert!(!host.has_journal().unwrap());
    drop(host);

    f.target("before", &"t".repeat(1000));
    let host = LoginHost::open(f.home.path()).unwrap();
    assert_eq!(
        host.replace_live(&f.tx, &f.tenant, &f.keychain.target, &f.keychain),
        Err(WriteError::Io)
    );
    assert!(f.keychain.read(None).unwrap() == before);
    assert_eq!(fs::read(f.profile()).unwrap(), profile);
    assert!(!host.has_journal().unwrap());
}
