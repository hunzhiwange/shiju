use super::{codex_catalog::*, codex_publish::*};
use crate::providers::{
    codex_catalog::{CATALOG_FILENAME, CodexFiles},
    live::{commit::*, recovery::RecoveryOutcome, write_guard::WriteError},
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, TenantScope},
};

struct Fixture {
    home: tempfile::TempDir,
    root: tempfile::TempDir,
    scope: ProjectScope,
}
impl Fixture {
    fn new() -> Self {
        let f = Self {
            home: tempfile::tempdir().unwrap(),
            root: tempfile::tempdir().unwrap(),
            scope: ProjectScope::new(
                TenantScope::new("p", "a").unwrap(),
                ProjectId::from("project-a"),
            )
            .unwrap(),
        };
        fs::set_permissions(f.root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = CodexFiles::open(f.home.path()).unwrap();
        f.tx()
            .commit_project_checked(
                &f.scope,
                &mut files.guard,
                &AppState::default(),
                true,
                &[],
                PendingTarget::default(),
                || Ok(()),
            )
            .unwrap();
        f.write("config.toml", b"# user configuration\nmodel='main'\nmodel_context_window=200000\n[history]\npersistence='save-all'\n");
        f.write("auth.json", br#"{"tokens":{"access_token":"fixture-only"}}"#);
        f
    }
    fn tx(&self) -> ProviderTransactions {
        ProviderTransactions::new(self.root.path(), ExecutionHostId::new("host").unwrap()).unwrap()
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        fs::write(self.home.path().join(".codex").join(name), bytes).unwrap();
    }
    fn read(&self, name: &str) -> Vec<u8> {
        fs::read(self.home.path().join(".codex").join(name)).unwrap()
    }
}
fn request<'a>(
    f: &'a Fixture,
    state: &'a AppState,
    config: &'a toml_edit::DocumentMut,
) -> Publication<'a> {
    Publication {
        scope: &f.scope,
        expected: state,
        expected_revision: "scope-revision-1",
        policy: CatalogPolicy::UseManaged,
        default: CodexCatalogDefault::ThirdParty(CodexCatalogRow { catalog: None, config }),
        members: &[],
    }
}
#[test]
fn aggregation_t15_codex_publish_preserves_contract() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    let config = "model='main'".parse().unwrap();
    let auth = f.read("auth.json");
    assert!(publish(&mut files, &tx, request(&f, &state, &config), || true).unwrap());
    let text = String::from_utf8(f.read("config.toml")).unwrap();
    let doc: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(doc["model_catalog_json"].as_str(), Some(CATALOG_FILENAME));
    assert_eq!(doc["history"]["persistence"].as_str(), Some("save-all"));
    assert!(text.contains("# user configuration"));
    assert_eq!(f.read("auth.json"), auth);
    let catalog: serde_json::Value = serde_json::from_slice(&f.read(CATALOG_FILENAME)).unwrap();
    assert_eq!(catalog["models"][0]["slug"], "main");
    assert!(tx.state(AppId::Codex).unwrap() == state);
    assert!(tx.pending(AppId::Codex).unwrap().is_none());
}

#[test]
fn aggregation_t15_preserve_external_catalog_requires_explicit_adoption() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    let config = "model='main'\nmodel_catalog_json='/never/read/vendor.json'".parse().unwrap();
    f.write(CATALOG_FILENAME, b"old managed bytes");
    let mut input = request(&f, &state, &config);
    input.policy = CatalogPolicy::Preserve;
    assert!(publish(&mut files, &tx, input, || true).unwrap());
    assert_eq!(f.read(CATALOG_FILENAME), b"old managed bytes");
    let doc: toml::Value =
        toml::from_str(&String::from_utf8(f.read("config.toml")).unwrap()).unwrap();
    assert_eq!(doc["model_catalog_json"].as_str(), Some("/never/read/vendor.json"));
    assert!(publish(&mut files, &tx, request(&f, &state, &config), || true).unwrap());
    let doc: toml::Value =
        toml::from_str(&String::from_utf8(f.read("config.toml")).unwrap()).unwrap();
    assert_eq!(doc["model_catalog_json"].as_str(), Some(CATALOG_FILENAME));
    assert!(!publish(&mut files, &tx, request(&f, &state, &config), || true).unwrap());
}

