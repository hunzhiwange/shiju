use super::{live::patch_json::object_value, openclaw::*};
use serde_json::json;
#[test]
fn cc04_openclaw_membership_keeps_unrelated_config() {
    let pre=br#"{/*keep*/"models":{"mode":"merge","providers":{"other":{"api":"fixture"}}},"agents":{"defaults":{"workspace":"keep"}},"memory":{"keep":true}}"#;
    let declaration = json!({"baseUrl":"https://fixture.invalid","api":"openai-completions","models":[{"id":"one"}]});
    let out = patch(Some(pre), "selected", Some(&declaration), None).unwrap();
    assert_eq!(object_value(&out).unwrap()["models"]["providers"]["selected"], declaration);
    assert!(String::from_utf8_lossy(&out).contains("/*keep*/"));
    let removed = patch(Some(&out), "selected", None, None).unwrap();
    assert_eq!(object_value(&removed).unwrap(), object_value(pre).unwrap());
}
#[test]
fn cc04_openclaw_primary_preserves_remaining_fallbacks() {
    let pre=br#"{"models":{"providers":{"selected":{"models":[{"id":"one"}]}}},"agents":{"defaults":{"model":{"primary":"old/model","fallbacks":["other/a","selected/one","other/b","selected/one"],"future":{"keep":true}},"workspace":"keep","models":{"other/a":{}}}},"memory":{"keep":true}}"#;
    let out = patch(Some(pre), "selected", None, Some("selected/one")).unwrap();
    let mut expected = object_value(pre).unwrap();
    expected["agents"]["defaults"]["model"]["primary"] = json!("selected/one");
    expected["agents"]["defaults"]["model"]["fallbacks"] = json!(["other/a", "other/b"]);
    assert_eq!(object_value(&out).unwrap(), expected);
}

use std::os::unix::fs::PermissionsExt;
#[test]
fn cc04_openclaw_transaction_failure_recovers_without_unique_pointer() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = OpenClawFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::OpenClaw).unwrap();
        let (name, pre) = files.snapshot().unwrap();
        let next =
            patch(pre.as_deref(), "one", Some(&json!({"models":[{"id":"m"}]})), None).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let result = files.apply(
            &tx,
            &TenantScope::new("p", "a").unwrap(),
            &expected,
            name,
            pre.as_deref(),
            next,
            || true,
        );
        HOOK.with_borrow_mut(|h| *h = None);
        assert!(result.is_err());
        drop(files);
        drop(tx);
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let mut files = OpenClawFiles::open(home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let state = tx.state(AppId::OpenClaw).unwrap();
        assert!(state.pointer.is_none());
        assert!(tx.pending(AppId::OpenClaw).unwrap().is_none());
        let bytes = read(home.path()).unwrap();
        assert_eq!(
            declarations(bytes.as_deref()).unwrap().contains_key("one"),
            phase != Phase::Intent
        );
        if phase != Phase::Intent {
            assert!(state.mode.unwrap().mode == Mode::Direct);
        }
    }
}
#[test]
fn cc04_openclaw_input_and_external_file_conflicts_do_not_publish() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = OpenClawFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::OpenClaw).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next = patch(pre.as_deref(), "one", Some(&json!({"models":[{"id":"m"}]})), None).unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || false),
        Err(WriteError::Conflict)
    );
    assert!(read(home.path()).unwrap().is_none());
    let path = home.path().join(".openclaw/openclaw.json");
    std::fs::write(&path, b"{/*external*/}").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"{/*external*/}");
    assert!(tx.state(AppId::OpenClaw).unwrap().mode.is_none());
}

