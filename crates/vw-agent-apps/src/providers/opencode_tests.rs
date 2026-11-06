use super::{live::patch_json::object_value, opencode::*};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use vw_api_types::app_providers::opencode::OpenCodeFormat;
#[test]
fn cc04_opencode_membership_preserves_other_providers() {
    let pre = br#"{/*keep*/"provider":{"other":{"options":{"apiKey":"fake"}}},"mcp":{"x":{}},"theme":"dark"}"#;
    let declaration = json!({"npm":"@ai-sdk/openai-compatible","models":{"one":{}}});
    let added = patch(Some(pre), "selected", OpenCodeFormat::V1, Some(&declaration), None).unwrap();
    let value = object_value(&added).unwrap();
    assert_eq!(value["provider"]["selected"], declaration);
    assert_eq!(value["provider"]["other"], json!({"options":{"apiKey":"fake"}}));
    assert!(String::from_utf8_lossy(&added).contains("/*keep*/"));
    let removed = patch(Some(&added), "selected", OpenCodeFormat::V1, None, None).unwrap();
    assert_eq!(object_value(&removed).unwrap(), object_value(pre).unwrap());
}
#[test]
fn cc04_opencode_default_model_keeps_small_model() {
    let pre = br#"{"provider":{"selected":{"models":{"one":{}}}},"small_model":"other/small"}"#;
    let out = patch(Some(pre), "selected", OpenCodeFormat::V1, None, Some("selected/one")).unwrap();
    let value = object_value(&out).unwrap();
    assert_eq!(value["model"], "selected/one");
    assert_eq!(value["small_model"], "other/small");
    assert!(
        patch(Some(pre), "selected", OpenCodeFormat::V1, None, Some("selected/missing")).is_err()
    );
}

