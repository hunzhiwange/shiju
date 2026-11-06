use super::omp::*;
use serde_json::json;
use vw_api_types::app_providers::omp::*;
#[test]
fn cc04_omp_roundtrip_keeps_comments_and_other_nodes() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join(".omp/agent");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.yml"), b"unreadable-default-fixture").unwrap();
    std::fs::write(dir.join("agent.db"), b"fake-login-byte-fixture").unwrap();
    let pre = b"# keep\nfuture: true\nproviders:\n  other:\n    oauth: native\n";
    let added = patch(
        Some(pre),
        "one",
        Some(&json!({"baseUrl":"https://example.com","api":"openai-completions","apiKey":"fake","models":[{"id":"m"}]})),
        OmpActivation::Add,
    )
    .unwrap();
    assert_eq!(state(Some(&added)).unwrap().members, vec!["one", "other"]);
    let edited =
        patch(Some(&added), "one", Some(&json!({"baseUrl":"https://example.com","api":"openai-completions","auth":"none","models":[{"id":"new"}]})), OmpActivation::IfMember)
            .unwrap();
    assert_eq!(declarations(Some(&edited)).unwrap()["one"].declaration["models"][0]["id"], "new");
    let removed = patch(Some(&edited), "one", None, OmpActivation::Add).unwrap();
    assert_eq!(state(Some(&removed)).unwrap().members, vec!["other"]);
    assert!(String::from_utf8_lossy(&removed).contains("# keep\nfuture: true"));
    assert_eq!(std::fs::read(dir.join("config.yml")).unwrap(), b"unreadable-default-fixture");
    assert_eq!(std::fs::read(dir.join("agent.db")).unwrap(), b"fake-login-byte-fixture");
}
#[test]
fn cc04_omp_rename_and_collision_are_rejected() {
    let pre = b"providers:\n  one:\n    apiKey: fake-current\n";
    assert!(
        patch(Some(pre), "one", Some(&json!({"apiKey":"fake-stale"})), OmpActivation::Add).is_err()
    );
    assert!(
        patch(
            Some(pre),
            "../renamed",
            Some(&json!({"baseUrl":"https://example.com"})),
            OmpActivation::IfMember
        )
        .is_err()
    );
    let out = patch(
        Some(pre),
        "new",
        Some(&json!({"baseUrl":"https://example.com"})),
        OmpActivation::IfMember,
    )
    .unwrap();
    assert_eq!(out, pre);
}