#[test]
fn cc04_openclaw_malformed_and_undeclared_models_are_rejected() {
    for pre in [
        b"".as_slice(),
        b"[]",
        br#"{"models":null}"#,
        br#"{"models":{"providers":[]}}"#,
        br#"{"agents":{"defaults":{"model":{"fallbacks":"bad"}}}}"#,
        br#"{"models":{},"models":{}}"#,
    ] {
        assert!(patch(Some(pre), "one", None, None).is_err());
    }
    for declaration in [
        json!({"models":{}}),
        json!({"models":[{"id":""}]}),
        json!({"models":[{"id":"x"},{"id":"x"}]}),
        json!({"agents":{}}),
    ] {
        assert!(patch(None, "one", Some(&declaration), None).is_err());
    }
    let pre = br#"{"models":{"providers":{"one":{"models":[{"id":"group/model"}]}}}}"#;
    assert!(patch(Some(pre), "one", None, Some("one/missing")).is_err());
    assert!(patch(Some(pre), "one", None, Some("one/group/model")).is_ok());
}
#[test]
fn cc04_openclaw_read_symlink_and_missing_removal_are_safe() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    assert!(read(home.path()).unwrap().is_none());
    assert!(!home.path().join(".openclaw").exists());
    let mut files = OpenClawFiles::open(home.path()).unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    use super::live::commit::*;
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::OpenClaw).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    files
        .apply(
            &tx,
            &TenantScope::new("p", "a").unwrap(),
            &expected,
            name,
            pre.as_deref(),
            patch(pre.as_deref(), "absent", None, None).unwrap(),
            || true,
        )
        .unwrap();
    assert!(read(home.path()).unwrap().is_none());
    drop(files);
    let outside = home.path().join("outside.json");
    std::fs::write(&outside, b"{}").unwrap();
    symlink(&outside, home.path().join(".openclaw/openclaw.json")).unwrap();
    assert!(read(home.path()).is_err());
    assert!(OpenClawFiles::open(home.path()).unwrap().snapshot().is_err());
    assert_eq!(std::fs::read(outside).unwrap(), b"{}");
}
#[test]
fn cc04_openclaw_new_model_seed_and_nested_comments_survive() {
    let out = patch(None, "one", Some(&json!({"models":[{"id":"m"}]})), None).unwrap();
    assert_eq!(object_value(&out).unwrap()["models"]["mode"], "merge");
    let pre=br#"{"models":{"mode":"replace","providers":{"one":{/*keep*/"apiKey":"fake-old","removed":true,"models":[{"id":"m"}]}}}}"#;
    let out =
        patch(Some(pre), "one", Some(&json!({"apiKey":"fake-new","models":[{"id":"m"}]})), None)
            .unwrap();
    assert!(String::from_utf8_lossy(&out).contains("/*keep*/"));
    assert_eq!(object_value(&out).unwrap()["models"]["mode"], "replace");
    assert!(object_value(&out).unwrap()["models"]["providers"]["one"].get("removed").is_none());
}

#[test]
fn cc04_openclaw_catalog_reads_members_without_unique_current_or_secrets() {
    use vw_api_types::{agent_apps::AppId, app_providers::catalog::*};
    let home = tempfile::tempdir().unwrap();
    let missing = super::catalog::read(home.path(), AppId::OpenClaw, vec![]);
    assert_eq!(missing.native_read, None);
    assert!(!home.path().join(".openclaw").exists());
    std::fs::create_dir(home.path().join(".openclaw")).unwrap();
    let file = home.path().join(".openclaw/openclaw.json");
    std::fs::write(&file, br#"{"models":{"providers":{"one":{"apiKey":"fake-secret"}}}}"#).unwrap();
    let result = super::catalog::read(home.path(), AppId::OpenClaw, vec![]);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].provider.id, "one");
    assert_eq!(result.rows[0].native_member, ProviderMembership::Present);
    assert_eq!(result.direct, ProviderSelection::NotApplicable);
    assert_eq!(result.in_use, ProviderSelection::NotApplicable);
    assert!(!serde_json::to_string(&result).unwrap().contains("fake-secret"));
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::OpenClaw, "one").is_err());
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::OpenClaw, "unused").is_ok());
    let before = std::fs::read(&file).unwrap();
    assert_eq!(before, read(home.path()).unwrap().unwrap());
    std::fs::write(&file, b"{bad}").unwrap();
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::OpenClaw, "unused").is_err());
}

#[test]
fn cc04_openclaw_json5_provider_edits_preserve_non_target_tokens() {
    let source = b"{models:{providers:{one:{models:[{id:'x'}]}}}, agents:{defaults:{model:{primary:'one/x'}}}, env:{n:NaN, key:'keep'}}";
    let next = patch(Some(source), "one", Some(&json!({"models":[{"id":"x"}], "api":"openai-completions"})), None).unwrap();
    assert!(String::from_utf8(next.clone()).unwrap().contains("env:{n:NaN, key:'keep'}"));
    assert_eq!(state(Some(&next)).unwrap().model.as_deref(), Some("one/x"));
}
