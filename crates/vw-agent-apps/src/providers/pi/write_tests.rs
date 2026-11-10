use super::{
    PiDirectoryChange::{self, Insert, Remove, Replace},
    PiDirectoryWriteError::{
        self, AlreadyExists, Forbidden, InvalidConfiguration, NotFound, RevisionConflict,
    },
    PiFiles, object_value, read_native_directory,
};
use crate::providers::live::commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions};
use serde_json::json;
use std::{cell::Cell, fs, os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{agent_apps::AppId, app_providers::pi::PiProfile, id::TenantScope};

struct Fixture {
    home: tempfile::TempDir,
    _root: tempfile::TempDir,
    directory: PathBuf,
    tx: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let directory = home.path().join("controlled");
        fs::create_dir(&directory).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("fixture-host").unwrap())
                .unwrap();
        Self {
            home,
            _root: root,
            directory,
            tx,
            tenant: TenantScope::new("project", "account").unwrap(),
        }
    }

    fn open(&self) -> PiFiles {
        PiFiles::open_directory(self.home.path(), Some(&self.directory), None).unwrap()
    }

    fn models(&self) -> PathBuf {
        self.directory.join("models.json")
    }

    fn revision(&self) -> String {
        read_native_directory(self.home.path(), Some(&self.directory), None)
            .unwrap()
            .source_revision
    }

    fn apply(
        &self,
        revision: &str,
        change: PiDirectoryChange<'_>,
    ) -> Result<(), PiDirectoryWriteError> {
        self.open().apply_directory_change(
            &self.tx,
            &self.tenant,
            revision,
            "fixture",
            change,
            || true,
        )
    }
}

fn profile(model: &str) -> PiProfile {
    PiProfile {
        name: "Fixture".into(),
        declaration: json!({
            "baseUrl": "https://example.invalid/v1",
            "api": "openai-completions",
            "models": [{"id": model, "name": model, "contextWindow": 16000, "maxTokens": 2048}],
        }),
    }
}

#[test]
fn aggregation_t36_pi_write_preserves_contract() {
    let fixture = Fixture::new();
    let models = fixture.models();
    let settings = fixture.directory.join("settings.json");
    let auth = fixture.directory.join("auth.json");
    let settings_bytes =
        br#"{"defaultProvider":"other","defaultModel":"selected","theme":"future"}"#;
    let auth_bytes = br#"{"fixture":{"type":"api_key","key":"fixture-only"}}"#;
    fs::write(&settings, settings_bytes).unwrap();
    fs::write(&auth, auth_bytes).unwrap();

    let stale = fixture.revision();
    let other = br#""other": { "future": true }"#;
    let external = br#"{
      // preserve untouched provider bytes and root fields
      "providers":{"other": { "future": true }},
      "future": 17,
    }"#;
    fs::write(&models, external).unwrap();
    assert_eq!(fixture.apply(&stale, Insert(&profile("first"))), Err(RevisionConflict));
    assert_eq!(fs::read(&models).unwrap(), external);

    fixture.apply(&fixture.revision(), Insert(&profile("first"))).unwrap();
    let added = fs::read(&models).unwrap();
    assert_eq!(fs::metadata(&models).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(fixture.apply(&fixture.revision(), Insert(&profile("first"))), Err(AlreadyExists));
    assert_eq!(fs::read(&models).unwrap(), added);

    fixture.apply(&fixture.revision(), Replace(&profile("second"))).unwrap();
    let replaced = fs::read(&models).unwrap();
    let document = object_value(&replaced).unwrap();
    assert_eq!(document["providers"]["fixture"]["models"][0]["id"], "second");
    assert_eq!(document["future"], 17);

    fixture.apply(&fixture.revision(), Remove).unwrap();
    let removed = fs::read(&models).unwrap();
    assert!(object_value(&removed).unwrap()["providers"].get("fixture").is_none());
    for bytes in [added, replaced, removed] {
        assert!(bytes.windows(other.len()).any(|window| window == other));
    }
    assert_eq!(fs::read(&settings).unwrap(), settings_bytes);
    assert_eq!(fs::read(&auth).unwrap(), auth_bytes);
}

#[test]
fn aggregation_t36_replace_cannot_enable_a_missing_member() {
    let fixture = Fixture::new();
    assert_eq!(fixture.apply(&fixture.revision(), Replace(&profile("first"))), Err(NotFound));
    assert!(!fixture.models().exists());
}

#[test]
fn aggregation_t36_remove_missing_is_idempotent() {
    let fixture = Fixture::new();
    let revision = fixture.revision();
    fixture.apply(&revision, Remove).unwrap();
    fixture.apply(&revision, Remove).unwrap();
    assert!(!fixture.models().exists());
    assert!(fixture.tx.state(AppId::Pi).unwrap().mode_tenant.is_none());
}

