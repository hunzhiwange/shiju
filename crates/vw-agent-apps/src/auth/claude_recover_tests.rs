use super::{
    claude_keychain::ClaudeKeychain, claude_login_files::Directory, claude_snapshot_tests::Keychain,
};
use crate::providers::{
    claude::ClaudeFile,
    claude_login::{LOGIN_CHECKPOINT, LoginHost, LoginPhase},
    live::{
        commit::{AppState, ExecutionHostId, HOOK, Phase, ProviderTransactions},
        write_guard::WriteError,
    },
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::DirBuilderExt, path::Path, process::Command};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

const CHILD_PHASE: &str = "SHIJU_CLAUDE_RECOVERY_TEST_PHASE";
const CHILD_HOME: &str = "SHIJU_CLAUDE_RECOVERY_TEST_HOME";
const SUCCESS_TEST: &str =
    "auth::claude_recover_tests::auth6_claude_recover_commits_expected_scoped_result";
const JOURNAL: &str = ".shiju-login.json";
const ORIGINAL_PROFILE: &[u8] =
    br#"{"oauthAccount":{"accountUuid":"original","organizationUuid":"org"},"mcpServers":{"keep":true}}"#;
const ORIGINAL_SETTINGS: &[u8] = br#"{"hooks":true}"#;

fn credentials(token: &str) -> Value {
    json!({"claudeAiOauth":{"refreshToken":token},"organizationUuid":"org"})
}

fn keychain(home: &Path) -> Keychain {
    Keychain {
        live: home.join(".claude"),
        target: home.join("target"),
        parked: home.join("parked"),
    }
}

fn transactions(home: &Path) -> ProviderTransactions {
    ProviderTransactions::new(&home.join("private"), ExecutionHostId::new("recovery-host").unwrap())
        .unwrap()
}

fn tenant() -> TenantScope {
    TenantScope::new("recovery-platform", "recovery-company").unwrap()
}

fn fixture(target_owner: &str) -> (tempfile::TempDir, Keychain) {
    let home = tempfile::tempdir().unwrap();
    let keychain = keychain(home.path());
    for dir in [&keychain.live, &keychain.target, &keychain.parked, &home.path().join("private")] {
        fs::DirBuilder::new().mode(0o700).create(dir).unwrap();
    }
    assert!(keychain.read(None).unwrap().is_none());
    assert!(keychain.read(Some(&keychain.target)).unwrap().is_none());
    keychain.write(Some(&keychain.target), Some(&credentials("target-token"))).unwrap();
    Directory::open(&keychain.live)
        .unwrap()
        .write(".credentials.json", Some(credentials("original-token").to_string().as_bytes()))
        .unwrap();
    fs::write(home.path().join(".claude.json"), ORIGINAL_PROFILE).unwrap();
    fs::write(keychain.live.join("settings.json"), ORIGINAL_SETTINGS).unwrap();
    fs::write(
        keychain.target.join(".claude.json"),
        json!({"oauthAccount":{"accountUuid":target_owner,"organizationUuid":"org"}}).to_string(),
    )
    .unwrap();
    (home, keychain)
}

