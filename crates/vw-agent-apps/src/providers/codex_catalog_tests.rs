use super::{
    codex_catalog::*,
    live::{commit::*, recovery::RecoveryOutcome, write_guard::WriteError},
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId, app_providers::codex_catalog::*, id::TenantScope, model_catalog::*,
};
fn profile(name: &str) -> CodexProfile {
    CodexProfile {
        name: name.into(),
        config: format!(
            "model_provider='vendor'\nmodel='{name}'\n[model_providers.vendor]\nbase_url='https://fixture.test/v1'\nwire_api='responses'\n"
        ),
        api_key: Some("fake-route-key".into()),
        preserve_login: false,
    }
}
fn catalog() -> SavedCodexCatalog {
    SavedCodexCatalog {
        models: vec![CodexNativeModel {
            slug: "saved-model".into(),
            display_name: "Saved model".into(),
            description: "isolated fixture".into(),
            base_instructions: "fixture instructions".into(),
            supports_reasoning_summaries: false,
            supports_parallel_tool_calls: false,
            context_window: None,
        }],
    }
}
struct Fixture {
    home: tempfile::TempDir,
    root: tempfile::TempDir,
    tenant: TenantScope,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        Self { home, root, tenant: TenantScope::new("p", "a").unwrap() }
    }
    fn tx(&self) -> ProviderTransactions {
        ProviderTransactions::new(self.root.path(), ExecutionHostId::new("host").unwrap()).unwrap()
    }
    fn read(&self, file: &str) -> Vec<u8> {
        fs::read(self.home.path().join(".codex").join(file)).unwrap()
    }
}
#[test]
fn cc04_codex_catalog_pointer_matches_committed_provider() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let row = profile("one");
    files
        .apply(
            &tx,
            &f.tenant,
            &tx.state(AppId::Codex).unwrap(),
            "one",
            None,
            &row,
            Some(&catalog()),
            || true,
        )
        .unwrap();
    let text = String::from_utf8(f.read("config.toml")).unwrap();
    let doc: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(doc.get("model_catalog_json").and_then(toml::Value::as_str), Some(CATALOG_FILENAME));
    assert!(!text.contains("modelCatalog"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&f.read(CATALOG_FILENAME)).unwrap()["models"]
            [0]["slug"],
        "saved-model"
    );
    assert_eq!(tx.state(AppId::Codex).unwrap().pointer.unwrap().id, "one");
    files
        .apply(
            &tx,
            &f.tenant,
            &tx.state(AppId::Codex).unwrap(),
            "two",
            Some(&settings(&row)),
            &profile("two"),
            None,
            || true,
        )
        .unwrap();
    assert!(!String::from_utf8(f.read("config.toml")).unwrap().contains("model_catalog_json"));
    assert!(!f.home.path().join(".codex").join(CATALOG_FILENAME).exists());
    assert_eq!(tx.state(AppId::Codex).unwrap().pointer.unwrap().id, "two");
}
#[test]
fn cc04_codex_catalog_failure_is_recoverable() {
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let f = Fixture::new();
        let tx = f.tx();
        let mut files = CodexFiles::open(f.home.path()).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let result = files.apply(
            &tx,
            &f.tenant,
            &AppState::default(),
            "one",
            None,
            &profile("one"),
            Some(&catalog()),
            || true,
        );
        HOOK.with_borrow_mut(|h| *h = None);
        assert_eq!(result, Err(WriteError::Io));
        drop(tx);
        let tx = f.tx();
        let outcome = tx.recover(&mut files.guard).unwrap();
        if phase == Phase::Intent {
            assert_eq!(outcome, Some(RecoveryOutcome::Discarded));
            assert!(tx.state(AppId::Codex).unwrap().pointer.is_none());
            assert!(!f.home.path().join(".codex/config.toml").exists());
        } else {
            assert_eq!(outcome, Some(RecoveryOutcome::RolledForward));
            assert_eq!(tx.state(AppId::Codex).unwrap().pointer.unwrap().id, "one");
            assert!(!f.read(CATALOG_FILENAME).is_empty());
            assert!(String::from_utf8(f.read("config.toml")).unwrap().contains(CATALOG_FILENAME));
        }
        assert!(tx.pending(AppId::Codex).unwrap().is_none());
    }
}
#[test]
fn cc04_codex_catalog_foreign_pointer_wins_and_profile_rejects() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let mut row = profile("one");
    row.config = format!("model_catalog_json='/never/read/custom.json'\n{}", row.config);
    files
        .apply(&tx, &f.tenant, &AppState::default(), "one", None, &row, Some(&catalog()), || true)
        .unwrap();
    assert!(String::from_utf8(f.read("config.toml")).unwrap().contains("/never/read/custom.json"));
    fs::write(
        f.home.path().join(".codex/config.toml"),
        "profile='active'\n[profiles.active]\nmodel_provider='foreign'\n",
    )
    .unwrap();
    let before = f.read("config.toml");
    let before_catalog = f.read(CATALOG_FILENAME);
    let state = tx.state(AppId::Codex).unwrap();
    assert_eq!(
        files.apply(&tx, &f.tenant, &state, "two", None, &profile("two"), None, || true),
        Err(WriteError::InvalidResource)
    );
    assert_eq!(f.read("config.toml"), before);
    assert_eq!(f.read(CATALOG_FILENAME), before_catalog);
    assert!(tx.state(AppId::Codex).unwrap() == state);
    assert!(tx.pending(AppId::Codex).unwrap().is_none());
}
#[test]
fn cc04_codex_catalog_saved_revision_conflict_leaves_native_unchanged() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    assert_eq!(
        files.apply(
            &tx,
            &f.tenant,
            &AppState::default(),
            "one",
            None,
            &profile("one"),
            Some(&catalog()),
            || false
        ),
        Err(WriteError::Conflict)
    );
    assert!(!f.home.path().join(".codex/config.toml").exists());
    assert!(tx.pending(AppId::Codex).unwrap().is_none());
}

