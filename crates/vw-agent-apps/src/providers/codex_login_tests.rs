use super::{
    codex_login::*,
    live::{
        commit::*,
        recovery::RecoveryOutcome,
        stage::{FileChange, WholeFile},
        write_guard::*,
    },
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};
fn login(user: &str, workspace: &str, generation: &str) -> Value {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let payload = URL_SAFE_NO_PAD.encode(json!({"sub":user}).to_string());
    json!({"auth_mode":"chatgpt", "tokens":{"id_token":format!("{header}.{payload}."),"account_id": workspace,"access_token":format!("access-{generation}"),"refresh_token":format!("refresh-{generation}")}})
}
struct Fixture {
    root: tempfile::TempDir,
    live: tempfile::TempDir,
    tx: ProviderTransactions,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let live = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        Self { root, live, tx }
    }
    fn write(&self, name: &str, value: &Value) {
        fs::write(self.live.path().join(name), serde_json::to_vec(value).unwrap()).unwrap();
    }
    fn read(&self, name: &str) -> Value {
        serde_json::from_slice(&fs::read(self.live.path().join(name)).unwrap()).unwrap()
    }
    fn input<'a>(&self, target: AuthTarget<'a>, leaving: Option<&'a Value>) -> LoginInput<'a> {
        LoginInput {
            target,
            leaving_official: leaving,
            official_logins: &[],
            third_party_keys: &[],
            live_is_managed: false,
            now: 1_800_000_000,
        }
    }
    fn apply(&self, target: AuthTarget<'_>, leaving: Option<&Value>) {
        let mut guard = lock_app(AppId::Codex, self.live.path()).unwrap();
        assert!(self.tx.recover(&mut guard).unwrap().is_none());
        let prepared = prepare(&guard, self.input(target, leaving)).unwrap();
        self.tx
            .commit(
                &TenantScope::new("p", "c").unwrap(),
                &mut guard,
                &prepared.changes(),
                PendingTarget::default(),
            )
            .unwrap();
    }
}
#[test]
fn cc04_codex_login_roundtrip_keeps_latest_login() {
    let f = Fixture::new();
    let empty = json!({});
    let first = login("alice", "plus", "old");
    f.write("auth.json", &first);
    f.apply(AuthTarget::ThirdParty { preserve: false }, Some(&empty));
    assert!(
        !f.live.path().join("auth.json").exists(),
        "third-party departure removes actual auth file"
    );
    f.apply(AuthTarget::Official { row_auth: &empty }, None);
    assert_eq!(f.read("auth.json"), first);
    let rotated = login("alice", "plus", "rotated");
    f.write("auth.json", &rotated);
    f.apply(AuthTarget::ThirdParty { preserve: false }, Some(&empty));
    f.apply(AuthTarget::Official { row_auth: &first }, None);
    assert_eq!(f.read("auth.json"), rotated, "row snapshot must not replace refreshed tokens");
    assert_eq!(
        fs::metadata(f.live.path().join("auth.json")).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(f.live.path().join(STASH_FILENAME)).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[test]
fn cc04_codex_login_logout_is_not_resurrected() {
    let f = Fixture::new();
    let empty = json!({});
    let old = login("alice", "plus", "old");
    f.write(STASH_FILENAME,&json!({"version":1,"last":"sub:alice|account:plus","logins":{"sub:alice|account:plus":old}}));
    // CLI logged out on the official card; old row/stash must not revive it.
    f.apply(AuthTarget::ThirdParty { preserve: false }, Some(&empty));
    f.apply(AuthTarget::Official { row_auth: &old }, None);
    assert!(!f.live.path().join("auth.json").exists());
    assert_eq!(f.read(STASH_FILENAME)["logins"], json!({}));
}
#[test]
fn cc04_codex_login_transaction_recovers_delete_and_stash_at_every_checkpoint() {
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let f = Fixture::new();
        let auth = login("alice", "team", "fresh");
        f.write("auth.json", &auth);
        let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
        let prepared =
            prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: false }, None)).unwrap();
        HOOK.with_borrow_mut(|hook| {
            *hook = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let result = f.tx.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &prepared.changes(),
            PendingTarget::default(),
        );
        HOOK.with_borrow_mut(|hook| *hook = None);
        assert_eq!(result, Err(WriteError::Io));
        drop(guard);
        let restarted =
            ProviderTransactions::new(f.root.path(), ExecutionHostId::new("host").unwrap())
                .unwrap();
        let outcomes = restarted.recover_on_startup();
        assert_eq!(outcomes.len(), 1);
        if phase == Phase::Intent {
            assert_eq!(outcomes[0].1, Ok(RecoveryOutcome::Discarded));
            assert_eq!(f.read("auth.json"), auth);
            assert!(!f.live.path().join(STASH_FILENAME).exists());
        } else {
            assert_eq!(outcomes[0].1, Ok(RecoveryOutcome::RolledForward));
            assert!(!f.live.path().join("auth.json").exists());
            assert_eq!(f.read(STASH_FILENAME)["logins"]["sub:alice|account:team"], auth);
        }
        assert!(restarted.pending(AppId::Codex).unwrap().is_none());
        assert!(
            !fs::read_dir(f.live.path()).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".shiju-live-"))
        );
    }
}
#[test]
fn cc04_codex_login_external_rotation_rejects_before_publication() {
    let f = Fixture::new();
    let old = login("alice", "plus", "old");
    let rotated = login("alice", "plus", "new");
    f.write("auth.json", &old);
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared =
        prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: false }, None)).unwrap();
    let path = f.live.path().join("auth.json");
    let bytes = serde_json::to_vec(&rotated).unwrap();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(move |_, _, at| {
            if at == Phase::BeforePublish {
                fs::write(&path, &bytes).unwrap();
            }
            Ok(())
        }))
    });
    let result = f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &prepared.changes(),
        PendingTarget::default(),
    );
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(WriteError::Conflict));
    assert_eq!(f.read("auth.json"), rotated);
    assert!(!f.live.path().join(STASH_FILENAME).exists());
    assert!(f.tx.pending(AppId::Codex).unwrap().is_none());
}
#[test]
fn cc04_codex_login_recovery_preserves_external_login_before_delete() {
    let f = Fixture::new();
    let old = login("alice", "plus", "old");
    let new = login("bob", "team", "new");
    f.write("auth.json", &old);
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared =
        prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: false }, None)).unwrap();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(
            |_, _, at| if at == Phase::Marked { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    assert!(
        f.tx.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &prepared.changes(),
            PendingTarget::default()
        )
        .is_err()
    );
    HOOK.with_borrow_mut(|hook| *hook = None);
    f.write("auth.json", &new);
    assert_eq!(
        f.tx.recover(&mut guard).unwrap(),
        Some(RecoveryOutcome::RolledForwardExcept { names: vec![b"auth.json".to_vec()] })
    );
    assert_eq!(f.read("auth.json"), new);
    assert_eq!(f.read(STASH_FILENAME)["logins"]["sub:alice|account:plus"], old);
}
#[test]
fn cc04_codex_login_preserve_leaves_current_login_and_proxy_logout() {
    let f = Fixture::new();
    let auth = login("alice", "plus", "new");
    f.write("auth.json", &auth);
    f.apply(AuthTarget::ThirdParty { preserve: true }, None);
    assert_eq!(f.read("auth.json"), auth);
    fs::remove_file(f.live.path().join("auth.json")).unwrap();
    f.apply(AuthTarget::ProxyThirdParty { restore_last: false }, None);
    assert!(!f.live.path().join("auth.json").exists());
}
#[test]
fn cc04_codex_login_invalid_resources_fail_without_modification() {
    for invalid in ["broken", "future", "symlink", "keyring", "bad-auth"] {
        let f = Fixture::new();
        let auth = login("alice", "plus", "new");
        f.write("auth.json", &auth);
        match invalid {
            "broken" => {
                fs::write(f.live.path().join(STASH_FILENAME), b"{bad").unwrap();
            }
            "future" => f.write(STASH_FILENAME, &json!({"version":2})),
            "symlink" => {
                symlink(f.live.path().join("auth.json"), f.live.path().join(STASH_FILENAME))
                    .unwrap()
            }
            "keyring" => {
                fs::write(
                    f.live.path().join("config.toml"),
                    b"cli_auth_credentials_store='keyring'",
                )
                .unwrap();
            }
            _ => {
                fs::write(f.live.path().join("auth.json"), b"{bad").unwrap();
            }
        }
        let before = fs::read(f.live.path().join("auth.json")).unwrap();
        let guard = lock_app(AppId::Codex, f.live.path()).unwrap();
        assert!(matches!(
            prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: false }, None)),
            Err(WriteError::InvalidResource)
        ));
        assert_eq!(fs::read(f.live.path().join("auth.json")).unwrap(), before);
        assert!(f.tx.pending(AppId::Codex).unwrap().is_none());
    }
}
#[test]
fn cc04_codex_login_empty_write_and_delete_are_distinct() {
    let f = Fixture::new();
    f.write("auth.json", &json!({}));
    let before = fs::read(f.live.path().join("auth.json")).unwrap();
    let empty = WholeFile::new(Some(&before), Some(vec![]));
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &[FileChange {
            name: OsStr::new("auth.json"),
            privacy: FilePrivacy::Private,
            patch: &empty,
        }],
        PendingTarget::default(),
    )
    .unwrap();
    assert_eq!(fs::read(f.live.path().join("auth.json")).unwrap(), b"");
    let delete = WholeFile::new(Some(b""), None);
    f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &[FileChange {
            name: OsStr::new("auth.json"),
            privacy: FilePrivacy::Private,
            patch: &delete,
        }],
        PendingTarget::default(),
    )
    .unwrap();
    assert!(!f.live.path().join("auth.json").exists());
}