fn crash_at(home: &Path, phase: &str) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", SUCCESS_TEST, "--nocapture"])
        .env(CHILD_PHASE, phase)
        .env(CHILD_HOME, home)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(86),
        "child missed {phase}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn interrupt_transaction(home: &Path, phase: &str) {
    // The parent owns deletion of the isolated entries, including after child exit.
    let keychain = keychain(home);
    let tx = transactions(home);
    let host = LoginHost::open(home).unwrap();
    let login_phase = match phase {
        "journal" => Some(LoginPhase::Journal),
        "keychain" | "rollback-keychain" => Some(LoginPhase::Keychain),
        "fallback" => Some(LoginPhase::Fallback),
        "profile" | "replace-profile" => Some(LoginPhase::Profile),
        "replace-cleared" => Some(LoginPhase::Cleared),
        _ => None,
    };
    LOGIN_CHECKPOINT.set(login_phase);
    if phase == "rollback-keychain" {
        host.recover(&tx, &keychain).unwrap();
        panic!("rollback checkpoint not reached");
    }
    if phase.starts_with("replace-") {
        host.replace_live(&tx, &tenant(), &keychain.target, &keychain).unwrap();
        panic!("replacement checkpoint not reached");
    }
    let op = host
        .begin(&tx, &tenant(), &keychain.target, &keychain, |previous| {
            previous.archive(&keychain.parked, &keychain)
        })
        .unwrap();
    assert!(op.is_some());
    let commit_phase = match phase {
        "intent" => Some(Phase::Intent),
        "marked" | "wrong-operation" | "wrong-tenant" => Some(Phase::Marked),
        "before-publish" => Some(Phase::BeforePublish),
        "file-published" => Some(Phase::FilePublished),
        "published" => Some(Phase::Published),
        "settled" => Some(Phase::Settled),
        _ => None,
    };
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(move |_, _, phase| {
            if Some(phase) == commit_phase {
                std::process::exit(86);
            }
            Ok(())
        }));
    });
    let scope = if phase == "wrong-tenant" {
        TenantScope::new("recovery-platform", "other-company").unwrap()
    } else {
        tenant()
    };
    let operation = if phase == "wrong-operation" { Some("stale-operation".into()) } else { op };
    ClaudeFile::open(home, false)
        .unwrap()
        .apply_with_login(
            &tx,
            &scope,
            &AppState::default(),
            None,
            &json!({"env":{"ANTHROPIC_MODEL":"fixture-model"}}),
            Some("bound".into()),
            || true,
            operation,
        )
        .unwrap();
    if phase == "provider-cleared" {
        std::process::exit(86);
    }
    panic!("publication checkpoint not reached");
}

#[test]
fn auth6_claude_recover_commits_expected_scoped_result() {
    if let Some(phase) = std::env::var_os(CHILD_PHASE) {
        interrupt_transaction(
            Path::new(&std::env::var_os(CHILD_HOME).unwrap()),
            phase.to_str().unwrap(),
        );
        return;
    }
    for (phase, published) in [
        ("journal", false),
        ("keychain", false),
        ("fallback", false),
        ("profile", false),
        ("rollback-keychain", false),
        ("intent", false),
        ("marked", true),
        ("before-publish", true),
        ("file-published", true),
        ("published", true),
        ("settled", true),
        ("provider-cleared", true),
        ("replace-profile", false),
        ("replace-cleared", false),
    ] {
        let replacement = phase.starts_with("replace-");
        let (home, keychain) = fixture(if replacement { "original" } else { "target" });
        if phase == "rollback-keychain" {
            crash_at(home.path(), "profile");
        }
        crash_at(home.path(), phase);
        let tx = transactions(home.path());
        let host = LoginHost::open(home.path()).unwrap();
        if tx.pending(AppId::Claude).unwrap().is_some() {
            assert_eq!(host.recover(&tx, &keychain), Err(WriteError::Recovered), "{phase}");
            let outcomes = tx.recover_on_startup();
            assert_eq!(outcomes.len(), 1, "{phase}");
            assert!(outcomes[0].1.is_ok(), "{phase}");
        }
        for _ in 0..2 {
            assert_eq!(host.recover(&tx, &keychain), Ok(()), "{phase}");
            assert!(!host.has_journal().unwrap(), "{phase}");
            assert!(tx.pending(AppId::Claude).unwrap().is_none(), "{phase}");
            let state = tx.state(AppId::Claude).unwrap();
            let profile = fs::read(home.path().join(".claude.json")).unwrap();
            let fallback =
                Directory::open(&keychain.live).unwrap().read(".credentials.json").unwrap();
            let settings = fs::read(keychain.live.join("settings.json")).unwrap();
            if published {
                let pointer = state.pointer.unwrap();
                assert_eq!(pointer.id, "bound", "{phase}");
                assert_eq!(pointer.tenant, tenant(), "{phase}");
                let settings: Value = serde_json::from_slice(&settings).unwrap();
                assert_eq!(settings["env"]["ANTHROPIC_MODEL"], "fixture-model", "{phase}");
                assert_eq!(settings["hooks"], true, "{phase}");
            } else {
                assert!(state == AppState::default(), "{phase}");
                assert_eq!(settings, ORIGINAL_SETTINGS, "{phase}");
            }
            if published || phase == "replace-cleared" {
                assert!(
                    keychain.read(None).unwrap() == Some(credentials("target-token")),
                    "{phase}"
                );
                assert!(fallback.is_none(), "{phase}");
                let profile: Value = serde_json::from_slice(&profile).unwrap();
                assert_eq!(
                    profile["oauthAccount"]["accountUuid"],
                    if replacement { "original" } else { "target" },
                    "{phase}"
                );
                assert_eq!(profile["mcpServers"]["keep"], true, "{phase}");
            } else {
                assert!(keychain.read(None).unwrap().is_none(), "{phase}");
                assert!(
                    fallback.as_deref()
                        == Some(credentials("original-token").to_string().as_bytes()),
                    "{phase}"
                );
                assert_eq!(profile, ORIGINAL_PROFILE, "{phase}");
            }
            assert!(
                keychain.read(Some(&keychain.target)).unwrap() == Some(credentials("target-token")),
                "{phase}"
            );
            assert!(
                keychain.read(Some(&keychain.parked)).unwrap()
                    == (!replacement).then(|| credentials("original-token")),
                "{phase}"
            );
        }
        keychain.write(None, None).unwrap();
        keychain.write(Some(&keychain.target), None).unwrap();
        keychain.write(Some(&keychain.parked), None).unwrap();
        assert!(keychain.read(None).unwrap().is_none(), "{phase}");
        assert!(keychain.read(Some(&keychain.target)).unwrap().is_none(), "{phase}");
        assert!(keychain.read(Some(&keychain.parked)).unwrap().is_none(), "{phase}");
    }
}

