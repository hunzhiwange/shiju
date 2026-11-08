use super::{
    claude_keychain::{ClaudeKeychain, MacOsKeychain},
    claude_rotation::{ClaudeLoginState, ClaudeLoginSwitchStatus, get_claude_login_switch},
    claude_snapshot_tests::Keychain,
};
use crate::providers::{
    claude::ClaudeFile,
    claude_login::{LoginHost, LoginMaterial},
    live::{
        commit::{AppState, ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    },
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::DirBuilderExt, path::Path, process::Command};
use vw_api_types::{TimestampMs, agent_apps_auth::AuthCredentialState, id::TenantScope};

const NOW: TimestampMs = TimestampMs(1_800_000_000_000);
const NEXT_EXPIRY: TimestampMs = TimestampMs(1_900_000_000_000);

fn credentials(version: u32) -> Value {
    json!({
        "organizationUuid": "org",
        "claudeAiOauth": {
            "refreshToken": format!("synthetic-{version}"),
            "accessToken": format!("access-{version}"),
            "expiresAt": NOW.0 + i64::from(version) * 1000,
            "refreshTokenExpiresAt": 1_700_000_000 + i64::from(version) * 100_000_000
        }
    })
}

fn profile(directory: &Path, owner: &str) {
    fs::write(
        directory.join(".claude.json"),
        json!({"oauthAccount":{"accountUuid":owner,"organizationUuid":"org"},
               "mcpServers":{"preserved":true}})
        .to_string(),
    )
    .unwrap();
}

struct Fixture {
    home: tempfile::TempDir,
    keychain: Keychain,
    tx: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let keychain = Keychain {
            live: home.path().join(".claude"),
            target: home.path().join("target"),
            parked: home.path().join("parked"),
        };
        let private = home.path().join("private");
        for dir in [&keychain.live, &keychain.target, &keychain.parked, &private] {
            fs::DirBuilder::new().mode(0o700).create(dir).unwrap();
        }
        assert!(keychain.read(None).unwrap().is_none());
        assert!(keychain.read(Some(&keychain.parked)).unwrap().is_none());
        keychain.write(None, Some(&credentials(0))).unwrap();
        keychain.write(Some(&keychain.parked), Some(&credentials(0))).unwrap();
        profile(home.path(), "original");
        profile(&keychain.parked, "original");
        Self {
            home,
            keychain,
            tx: ProviderTransactions::new(&private, ExecutionHostId::new("fixture-host").unwrap())
                .unwrap(),
            tenant: TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        }
    }

    fn status(&self) -> Result<ClaudeLoginSwitchStatus, WriteError> {
        get_claude_login_switch(self.home.path(), &self.tx, &self.tenant, &self.keychain, NOW)
    }
}

#[test]
fn claude_cli_rotation_fixture_process() {
    let Some(path) = std::env::var_os("SHIJU_TEST_CLAUDE_ROTATION_SLOT") else { return };
    let path = Path::new(&path);
    let directory = super::claude_login_files::Directory::open(path).unwrap();
    let _refresh = directory.cli_lock(".oauth_refresh.lock").unwrap();
    let _storage = directory.cli_lock(".storage-write.lock").unwrap();
    MacOsKeychain.write(Some(path), Some(&credentials(2))).unwrap();
}

