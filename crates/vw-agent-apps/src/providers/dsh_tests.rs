use super::dsh::*;
use serde_json::json;
use vw_api_types::app_providers::dsh::*;
fn profile() -> DshProfile {
    DshProfile {
        name: "Fixture".into(),
        declaration: json!({"api":"openai-completions","options":{"baseURL":"https://example.com/v1","apiKey":"fake-secret"},"models":{"model":{}}}),
    }
}
#[test]
fn cc04_dsh_provider_edit_keeps_native_account() {
    let pre=Snapshot::from([("credentials".into(),Some(b"version: 1\nrefs: {OTHER: fake-other}\nrecords:\n  deepseek-account-platform/default:\n    kind: grant\n    payload: {version: 1, token: fake-native, issuer: 'https://platform.deepseek.com'}\n".to_vec())),("web".into(),Some(b"# native\n- id: other\n  config: {value: !!js process.env.VALUE}\n".to_vec())),("desktop".into(),None),("home".into(),None)]);
    let next = plan(&pre, "one", &profile(), DshActivation::Add).unwrap();
    let before: serde_yaml::Value =
        serde_yaml::from_slice(pre["credentials"].as_ref().unwrap()).unwrap();
    let after: serde_yaml::Value =
        serde_yaml::from_slice(next["credentials"].as_ref().unwrap()).unwrap();
    assert_eq!(after["records"], before["records"]);
    assert_eq!(after["refs"]["OTHER"], before["refs"]["OTHER"]);
    assert_eq!(after["refs"]["SHIJU_DSH_ONE"], serde_yaml::Value::from("fake-secret"));
    assert!(
        String::from_utf8_lossy(next["web"].as_ref().unwrap())
            .contains("value: !!js process.env.VALUE")
    );
    assert!(state(&next).unwrap().members.contains(&"one".to_string()));
}
#[test]
fn cc04_dsh_invalid_binding_is_explicit() {
    assert!(!valid_binding(Some(&json!({"meta":{"authBinding":{"accountId":"missing"}}}))));
    assert!(valid_binding(None));
    assert!(valid_binding(Some(&json!({"name":"fixture"}))));
}
#[test]
fn cc04_dsh_profiles_refuse_divergence_shadowing_and_missing_credentials() {
    let pre = Snapshot::from([
        ("web".into(), None),
        ("desktop".into(), None),
        ("credentials".into(), None),
        ("home".into(), None),
    ]);
    let mut next = plan(&pre, "one", &profile(), DshActivation::Add).unwrap();
    assert!(plan(&next, "one", &profile(), DshActivation::Add).is_err());
    next.insert(
        "desktop".into(),
        Some(
            String::from_utf8(next["desktop"].clone().unwrap())
                .unwrap()
                .replace("example.com", "other.example")
                .into_bytes(),
        ),
    );
    assert!(plan(&next, "one", &profile(), DshActivation::IfMember).is_err());
    let mut shadowed = pre.clone();
    shadowed.insert("home".into(), Some(b"- id: llm-pi-ai\n  config: {providers: {}}\n".to_vec()));
    assert!(plan(&shadowed, "one", &profile(), DshActivation::Add).is_err());
    let mut p = profile();
    p.declaration["options"]["apiKey"] = json!("");
    p.declaration["apiKeyEnv"] = json!("MISSING");
    assert!(!credential_binding_valid(&pre, &p));
    assert!(plan(&pre, "one", &p, DshActivation::Add).is_err());
    assert_eq!(plan(&pre, "one", &profile(), DshActivation::IfMember).unwrap(), pre);
}
#[test]
fn cc04_dsh_incremental_edit_keeps_siblings_and_native_plugin_bytes() {
    let web=b"# keep\n- id: llm-pi-ai\n  disabled: false\n  config:\n    future: kept\n    providers:\n      other: # native sibling\n        apiKeyEnv: OTHER\n        models: []\n- id: native\n  config: {value: !!js process.env.VALUE}\n";
    let pre = Snapshot::from([
        ("web".into(), Some(web.to_vec())),
        ("desktop".into(), None),
        ("credentials".into(), None),
        ("home".into(), None),
    ]);
    let added = plan(&pre, "one", &profile(), DshActivation::Add).unwrap();
    let mut p = profile();
    p.declaration["options"]["apiKey"] = json!("fake-edited");
    let edited = plan(&added, "one", &p, DshActivation::IfMember).unwrap();
    let text = String::from_utf8(edited["web"].clone().unwrap()).unwrap();
    assert!(text.contains("other: # native sibling\n        apiKeyEnv: OTHER\n        models: []"));
    assert!(text.ends_with("- id: native\n  config: {value: !!js process.env.VALUE}\n"));
    assert!(text.contains("future: kept"));
    let creds: serde_yaml::Value =
        serde_yaml::from_slice(edited["credentials"].as_ref().unwrap()).unwrap();
    assert_eq!(creds["refs"]["SHIJU_DSH_ONE"], "fake-edited");
}
#[test]
fn cc04_dsh_all_profiles_recover_one_intent_and_use_private_files() {
    use super::live::{commit::*, write_guard::WriteError};
    use std::os::unix::fs::PermissionsExt;
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir_all(home.path().join(".dsh/profiles/headless")).unwrap();
        std::fs::write(home.path().join(".dsh/profiles/headless/cordis.patch.yml"), "[]\n")
            .unwrap();
        std::fs::create_dir_all(home.path().join(".dsh/profiles/sdk")).unwrap();
        std::fs::write(home.path().join(".dsh/profiles/sdk/cordis.patch.yml"), "# sdk\n[]\n")
            .unwrap();
        let mut files = DshFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let expected = tx.state(AppId::Dsh).unwrap();
        let pre = files.snapshot().unwrap();
        assert!(!pre.contains_key("acp"));
        let next = plan(&pre, "one", &profile(), DshActivation::Add).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let result =
            files.apply(&tx, &TenantScope::new("p", "a").unwrap(), &expected, &pre, &next, || true);
        HOOK.with_borrow_mut(|h| *h = None);
        assert!(result.is_err());
        drop(files);
        drop(tx);
        let mut files = DshFiles::open(home.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        let recovered = files.snapshot().unwrap();
        for name in ["web", "desktop", "headless"] {
            let text = String::from_utf8(recovered[name].clone().unwrap_or_default()).unwrap();
            assert_eq!(text.contains("SHIJU_DSH_ONE"), phase != Phase::Intent);
            if phase != Phase::Intent {
                assert_eq!(
                    std::fs::metadata(
                        home.path().join(format!(".dsh/profiles/{name}/cordis.patch.yml"))
                    )
                    .unwrap()
                    .permissions()
                    .mode()
                        & 0o777,
                    0o600
                );
            }
        }
        assert_eq!(
            std::fs::read(home.path().join(".dsh/profiles/sdk/cordis.patch.yml")).unwrap(),
            b"# sdk\n[]\n"
        );
        assert!(tx.state(AppId::Dsh).unwrap().pointer.is_none());
        assert!(tx.pending(AppId::Dsh).unwrap().is_none());
    }
}
#[test]
fn cc04_dsh_symlinks_native_locks_and_external_edits_are_rejected() {
    use super::live::commit::*;
    use std::os::unix::fs::PermissionsExt;
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut files = DshFiles::open(home.path()).unwrap();
    let pre = files.snapshot().unwrap();
    let next = plan(&pre, "one", &profile(), DshActivation::Add).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let expected = tx.state(AppId::Dsh).unwrap();
    assert!(
        files
            .apply(&tx, &TenantScope::new("p", "a").unwrap(), &expected, &pre, &next, || false)
            .is_err()
    );
    std::fs::write(home.path().join(".dsh/profiles/web/cordis.patch.yml"), "# external\n[]\n")
        .unwrap();
    assert!(
        files
            .apply(&tx, &TenantScope::new("p", "a").unwrap(), &expected, &pre, &next, || true)
            .is_err()
    );
    drop(files);
    let lock = home.path().join(".dsh/.credentials.yaml.lock");
    std::fs::write(&lock, "native-owner").unwrap();
    assert!(DshFiles::open(home.path()).is_err());
    assert!(lock.exists());
    std::fs::remove_file(lock).unwrap();
    std::fs::remove_file(home.path().join(".dsh/profiles/web/cordis.patch.yml")).unwrap();
    std::os::unix::fs::symlink(
        "/tmp/foreign",
        home.path().join(".dsh/profiles/web/cordis.patch.yml"),
    )
    .unwrap();
    assert!(read(home.path()).is_err());
    let another = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/tmp", another.path().join(".dsh")).unwrap();
    assert!(DshFiles::open(another.path()).is_err());
}
