use super::pi::*;
use serde_json::json;
use vw_api_types::app_providers::pi::*;
#[test]
fn cc04_pi_membership_preserves_defaults_and_login() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join(".pi/agent");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), b"unreadable-default-fixture").unwrap();
    std::fs::write(dir.join("auth.json"), b"fake-login-byte-fixture").unwrap();
    let pre = b"{\n// keep\n\"future\":true,\"providers\":{\"other\":{\"oauth\":\"native\"}}}";
    let added = patch(
        Some(pre),
        "one",
        Some(&json!({"apiKey":"fake","models":[{"id":"m"}]})),
        PiActivation::Add,
    )
    .unwrap();
    assert_eq!(state(Some(&added)).unwrap().members, vec!["one", "other"]);
    let edited =
        patch(Some(&added), "one", Some(&json!({"models":[{"id":"new"}]})), PiActivation::IfMember)
            .unwrap();
    assert_eq!(declarations(Some(&edited)).unwrap()["one"].declaration["models"][0]["id"], "new");
    let removed = patch(Some(&edited), "one", None, PiActivation::Add).unwrap();
    assert_eq!(state(Some(&removed)).unwrap().members, vec!["other"]);
    assert!(String::from_utf8_lossy(&removed).contains("// keep\n\"future\":true"));
    assert_eq!(std::fs::read(dir.join("settings.json")).unwrap(), b"unreadable-default-fixture");
    assert_eq!(std::fs::read(dir.join("auth.json")).unwrap(), b"fake-login-byte-fixture");
}
#[test]
fn cc04_pi_rename_and_collision_are_rejected() {
    let pre = b"{\"providers\":{\"one\":{\"apiKey\":\"fake-current\"}}}";
    assert!(
        patch(Some(pre), "one", Some(&json!({"apiKey":"fake-stale"})), PiActivation::Add).is_err()
    );
    assert!(patch(Some(pre), "../renamed", Some(&json!({})), PiActivation::IfMember).is_err());
    let out = patch(Some(pre), "new", Some(&json!({})), PiActivation::IfMember).unwrap();
    assert_eq!(out, pre);
}

#[test]
fn cc04_pi_invalid_documents_and_paths_fail_closed() {
    for bytes in [
        b"{\"providers\":null}".as_slice(),
        b"{\"providers\":{\"one\":3}}",
        b"{\"providers\":{},\"providers\":{}}",
        b"{providers:{}}",
    ] {
        assert!(declarations(Some(bytes)).is_err());
    }
    let home = tempfile::tempdir().unwrap();
    assert!(directory_components(home.path(), Some(std::ffi::OsStr::new("relative"))).is_err());
    assert!(directory_components(home.path(), Some(std::ffi::OsStr::new("/tmp/foreign"))).is_err());
    assert!(
        directory_components(home.path(), Some(home.path().join("../escape").as_os_str())).is_err()
    );
    assert_eq!(directory_components(home.path(), None).unwrap(), vec![".pi", "agent"]);
    assert_eq!(
        directory_components(home.path(), Some(home.path().join("custom").as_os_str())).unwrap(),
        vec!["custom"]
    );
    assert_eq!(patch(None, "missing", None, PiActivation::Add).unwrap(), b"");
    std::os::unix::fs::symlink("/tmp", home.path().join(".pi")).unwrap();
    assert!(PiFiles::open(home.path()).is_err());
    assert!(read(home.path()).is_err());
}
use std::os::unix::fs::PermissionsExt;
#[test]
fn cc04_pi_transaction_failure_recovers_without_unique_pointer() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut files = PiFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::Pi).unwrap();
        let (name, pre) = files.snapshot().unwrap();
        let next =
            patch(pre.as_deref(), "one", Some(&json!({"models":[{"id":"m"}]})), PiActivation::Add)
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
        let mut files = PiFiles::open(home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let state = tx.state(AppId::Pi).unwrap();
        assert!(state.pointer.is_none());
        assert!(tx.pending(AppId::Pi).unwrap().is_none());
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
fn cc04_pi_input_and_external_file_conflicts_do_not_publish() {
    use super::live::{commit::*, write_guard::WriteError};
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut files = PiFiles::open(home.path()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::Pi).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next =
        patch(pre.as_deref(), "one", Some(&json!({"models":[{"id":"m"}]})), PiActivation::Add)
            .unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || false),
        Err(WriteError::Conflict)
    );
    assert!(read(home.path()).unwrap().is_none());
    let path = home.path().join(".pi/agent/models.json");
    std::fs::write(&path, b"{\"external\":true}").unwrap();
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next, || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"{\"external\":true}");
    assert!(tx.state(AppId::Pi).unwrap().mode.is_none());
}