#[test]
fn auth6_claude_rotation_commits_expected_scoped_result() {
    let f = Fixture::new();
    assert_eq!(
        f.status(),
        Ok(ClaudeLoginSwitchStatus::Active(ClaudeLoginState {
            owner: Some("original:org".into()),
            credential_state: AuthCredentialState::ReauthRequired,
            login_expires_at: Some(TimestampMs(1_700_000_000_000)),
        })),
        "the initial status must reflect the refresh token expiry",
    );
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "auth::claude_rotation_tests::claude_cli_rotation_fixture_process"])
        .env("SHIJU_TEST_CLAUDE_ROTATION_SLOT", &f.keychain.live)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    assert_eq!(
        f.status(),
        Ok(ClaudeLoginSwitchStatus::Active(ClaudeLoginState {
            owner: Some("original:org".into()),
            credential_state: AuthCredentialState::Ready,
            login_expires_at: Some(NEXT_EXPIRY),
        })),
    );

    profile(&f.keychain.target, "next");
    f.keychain.write(Some(&f.keychain.target), Some(&credentials(1))).unwrap();
    fs::write(f.keychain.live.join("settings.json"), b"{}").unwrap();
    let host = LoginHost::open(f.home.path()).unwrap();
    let operation = host
        .begin(&f.tx, &f.tenant, &f.keychain.target, &f.keychain, |previous| {
            previous.archive(&f.keychain.parked, &f.keychain)
        })
        .unwrap();
    assert!(operation.is_some());
    ClaudeFile::open(f.home.path(), false)
        .unwrap()
        .apply_with_login(
            &f.tx,
            &f.tenant,
            &AppState::default(),
            None,
            &json!({"env":{}}),
            Some("bound".into()),
            || true,
            operation,
        )
        .unwrap();
    host.settle(&f.tx, &f.keychain).unwrap();
    let stored = f.keychain.read(Some(&f.keychain.parked)).unwrap();
    assert!(stored == Some(credentials(2)));
    assert!(f.keychain.read(None).unwrap() == Some(credentials(1)));
    assert_eq!(
        LoginMaterial::from_account(&f.keychain.parked, &f.keychain)
            .unwrap()
            .identity()
            .unwrap()
            .as_deref(),
        Some("original:org")
    );
    drop(host);
    assert_eq!(
        f.status(),
        Ok(ClaudeLoginSwitchStatus::Active(ClaudeLoginState {
            owner: Some("next:org".into()),
            credential_state: AuthCredentialState::ReauthRequired,
            login_expires_at: Some(NOW),
        })),
    );
    let other = TenantScope::new("fixture-platform", "other-company").unwrap();
    assert_eq!(
        get_claude_login_switch(f.home.path(), &f.tx, &other, &f.keychain, NOW),
        Err(WriteError::Conflict),
    );
    f.keychain.write(None, None).unwrap();
    f.keychain.write(Some(&f.keychain.parked), None).unwrap();
    assert!(f.keychain.read(None).unwrap().is_none());
    assert!(f.keychain.read(Some(&f.keychain.parked)).unwrap().is_none());
}

#[test]
fn auth6_claude_rotation_rejects_invalid_or_stale_input() {
    let f = Fixture::new();
    let host = LoginHost::open(f.home.path()).unwrap();
    let old = host.material(&f.keychain).unwrap();
    profile(&f.keychain.parked, "other");
    let unchanged = fs::read(f.keychain.parked.join(".claude.json")).unwrap();
    assert_eq!(
        old.archive(&f.keychain.parked, &f.keychain),
        Err(WriteError::Conflict),
        "a stale refresh must not write into a directory now owned by another account",
    );
    assert_eq!(fs::read(f.keychain.parked.join(".claude.json")).unwrap(), unchanged);
    assert!(f.keychain.read(Some(&f.keychain.parked)).unwrap() == Some(credentials(0)));

    profile(&f.keychain.parked, "original");
    f.keychain.write(Some(&f.keychain.parked), Some(&credentials(2))).unwrap();
    assert_eq!(old.archive(&f.keychain.parked, &f.keychain), Err(WriteError::Conflict));
    assert!(f.keychain.read(Some(&f.keychain.parked)).unwrap() == Some(credentials(2)));
    assert!(f.keychain.read(None).unwrap() == Some(credentials(0)));
    drop(host);

    let mut mismatch = credentials(2);
    mismatch["organizationUuid"] = json!("other-org");
    f.keychain.write(None, Some(&mismatch)).unwrap();
    assert_eq!(f.status(), Err(WriteError::InvalidResource));
    let override_path = f.keychain.target.to_str().unwrap();
    f.tx.set_claude_directory(f.home.path(), &f.tenant, Some(override_path)).unwrap();
    assert_eq!(f.status(), Ok(ClaudeLoginSwitchStatus::OverrideDir));
    let other = TenantScope::new("fixture-platform", "other-company").unwrap();
    assert_eq!(
        get_claude_login_switch(f.home.path(), &f.tx, &other, &f.keychain, NOW),
        Err(WriteError::Conflict),
    );
}
