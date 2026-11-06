use super::mcode::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::mcode::*;
fn declaration() -> Value {
    json!({"kind":"custom","api":"openai-completions","options":{"baseURL":"https://example.com/v1","apiKey":"fake-secret"},"models":{"m":{}}})
}
#[test]
fn cc04_mcode_updates_only_selected_node() {
    let pre=b"# keep\ndefaultModel: minimax/native\nminimax_api: {apiKey: fake-account}\ncustom_provider:\n  other:\n    kind: minimax\n    future: true\n";
    let added = patch(Some(pre), "one", Some(&declaration()), McodeActivation::Add).unwrap();
    assert_eq!(state(Some(&added)).unwrap().members, vec!["one"]);
    assert!(
        String::from_utf8_lossy(&added).contains("  other:\n    kind: minimax\n    future: true\n")
    );
    let mut edited = declaration();
    edited["options"]["apiKey"] = json!("fake-edited");
    let edited = patch(Some(&added), "one", Some(&edited), McodeActivation::IfMember).unwrap();
    assert_eq!(
        declarations(Some(&edited)).unwrap()["one"].declaration["options"]["apiKey"],
        "fake-edited"
    );
    assert_eq!(patch(Some(&edited), "one", None, McodeActivation::Add).unwrap(), pre);
}
#[test]
fn cc04_mcode_editor_rejects_extra_yaml_scope() {
    for text in [
        "custom_provider:\n  one: {}\ndefaultModel: changed\n",
        "custom_provider:\n  one: {}\n  other: {}\n",
        "custom_provider:\n  wrong: {}\n",
    ] {
        assert!(parse_editor("one", text).is_err(), "{text}");
    }
    let yaml = serde_yaml::to_string(&json!({"custom_provider":{"one":declaration()}})).unwrap();
    assert_eq!(parse_editor("one", &yaml).unwrap(), declaration());
}
#[test]
fn cc04_mcode_default_models_and_account_nodes_are_protected() {
    for field in ["defaultModel", "defaultLightModel"] {
        let yaml=serde_yaml::to_string(&json!({field:"custom_provider:one/m","custom_provider":{"one":declaration(),"account":{"kind":"minimax"}}})).unwrap();
        assert!(patch(Some(yaml.as_bytes()), "one", None, McodeActivation::Add).is_err());
        for replacement in [
            json!({"models":{"other":{}}}),
            json!({"enabled":false}),
            json!({"models":{"m":{"enabled":false}}}),
        ] {
            let mut next = declaration();
            for (key, value) in replacement.as_object().unwrap() {
                next[key] = value.clone();
            }
            assert!(
                patch(Some(yaml.as_bytes()), "one", Some(&next), McodeActivation::IfMember)
                    .is_err()
            );
        }
        assert!(
            patch(Some(yaml.as_bytes()), "one", Some(&declaration()), McodeActivation::IfMember)
                .is_ok()
        );
        assert!(
            patch(
                Some(yaml.as_bytes()),
                "account",
                Some(&declaration()),
                McodeActivation::IfMember
            )
            .is_err()
        );
        assert!(patch(Some(yaml.as_bytes()), "account", None, McodeActivation::Add).is_err());
    }
}
#[test]
fn cc04_mcode_schema_rejects_invalid_fields() {
    for (key, value) in [
        ("api", json!(null)),
        ("options", json!({"baseURL":"file:///tmp/config","apiKey":"fake"})),
        ("models", json!({"m":null})),
        ("models", json!({"m":{"limit":{"output":"wrong"}}})),
        ("kind", json!("minimax")),
        ("custom_provider", json!({})),
        ("options", json!({"baseURL":"https://example.com","apiKey":"[redacted]"})),
    ] {
        let mut node = declaration();
        node[key] = value;
        assert!(
            validate(&McodeProfile { name: "Fixture".into(), declaration: node }).is_err(),
            "{key}"
        );
    }
    let mut node = declaration();
    node.as_object_mut().unwrap().remove("api");
    validate(&McodeProfile { name: "Fixture".into(), declaration: node }).unwrap();
}
use std::os::unix::fs::PermissionsExt;
#[test]
fn cc04_mcode_transaction_failure_recovers_without_unique_pointer() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = McodeFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::Mcode).unwrap();
        let (name, pre) = files.snapshot().unwrap();
        let next =
            patch(pre.as_deref(), "one", Some(&declaration()), McodeActivation::Add).unwrap();
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
        let mut files = McodeFiles::open(home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let state = tx.state(AppId::Mcode).unwrap();
        assert!(state.pointer.is_none());
        assert!(tx.pending(AppId::Mcode).unwrap().is_none());
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
fn cc04_mcode_input_and_external_file_conflicts_do_not_publish() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = McodeFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::Mcode).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next = patch(pre.as_deref(), "one", Some(&declaration()), McodeActivation::Add).unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || false),
        Err(WriteError::Conflict)
    );
    assert!(read(home.path()).unwrap().is_none());
    let path = home.path().join(".minimax/config.yaml");
    std::fs::write(&path, b"{\"external\":true}").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"{\"external\":true}");
    assert!(tx.state(AppId::Mcode).unwrap().mode.is_none());
}