#[test]
fn cc04_codex_login_workspaces_and_one_time_row_seed() {
    let f = Fixture::new();
    let plus = login("alice", "plus", "old");
    let team = login("alice", "team", "old");
    let rows = [plus.clone(), team.clone()];
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared = prepare(
        &guard,
        LoginInput {
            official_logins: &rows,
            ..f.input(AuthTarget::Official { row_auth: &plus }, None)
        },
    )
    .unwrap();
    f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &prepared.changes(),
        PendingTarget::default(),
    )
    .unwrap();
    drop(guard);
    assert_eq!(f.read("auth.json"), plus);
    let rotated = login("alice", "plus", "latest");
    f.write("auth.json", &rotated);
    f.apply(AuthTarget::Official { row_auth: &team }, Some(&plus));
    assert_eq!(f.read("auth.json"), team);
    f.apply(AuthTarget::Official { row_auth: &plus }, Some(&team));
    assert_eq!(f.read("auth.json"), rotated);
    fs::remove_file(f.live.path().join("auth.json")).unwrap();
    f.apply(AuthTarget::ThirdParty { preserve: false }, Some(&plus));
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared = prepare(
        &guard,
        LoginInput {
            official_logins: &rows,
            ..f.input(AuthTarget::Official { row_auth: &plus }, None)
        },
    )
    .unwrap();
    f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &prepared.changes(),
        PendingTarget::default(),
    )
    .unwrap();
    assert!(
        !f.live.path().join("auth.json").exists(),
        "initialized stash forbids reseeding logged-out login"
    );
}
#[test]
fn cc04_codex_login_managed_and_proven_residue_never_enter_stash() {
    for managed in [true, false] {
        let f = Fixture::new();
        let auth = if managed {
            login("managed", "team", "latest")
        } else {
            json!({"OPENAI_API_KEY":"fake-third-party","last_refresh":"metadata"})
        };
        f.write("auth.json", &auth);
        let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
        let keys = ["fake-third-party".to_owned()];
        let prepared = prepare(
            &guard,
            LoginInput {
                live_is_managed: managed,
                third_party_keys: &keys,
                ..f.input(AuthTarget::ThirdParty { preserve: true }, None)
            },
        )
        .unwrap();
        assert!(!prepared.login_on_disk);
        f.tx.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &prepared.changes(),
            PendingTarget::default(),
        )
        .unwrap();
        assert!(!f.live.path().join("auth.json").exists());
        assert_eq!(f.read(STASH_FILENAME)["logins"], json!({}));
    }
}
#[test]
fn cc04_codex_login_unproven_api_key_is_preserved_and_debug_is_redacted() {
    let f = Fixture::new();
    f.write("auth.json", &json!({"OPENAI_API_KEY":"fake-personal"}));
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared =
        prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: true }, None)).unwrap();
    assert!(prepared.login_on_disk);
    assert_eq!(format!("{prepared:?}"), "PreparedLogin([redacted])");
    f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &prepared.changes(),
        PendingTarget::default(),
    )
    .unwrap();
    assert_eq!(f.read("auth.json")["OPENAI_API_KEY"], "fake-personal");
}
#[test]
fn cc04_codex_login_delete_intent_rejects_missing_digest_and_old_version() {
    let f = Fixture::new();
    f.write("auth.json", &login("alice", "team", "old"));
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared =
        prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: false }, None)).unwrap();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(
            |_, _, at| if at == Phase::Intent { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    assert!(
        f.tx.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &prepared.changes(),
            PendingTarget::default()
        )
        .is_err()
    );
    HOOK.with_borrow_mut(|h| *h = None);
    let pending = f.tx.pending(AppId::Codex).unwrap().unwrap();
    assert_eq!(pending.version, 2);
    assert!(pending.files[0].planned.is_none());
    let mut malformed = serde_json::to_value(&pending).unwrap();
    malformed["files"][0].as_object_mut().unwrap().remove("planned");
    assert!(serde_json::from_value::<Pending>(malformed).is_err());
    let mut old = pending;
    old.version = 1;
    fs::write(
        f.root.path().join("provider-live/aG9zdA/codex.pending.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    assert_eq!(f.tx.recover(&mut guard), Err(WriteError::InvalidResource));
    assert!(f.live.path().join("auth.json").exists());
}

#[path = "codex_login_policy_tests.rs"]
mod policy;
#[path = "codex_login_transaction_tests.rs"]
mod transaction;