use std::os::unix::fs::PermissionsExt;
#[test]
fn cc04_omp_transaction_failure_recovers_without_unique_pointer() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = OmpFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::Omp).unwrap();
        let (name, pre) = files.snapshot().unwrap();
        let next =
            patch(pre.as_deref(), "one", Some(&json!({"baseUrl":"https://example.com", "apiKey":"fake", "api":"openai-completions", "models":[{"id":"m"}]})), OmpActivation::Add)
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
        let mut files = OmpFiles::open(home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let state = tx.state(AppId::Omp).unwrap();
        assert!(state.pointer.is_none());
        assert!(tx.pending(AppId::Omp).unwrap().is_none());
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
fn cc04_omp_input_and_external_file_conflicts_do_not_publish() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = OmpFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::Omp).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next =
        patch(pre.as_deref(), "one", Some(&json!({"baseUrl":"https://example.com", "apiKey":"fake", "api":"openai-completions", "models":[{"id":"m"}]})), OmpActivation::Add)
            .unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || false),
        Err(WriteError::Conflict)
    );
    assert!(read(home.path()).unwrap().is_none());
    let path = home.path().join(".omp/agent/models.yml");
    std::fs::write(&path, b"{\"external\":true}").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"{\"external\":true}");
    assert!(tx.state(AppId::Omp).unwrap().mode.is_none());
}
#[test]
fn cc04_omp_membership_never_changes_default_or_login() {
    use super::live::commit::*;
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut files = OmpFiles::open(home.path()).unwrap();
    let dir = home.path().join(".omp/agent");
    std::fs::write(dir.join("config.yml"), b"modelRoles:\n  default: native/model\n").unwrap();
    std::fs::write(dir.join("agent.db"), b"fake-native-login").unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    for declaration in
        [Some(json!({"baseUrl":"https://fake.example"})), None, Some(json!({"apiKey":"fake"}))]
    {
        let (name, pre) = files.snapshot().unwrap();
        let next = patch(pre.as_deref(), "one", declaration.as_ref(), OmpActivation::Add).unwrap();
        files
            .apply(
                &tx,
                &TenantScope::new("p", "a").unwrap(),
                &tx.state(AppId::Omp).unwrap(),
                name,
                pre.as_deref(),
                next,
                || true,
            )
            .unwrap();
        assert_eq!(
            state(read(home.path()).unwrap().as_deref()).unwrap().members.contains(&"one".into()),
            declaration.is_some()
        );
        assert_eq!(
            std::fs::read(dir.join("config.yml")).unwrap(),
            b"modelRoles:\n  default: native/model\n"
        );
        assert_eq!(std::fs::read(dir.join("agent.db")).unwrap(), b"fake-native-login");
    }
}
#[test]
fn cc04_omp_unsafe_yaml_and_invalid_replacements_are_rejected() {
    for bytes in [
        "providers: {p: {apiKey: key}}\n",
        "providers:\n  p: &anchor\n    apiKey: key\n",
        "providers:\n  p: {apiKey: a}\n  p: {apiKey: b}\n",
        "providers:\n  p:\n    apiKey: a\n    apiKey: b\n",
        "providers: 42\n",
        "[]\n",
        "---\nproviders:\n...\n",
        "providers:\n  p:\n    <<: {apiKey: old}\n",
    ] {
        assert!(
            patch(Some(bytes.as_bytes()), "p", Some(&json!({"apiKey":"fake"})), OmpActivation::Add)
                .is_err(),
            "{bytes}"
        );
    }
    for declaration in [
        json!({}),
        json!({"apiKey":""}),
        json!({"baseUrl":42}),
        json!({"apiKey":"fake","api":"future-api"}),
        json!({"baseUrl":"https://fake.example","apiKey":"fake","api":"openai-completions","models":[{"id":"m","compat":{"thinkingFormat":"deepseek"}}]}),
    ] {
        assert!(patch(None, "p", Some(&declaration), OmpActivation::Add).is_err());
    }
    let before = b"# keep\nproviders:\n  future: {api: future-api, discovery: {type: future}}\n";
    let added =
        patch(Some(before), "p", Some(&json!({"apiKey":"fake"})), OmpActivation::Add).unwrap();
    assert!(added.starts_with(before));
    assert_eq!(patch(Some(&added), "p", None, OmpActivation::Add).unwrap(), before);
    for pre in [None, Some(b"# keep\n".as_slice()), Some(b"providers: null # keep\n".as_slice())] {
        let added = patch(pre, "p", Some(&json!({"apiKey":"fake"})), OmpActivation::Add).unwrap();
        let removed = patch(Some(&added), "p", None, OmpActivation::Add).unwrap();
        assert!(state(Some(&removed)).unwrap().members.is_empty());
        assert!(
            patch(Some(&removed), "p", Some(&json!({"apiKey":"fake"})), OmpActivation::Add).is_ok()
        );
    }
}
#[test]
fn cc04_omp_native_paths_precedence_legacy_and_symlinks_fail_closed() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(
        native_components(home.path(), None, None, None, None).unwrap(),
        vec![".omp", "agent"]
    );
    assert_eq!(
        native_components(
            home.path(),
            Some(".custom"),
            Some("work"),
            None,
            Some(std::ffi::OsStr::new("ignored"))
        )
        .unwrap(),
        vec![".custom", "profiles", "work", "agent"]
    );
    let inherited = home.path().join(".omp/profiles/work/agent");
    assert_eq!(
        native_components(
            home.path(),
            None,
            Some("default"),
            Some("work"),
            Some(inherited.as_os_str())
        )
        .unwrap(),
        vec![".omp", "agent"]
    );
    for name in ["../other", "UPPER", "con", "nul.txt", "trailing."] {
        assert!(native_components(home.path(), None, Some(name), None, None).is_err());
    }
    assert!(native_components(home.path(), Some("../escape"), None, None, None).is_err());
    assert!(directory_components(home.path(), Some(std::ffi::OsStr::new("relative"))).is_err());
    assert!(directory_components(home.path(), Some(std::ffi::OsStr::new("/tmp/foreign"))).is_err());
    let files = OmpFiles::open(home.path()).unwrap();
    let dir = home.path().join(".omp/agent");
    std::fs::write(dir.join("models.json"), b"legacy fixture").unwrap();
    assert!(read(home.path()).is_err());
    assert!(files.snapshot().is_err());
    std::fs::write(dir.join("models.yaml"), b"providers: null\n").unwrap();
    assert_eq!(files.snapshot().unwrap().0, "models.yaml");
    assert_eq!(read(home.path()).unwrap().unwrap(), b"providers: null\n");
    std::fs::write(dir.join("models.yml"), b"# wins\n").unwrap();
    assert_eq!(files.snapshot().unwrap().0, "models.yml");
    assert_eq!(read(home.path()).unwrap().unwrap(), b"# wins\n");
    std::fs::remove_file(dir.join("models.yml")).unwrap();
    std::os::unix::fs::symlink(dir.join("config.yml"), dir.join("models.yml")).unwrap();
    assert!(read(home.path()).is_err());
    assert!(files.snapshot().is_err());
    drop(files);
    let unsafe_home = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/tmp", unsafe_home.path().join(".omp")).unwrap();
    assert!(OmpFiles::open(unsafe_home.path()).is_err());
    assert!(read(unsafe_home.path()).is_err());
}
#[test]
fn cc04_omp_precedence_change_after_intent_preserves_native_files() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut files = OmpFiles::open(home.path()).unwrap();
    let dir = home.path().join(".omp/agent");
    let original = b"# fallback\nproviders:\n  other:\n    apiKey: fake\n";
    std::fs::write(dir.join("models.yaml"), original).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    assert_eq!(name, "models.yaml");
    let next =
        patch(pre.as_deref(), "one", Some(&json!({"apiKey":"fake-next"})), OmpActivation::Add)
            .unwrap();
    let preferred = dir.join("models.yml");
    let external = preferred.clone();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::Intent {
                std::fs::write(&external, b"# external preferred\n").unwrap();
            }
            Ok(())
        }))
    });
    let result = files.apply(
        &tx,
        &TenantScope::new("p", "a").unwrap(),
        &tx.state(AppId::Omp).unwrap(),
        name,
        pre.as_deref(),
        next,
        || true,
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(result, Err(WriteError::Conflict));
    tx.recover(&mut files.guard).unwrap();
    assert_eq!(std::fs::read(dir.join("models.yaml")).unwrap(), original);
    assert_eq!(std::fs::read(preferred).unwrap(), b"# external preferred\n");
    assert!(tx.pending(AppId::Omp).unwrap().is_none());
}