#[test]
fn cc04_codex_catalog_login_auth_stash_and_catalog_share_one_intent() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let auth = serde_json::json!({"auth_mode":"chatgpt","tokens":{"id_token":"eyJhbGciOiJub25lIn0.eyJzdWIiOiJhbGljZSJ9.","access_token":"fake-access","refresh_token":"fake-refresh","account_id":"team"}});
    fs::write(f.home.path().join(".codex/auth.json"), auth.to_string()).unwrap();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(|tx, app, at| {
            if at == Phase::Marked {
                assert_eq!(
                    tx.pending(app)?.unwrap().files.len(),
                    4,
                    "auth/stash/config/catalog must belong to one pending intent"
                );
                return Err(WriteError::Io);
            }
            Ok(())
        }))
    });
    let result = files.apply(
        &tx,
        &f.tenant,
        &AppState::default(),
        "one",
        None,
        &profile("one"),
        Some(&catalog()),
        || true,
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(result, Err(WriteError::Io));
    assert!(f.home.path().join(".codex/auth.json").exists());
    assert!(tx.state(AppId::Codex).unwrap().pointer.is_none());
    assert_eq!(tx.recover(&mut files.guard).unwrap(), Some(RecoveryOutcome::RolledForward));
    assert!(!f.home.path().join(".codex/auth.json").exists());
    let stash: serde_json::Value =
        serde_json::from_slice(&f.read(super::codex_login::STASH_FILENAME)).unwrap();
    assert_eq!(stash["logins"]["sub:alice|account:team"], auth);
    assert!(f.home.path().join(".codex").join(CATALOG_FILENAME).exists());
    assert_eq!(tx.state(AppId::Codex).unwrap().pointer.unwrap().id, "one");
}
#[test]
fn cc04_codex_catalog_reapply_uses_committed_exclusive_ownership() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let mut row = profile("one");
    row.config = format!("model_context_window=12345\n{}", row.config);
    files.apply(&tx, &f.tenant, &AppState::default(), "one", None, &row, None, || true).unwrap();
    let edited = profile("one");
    files
        .apply(
            &tx,
            &f.tenant,
            &tx.state(AppId::Codex).unwrap(),
            "one",
            Some(&settings(&edited)),
            &edited,
            None,
            || true,
        )
        .unwrap();
    assert!(!String::from_utf8(f.read("config.toml")).unwrap().contains("model_context_window"));
}

#[test]
fn cc04_codex_catalog_saved_input_change_before_marker_requires_replanning() {
    use std::cell::Cell;
    use std::rc::Rc;
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let valid = Rc::new(Cell::new(true));
    let edited = valid.clone();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| {
            if at == Phase::Intent {
                edited.set(false)
            }
            Ok(())
        }))
    });
    let result = files.apply(
        &tx,
        &f.tenant,
        &AppState::default(),
        "one",
        None,
        &profile("one"),
        Some(&catalog()),
        || valid.get(),
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(result, Err(WriteError::Conflict));
    assert!(!tx.pending(AppId::Codex).unwrap().unwrap().published);
    assert!(!f.home.path().join(".codex/config.toml").exists());
    assert_eq!(tx.recover(&mut files.guard).unwrap(), Some(RecoveryOutcome::Discarded));
    assert!(tx.state(AppId::Codex).unwrap().pointer.is_none());
}
#[test]
fn cc04_codex_catalog_external_catalog_change_survives_recovery() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(
            |_, _, at| if at == Phase::Marked { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    let result = files.apply(
        &tx,
        &f.tenant,
        &AppState::default(),
        "one",
        None,
        &profile("one"),
        Some(&catalog()),
        || true,
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(result, Err(WriteError::Io));
    fs::write(f.home.path().join(".codex").join(CATALOG_FILENAME), b"external edit").unwrap();
    assert!(matches!(
        tx.recover(&mut files.guard).unwrap(),
        Some(RecoveryOutcome::RolledForwardExcept { .. })
    ));
    assert_eq!(f.read(CATALOG_FILENAME), b"external edit");
}

#[test]
fn cc04_codex_catalog_managed_pointer_basename_never_survives_without_saved_data() {
    for pointer in ["/old/cc-switch-model-catalog.json", "/old/shiju-model-catalog.json"] {
        let f = Fixture::new();
        let tx = f.tx();
        let mut files = CodexFiles::open(f.home.path()).unwrap();
        let mut row = profile("one");
        row.config = format!("model_catalog_json='{pointer}'\n{}", row.config);
        files
            .apply(&tx, &f.tenant, &AppState::default(), "one", None, &row, None, || true)
            .unwrap();
        assert!(
            !String::from_utf8(f.read("config.toml")).unwrap().contains("model_catalog_json"),
            "managed pointer is owned by the saved catalog projection even when its old directory differs"
        );
    }
}