#[test]
fn cc04_mcode_paths_symlinks_and_native_locks_fail_closed() {
    let home = tempfile::tempdir().unwrap();
    assert!(directory_components(home.path(), Some(std::ffi::OsStr::new("relative"))).is_err());
    assert!(directory_components(home.path(), Some(std::ffi::OsStr::new("/tmp/foreign"))).is_err());
    let directory = home.path().join(".minimax");
    std::fs::create_dir(&directory).unwrap();
    let lock = directory.join("config.yaml.lock");
    std::fs::create_dir(&lock).unwrap();
    assert!(McodeFiles::open(home.path()).is_err());
    assert!(lock.exists());
    std::fs::remove_dir(&lock).unwrap();
    let files = McodeFiles::open(home.path()).unwrap();
    assert!(lock.exists());
    drop(files);
    assert!(!lock.exists());
    std::os::unix::fs::symlink("/tmp/foreign", directory.join("config.yaml")).unwrap();
    assert!(read(home.path()).is_err());
    let files = McodeFiles::open(home.path()).unwrap();
    assert!(files.snapshot().is_err());
    drop(files);
    let foreign = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/tmp", foreign.path().join(".minimax")).unwrap();
    assert!(McodeFiles::open(foreign.path()).is_err());
    assert!(read(foreign.path()).is_err());
}
#[test]
fn cc04_mcode_unsafe_yaml_and_missing_removal_are_safe() {
    for bytes in [
        "custom_provider: {one: {}}\n",
        "custom_provider:\n  one: &anchor {}\n",
        "custom_provider:\n  one: {}\n  one: {}\n",
        "custom_provider: 42\n",
        "[]\n",
        "---\ncustom_provider:\n...\n",
    ] {
        assert!(
            patch(Some(bytes.as_bytes()), "one", Some(&declaration()), McodeActivation::Add)
                .is_err(),
            "{bytes}"
        );
    }
    assert_eq!(patch(None, "missing", None, McodeActivation::Add).unwrap(), b"");
    let empty = b"custom_provider: null # keep\n";
    let added = patch(Some(empty), "one", Some(&declaration()), McodeActivation::Add).unwrap();
    let removed = patch(Some(&added), "one", None, McodeActivation::Add).unwrap();
    assert!(state(Some(&removed)).unwrap().members.is_empty());
    assert!(patch(Some(&removed), "one", Some(&declaration()), McodeActivation::Add).is_ok());
}
#[test]
fn cc04_mcode_native_directory_precedence_is_authorized() {
    use std::ffi::OsStr;
    let home = tempfile::tempdir().unwrap();
    assert_eq!(native_components(home.path(), None, None).unwrap(), vec![".minimax"]);
    let primary = home.path().join("primary");
    let legacy = home.path().join("legacy");
    assert_eq!(
        native_components(home.path(), Some(primary.as_os_str()), Some(legacy.as_os_str()))
            .unwrap(),
        vec!["primary"]
    );
    assert_eq!(
        native_components(home.path(), Some(OsStr::new("  ")), Some(legacy.as_os_str())).unwrap(),
        vec!["legacy"]
    );
    assert!(
        native_components(home.path(), Some(OsStr::new("/tmp/foreign")), Some(legacy.as_os_str()))
            .is_err()
    );
    let escape = home.path().join("../foreign");
    assert!(native_components(home.path(), Some(escape.as_os_str()), None).is_err());
}
