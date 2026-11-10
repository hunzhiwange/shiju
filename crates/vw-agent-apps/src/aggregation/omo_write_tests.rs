use super::omo_write::{OmoChange, OmoWriteError, snapshot, write};
use crate::providers::live::{
    commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions},
    patch_json::object_value,
};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

struct Fixture {
    home: tempfile::TempDir,
    _private: tempfile::TempDir,
    tx: ProviderTransactions,
    path: PathBuf,
}

impl Fixture {
    fn new(source: &str) -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("omo-fixture").unwrap())
                .unwrap();
        let path = home.path().join(".omo/omo.jsonc");
        fs::create_dir(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        Self { home, _private: private, tx, path }
    }
}

#[test]
fn aggregation_t33_omo_write_preserves_contract() {
    let source = concat!(
        "\u{feff}{\r\n",
        "  // Shared migration metadata\r\n",
        "  \"_migrations\": [\"done\"],\r\n",
        "  \"[opencode]\": {\r\n",
        "    \"agents\": {\"builder\": {\r\n",
        "      // Model override\r\n",
        "      \"model\": \"old/model\", \"temperature\": 0.2\r\n",
        "    }},\r\n",
        "    \"categories\": {\"quick\": {\"model\": \"provider/fast\"}}\r\n",
        "  },\r\n",
        "  // Codex configuration\r\n",
        "  \"[codex]\": {\"agents\": {\"reviewer\": {}}},\r\n",
        "}\r\n",
    );
    let fixture = Fixture::new(source);
    let home = fixture.home.path();
    let legacy = home.join(".config/opencode/oh-my-openagent.jsonc");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, "{}").unwrap();
    let secondary = home.join(".omo/omo.json");
    fs::write(&secondary, r#"{"[opencode]":{"agents":{"secondary":{}}}}"#).unwrap();
    let tenant = TenantScope::default();
    let profile = json!({
        "agents": {"builder": {"model": "new/model", "temperature": 0.2}},
        "categories": {"quick": {"model": "provider/fast"}},
    });

    let expected = snapshot(home).unwrap();
    assert_eq!(
        write(
            home,
            &fixture.tx,
            &tenant,
            &expected,
            OmoChange::Apply(profile.as_object().unwrap()),
            || true
        ),
        Ok(true)
    );
    assert_eq!(
        fs::read_to_string(&fixture.path).unwrap(),
        source.replace("old/model", "new/model")
    );
    let expected = snapshot(home).unwrap();
    assert_eq!(write(home, &fixture.tx, &tenant, &expected, OmoChange::Disable, || true), Ok(true));
    let disabled = fs::read(&fixture.path).unwrap();
    assert_eq!(
        object_value(&disabled).unwrap(),
        json!({"_migrations":["done"], "[codex]":{"agents":{"reviewer":{}}}})
    );
    let disabled_text = std::str::from_utf8(&disabled).unwrap();
    assert!(disabled_text.contains(
        "  // Codex configuration\r\n  \"[codex]\": {\"agents\": {\"reviewer\": {}}},\r\n"
    ));
    assert!(
        disabled_text
            .contains("  // Shared migration metadata\r\n  \"_migrations\": [\"done\"],\r\n")
    );
    assert!(!disabled_text.replace("\r\n", "").contains('\n'));
    let ino = fs::metadata(&fixture.path).unwrap().ino();
    let expected = snapshot(home).unwrap();
    assert_eq!(
        write(home, &fixture.tx, &tenant, &expected, OmoChange::Disable, || true),
        Ok(false)
    );
    assert_eq!(fs::metadata(&fixture.path).unwrap().ino(), ino);
    assert_eq!(fs::read(&fixture.path).unwrap(), disabled);
    assert_eq!(fs::read_to_string(legacy).unwrap(), "{}");
    assert_eq!(
        fs::read_to_string(secondary).unwrap(),
        r#"{"[opencode]":{"agents":{"secondary":{}}}}"#
    );
}