#[test]
fn aggregation_t15_publishes_default_and_members_with_independent_windows() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    let config = "model='main'\nmodel_context_window=200000".parse().unwrap();
    let member = "model='other'\nmodel_context_window=100000".parse().unwrap();
    let members = [CodexCatalogMember {
        key: "vendor",
        provider_name: "Vendor",
        row: CodexCatalogRow { catalog: None, config: &member },
    }];
    let mut input = request(&f, &state, &config);
    input.members = &members;
    publish(&mut files, &tx, input, || true).unwrap();
    let doc: toml::Value =
        toml::from_str(&String::from_utf8(f.read("config.toml")).unwrap()).unwrap();
    assert!(doc.get("model_context_window").is_none());
    let catalog: serde_json::Value = serde_json::from_slice(&f.read(CATALOG_FILENAME)).unwrap();
    assert_eq!(catalog["models"][0]["context_window"], 200000);
    assert_eq!(catalog["models"][1]["context_window"], 100000);
    assert_eq!(catalog["models"][1]["slug"], "ccs-vendor/other");
}

#[test]
fn aggregation_t15_late_revision_discards_unpublished_intent() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    let config = "model='main'".parse().unwrap();
    let before = f.read("config.toml");
    let changed = std::rc::Rc::new(std::cell::Cell::new(false));
    let flag = changed.clone();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(move |_, _, phase| {
            if phase == Phase::Intent {
                flag.set(true);
            }
            Ok(())
        }))
    });
    let result = publish(&mut files, &tx, request(&f, &state, &config), || !changed.get());
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(
        result,
        Err(PublishFailure {
            code: CodexPublishError::RevisionConflict,
            previous_revision: "scope-revision-1",
            recovery_pending: false
        })
    );
    assert_eq!(f.read("config.toml"), before);
    assert!(!f.home.path().join(".codex").join(CATALOG_FILENAME).exists());
    assert!(tx.pending(AppId::Codex).unwrap().is_none());
}

#[test]
fn aggregation_t15_write_failure_keeps_previous_revision_and_recovery_owner() {
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let f = Fixture::new();
        let tx = f.tx();
        let mut files = CodexFiles::open(f.home.path()).unwrap();
        let state = tx.state(AppId::Codex).unwrap();
        let config = "model='main'".parse().unwrap();
        let before = f.read("config.toml");
        let auth = f.read("auth.json");
        HOOK.with_borrow_mut(|hook| {
            *hook = Some(Box::new(
                move |_, _, at| {
                    if at == phase { Err(WriteError::Io) } else { Ok(()) }
                },
            ))
        });
        let result = publish(&mut files, &tx, request(&f, &state, &config), || true);
        HOOK.with_borrow_mut(|hook| *hook = None);
        let pending = phase != Phase::Intent;
        assert_eq!(
            result,
            Err(PublishFailure {
                code: CodexPublishError::TargetFailed,
                previous_revision: "scope-revision-1",
                recovery_pending: pending
            })
        );
        assert_eq!(f.read("auth.json"), auth);
        assert!(tx.state(AppId::Codex).unwrap() == state);
        if !pending {
            assert_eq!(f.read("config.toml"), before);
            assert!(!f.home.path().join(".codex").join(CATALOG_FILENAME).exists());
        } else {
            let intent = tx.pending(AppId::Codex).unwrap().unwrap();
            assert_eq!(intent.files.len(), 2);
            assert!(intent.published);
            // Reopen the durable state owner as after an executor restart.
            drop(tx);
            let tx = f.tx();
            assert_eq!(
                tx.recover_project(&f.scope, &mut files.guard).unwrap(),
                Some(RecoveryOutcome::RolledForward)
            );
            assert!(tx.pending(AppId::Codex).unwrap().is_none());
            let doc: toml::Value =
                toml::from_str(&String::from_utf8(f.read("config.toml")).unwrap()).unwrap();
            assert_eq!(doc["model_catalog_json"].as_str(), Some(CATALOG_FILENAME));
            assert!(!f.read(CATALOG_FILENAME).is_empty());
        }
    }
}

