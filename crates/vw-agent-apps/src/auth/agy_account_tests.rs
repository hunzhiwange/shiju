use super::{
    agy_account::{AgyNativeTarget, agy_account_is_running, switch_account_in},
    agy_login::MANAGED_SERVICE,
    flow::AuthAccountScope,
    keychain::{EntryError, KeychainEntry},
};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use security_framework::os::macos::keychain::{SecKeychain, SecPreferencesDomain};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

pub(super) fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:agy-test-owner",
        host: "agy-test-host",
        service: AuthService::AgyOauth,
    }
}

pub(super) fn credential(subject: &str, generation: u8) -> Value {
    let claims = URL_SAFE_NO_PAD.encode(json!({"sub": subject}).to_string());
    json!({
        "token": {
            "access_token": format!("synthetic-access-{generation}"),
            "refresh_token": format!("synthetic-refresh-{generation}"),
            "token_type": "Bearer",
            "expiry": format!("2030-01-{generation:02}T00:00:00Z"),
            "expires_in": 3600,
        },
        "id_token": format!("header.{claims}.{}", "x".repeat(1500)),
        "auth_method": "consumer",
        "unknown": {"field": generation},
    })
}

pub(super) fn encode(credential: &Value) -> String {
    format!("go-keyring-base64:{}", STANDARD.encode(credential.to_string()))
}

pub(super) struct Fixture {
    pub(super) tenant: TenantScope,
    keychain: SecKeychain,
    pub(super) native_service: String,
    home: PathBuf,
    ids: Vec<String>,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let nonce = Uuid::new_v4().simple();
        Self {
            tenant: TenantScope::new("platform", format!("agy-test-{nonce}")).unwrap(),
            keychain: SecKeychain::default_for_domain(SecPreferencesDomain::User).unwrap(),
            native_service: format!("shiju.agy.switch.test.{nonce}"),
            home: PathBuf::from(std::env::var_os("HOME").unwrap()),
            ids: Vec::new(),
        }
    }

    pub(super) fn managed(&mut self, subject: &str, credential: &Value, state: &str) -> String {
        let scope = scope(&self.tenant);
        let identity = serde_json::to_vec(&(
            scope.tenant,
            scope.principal,
            scope.host,
            scope.service,
            subject,
        ))
        .unwrap();
        let id = URL_SAFE_NO_PAD.encode(Sha256::digest(identity));
        let record = json!({
            "account": {
                "account_id": id,
                "service": "agy_oauth",
                "label": "Synthetic account",
                "credential_state": state,
                "is_default": false,
            },
            "credential": credential,
        });
        self.keychain
            .add_generic_password(MANAGED_SERVICE, &id, &serde_json::to_vec(&record).unwrap())
            .unwrap();
        self.ids.push(id.clone());
        id
    }

    pub(super) fn record(&self, id: &str) -> Value {
        let (data, _) = self.keychain.find_generic_password(MANAGED_SERVICE, id).unwrap();
        serde_json::from_slice(&data).unwrap()
    }

    pub(super) fn entry(&self) -> KeychainEntry<'_> {
        KeychainEntry::new(&self.native_service, "antigravity").unwrap()
    }

    pub(super) fn target<'a>(&'a self, owner: Option<AuthAccountScope<'a>>) -> AgyNativeTarget<'a> {
        AgyNativeTarget { home: &self.home, exclusive_owner: owner }
    }

    pub(super) fn switch(&self, id: &str) -> Result<(), AuthErrorCode> {
        let scope = scope(&self.tenant);
        switch_account_in(scope, id, self.target(Some(scope)), &self.entry())
    }

    pub(super) fn remove_entries(&mut self) {
        self.entry().delete().unwrap();
        assert!(self.entry().read().unwrap().is_none());
        for id in self.ids.drain(..) {
            if let Ok((_, item)) = self.keychain.find_generic_password(MANAGED_SERVICE, &id) {
                item.delete();
            }
            assert!(self.keychain.find_generic_password(MANAGED_SERVICE, &id).is_err());
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.entry().delete();
        for id in self.ids.drain(..) {
            if let Ok((_, item)) = self.keychain.find_generic_password(MANAGED_SERVICE, &id) {
                item.delete();
            }
        }
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn auth6_agy_switch_commits_expected_scoped_result() {
    let mut fixture = Fixture::new();
    let first = credential("first", 1);
    let second = credential("second", 1);
    let a = fixture.managed("first", &first, "ready");
    let b = fixture.managed("second", &second, "ready");
    assert_eq!(fixture.switch(&a), Ok(()), "authorized managed account must switch");
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&first).into_bytes()));

    let dir = tempfile::tempdir().unwrap();
    let executable = dir.path().join("agy");
    std::fs::copy("/bin/sleep", &executable).unwrap();
    let mut running = ChildGuard(Command::new(executable).arg("30").spawn().unwrap());
    let pid = running.0.id().to_string();
    let process = Command::new("/bin/ps").args(["-p", &pid, "-o", "comm="]).output().unwrap();
    assert_eq!(
        Path::new(String::from_utf8_lossy(&process.stdout).trim()).file_name(),
        Some(std::ffi::OsStr::new("agy")),
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while !agy_account_is_running() {
        assert!(running.0.try_wait().unwrap().is_none());
        assert!(Instant::now() < deadline, "running agy must be reported");
        thread::sleep(Duration::from_millis(10));
    }
    let mut rotated = credential("first", 2);
    rotated["token"]["refresh_token"] = first["token"]["refresh_token"].clone();
    fixture.entry().replace_base64(Some(encode(&first).as_bytes()), &encode(&rotated)).unwrap();
    assert_eq!(fixture.switch(&a), Ok(()));
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&rotated).into_bytes()));
    assert_eq!(fixture.record(&a)["credential"], first);
    assert_eq!(fixture.switch(&b), Ok(()));
    drop(running);
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&second).into_bytes()));
    assert_eq!(fixture.record(&a)["credential"], rotated);
    assert_eq!(fixture.switch(&a), Ok(()));
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&rotated).into_bytes()));
    fixture.remove_entries();
}

