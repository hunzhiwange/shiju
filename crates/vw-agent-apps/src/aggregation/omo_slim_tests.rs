use super::omo_slim::{OmoSlimChange, OmoSlimError, snapshot, write};
use crate::providers::live::{
    commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions},
    patch_json::object_value,
    recovery::RecoveryOutcome,
    write_guard::WriteError,
};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

#[test]
fn aggregation_t34_omo_slim_preserves_contract() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let directory = home.join("custom-opencode");
    fs::create_dir(&directory).unwrap();
    fs::create_dir(home.join(".omo")).unwrap();
    let standard = home.join(".omo/omo.jsonc");
    let standard_source = r#"{"[opencode]":{"agents":{"standard":{}}}}"#;
    fs::write(&standard, standard_source).unwrap();
    let preferred = directory.join("oh-my-opencode-slim.jsonc");
    fs::write(
        &preferred,
        r#"{
            // Slim profile
            "agents":{"builder":{"model":"provider/slim","apiKey":"fixture-key"}},
            "categories":{"quick":{}},
            "disabled_agents":["reviewer"],
        }"#,
    )
    .unwrap();
    fs::write(directory.join("oh-my-opencode-slim.json"), r#"{"agents":{"fallback":{}}}"#).unwrap();

    let view =
        snapshot(home, Some("~/custom-opencode")).unwrap().read(Some("slim-profile")).unwrap();
    assert_eq!(view.current_provider_id.as_deref(), Some("slim-profile"));
    assert_eq!(json!(view.agents), json!({"builder":{"model":"provider/slim"}}));
    assert_eq!(json!(view.other_fields), json!({"disabled_agents":["reviewer"]}));
    assert_eq!(fs::read_to_string(standard).unwrap(), standard_source);
}

struct Fixture {
    home: tempfile::TempDir,
    _private: tempfile::TempDir,
    tx: ProviderTransactions,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(
            private.path(),
            ExecutionHostId::new("slim-fixture").unwrap(),
        )
        .unwrap();
        Self { home, _private: private, tx }
    }
}