#[test]
fn auth6_claude_recover_rejects_invalid_or_stale_input() {
    for case in
        ["profile", "fallback", "keychain", "wrong-operation", "wrong-tenant", "invalid-journal"]
    {
        let (home, keychain) = fixture("target");
        crash_at(home.path(), if case.starts_with("wrong-") { case } else { "profile" });
        let directory = Directory::open(&keychain.live).unwrap();
        let profile = home.path().join(".claude.json");
        match case {
            "profile" => fs::write(
                &profile,
                br#"{"oauthAccount":{"accountUuid":"later","organizationUuid":"org"}}"#,
            )
            .unwrap(),
            "fallback" => directory.write(".credentials.json", Some(b"{}")).unwrap(),
            "keychain" => keychain.write(None, Some(&credentials("later-token"))).unwrap(),
            "invalid-journal" => directory.write(JOURNAL, Some(b"{}")).unwrap(),
            _ => {}
        }
        let journal = directory.read(JOURNAL).unwrap().unwrap();
        let fallback = directory.read(".credentials.json").unwrap();
        let profile_before = fs::read(&profile).unwrap();
        let keychain_before = keychain.read(None).unwrap();
        let tx = transactions(home.path());
        let host = LoginHost::open(home.path()).unwrap();
        let expected = if case == "invalid-journal" {
            WriteError::InvalidResource
        } else {
            WriteError::Conflict
        };
        assert_eq!(host.recover(&tx, &keychain), Err(expected), "{case}");
        assert!(directory.read(JOURNAL).unwrap().as_ref() == Some(&journal), "{case}");
        assert!(directory.read(".credentials.json").unwrap() == fallback, "{case}");
        assert!(keychain.read(None).unwrap() == keychain_before, "{case}");
        assert_eq!(fs::read(profile).unwrap(), profile_before, "{case}");
        assert_eq!(
            fs::read(keychain.live.join("settings.json")).unwrap(),
            ORIGINAL_SETTINGS,
            "{case}"
        );
        assert!(tx.state(AppId::Claude).unwrap() == AppState::default(), "{case}");
        assert_eq!(
            tx.pending(AppId::Claude).unwrap().is_some(),
            case.starts_with("wrong-"),
            "{case}"
        );
    }
}
