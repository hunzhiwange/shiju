use super::kimi::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::kimi::*;
fn declaration() -> Value {
    json!({"provider":{"type":"openai","base_url":"https://example.com/v1","api_key":"fake-secret"},"models":{"alias":{"model":"model","provider":"one","max_context_size":32000,"capabilities":["future"]}}})
}
#[test]
fn cc04_kimi_roundtrip_keeps_unrelated_tables() {
    let pre=b"# keep\ndefault_model = 'native'\n[providers.other]\ntype = 'kimi'\n[providers.other.oauth]\nkey = 'oauth/kimi-code'\nstorage = 'file'\n[models.native]\nprovider = 'other'\nmodel = 'native'\nmax_context_size = 32000\n[thinking]\neffort = 'high' # keep\n";
    let added = patch(Some(pre), "one", Some(&declaration()), KimiActivation::Add).unwrap();
    assert!(declarations(Some(&added)).unwrap().contains_key("one"));
    let mut edited = declaration();
    edited["provider"]["api_key"] = json!("fake-edited");
    let edited = patch(Some(&added), "one", Some(&edited), KimiActivation::IfMember).unwrap();
    assert_eq!(
        declarations(Some(&edited)).unwrap()["one"].declaration["provider"]["api_key"],
        "fake-edited"
    );
    let removed = patch(Some(&edited), "one", None, KimiActivation::Add).unwrap();
    assert_eq!(removed, pre);
}
#[test]
fn cc04_kimi_rejects_missing_provider_model_reference() {
    let mut node = declaration();
    node["models"]["alias"]["provider"] = json!("missing");
    assert!(patch(None, "one", Some(&node), KimiActivation::Add).is_err());
    let malformed = b"default_model='lost'\n[providers.one]\ntype='openai'\n";
    assert!(patch(Some(malformed), "one", Some(&declaration()), KimiActivation::IfMember).is_err());
}
#[test]
fn cc04_kimi_defaults_secondary_collisions_and_oauth_are_protected() {
    let added = patch(None, "one", Some(&declaration()), KimiActivation::Add).unwrap();
    assert!(patch(Some(&added), "one", None, KimiActivation::Add).is_err());
    let mut other = declaration();
    other["models"]["alias"]["provider"] = json!("other");
    assert!(patch(Some(&added), "other", Some(&other), KimiActivation::Add).is_err());
    let mut native: Value = toml::from_str(std::str::from_utf8(&added).unwrap()).unwrap();
    native.as_object_mut().unwrap().remove("default_model");
    native["secondary_model"] = json!({"models":{"alias":{}}});
    let bytes = toml::to_string(&native).unwrap();
    assert!(patch(Some(bytes.as_bytes()), "one", None, KimiActivation::Add).is_err());
    native.as_object_mut().unwrap().remove("secondary_model");
    native["providers"]["one"] = json!({"type":"kimi","base_url":"https://api.kimi.com/coding/v1","oauth":{"storage":"file","key":"oauth/kimi-code"}});
    let bytes = toml::to_string(&native).unwrap();
    let edited =
        patch(Some(bytes.as_bytes()), "one", Some(&declaration()), KimiActivation::IfMember)
            .unwrap();
    assert_eq!(
        declarations(Some(&edited)).unwrap()["one"].declaration["provider"],
        native["providers"]["one"]
    );
    assert!(
        patch(
            None,
            "one",
            Some(&declarations(Some(&edited)).unwrap()["one"].declaration),
            KimiActivation::Add
        )
        .is_err()
    );
    assert!(patch(Some(bytes.as_bytes()), "one", None, KimiActivation::Add).is_err());
}
#[test]
fn cc04_kimi_editor_scope_and_schema_are_checked() {
    let node = declaration();
    let doc = json!({"providers":{"one":node["provider"]},"models":node["models"]});
    let text = toml::to_string(&doc).unwrap();
    assert_eq!(parse_editor("one", &text).unwrap(), node);
    for extra in [
        json!({"default_model":"alias"}),
        json!({"providers":{"other":{}}}),
        json!({"services":{}}),
    ] {
        let mut bad = doc.clone();
        for (k, v) in extra.as_object().unwrap() {
            bad[k] = v.clone();
        }
        assert!(parse_editor("one", &toml::to_string(&bad).unwrap()).is_err());
    }
    for (path, value) in [
        ("type", json!("unsupported")),
        ("api_key", json!("[redacted]")),
        ("base_url", json!("file:///tmp/x")),
    ] {
        let mut bad = node.clone();
        bad["provider"][path] = value;
        assert!(patch(None, "one", Some(&bad), KimiActivation::Add).is_err());
    }
    assert_eq!(patch(None, "absent", None, KimiActivation::Add).unwrap(), b"");
    for text in ["[providers\n", "providers=3\n", "[models.orphan]\nprovider='missing'\n"] {
        assert!(state(Some(text.as_bytes())).is_err());
    }
}
#[test]
fn cc04_kimi_files_reject_symlinks_and_write_private() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let mut files = KimiFiles::open(home.path()).unwrap();
    assert!(files.snapshot().unwrap().1.is_none());
    files
        .guard
        .write(
            std::ffi::OsStr::new("config.toml"),
            b"# private\n",
            super::live::write_guard::FilePrivacy::Private,
        )
        .unwrap();
    assert_eq!(
        std::fs::metadata(home.path().join(".kimi-code/config.toml")).unwrap().permissions().mode()
            & 0o777,
        0o600
    );
    drop(files);
    std::fs::remove_file(home.path().join(".kimi-code/config.toml")).unwrap();
    std::os::unix::fs::symlink("/tmp/foreign", home.path().join(".kimi-code/config.toml")).unwrap();
    assert!(read(home.path()).is_err());
    assert!(KimiFiles::open(home.path()).unwrap().snapshot().is_err());
    let foreign = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/tmp", foreign.path().join(".kimi-code")).unwrap();
    assert!(KimiFiles::open(foreign.path()).is_err());
}

use std::os::unix::fs::PermissionsExt;
#[test]
fn cc04_kimi_transaction_failure_recovers_without_unique_pointer() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = KimiFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::Kimi).unwrap();
        let (name, pre) = files.snapshot().unwrap();
        let next = patch(pre.as_deref(), "one", Some(&declaration()), KimiActivation::Add).unwrap();
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
        let mut files = KimiFiles::open(home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let state = tx.state(AppId::Kimi).unwrap();
        assert!(state.pointer.is_none());
        assert!(tx.pending(AppId::Kimi).unwrap().is_none());
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
fn cc04_kimi_input_and_external_file_conflicts_do_not_publish() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = KimiFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::Kimi).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next = patch(pre.as_deref(), "one", Some(&declaration()), KimiActivation::Add).unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || false),
        Err(WriteError::Conflict)
    );
    assert!(read(home.path()).unwrap().is_none());
    let path = home.path().join(".kimi-code/config.toml");
    std::fs::write(&path, b"{\"external\":true}").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"{\"external\":true}");
    assert!(tx.state(AppId::Kimi).unwrap().mode.is_none());
}
