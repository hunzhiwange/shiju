use super::hermes::*;
use serde_json::json;
#[test]
fn cc04_hermes_roundtrip_keeps_unrelated_yaml() {
    let pre=b"# keep\nagent: {max_turns: 50}\ncustom_providers:\n  - name: one\n    api_key: fake-old\n    key_env: FIXTURE_KEY\n    future: keep\n  - name: other\n    api_key: fake-other\n";
    let out = patch(
        Some(pre),
        "one",
        Some(&json!({"api_key":"fake-new", "models":[{"id":"group/model","context_length":8192}]})),
    )
    .unwrap();
    let rows = declarations(Some(&out)).unwrap();
    assert_eq!(rows["one"].declaration["api_key"], "fake-new");
    assert_eq!(rows["one"].declaration["key_env"], "FIXTURE_KEY");
    assert_eq!(rows["one"].declaration["future"], "keep");
    assert!(String::from_utf8_lossy(&out).contains("# keep\nagent: {max_turns: 50}\n"));
    assert!(String::from_utf8_lossy(&out).contains("  - name: other\n    api_key: fake-other\n"));
}
#[test]
fn cc04_hermes_external_edit_is_not_lost() {
    let pre = b"custom_providers:\n  - name: one\n    api_key: fake-old\n    external: keep\n";
    let out = patch(Some(pre), "one", Some(&json!({"api_key":"fake-new"}))).unwrap();
    let rows = declarations(Some(&out)).unwrap();
    assert_eq!(rows["one"].declaration["external"], "keep");
    assert_eq!(rows["one"].declaration["api_key"], "fake-new");
}
use std::os::unix::fs::PermissionsExt;
#[test]
fn cc04_hermes_transaction_failure_recovers_without_unique_pointer() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = HermesFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::Hermes).unwrap();
        let (name, pre) = files.snapshot().unwrap();
        let next = patch(pre.as_deref(), "one", Some(&json!({"models":[{"id":"m"}]}))).unwrap();
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
        let mut files = HermesFiles::open(home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let state = tx.state(AppId::Hermes).unwrap();
        assert!(state.pointer.is_none());
        assert!(tx.pending(AppId::Hermes).unwrap().is_none());
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
fn cc04_hermes_input_and_external_file_conflicts_do_not_publish() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = HermesFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::Hermes).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next = patch(pre.as_deref(), "one", Some(&json!({"models":[{"id":"m"}]}))).unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || false),
        Err(WriteError::Conflict)
    );
    assert!(read(home.path()).unwrap().is_none());
    let path = home.path().join(".hermes/config.yaml");
    std::fs::write(&path, b"# external\nagent: {}\n").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"# external\nagent: {}\n");
    assert!(tx.state(AppId::Hermes).unwrap().mode.is_none());
}