#[test]
fn aggregation_t33_omo_write_refuses_external_edits_at_publication() {
    let fixture = Fixture::new(r#"{"[opencode]":{"agents":{}}}"#);
    let home = fixture.home.path();
    let expected = snapshot(home).unwrap();
    let path = fixture.path.clone();
    let concurrent = r#"{"[opencode]":{"agents":{}},"[codex]":{"changed":true}}"#;
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(move |_, app, phase| {
            if app == AppId::OpenCode && phase == Phase::BeforePublish {
                fs::write(&path, concurrent).unwrap();
            }
            Ok(())
        }));
    });
    let result =
        write(home, &fixture.tx, &TenantScope::default(), &expected, OmoChange::Disable, || true);
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(OmoWriteError::RevisionConflict));
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), concurrent);
    assert!(fixture.tx.pending(AppId::OpenCode).unwrap().is_none());
}

#[test]
fn aggregation_t33_omo_write_rejects_bad_config_and_unsafe_targets() {
    let fixture = Fixture::new("{}");
    let home = fixture.home.path();
    let tenant = TenantScope::default();
    let expected = snapshot(home).unwrap();
    assert_eq!(
        write(home, &fixture.tx, &tenant, &expected, OmoChange::Disable, || false),
        Err(OmoWriteError::RevisionConflict)
    );
    assert_eq!(snapshot(std::path::Path::new(".")).err(), Some(OmoWriteError::Forbidden));
    for malformed in [
        r#"{"[opencode]":{"agents":/* unterminated"#,
        r#"{"[opencode]":[]}"#,
        r#"{"[opencode]":{"agents":[]}}"#,
        r#"{"[opencode]":{},"[open\u0063ode]":{}}"#,
    ] {
        fs::write(&fixture.path, malformed).unwrap();
        let expected = snapshot(home).unwrap();
        assert_eq!(
            write(home, &fixture.tx, &tenant, &expected, OmoChange::Disable, || true),
            Err(OmoWriteError::InvalidConfiguration)
        );
        assert_eq!(fs::read_to_string(&fixture.path).unwrap(), malformed);
    }

    fs::remove_file(&fixture.path).unwrap();
    assert_eq!(snapshot(home).err(), Some(OmoWriteError::UnsupportedTarget));
    let target = home.join(".omo/omo.json");
    fs::write(&target, "{}").unwrap();
    std::os::unix::fs::symlink(&target, &fixture.path).unwrap();
    assert_eq!(snapshot(home).err(), Some(OmoWriteError::TargetFailed));
    assert_eq!(fs::read_to_string(target).unwrap(), "{}");
}