#[test]
fn auth6_agy_switch_rejects_invalid_or_stale_input() {
    let mut fixture = Fixture::new();
    let first = fixture.managed("first", &credential("first", 1), "ready");
    let stale = fixture.managed("stale", &credential("stale", 1), "reauth_required");
    let misbound = fixture.managed("expected", &credential("different", 1), "ready");
    let owner = scope(&fixture.tenant);
    assert_eq!(
        switch_account_in(owner, &first, fixture.target(None), &fixture.entry()),
        Err(AuthErrorCode::Forbidden),
        "the native slot requires an exclusive owner",
    );
    let other_tenant = TenantScope::new("platform", "agy-test-other").unwrap();
    assert_eq!(
        switch_account_in(
            scope(&other_tenant),
            &first,
            fixture.target(Some(owner)),
            &fixture.entry()
        ),
        Err(AuthErrorCode::Forbidden),
    );
    assert_eq!(
        switch_account_in(
            owner,
            &first,
            AgyNativeTarget { home: Path::new("/"), exclusive_owner: Some(owner) },
            &fixture.entry(),
        ),
        Err(AuthErrorCode::Forbidden),
    );
    assert_eq!(fixture.switch("../invalid"), Err(AuthErrorCode::InvalidInput));
    assert_eq!(fixture.switch(&stale), Err(AuthErrorCode::ReauthRequired));
    assert_eq!(fixture.switch(&misbound), Err(AuthErrorCode::NotFound));
    assert!(fixture.entry().read().unwrap().is_none());

    fixture.entry().write("unrecognized-native-login").unwrap();
    assert_eq!(
        fixture.entry().replace_base64(None, &encode(&credential("first", 1))),
        Err(EntryError::Conflict),
    );
    let mut oversized = credential("first", 2);
    oversized["unknown"] = Value::String("x".repeat(5000));
    assert_eq!(
        fixture.entry().replace_base64(Some(b"unrecognized-native-login"), &encode(&oversized)),
        Err(EntryError::InvalidPayload),
    );
    assert_eq!(fixture.switch(&first), Err(AuthErrorCode::Conflict));
    assert_eq!(
        fixture.entry().read().unwrap().as_deref(),
        Some(b"unrecognized-native-login".as_slice())
    );
    let other_grant = encode(&credential("first", 2));
    fixture.entry().replace_base64(Some(b"unrecognized-native-login"), &other_grant).unwrap();
    let second = fixture.managed("second", &credential("second", 1), "ready");
    assert_eq!(fixture.switch(&first), Err(AuthErrorCode::Conflict));
    assert_eq!(fixture.switch(&second), Err(AuthErrorCode::Conflict));
    assert_eq!(fixture.entry().read().unwrap().as_deref(), Some(other_grant.as_bytes()));
    assert_eq!(fixture.record(&first)["credential"], credential("first", 1));
    fixture.remove_entries();
}