#[test]
fn cc04_opencode_native_precedence_and_removal_never_resurrect_legacy() {
    let pre=br#"{"provider":{"same":{"options":{"apiKey":"fake"}},"other":{}},"providers":{"same":{"package":"fixture","models":{"m":{}}},"invalid":{"settings":{"transport":"unknown"}}}}"#;
    assert_eq!(declarations(Some(pre)).unwrap()["same"].format, OpenCodeFormat::V2);
    assert!(
        patch(
            Some(pre),
            "same",
            OpenCodeFormat::V1,
            Some(&json!({"npm":"fixture","models":{"m":{}}})),
            None
        )
        .is_err()
    );
    let removed = patch(Some(pre), "same", OpenCodeFormat::V1, None, None).unwrap();
    let value = object_value(&removed).unwrap();
    assert!(value["provider"].get("same").is_none());
    assert!(value["providers"].get("same").is_none());
    assert_eq!(value["providers"]["invalid"], json!({"settings":{"transport":"unknown"}}));
    assert_eq!(
        declarations(Some(&removed)).unwrap().into_keys().collect::<Vec<_>>(),
        vec!["other"]
    );
}
#[test]
fn cc04_opencode_malformed_shapes_and_partial_new_ids_are_rejected() {
    for pre in [
        b"".as_slice(),
        b"[]",
        b"{\"provider\":null}",
        b"{\"providers\":[]}",
        b"{\"provider\":{},\"provider\":{}}",
    ] {
        assert!(patch(Some(pre), "one", OpenCodeFormat::V1, None, None).is_err());
    }
    assert!(
        patch(None, "one", OpenCodeFormat::V1, Some(&json!({"options":{"apiKey":"fake"}})), None)
            .is_err()
    );
    assert!(
        patch(None, "one", OpenCodeFormat::V2, Some(&json!({"settings":{"timeout":"bad"}})), None)
            .is_err()
    );
    let native = patch(
        None,
        "one",
        OpenCodeFormat::V2,
        Some(&json!({"models":{"group/model":{"name":"fixture"}}})),
        None,
    )
    .unwrap();
    assert!(patch(Some(&native), "one", OpenCodeFormat::V2, None, Some("one/group/model")).is_ok());
}
#[test]
fn cc04_opencode_jsonc_priority_and_symlinks_are_guarded() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join(".config/opencode");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("opencode.json"), b"{\"provider\":{\"json\":{}}}").unwrap();
    let bytes = b"{/*keep*/\"provider\":{\"jsonc\":{}}}";
    std::fs::write(dir.join("opencode.jsonc"), bytes).unwrap();
    assert_eq!(read(home.path()).unwrap().as_deref(), Some(bytes.as_slice()));
    let files = OpenCodeFiles::open(home.path()).unwrap();
    assert_eq!(files.snapshot().unwrap().0, "opencode.jsonc");
    drop(files);
    std::fs::remove_file(dir.join("opencode.jsonc")).unwrap();
    symlink(dir.join("opencode.json"), dir.join("opencode.jsonc")).unwrap();
    assert!(read(home.path()).is_err());
    let files = OpenCodeFiles::open(home.path()).unwrap();
    assert!(files.snapshot().is_err());
}
#[test]
fn cc04_opencode_transaction_failure_recovers_without_unique_pointer() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = OpenCodeFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::OpenCode).unwrap();
        let (name, pre) = files.snapshot().unwrap();
        let next = patch(
            pre.as_deref(),
            "one",
            OpenCodeFormat::V2,
            Some(&json!({"models":{"m":{}}})),
            None,
        )
        .unwrap();
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
        let mut files = OpenCodeFiles::open(home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let state = tx.state(AppId::OpenCode).unwrap();
        assert!(state.pointer.is_none());
        assert!(tx.pending(AppId::OpenCode).unwrap().is_none());
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
fn cc04_opencode_input_and_external_file_conflicts_do_not_publish() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = OpenCodeFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::OpenCode).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next =
        patch(pre.as_deref(), "one", OpenCodeFormat::V2, Some(&json!({"models":{"m":{}}})), None)
            .unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || false),
        Err(WriteError::Conflict)
    );
    assert!(read(home.path()).unwrap().is_none());
    let path = home.path().join(".config/opencode/opencode.json");
    std::fs::write(&path, b"{/*external*/}").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"{/*external*/}");
    assert!(tx.state(AppId::OpenCode).unwrap().mode.is_none());
}
#[test]
fn cc04_opencode_new_jsonc_precedence_conflicts_and_missing_removal_stays_missing() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = OpenCodeFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::OpenCode).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next = patch(pre.as_deref(), "absent", OpenCodeFormat::V1, None, None).unwrap();
    files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true).unwrap();
    assert!(read(home.path()).unwrap().is_none());
    let expected = tx.state(AppId::OpenCode).unwrap();
    let next = patch(pre.as_deref(), "one", OpenCodeFormat::V2, Some(&json!({})), None).unwrap();
    std::fs::write(home.path().join(".config/opencode/opencode.jsonc"), b"{/*external*/}").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert!(!home.path().join(".config/opencode/opencode.json").exists());
}
#[test]
fn cc04_opencode_disabled_model_is_rejected_and_read_does_not_provision() {
    let home = tempfile::tempdir().unwrap();
    assert!(read(home.path()).unwrap().is_none());
    assert!(!home.path().join(".config").exists());
    let pre=br#"{"providers":{"one":{"models":{"disabled":{"disabled":true},"valid":{}}}},"small_model":"keep"}"#;
    assert!(patch(Some(pre), "one", OpenCodeFormat::V2, None, Some("one/disabled")).is_err());
    let out = patch(Some(pre), "one", OpenCodeFormat::V2, None, Some("one/valid")).unwrap();
    assert_eq!(object_value(&out).unwrap()["small_model"], "keep");
}
#[test]
fn cc04_opencode_edit_retains_nested_comments_and_removes_only_deleted_fields() {
    let pre=br#"{"provider":{"one":{/*provider*/"options":{/*key*/"apiKey":"fake-old","remove":1},"models":{"m":{/*model*/"name":"old"}}}},"small_model":"keep"}"#;
    let declaration = json!({"options":{"apiKey":"fake-new"},"models":{"m":{"name":"new"}}});
    let out = patch(Some(pre), "one", OpenCodeFormat::V1, Some(&declaration), None).unwrap();
    let text = String::from_utf8_lossy(&out);
    for comment in ["/*provider*/", "/*key*/", "/*model*/"] {
        assert!(text.contains(comment));
    }
    assert_eq!(object_value(&out).unwrap()["provider"]["one"], declaration);
}
