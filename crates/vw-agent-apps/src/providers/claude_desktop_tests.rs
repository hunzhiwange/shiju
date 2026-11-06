use super::{
    claude_desktop::*,
    live::{commit::*, write_guard::WriteError},
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{agent_apps::AppId, app_providers::claude_desktop::*, id::TenantScope};
fn third(token: &str) -> ClaudeDesktopProfile {
    ClaudeDesktopProfile {
        name: "Fixture".into(),
        config: ClaudeDesktopConfig::ThirdParty {
            base_url: "https://fixture.test".into(),
            auth_token: token.into(),
        },
    }
}
fn official() -> ClaudeDesktopProfile {
    ClaudeDesktopProfile { name: "Official".into(), config: ClaudeDesktopConfig::Official {} }
}
struct Fixture {
    home: tempfile::TempDir,
    root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let f = Self { home: tempfile::tempdir().unwrap(), root: tempfile::tempdir().unwrap() };
        fs::set_permissions(f.root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        drop(ClaudeDesktopFiles::open(f.home.path()).unwrap());
        for (index, value) in [json!({"deploymentMode":"1p","mcpServers":{"fixture":{}}}), json!({"deploymentMode":"1p","other":true}), json!({"userAutoMode":true,"coworkEgressAllowedHosts":["corp.test"],"disableDeploymentModeChooser":false,"inferenceModels":["stale"]}), json!({"entries":[{"id":"mine","name":"Mine"},{"id":PROFILE_ID,"note":"keep"}],"appliedId":"mine","extra":true})].into_iter().enumerate() { fs::write(f.file(index), value.to_string()).unwrap(); }
        f
    }
    fn file(&self, index: usize) -> PathBuf {
        self.home.path().join("Library/Application Support").join(match index {
            0 => "Claude/claude_desktop_config.json".into(),
            1 => "Claude-3p/claude_desktop_config.json".into(),
            2 => format!("Claude-3p/configLibrary/{PROFILE_ID}.json"),
            _ => "Claude-3p/configLibrary/_meta.json".into(),
        })
    }
    fn read(&self, index: usize) -> Value {
        serde_json::from_slice(&fs::read(self.file(index)).unwrap()).unwrap()
    }
    fn tx(&self) -> ProviderTransactions {
        ProviderTransactions::new(self.root.path(), ExecutionHostId::new("host").unwrap()).unwrap()
    }
    fn apply(
        &self,
        tx: &ProviderTransactions,
        files: &mut ClaudeDesktopFiles,
        id: &str,
        profile: &ClaudeDesktopProfile,
    ) -> Result<(), WriteError> {
        files.apply(
            tx,
            &TenantScope::new("p", "a").unwrap(),
            &tx.state(AppId::ClaudeDesktop).unwrap(),
            id,
            profile,
            || true,
        )
    }
}
#[test]
fn cc04_claude_desktop_direct_profile_keeps_other_fields() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    f.apply(&tx, &mut files, "one", &third("fake-one")).unwrap();
    assert_eq!(f.read(2)["userAutoMode"], true);
    assert_eq!(f.read(2)["coworkEgressAllowedHosts"], json!(["corp.test"]));
    assert_eq!(f.read(2)["disableDeploymentModeChooser"], false);
    assert!(f.read(2).get("inferenceModels").is_none());
    assert_eq!(f.read(3)["entries"][1]["note"], "keep");
    assert_eq!(f.read(3)["appliedId"], PROFILE_ID);
    assert_eq!(f.read(0)["deploymentMode"], "3p");
    assert_eq!(f.read(1)["deploymentMode"], "3p");
    assert_eq!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.unwrap().id, "one");
    f.apply(&tx, &mut files, "two", &third("fake-two")).unwrap();
    assert_eq!(f.read(2)["inferenceGatewayApiKey"], "fake-two");
    f.apply(&tx, &mut files, "official", &official()).unwrap();
    assert_eq!(f.read(0)["deploymentMode"], "1p");
    assert_eq!(f.read(1)["deploymentMode"], "1p");
    assert!(f.read(2).get("inferenceGatewayApiKey").is_none());
    assert_eq!(f.read(2)["userAutoMode"], true);
    assert_eq!(f.read(3)["entries"], json!([{"id":"mine","name":"Mine"}]));
    assert_eq!(f.read(3)["appliedId"], "mine");
    assert_eq!(f.read(0)["mcpServers"], json!({"fixture":{}}));
    assert_eq!(fs::metadata(f.file(2)).unwrap().permissions().mode() & 0o777, 0o600);
}
#[test]
fn cc04_claude_desktop_generic_import_is_unsupported() {
    assert_eq!(generic_import(), Err("claude_desktop.import_unsupported"));
}
#[test]
fn cc04_claude_desktop_failures_recover_all_directories_and_pointer() {
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let f = Fixture::new();
        let tx = f.tx();
        let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        assert!(f.apply(&tx, &mut files, "one", &third("fake-key")).is_err());
        HOOK.with_borrow_mut(|h| *h = None);
        if let Some(p) = tx.pending(AppId::ClaudeDesktop).unwrap() {
            assert_eq!(p.version, 3);
            assert_eq!(p.files.len(), 4);
            assert_eq!(p.files.iter().map(|f| f.directory).collect::<Vec<_>>(), vec![0, 1, 2, 2]);
        }
        drop(files);
        drop(tx);
        let tx = f.tx();
        let recovered = tx.recover_on_startup();
        assert_eq!(recovered.len(), 1);
        assert!(recovered[0].1.is_ok());
        if phase == Phase::Intent {
            assert!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.is_none());
            assert_eq!(f.read(0)["deploymentMode"], "1p");
        } else {
            assert_eq!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.unwrap().id, "one");
            assert_eq!(f.read(0)["deploymentMode"], "3p");
            assert_eq!(f.read(1)["deploymentMode"], "3p");
            assert_eq!(f.read(2)["inferenceGatewayApiKey"], "fake-key");
            assert_eq!(f.read(3)["appliedId"], PROFILE_ID);
        }
        assert!(tx.pending(AppId::ClaudeDesktop).unwrap().is_none());
    }
}
#[test]
fn cc04_claude_desktop_partial_publication_preserves_external_edit() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(
            |_, _, at| if at == Phase::FilePublished { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    assert!(f.apply(&tx, &mut files, "one", &third("fake-key")).is_err());
    HOOK.with_borrow_mut(|h| *h = None);
    fs::write(f.file(2), b"{\"external\":true}").unwrap();
    drop(files);
    let recovered = tx.recover_on_startup();
    assert!(matches!(
        &recovered[0].1,
        Ok(super::live::recovery::RecoveryOutcome::RolledForwardExcept { .. })
    ));
    assert_eq!(f.read(2), json!({"external":true}));
    assert_eq!(f.read(1)["deploymentMode"], "3p");
    assert_eq!(f.read(3)["appliedId"], PROFILE_ID);
}
#[test]
fn cc04_claude_desktop_invalid_json_and_symlinks_are_rejected() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    let before = fs::read(f.file(0)).unwrap();
    fs::write(f.file(3), b"[]").unwrap();
    assert!(f.apply(&tx, &mut files, "one", &third("fake-key")).is_err());
    assert_eq!(fs::read(f.file(0)).unwrap(), before);
    assert!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.is_none());
    drop(files);
    fs::remove_file(f.file(3)).unwrap();
    std::os::unix::fs::symlink(f.file(0), f.file(3)).unwrap();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    assert!(f.apply(&tx, &mut files, "one", &third("fake-key")).is_err());
    drop(files);
    let library = f.file(2).parent().unwrap().to_path_buf();
    fs::rename(&library, library.with_file_name("moved")).unwrap();
    std::os::unix::fs::symlink(library.with_file_name("moved"), &library).unwrap();
    assert!(ClaudeDesktopFiles::open(f.home.path()).is_err());
}
#[test]
fn cc04_claude_desktop_recovery_rejects_replaced_directory_before_any_publication() {
    for symlink in [false, true] {
        let f = Fixture::new();
        let tx = f.tx();
        let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                |_, _, at| if at == Phase::Marked { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        assert!(f.apply(&tx, &mut files, "one", &third("fake-key")).is_err());
        HOOK.with_borrow_mut(|h| *h = None);
        drop(files);
        let dir = f.file(2).parent().unwrap().to_path_buf();
        let moved = dir.with_file_name("moved");
        fs::rename(&dir, &moved).unwrap();
        if symlink {
            std::os::unix::fs::symlink(&moved, &dir).unwrap();
        } else {
            fs::create_dir(&dir).unwrap();
        }
        let results = tx.recover_on_startup();
        assert!(results[0].1.is_err());
        assert!(tx.pending(AppId::ClaudeDesktop).unwrap().is_some());
        assert_eq!(f.read(0)["deploymentMode"], "1p");
        if symlink {
            fs::remove_file(&dir).unwrap();
        } else {
            fs::remove_dir(&dir).unwrap();
        }
        fs::rename(&moved, &dir).unwrap();
        assert!(tx.recover_on_startup()[0].1.is_ok());
        assert_eq!(f.read(2)["inferenceGatewayApiKey"], "fake-key");
    }
}
#[test]
fn cc04_claude_desktop_input_changed_after_intent_is_discarded() {
    use std::{cell::Cell, rc::Rc};
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    let before: Vec<_> = (0..4).map(|i| fs::read(f.file(i)).unwrap()).collect();
    let valid = Rc::new(Cell::new(true));
    let hook = valid.clone();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| {
            if at == Phase::Intent {
                hook.set(false);
            }
            Ok(())
        }))
    });
    let result = files.apply(
        &tx,
        &TenantScope::new("p", "a").unwrap(),
        &tx.state(AppId::ClaudeDesktop).unwrap(),
        "one",
        &third("fake-key"),
        || valid.get(),
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(result, Err(WriteError::Conflict));
    drop(files);
    assert!(tx.recover_on_startup()[0].1.is_ok());
    for (i, bytes) in before.iter().enumerate() {
        assert_eq!(&fs::read(f.file(i)).unwrap(), bytes);
    }
    assert!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.is_none());
}
#[test]
fn cc04_claude_desktop_mid_publication_conflict_keeps_external_file_and_forwards_remaining() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    let external = f.file(1);
    let mut published = 0;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| {
            if at == Phase::FilePublished {
                published += 1;
            }
            if at == Phase::BeforePublish && published == 1 {
                fs::write(&external, b"{\"external\":true}").unwrap();
            }
            Ok(())
        }))
    });
    assert_eq!(f.apply(&tx, &mut files, "one", &third("fake-key")), Err(WriteError::Conflict));
    HOOK.with_borrow_mut(|h| *h = None);
    drop(files);
    assert!(tx.pending(AppId::ClaudeDesktop).unwrap().unwrap().published);
    assert!(matches!(
        &tx.recover_on_startup()[0].1,
        Ok(super::live::recovery::RecoveryOutcome::RolledForwardExcept { .. })
    ));
    assert_eq!(f.read(1), json!({"external":true}));
    assert_eq!(f.read(2)["inferenceGatewayApiKey"], "fake-key");
    assert_eq!(f.read(3)["appliedId"], PROFILE_ID);
    assert_eq!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.unwrap().id, "one");
}
#[test]
fn cc04_claude_desktop_pending_cannot_select_unregistered_directory() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(
            |_, _, at| if at == Phase::Marked { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    assert!(f.apply(&tx, &mut files, "one", &third("fake-key")).is_err());
    HOOK.with_borrow_mut(|h| *h = None);
    drop(files);
    let path = f.root.path().join("provider-live/aG9zdA/claude-desktop.pending.json");
    let original = fs::read(&path).unwrap();
    let mut pending: Value = serde_json::from_slice(&original).unwrap();
    pending["files"][3]["directory"] = json!(99);
    fs::write(&path, pending.to_string()).unwrap();
    assert!(tx.recover_on_startup()[0].1.is_err());
    assert_eq!(f.read(0)["deploymentMode"], "1p");
    assert!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.is_none());
    fs::write(&path, original).unwrap();
    assert!(tx.recover_on_startup()[0].1.is_ok());
}
#[test]
fn cc04_claude_desktop_official_cleanup_keeps_enterprise_user_options_and_invalid_credentials_fail()
{
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    fs::write(f.file(1),json!({"enterpriseConfig":{"inferenceProvider":"gateway","inferenceGatewayApiKey":"fake-stale","userOption":true}}).to_string()).unwrap();
    f.apply(&tx, &mut files, "official", &official()).unwrap();
    assert_eq!(f.read(1)["enterpriseConfig"], json!({"userOption":true}));
    fs::write(f.file(1), json!({"enterpriseConfig":{"inferenceProvider":"gateway"}}).to_string())
        .unwrap();
    f.apply(&tx, &mut files, "official", &official()).unwrap();
    assert!(f.read(1).get("enterpriseConfig").is_none());
    assert!(settings(&third("\nsecret")).is_err());
    let mut bad = third("fake-key");
    if let ClaudeDesktopConfig::ThirdParty { base_url, .. } = &mut bad.config {
        *base_url = "file:///tmp/outside".into();
    }
    assert!(settings(&bad).is_err());
}