#[test]
fn aggregation_t34_apply_and_disable_touch_only_slim_candidates() {
    let fixture = Fixture::new();
    let home = fixture.home.path();
    let tenant = TenantScope::default();
    let directory = home.join("custom-opencode");
    fs::create_dir(&directory).unwrap();
    fs::create_dir(home.join(".omo")).unwrap();
    let standards = [
        home.join(".omo/omo.jsonc"),
        directory.join("oh-my-openagent.jsonc"),
        directory.join("opencode.jsonc"),
        directory.join("oh-my-opencode.json"),
    ];
    for path in &standards {
        fs::write(path, r#"{"marker":true}"#).unwrap();
    }
    let preferred = directory.join("oh-my-opencode-slim.jsonc");
    let fallback = directory.join("oh-my-opencode-slim.json");
    fs::write(&fallback, r#"{"agents":{"fallback":{}}}"#).unwrap();
    fs::write(
        &preferred,
        concat!(
            "\u{feff}{\r\n",
            "  // Keep local field\r\n",
            "  \"local\": true,\r\n",
            "  \"agents\": {\"builder\":{\r\n",
            "    // Keep model note\r\n",
            "    \"model\":\"old/model\"\r\n",
            "  }},\r\n",
            "  \"categories\":{\"old\":{}},\r\n",
            "}\r\n"
        ),
    )
    .unwrap();
    let profile = json!({
        "agents": {"builder":{"model":"provider/slim"}},
        "categories": {"incoming": {}},
        "disabled_agents": ["reviewer"]
    });
    let fields = profile.as_object().unwrap();
    let directory_override = Some("~/custom-opencode");
    let current = || snapshot(home, directory_override).unwrap();
    let apply = || OmoSlimChange::Apply { fields, standard_provider_id: None };
    assert_eq!(write(home, &fixture.tx, &tenant, &current(), apply(), || true), Ok(true));
    let result = fs::read_to_string(&preferred).unwrap();
    assert_eq!(
        object_value(result.as_bytes()).unwrap(),
        json!({
            "agents": {"builder":{"model":"provider/slim"}},
            "disabled_agents": ["reviewer"],
            "local": true,
        })
    );
    assert!(result.starts_with('\u{feff}'));
    assert!(result.contains("// Keep local field\r\n"));
    assert!(result.contains("// Keep model note\r\n"));
    assert!(!result.replace("\r\n", "").contains('\n'));
    assert_eq!(fs::read_to_string(&fallback).unwrap(), r#"{"agents":{"fallback":{}}}"#);
    assert_eq!(write(home, &fixture.tx, &tenant, &current(), apply(), || true), Ok(false));
    assert_eq!(
        write(home, &fixture.tx, &tenant, &current(), OmoSlimChange::Disable, || true),
        Ok(true)
    );
    assert!(!preferred.exists());
    assert!(!fallback.exists());
    assert_eq!(
        write(home, &fixture.tx, &tenant, &current(), OmoSlimChange::Disable, || true),
        Ok(false)
    );
    assert_eq!(current().read(None).unwrap().current_provider_id, None);
    for path in &standards {
        assert_eq!(fs::read_to_string(path).unwrap(), r#"{"marker":true}"#);
    }
}

#[test]
fn aggregation_t34_first_apply_and_existing_json_fallback() {
    let fixture = Fixture::new();
    let home = fixture.home.path();
    let tenant = TenantScope::default();
    let preferred = home.join(".config/opencode/oh-my-opencode-slim.jsonc");
    let fallback = preferred.with_extension("json");
    let current = || snapshot(home, None).unwrap();
    let profile = json!({"agents":{"builder":{}}});
    let apply = || OmoSlimChange::Apply {
        fields: profile.as_object().unwrap(),
        standard_provider_id: None,
    };
    assert_eq!(
        write(home, &fixture.tx, &tenant, &current(), OmoSlimChange::Disable, || true),
        Ok(false)
    );
    assert!(!preferred.parent().unwrap().exists());
    assert_eq!(write(home, &fixture.tx, &tenant, &current(), apply(), || true), Ok(true));
    assert!(preferred.exists());

    fs::write(&fallback, "{}").unwrap();
    let stale = current();
    fs::remove_file(&preferred).unwrap();
    assert_eq!(
        write(home, &fixture.tx, &tenant, &stale, apply(), || true),
        Err(OmoSlimError::RevisionConflict)
    );
    assert_eq!(write(home, &fixture.tx, &tenant, &current(), apply(), || true), Ok(true));
    assert!(!preferred.exists());
    assert_eq!(object_value(&fs::read(&fallback).unwrap()).unwrap(), profile);
}

#[test]
fn aggregation_t34_refuses_stale_scope_and_conflicting_variant() {
    let fixture = Fixture::new();
    let home = fixture.home.path();
    let tenant = TenantScope::default();
    let directory = home.join(".config/opencode");
    fs::create_dir_all(&directory).unwrap();
    let preferred = directory.join("oh-my-opencode-slim.jsonc");
    let fallback = directory.join("oh-my-opencode-slim.json");
    fs::write(&preferred, "{}").unwrap();
    let profile = json!({"agents":{"builder":{}}});
    let fields = profile.as_object().unwrap();
    let current = || snapshot(home, None).unwrap();
    assert_eq!(
        write(
            home,
            &fixture.tx,
            &tenant,
            &current(),
            OmoSlimChange::Apply { fields, standard_provider_id: Some("standard-profile") },
            || true,
        ),
        Err(OmoSlimError::VariantConflict)
    );
    assert_eq!(
        write(
            home,
            &fixture.tx,
            &tenant,
            &current(),
            OmoSlimChange::Apply { fields, standard_provider_id: None },
            || false,
        ),
        Err(OmoSlimError::RevisionConflict)
    );
    let stale = current();
    fs::write(&fallback, "{}").unwrap();
    assert_eq!(
        write(home, &fixture.tx, &tenant, &stale, OmoSlimChange::Disable, || true),
        Err(OmoSlimError::RevisionConflict)
    );
    assert_eq!(fs::read_to_string(&preferred).unwrap(), "{}");
    assert_eq!(fs::read_to_string(&fallback).unwrap(), "{}");
}

#[test]
fn aggregation_t34_refuses_invalid_and_unsafe_targets() {
    let fixture = Fixture::new();
    let home = fixture.home.path();
    let directory = home.join(".config/opencode");
    fs::create_dir_all(&directory).unwrap();
    let preferred = directory.join("oh-my-opencode-slim.jsonc");
    let fallback = preferred.with_extension("json");
    fs::write(&preferred, r#"{"agents":1}"#).unwrap();
    fs::write(&fallback, "{}").unwrap();
    assert_eq!(snapshot(home, Some("../outside")).err(), Some(OmoSlimError::Forbidden));
    let current = snapshot(home, None).unwrap();
    assert_eq!(current.read(None), Err(OmoSlimError::InvalidConfiguration));
    let fields = json!({"agents":{"builder":{}}});
    assert_eq!(
        write(
            home,
            &fixture.tx,
            &TenantScope::default(),
            &current,
            OmoSlimChange::Apply {
                fields: fields.as_object().unwrap(),
                standard_provider_id: None
            },
            || true
        ),
        Err(OmoSlimError::InvalidConfiguration)
    );
    fs::remove_file(&preferred).unwrap();
    std::os::unix::fs::symlink(&fallback, &preferred).unwrap();
    assert_eq!(snapshot(home, None).err(), Some(OmoSlimError::TargetFailed));
    assert_eq!(fs::read_to_string(&fallback).unwrap(), "{}");
}

#[test]
fn aggregation_t34_disable_recovers_partial_publication() {
    let fixture = Fixture::new();
    let home = fixture.home.path();
    let directory = home.join(".config/opencode");
    fs::create_dir_all(&directory).unwrap();
    let preferred = directory.join("oh-my-opencode-slim.jsonc");
    let fallback = preferred.with_extension("json");
    fs::write(&preferred, "{}").unwrap();
    fs::write(&fallback, "{}").unwrap();
    let expected = snapshot(home, None).unwrap();
    HOOK.set(Some(Box::new(|_, app, phase| {
        if app == AppId::OpenCode && phase == Phase::FilePublished {
            Err(WriteError::Io)
        } else {
            Ok(())
        }
    })));
    let result = write(
        home,
        &fixture.tx,
        &TenantScope::default(),
        &expected,
        OmoSlimChange::Disable,
        || true,
    );
    HOOK.set(None);
    assert_eq!(result, Err(OmoSlimError::TargetFailed));
    assert!(!preferred.exists());
    assert!(fallback.exists());
    assert!(fixture.tx.pending(AppId::OpenCode).unwrap().unwrap().published);
    assert_eq!(
        fixture.tx.recover_on_startup(),
        vec![(AppId::OpenCode, Ok(RecoveryOutcome::RolledForward))]
    );
    assert!(!fallback.exists());
    assert!(fixture.tx.pending(AppId::OpenCode).unwrap().is_none());
}

#[test]
fn aggregation_t34_first_apply_provisions_saved_override() {
    let fixture = Fixture::new();
    let home = fixture.home.path();
    let override_dir = "~/custom/open-code";
    let expected = snapshot(home, Some(override_dir)).unwrap();
    let profile = json!({"agents":{"builder":{}}});
    assert_eq!(
        write(
            home,
            &fixture.tx,
            &TenantScope::default(),
            &expected,
            OmoSlimChange::Apply {
                fields: profile.as_object().unwrap(),
                standard_provider_id: None
            },
            || true
        ),
        Ok(true)
    );
    assert!(home.join("custom/open-code/oh-my-opencode-slim.jsonc").exists());
    assert!(!home.join(".config").exists());
}