#[test]
fn cc04_hermes_overlay_is_read_only_and_list_wins() {
    let pre=b"providers:\n  key:\n    name: overlay\n    api_key: fake-overlay\n    models: {m: {}}\ncustom_providers:\n  - name: shared\n    api_key: fake-list\n";
    let state = state(Some(pre)).unwrap();
    assert_eq!(state.members, vec!["overlay", "shared"]);
    assert_eq!(state.read_only_members, vec!["overlay"]);
    for id in ["overlay", "key"] {
        assert!(patch(Some(pre), id, Some(&json!({"api_key":"fake-new"}))).is_err());
        assert!(patch(Some(pre), id, None).is_err());
    }
    let duplicate=b"providers:\n  shared: {api_key: fake-dict}\ncustom_providers:\n  - name: shared\n    api_key: fake-list\n";
    assert_eq!(
        declarations(Some(duplicate)).unwrap()["shared"].declaration["api_key"],
        "fake-list"
    );
    assert!(state_fn(Some(duplicate)).read_only_members.is_empty());
}
fn state_fn(pre: Option<&[u8]>) -> vw_api_types::app_providers::hermes::HermesState {
    state(pre).unwrap()
}
#[test]
fn cc04_hermes_missing_removal_and_invalid_layouts_are_safe() {
    assert!(patch(None, "absent", None).unwrap().is_empty());
    for pre in [
        b"[]".as_slice(),
        b"custom_providers: {}",
        b"custom_providers:\n - name: one\n - name: one\n",
        b"custom_providers: []\ncustom_providers: []",
        b"providers: []",
    ] {
        assert!(patch(Some(pre), "one", Some(&json!({"api_key":"fake"}))).is_err());
    }
    for pre in [
        b"custom_providers: []\n".as_slice(),
        b"custom_providers: null # keep\n",
        b"agent: {}",
        b"custom_providers:\n- name: other\n  api_key: fake\n",
    ] {
        let out = patch(
            Some(pre),
            "one",
            Some(&json!({"apiKey":"fake-new","models":[{"id":"first"},{"id":"second"}]})),
        )
        .unwrap_or_else(|error| panic!("fixture {:?}: {error:?}", String::from_utf8_lossy(pre)));
        assert_eq!(declarations(Some(&out)).unwrap()["one"].declaration["api_key"], "fake-new");
        let v: serde_json::Value = serde_yaml::from_slice(&out).unwrap();
        let one =
            v["custom_providers"].as_array().unwrap().iter().find(|e| e["name"] == "one").unwrap();
        assert_eq!(one["model"], "first");
        assert!(one["models"].is_object());
        assert!(one.get("apiKey").is_none());
        let removed = patch(Some(&out), "one", None).unwrap();
        assert!(!declarations(Some(&removed)).unwrap().contains_key("one"));
    }
    assert!(patch(Some(b"custom_providers: [{name: one}]"), "one", Some(&json!({}))).is_err());
}
#[test]
fn cc04_hermes_catalog_and_symlink_reads_do_not_expose_credentials() {
    use vw_api_types::{agent_apps::AppId, app_providers::catalog::*};
    let home = tempfile::tempdir().unwrap();
    assert!(read(home.path()).unwrap().is_none());
    assert!(!home.path().join(".hermes").exists());
    std::fs::create_dir(home.path().join(".hermes")).unwrap();
    let file = home.path().join(".hermes/config.yaml");
    std::fs::write(&file, b"custom_providers:\n  - name: one\n    api_key: fake-secret\n").unwrap();
    let result = super::catalog::read(home.path(), AppId::Hermes, vec![]);
    assert_eq!(result.rows[0].provider.id, "one");
    assert_eq!(result.rows[0].native_member, ProviderMembership::Present);
    assert_eq!(result.direct, ProviderSelection::NotApplicable);
    assert!(!serde_json::to_string(&result).unwrap().contains("fake-secret"));
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::Hermes, "one").is_err());
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::Hermes, "unused").is_ok());
    std::fs::remove_file(&file).unwrap();
    let outside = home.path().join("outside");
    std::fs::write(&outside, b"agent: {}\n").unwrap();
    std::os::unix::fs::symlink(&outside, &file).unwrap();
    assert!(read(home.path()).is_err());
    assert!(HermesFiles::open(home.path()).unwrap().snapshot().is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"agent: {}\n");
}

#[test]
fn cc04_hermes_home_uses_trimmed_literal_authorized_directory() {
    use std::{ffi::OsStr, path::Path};
    let home = Path::new("/fixture/home");
    assert_eq!(directory_components(home, None).unwrap(), vec![".hermes"]);
    assert_eq!(
        directory_components(home, Some(OsStr::new(" /fixture/home/custom/hermes "))).unwrap(),
        vec!["custom", "hermes"]
    );
    for p in
        ["~/.hermes", "relative", "/outside/hermes", "/fixture/home/../outside", "/fixture/home"]
    {
        assert!(directory_components(home, Some(OsStr::new(p))).is_err());
    }
}

#[test]
fn cc04_hermes_form_order_and_legacy_credentials_survive_normalization() {
    let form =
        normalize_declaration(&json!({"models":[{"id":"z-first"},{"id":"a-second"}]})).unwrap();
    let pre = b"custom_providers:\n  - name: one\n    apiKey: fake-legacy\n    future: keep\n";
    let out = patch(Some(pre), "one", Some(&form)).unwrap();
    let doc: serde_json::Value = serde_yaml::from_slice(&out).unwrap();
    assert_eq!(doc["custom_providers"][0]["model"], "z-first");
    assert_eq!(doc["custom_providers"][0]["api_key"], "fake-legacy");
    assert!(doc["custom_providers"][0].get("apiKey").is_none());
}