#[test]
fn cc04_claude_desktop_direct_credentials_follow_existing_endpoint_policy() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = ClaudeDesktopFiles::open(f.home.path()).unwrap();
    let before: Vec<_> = (0..4).map(|i| fs::read(f.file(i)).unwrap()).collect();
    for url in
        ["http://remote.fixture.test", "https://fixture.te\nst", "https://user:secret@fixture.test"]
    {
        let profile = ClaudeDesktopProfile {
            name: "Fixture".into(),
            config: ClaudeDesktopConfig::ThirdParty {
                base_url: url.into(),
                auth_token: "fake-key".into(),
            },
        };
        assert!(
            f.apply(&tx, &mut files, "invalid", &profile).is_err(),
            "invalid endpoint was applied"
        );
        for (i, bytes) in before.iter().enumerate() {
            assert_eq!(&fs::read(f.file(i)).unwrap(), bytes);
        }
        assert!(tx.state(AppId::ClaudeDesktop).unwrap().pointer.is_none());
    }
    let profile = ClaudeDesktopProfile {
        name: "Local".into(),
        config: ClaudeDesktopConfig::ThirdParty {
            base_url: " http://127.0.0.1:1234/v1 ".into(),
            auth_token: " fake-key ".into(),
        },
    };
    f.apply(&tx, &mut files, "local", &profile).unwrap();
    assert_eq!(f.read(2)["inferenceGatewayBaseUrl"], "http://127.0.0.1:1234/v1");
    assert_eq!(f.read(2)["inferenceGatewayApiKey"], "fake-key");
}