#[test]
fn aggregation_t15_missing_official_catalog_and_invalid_config_leave_files_unchanged() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    let config = "model='main'".parse().unwrap();
    let before = f.read("config.toml");
    let mut input = request(&f, &state, &config);
    input.default = CodexCatalogDefault::Official { native: &[], config: &config };
    assert_eq!(
        publish(&mut files, &tx, input, || true).unwrap_err().code,
        CodexPublishError::UnsupportedTarget
    );
    assert_eq!(f.read("config.toml"), before);
    f.write("config.toml", b"broken = [");
    assert_eq!(
        publish(&mut files, &tx, request(&f, &state, &config), || true).unwrap_err().code,
        CodexPublishError::InvalidConfiguration
    );
    assert_eq!(f.read("config.toml"), b"broken = [");
    assert!(tx.pending(AppId::Codex).unwrap().is_none());
}

#[test]
fn aggregation_t15_rejects_foreign_ownership_and_stale_revision() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    let config = "model='main'".parse().unwrap();
    assert_eq!(
        publish(&mut files, &tx, request(&f, &state, &config), || false).unwrap_err().code,
        CodexPublishError::RevisionConflict
    );
    let foreign =
        ProjectScope::new(f.scope.tenant_scope().clone(), ProjectId::from("project-b")).unwrap();
    let mut input = request(&f, &state, &config);
    input.scope = &foreign;
    assert_eq!(
        publish(&mut files, &tx, input, || true).unwrap_err().code,
        CodexPublishError::Forbidden
    );
    assert!(!f.home.path().join(".codex").join(CATALOG_FILENAME).exists());
}

#[test]
fn aggregation_t15_unbound_slot_is_not_implicitly_claimed() {
    let f = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = AppState::default();
    let config = "model='main'".parse().unwrap();
    let before = f.read("config.toml");
    assert_eq!(
        publish(&mut files, &tx, request(&f, &state, &config), || true).unwrap_err().code,
        CodexPublishError::Forbidden
    );
    assert_eq!(f.read("config.toml"), before);
    assert!(tx.state(AppId::Codex).unwrap().project_scope.is_none());
    assert!(tx.pending(AppId::Codex).unwrap().is_none());
}

#[test]
fn aggregation_t15_unsafe_native_targets_do_not_follow_symlinks() {
    for name in ["config.toml", CATALOG_FILENAME] {
        let f = Fixture::new();
        let external = tempfile::tempdir().unwrap();
        let target = external.path().join("keep");
        fs::write(&target, b"external contents").unwrap();
        let path = f.home.path().join(".codex").join(name);
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        std::os::unix::fs::symlink(&target, &path).unwrap();
        let tx = f.tx();
        let mut files = CodexFiles::open(f.home.path()).unwrap();
        let state = tx.state(AppId::Codex).unwrap();
        let config = "model='main'".parse().unwrap();
        assert_eq!(
            publish(&mut files, &tx, request(&f, &state, &config), || true).unwrap_err().code,
            CodexPublishError::InvalidConfiguration
        );
        assert_eq!(fs::read(&target).unwrap(), b"external contents");
        assert!(fs::symlink_metadata(&path).unwrap().file_type().is_symlink());
        assert!(tx.pending(AppId::Codex).unwrap().is_none());
    }
}

#[test]
fn aggregation_t15_pending_requires_its_original_project_recovery() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = CodexFiles::open(f.home.path()).unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    let config = "model='main'".parse().unwrap();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(
            |_, _, phase| {
                if phase == Phase::Marked { Err(WriteError::Io) } else { Ok(()) }
            },
        ))
    });
    assert!(publish(&mut files, &tx, request(&f, &state, &config), || true).is_err());
    HOOK.with_borrow_mut(|hook| *hook = None);
    let before = f.read("config.toml");
    assert_eq!(
        publish(&mut files, &tx, request(&f, &state, &config), || true).unwrap_err().code,
        CodexPublishError::RecoveryPending
    );
    let foreign =
        ProjectScope::new(f.scope.tenant_scope().clone(), ProjectId::from("project-b")).unwrap();
    let mut input = request(&f, &state, &config);
    input.scope = &foreign;
    assert_eq!(
        publish(&mut files, &tx, input, || true).unwrap_err().code,
        CodexPublishError::Forbidden
    );
    assert_eq!(tx.recover_project(&foreign, &mut files.guard), Err(WriteError::Conflict));
    assert_eq!(f.read("config.toml"), before);
    tx.recover_project(&f.scope, &mut files.guard).unwrap();
}