#[test]
fn aggregation_t36_replace_keeps_native_credentials_and_rejects_payload_secrets() {
    let fixture = Fixture::new();
    let mut node = profile("first").declaration;
    node["apiKey"] = json!("fixture-key");
    node["headers"] = json!({"X-Fixture": "fixture-header"});
    node["models"][0]["compat"] = json!({"supportsDeveloperRole": true});
    fs::write(fixture.models(), json!({"providers": {"fixture": node}}).to_string()).unwrap();

    let mut changed = profile("first");
    changed.declaration["models"][0]["name"] = json!("updated");
    fixture.apply(&fixture.revision(), Replace(&changed)).unwrap();
    let updated = fs::read(fixture.models()).unwrap();
    let document = object_value(&updated).unwrap();
    let actual = &document["providers"]["fixture"];
    assert_eq!(actual["apiKey"], node["apiKey"]);
    assert_eq!(actual["headers"], node["headers"]);
    assert_eq!(actual["models"][0]["compat"], node["models"][0]["compat"]);
    assert_eq!(actual["models"][0]["name"], "updated");

    let mut retargeted = changed.clone();
    retargeted.declaration["baseUrl"] = json!("https://other.invalid/v1");
    changed.declaration["models"][0]["headers"] = json!({"Authorization": "fixture-payload"});
    for profile in [&changed, &retargeted] {
        assert_eq!(fixture.apply(&fixture.revision(), Replace(profile)), Err(InvalidConfiguration));
        assert_eq!(fs::read(fixture.models()).unwrap(), updated);
    }
}

#[test]
fn aggregation_t36_checks_ownership_and_upstream_revision() {
    let fixture = Fixture::new();
    let revision = fixture.revision();
    let calls = Cell::new(0);
    let result = fixture.open().apply_directory_change(
        &fixture.tx,
        &fixture.tenant,
        &revision,
        "fixture",
        Insert(&profile("first")),
        || {
            calls.set(calls.get() + 1);
            calls.get() == 1
        },
    );
    assert_eq!(result, Err(RevisionConflict));
    assert!(!fixture.models().exists());

    fixture.apply(&revision, Insert(&profile("first"))).unwrap();
    let current = fs::read(fixture.models()).unwrap();
    let other = TenantScope::new("project", "other-account").unwrap();
    let result = fixture.open().apply_directory_change(
        &fixture.tx,
        &other,
        &fixture.revision(),
        "fixture",
        Remove,
        || true,
    );
    assert_eq!(result, Err(Forbidden));
    assert_eq!(fs::read(fixture.models()).unwrap(), current);
}

#[test]
fn aggregation_t36_external_edit_at_publication_does_not_get_overwritten() {
    let fixture = Fixture::new();
    let path = fixture.models();
    let other_path = path.clone();
    let external = br#"{"providers":{"new-external":{}}}"#;
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(move |_, _, phase| {
            if phase == Phase::BeforePublish {
                fs::write(&other_path, external).unwrap();
            }
            Ok(())
        }));
    });
    let result = fixture.apply(&fixture.revision(), Insert(&profile("first")));
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(RevisionConflict));
    assert_eq!(fs::read(path).unwrap(), external);
    assert!(fixture.tx.pending(AppId::Pi).unwrap().is_none());
}

#[test]
fn aggregation_t36_revision_cannot_be_reused_for_another_directory() {
    let fixture = Fixture::new();
    let other = fixture.home.path().join("other");
    fs::create_dir(&other).unwrap();
    let mut files = PiFiles::open_directory(fixture.home.path(), Some(&other), None).unwrap();
    let result = files.apply_directory_change(
        &fixture.tx,
        &fixture.tenant,
        &fixture.revision(),
        "fixture",
        Insert(&profile("first")),
        || true,
    );
    assert_eq!(result, Err(RevisionConflict));
    assert!(!fixture.models().exists());
    assert!(!other.join("models.json").exists());
}

#[test]
fn aggregation_t36_unsafe_models_target_does_not_follow_symlink() {
    use super::PiDirectoryWriteError::TargetFailed;
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let revision = fixture.revision();
    let other = fixture.home.path().join("other.json");
    fs::write(&other, b"{}").unwrap();
    symlink(&other, fixture.models()).unwrap();

    assert_eq!(fixture.apply(&revision, Insert(&profile("first"))), Err(TargetFailed));
    assert_eq!(fs::read(&other).unwrap(), b"{}");
    assert_eq!(fs::read_link(fixture.models()).unwrap(), other);
}

#[test]
fn aggregation_t36_pending_intent_requires_recovery_before_another_write() {
    use super::{
        PiDirectoryWriteError::{RecoveryPending, TargetFailed},
        WriteError,
    };

    let fixture = Fixture::new();
    let revision = fixture.revision();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(
            |_, _, phase| {
                if phase == Phase::Intent { Err(WriteError::Io) } else { Ok(()) }
            },
        ));
    });
    let result = fixture.apply(&revision, Insert(&profile("first")));
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(TargetFailed));
    assert_eq!(fixture.apply(&revision, Insert(&profile("second"))), Err(RecoveryPending));
    assert!(!fixture.models().exists());
    let mut files = fixture.open();
    assert!(fixture.tx.recover(&mut files.guard).unwrap().is_some());
    assert!(!fixture.models().exists());
}
