use super::{
    claude_login::*,
    live::{commit::*, write_guard::WriteError},
};
use crate::auth::claude_keychain::ClaudeKeychain;
use crate::auth::claude_keychain::KeychainError;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Mutex,
};
use vw_api_types::id::TenantScope;
#[derive(Default)]
struct Keychain(Mutex<BTreeMap<Option<PathBuf>, Value>>);
impl ClaudeKeychain for Keychain {
    fn authorize_home(&self, _: &Path) -> Result<(), KeychainError> {
        Ok(())
    }
    fn read(&self, path: Option<&Path>) -> Result<Option<Value>, KeychainError> {
        Ok(self.0.lock().unwrap().get(&path.map(Path::to_path_buf)).cloned())
    }
    fn write(&self, path: Option<&Path>, value: Option<&Value>) -> Result<(), KeychainError> {
        let mut map = self.0.lock().unwrap();
        let key = path.map(Path::to_path_buf);
        if let Some(value) = value {
            map.insert(key, value.clone());
        } else {
            map.remove(&key);
        }
        Ok(())
    }
}
fn fixture() -> (tempfile::TempDir, ProviderTransactions, TenantScope, Keychain) {
    let root = tempfile::tempdir().unwrap();
    for dir in [".claude", "target", "private"] {
        std::fs::create_dir(root.path().join(dir)).unwrap();
        std::fs::set_permissions(root.path().join(dir), std::fs::Permissions::from_mode(0o700))
            .unwrap();
    }
    let tx = ProviderTransactions::new(
        &root.path().join("private"),
        ExecutionHostId::new("host").unwrap(),
    )
    .unwrap();
    let keychain = Keychain::default();
    keychain.write(None,Some(&json!({"claudeAiOauth":{"refreshToken":"fake-original"},"organizationUuid":"org","mcpOAuth":{"keep":true}}))).unwrap();
    std::fs::write(root.path().join(".claude.json"),br#"{"oauthAccount":{"accountUuid":"original","organizationUuid":"org"},"mcpServers":{"keep":true}}"#).unwrap();
    std::fs::write(root.path().join(".claude/settings.json"), b"{\"hooks\":true}").unwrap();
    (root, tx, TenantScope::new("p", "c").unwrap(), keychain)
}
fn target(root: &Path, k: &Keychain) {
    k.write(Some(&root.join("target")),Some(&json!({"claudeAiOauth":{"refreshToken":"fake-target"},"organizationUuid":"org","mcpOAuth":{"foreign":true}}))).unwrap();
    std::fs::write(
        root.join("target/.claude.json"),
        br#"{"oauthAccount":{"accountUuid":"target","organizationUuid":"org"}}"#,
    )
    .unwrap();
}
#[test]
fn cc04_claude_login_failure_leaves_pointer_and_settings() {
    let (root, tx, tenant, k) = fixture();
    let before = std::fs::read(root.path().join(".claude/settings.json")).unwrap();
    let host = LoginHost::open(root.path()).unwrap();
    assert!(host.begin(&tx, &tenant, &root.path().join("target"), &k, |_| Ok(())).is_err());
    assert_eq!(std::fs::read(root.path().join(".claude/settings.json")).unwrap(), before);
    assert!(tx.state(vw_api_types::agent_apps::AppId::Claude).unwrap().pointer.is_none());
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-original");
}
#[test]
fn cc04_claude_login_write_failure_restores_original() {
    let (root, tx, tenant, k) = fixture();
    target(root.path(), &k);
    let before = std::fs::read(root.path().join(".claude.json")).unwrap();
    let host = LoginHost::open(root.path()).unwrap();
    host.begin(&tx, &tenant, &root.path().join("target"), &k, |_| Ok(())).unwrap();
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-target");
    // A real invalid settings file is rejected before ProviderTransactions publishes any file.
    std::fs::write(root.path().join(".claude/settings.json"), b"invalid").unwrap();
    let mut file = super::claude::ClaudeFile::open(root.path(), false).unwrap();
    assert!(matches!(
        file.apply(
            &tx,
            &tenant,
            &AppState::default(),
            None,
            &json!({"env":{}}),
            Some("target".into()),
            || true
        ),
        Err(WriteError::InvalidResource)
    ));
    host.settle(&tx, &k).unwrap();
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-original");
    assert_eq!(std::fs::read(root.path().join(".claude.json")).unwrap(), before);
    assert!(!host.has_journal().unwrap());
    assert!(tx.state(vw_api_types::agent_apps::AppId::Claude).unwrap().pointer.is_none());
}

#[test]
fn cc04_claude_login_unpublished_restart_recovers_original() {
    let (root, tx, tenant, k) = fixture();
    target(root.path(), &k);
    {
        let host = LoginHost::open(root.path()).unwrap();
        host.begin(&tx, &tenant, &root.path().join("target"), &k, |_| Ok(())).unwrap();
    }
    let host = LoginHost::open(root.path()).unwrap();
    host.recover(&tx, &k).unwrap();
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-original");
    assert!(!host.has_journal().unwrap());
}
#[test]
fn cc04_claude_login_published_failure_keeps_target_and_recovers_with_pending() {
    use super::live::commit::{HOOK, Phase};
    let (root, tx, tenant, k) = fixture();
    target(root.path(), &k);
    let host = LoginHost::open(root.path()).unwrap();
    let op = host.begin(&tx, &tenant, &root.path().join("target"), &k, |_| Ok(())).unwrap();
    let mut file = super::claude::ClaudeFile::open(root.path(), false).unwrap();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(
            |_, _, phase| if phase == Phase::Published { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    let result = file.apply_with_login(
        &tx,
        &tenant,
        &AppState::default(),
        None,
        &json!({"env":{"ANTHROPIC_MODEL":"fixture"}}),
        Some("bound".into()),
        || true,
        op,
    );
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert!(result.is_err());
    assert!(host.settle(&tx, &k).is_err());
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-target");
    assert!(tx.pending(vw_api_types::agent_apps::AppId::Claude).unwrap().unwrap().published);
    drop(file);
    let outcomes = tx.recover_on_startup();
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].1.is_ok());
    host.recover(&tx, &k).unwrap();
    assert!(!host.has_journal().unwrap());
    assert_eq!(
        tx.state(vw_api_types::agent_apps::AppId::Claude).unwrap().pointer.unwrap().id,
        "bound"
    );
}
#[test]
fn cc04_claude_login_recovery_preserves_external_sign_in() {
    let (root, tx, tenant, k) = fixture();
    target(root.path(), &k);
    let host = LoginHost::open(root.path()).unwrap();
    host.begin(&tx, &tenant, &root.path().join("target"), &k, |_| Ok(())).unwrap();
    k.write(None, Some(&serde_json::json!({"claudeAiOauth":{"refreshToken":"fake-external"}})))
        .unwrap();
    assert!(host.recover(&tx, &k).is_err());
    assert!(host.has_journal().unwrap());
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-external");
}

#[test]
fn cc04_claude_login_active_cli_lock_is_preserved() {
    let (root, _, _, _) = fixture();
    let lock = root.path().join(".claude/.storage-write.lock");
    std::fs::create_dir(&lock).unwrap();
    assert!(LoginHost::open(root.path()).is_err());
    assert!(lock.is_dir());
}
#[test]
fn cc04_claude_login_file_fallback_and_empty_original_are_restored() {
    for fallback in [false, true] {
        let (root, tx, tenant, k) = fixture();
        target(root.path(), &k);
        k.write(None, None).unwrap();
        let path = root.path().join(".claude/.credentials.json");
        if fallback {
            std::fs::write(
                &path,
                br#"{"claudeAiOauth":{"refreshToken":"fake-original"},"organizationUuid":"org"}"#,
            )
            .unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        } else {
            std::fs::write(root.path().join(".claude.json"), b"{}").unwrap();
        }
        let before = std::fs::read(&path).ok();
        let host = LoginHost::open(root.path()).unwrap();
        host.begin(&tx, &tenant, &root.path().join("target"), &k, |_| Ok(())).unwrap();
        host.settle(&tx, &k).unwrap();
        assert!(k.read(None).unwrap().is_none());
        assert_eq!(std::fs::read(path).ok(), before);
    }
}
#[test]
fn cc04_claude_login_real_keychain_rejects_another_home_without_access() {
    let home = tempfile::tempdir().unwrap();
    assert!(crate::auth::claude_keychain::MacOsKeychain.authorize_home(home.path()).is_err());
}

#[test]
fn cc04_claude_login_partial_keychain_failure_restores_before_file_commit() {
    struct FailOnce {
        inner: Keychain,
        fail: std::sync::atomic::AtomicBool,
    }
    impl ClaudeKeychain for FailOnce {
        fn authorize_home(&self, _: &Path) -> Result<(), KeychainError> {
            Ok(())
        }
        fn read(&self, path: Option<&Path>) -> Result<Option<Value>, KeychainError> {
            self.inner.read(path)
        }
        fn write(&self, path: Option<&Path>, value: Option<&Value>) -> Result<(), KeychainError> {
            self.inner.write(path, value)?;
            if path.is_none() && self.fail.swap(false, std::sync::atomic::Ordering::SeqCst) {
                return Err(KeychainError);
            }
            Ok(())
        }
    }
    let (root, tx, tenant, k) = fixture();
    target(root.path(), &k);
    let before = k.read(None).unwrap();
    let profile = std::fs::read(root.path().join(".claude.json")).unwrap();
    let keychain = FailOnce { inner: k, fail: std::sync::atomic::AtomicBool::new(true) };
    let host = LoginHost::open(root.path()).unwrap();
    assert!(host.begin(&tx, &tenant, &root.path().join("target"), &keychain, |_| Ok(())).is_err());
    assert_eq!(keychain.read(None).unwrap(), before);
    assert!(!host.has_journal().unwrap());
    assert_eq!(std::fs::read(root.path().join(".claude.json")).unwrap(), profile);
    assert!(tx.state(vw_api_types::agent_apps::AppId::Claude).unwrap().pointer.is_none());
}

#[test]
fn cc04_claude_login_other_host_cannot_consume_shared_home_journal() {
    let (root, tx, tenant, k) = fixture();
    target(root.path(), &k);
    let host = LoginHost::open(root.path()).unwrap();
    host.begin(&tx, &tenant, &root.path().join("target"), &k, |_| Ok(())).unwrap();
    let alias = ProviderTransactions::new(
        &root.path().join("private"),
        ExecutionHostId::new("alias").unwrap(),
    )
    .unwrap();
    assert!(!host.journal_matches_host(&alias).unwrap());
    assert!(host.recover(&alias, &k).is_err());
    assert!(host.has_journal().unwrap());
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-target");
    assert!(host.journal_matches_host(&tx).unwrap());
    host.recover(&tx, &k).unwrap();
    assert_eq!(k.read(None).unwrap().unwrap()["claudeAiOauth"]["refreshToken"], "fake-original");
}
