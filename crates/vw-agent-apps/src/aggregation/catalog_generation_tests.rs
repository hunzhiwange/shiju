use super::catalog_generation::*;
use crate::providers::{
    codex_catalog::{CATALOG_FILENAME, CodexFiles},
    live::{commit::*, write_guard::WriteError},
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

struct Fixture {
    home: tempfile::TempDir,
    _root: tempfile::TempDir,
    tx: ProviderTransactions,
    scope: ProjectScope,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let scope =
            ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from("project"))
                .unwrap();
        let mut files = CodexFiles::open(home.path()).unwrap();
        tx.commit_project_checked(
            &scope,
            &mut files.guard,
            &AppState::default(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
        drop(files);
        let f = Self { home, _root: root, tx, scope };
        f.write("config.toml", &format!("model_catalog_json = '{CATALOG_FILENAME}'\n"));
        f.write(CATALOG_FILENAME, "catalog-a");
        f.login("user-a", "token-a");
        f
    }
    fn write(&self, name: &str, value: &str) {
        fs::write(self.home.path().join(".codex").join(name), value).unwrap();
    }
    fn login(&self, user: &str, token: &str) {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload =
            URL_SAFE_NO_PAD.encode(serde_json::json!({"sub":user,"nonce":token}).to_string());
        self.write("auth.json", &serde_json::json!({"tokens":{"account_id":"shared-workspace","id_token":format!("{header}.{payload}.signature"),"access_token":token}}).to_string());
    }
    fn observe(&self, time: u64) -> Snapshot {
        observe(&mut CodexFiles::open(self.home.path()).unwrap(), &self.tx, &self.scope, time)
            .unwrap()
    }
    fn read(&self) -> Snapshot {
        read(&CodexFiles::open(self.home.path()).unwrap(), &self.tx, &self.scope).unwrap()
    }
}
#[test]
fn aggregation_t26_catalog_generation_preserves_contract() {
    let f = Fixture::new();
    let first = f.observe(10_000);
    assert_eq!(first.generation_counts(), (1, 1));
    assert!(!first.unverified());
    let unchanged = f.observe(20_000);
    assert_eq!(first.revision(), unchanged.revision());
    assert_eq!(unchanged.generation_counts(), (1, 1));
    f.write("config.toml", "");
    f.observe(30_000);
    f.write("config.toml", &format!("model_catalog_json='{CATALOG_FILENAME}'"));
    let reconnected = f.observe(40_000);
    assert_eq!(first.revision(), reconnected.revision());
    assert!(!reconnected.catalog_stale_at(15_000));
    assert!(reconnected.catalog_stale_at(35_000));
    assert_eq!(reconnected.generation_counts(), (3, 1));
    assert!(!reconnected.catalog_stale_at(42_000));
    assert!(reconnected.catalog_stale_at(41_999));
}

#[test]
fn aggregation_t26_login_tracks_user_and_workspace_without_tokens() {
    let f = Fixture::new();
    let first = f.observe(10_000);
    f.login("user-a", "rotated-token");
    let rotated = f.observe(20_000);
    assert_eq!(first.revision(), rotated.revision());
    assert_eq!(rotated.generation_counts(), (1, 1));
    f.login("user-b", "secret-token-b");
    let switched = f.observe(30_000);
    assert_ne!(first.revision(), switched.revision());
    assert_eq!(switched.generation_counts(), (1, 2));
    assert!(switched.login_stale_at(15_000));
    assert!(!switched.login_stale_at(32_000));
    f.login("user-a", "secret-token-c");
    let returned = f.observe(40_000);
    assert_eq!(returned.revision(), first.revision());
    assert!(!returned.login_stale_at(15_000));
    assert!(returned.login_stale_at(35_000));
    let history = fs::read_to_string(f.home.path().join(".codex").join(HISTORY_FILENAME)).unwrap();
    for secret in [
        "user-a",
        "user-b",
        "shared-workspace",
        "secret-token",
        "catalog-a",
        "id_token",
        "access_token",
    ] {
        assert!(!history.contains(secret));
    }
    let mode = fs::metadata(f.home.path().join(".codex").join(HISTORY_FILENAME))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn aggregation_t26_read_is_side_effect_free_and_external_changes_unverified() {
    let f = Fixture::new();
    assert_eq!(f.read().generation_counts(), (0, 0));
    assert!(f.read().unverified());
    assert!(!f.home.path().join(".codex").join(HISTORY_FILENAME).exists());
    let first = f.observe(10_000);
    let before = fs::read(f.home.path().join(".codex").join(HISTORY_FILENAME)).unwrap();
    f.write(CATALOG_FILENAME, "catalog-b");
    let read = f.read();
    assert!(read.unverified());
    assert_ne!(read.revision(), first.revision());
    assert_eq!(read.generation_counts(), (1, 1));
    assert_eq!(before, fs::read(f.home.path().join(".codex").join(HISTORY_FILENAME)).unwrap());
    f.observe(20_000);
    assert!(!f.read().unverified());
}

#[test]
fn aggregation_t26_foreign_pointer_and_non_file_login_are_not_followed() {
    let f = Fixture::new();
    use std::os::unix::fs::symlink;
    fs::remove_file(f.home.path().join(".codex/auth.json")).unwrap();
    symlink("/nonexistent-secret", f.home.path().join(".codex/auth.json")).unwrap();
    for mode in ["keyring", "auto", "ephemeral"] {
        f.write(
            "config.toml",
            &format!(
                "model_catalog_json='/nonexistent-secret'\ncli_auth_credentials_store='{mode}'"
            ),
        );
        let snapshot = f.observe(10_000);
        assert_eq!(snapshot.generation_counts(), (1, 0));
        assert!(!snapshot.catalog_stale_at(0));
        assert!(!snapshot.login_stale_at(0));
    }
}

#[test]
fn aggregation_t26_history_is_bounded_and_clock_rollback_does_not_write() {
    let f = Fixture::new();
    for n in 0..40 {
        f.write(CATALOG_FILENAME, &format!("catalog-{n}"));
        f.observe(10_000 + n * 10_000);
    }
    assert_eq!(f.read().generation_counts(), (32, 1));
    let before = fs::read(f.home.path().join(".codex").join(HISTORY_FILENAME)).unwrap();
    f.write(CATALOG_FILENAME, "later-catalog");
    assert!(matches!(
        observe(&mut CodexFiles::open(f.home.path()).unwrap(), &f.tx, &f.scope, 1),
        Err(WriteError::Conflict)
    ));
    assert_eq!(before, fs::read(f.home.path().join(".codex").join(HISTORY_FILENAME)).unwrap());
}

#[test]
fn aggregation_t26_invalid_inputs_scope_and_history_fail_closed() {
    let f = Fixture::new();
    let other =
        ProjectScope::new(f.scope.tenant_scope().clone(), ProjectId::from("other")).unwrap();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    assert!(read(&files, &f.tx, &other).is_err());
    f.write("config.toml", "broken = [");
    assert!(read(&files, &f.tx, &f.scope).is_err());
    f.write("config.toml", &format!("model_catalog_json='{CATALOG_FILENAME}'"));
    f.write("auth.json", "{broken");
    assert!(read(&files, &f.tx, &f.scope).is_err());
    f.write("auth.json", "{}");
    f.write(HISTORY_FILENAME, "{broken");
    assert!(observe(&mut files, &f.tx, &f.scope, 10_000).is_err());
    f.write(HISTORY_FILENAME, r#"{"catalog":[],"login":[]}"#);
    fs::remove_file(f.home.path().join(".codex").join(CATALOG_FILENAME)).unwrap();
    std::os::unix::fs::symlink(
        "/nonexistent-secret",
        f.home.path().join(".codex").join(CATALOG_FILENAME),
    )
    .unwrap();
    assert!(read(&files, &f.tx, &f.scope).is_err());
}