#[test]
fn aggregation_t33_omo_write_rejects_changed_file_precedence() {
    let fixture = Fixture::new(r#"{"[opencode]":{}}"#);
    let home = fixture.home.path();
    let secondary = home.join(".omo/omo.json");
    fs::rename(&fixture.path, &secondary).unwrap();
    let expected = snapshot(home).unwrap();
    fs::write(&fixture.path, r#"{"[opencode]":{}}"#).unwrap();
    assert_eq!(
        write(home, &fixture.tx, &TenantScope::default(), &expected, OmoChange::Disable, || true),
        Err(OmoWriteError::RevisionConflict)
    );
    assert_eq!(fs::read(&fixture.path).unwrap(), fs::read(secondary).unwrap());
}

#[test]
fn aggregation_t33_omo_write_preserves_trivia_in_changed_arrays() {
    let source = concat!(
        "{\r\n",
        "  \"[opencode]\": {\r\n",
        "    \"disabled_agents\": [\r\n",
        "      \"one\", // Keep this explanation\r\n",
        "      \"two\" // Keep the last explanation\r\n",
        "    ],\r\n",
        "    \"agents\": {\"old\":{}, /* keep */ \"builder\":{\"model\":\"old/model\"}}\r\n",
        "  },\r\n",
        "  \"[codex]\": {}\r\n",
        "}\r\n",
    );
    let fixture = Fixture::new(source);
    let home = fixture.home.path();
    let tenant = TenantScope::default();
    for disabled_agents in [json!(["one", "two", "three"]), json!(["one"]), json!([])] {
        let profile = json!({
            "disabled_agents": disabled_agents,
            "agents": {"builder": {"model": "new/model"}, "new": {}},
        });
        let expected = snapshot(home).unwrap();
        assert_eq!(
            write(
                home,
                &fixture.tx,
                &tenant,
                &expected,
                OmoChange::Apply(profile.as_object().unwrap()),
                || true,
            ),
            Ok(true)
        );
        let result = fs::read_to_string(&fixture.path).unwrap();
        assert_eq!(
            object_value(result.as_bytes()).unwrap(),
            json!({"[opencode]": profile, "[codex]": {}})
        );
        for comment in
            ["// Keep this explanation\r\n", "// Keep the last explanation\r\n", "/* keep */"]
        {
            assert!(result.contains(comment));
        }
        assert!(!result.replace("\r\n", "").contains('\n'));
    }
}

#[test]
fn aggregation_t33_omo_write_inserts_the_section_without_rewriting_codex() {
    let source = "{\r\n  // Keep Codex\r\n  \"[codex]\":{\"enabled\":true}\r\n}\r\n";
    let fixture = Fixture::new(source);
    let home = fixture.home.path();
    let tenant = TenantScope::default();
    let profile = json!({"agents": {"builder": {"model": "provider/model"}}});
    let expected = snapshot(home).unwrap();
    let apply = || OmoChange::Apply(profile.as_object().unwrap());
    assert_eq!(write(home, &fixture.tx, &tenant, &expected, apply(), || true), Ok(true));
    let result = fs::read_to_string(&fixture.path).unwrap();
    assert_eq!(
        object_value(result.as_bytes()).unwrap(),
        json!({"[codex]":{"enabled":true}, "[opencode]": profile})
    );
    assert!(result.contains("// Keep Codex\r\n  \"[codex]\":{\"enabled\":true}"));
    let ino = fs::metadata(&fixture.path).unwrap().ino();
    let expected = snapshot(home).unwrap();
    assert_eq!(write(home, &fixture.tx, &tenant, &expected, apply(), || true), Ok(false));
    assert_eq!(fs::metadata(&fixture.path).unwrap().ino(), ino);
}

#[test]
fn aggregation_t33_omo_write_leaves_other_opencode_recovery_to_its_owner() {
    use crate::providers::{
        live::{
            stage::{FileChange, WholeFile},
            write_guard::{FilePrivacy, WriteError},
        },
        opencode::OpenCodeFiles,
    };
    use std::ffi::OsStr;

    let source = r#"{"[opencode]":{"agents":{}}}"#;
    let fixture = Fixture::new(source);
    let home = fixture.home.path();
    let tenant = TenantScope::default();
    let mut files = OpenCodeFiles::open(home).unwrap();
    let patch = WholeFile::new(None, Some(b"{}".to_vec()));
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(
            |_, _, phase| {
                if phase == Phase::Intent { Err(WriteError::Io) } else { Ok(()) }
            },
        ));
    });
    let staged = fixture.tx.commit(
        &tenant,
        &mut files.guard,
        &[FileChange {
            name: OsStr::new("opencode.json"),
            privacy: FilePrivacy::Private,
            patch: &patch,
        }],
        Default::default(),
    );
    HOOK.with_borrow_mut(|hook| *hook = None);
    drop(files);
    assert_eq!(staged, Err(WriteError::Io));

    let expected = snapshot(home).unwrap();
    assert_eq!(
        write(home, &fixture.tx, &tenant, &expected, OmoChange::Disable, || true),
        Err(OmoWriteError::RecoveryPending)
    );
    assert!(fixture.tx.pending(AppId::OpenCode).unwrap().is_some());
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), source);
}
