use super::*;
#[test]
fn cc04_codex_login_proxy_restores_unexpired_but_official_can_restore_expired() {
    for expired in [false, true] {
        let f = Fixture::new();
        let mut auth = login("alice", "team", "old");
        let payload = URL_SAFE_NO_PAD
            .encode(json!({"exp":if expired {1} else {1_900_000_000_i64}}).to_string());
        auth["tokens"]["access_token"] = json!(format!("e30.{payload}."));
        f.write("auth.json", &auth);
        f.apply(AuthTarget::ThirdParty { preserve: false }, None);
        f.apply(AuthTarget::ProxyThirdParty { restore_last: true }, None);
        if expired {
            assert!(!f.live.path().join("auth.json").exists());
        } else {
            assert_eq!(f.read("auth.json"), auth);
        }
        f.apply(AuthTarget::Official { row_auth: &json!({}) }, None);
        assert_eq!(f.read("auth.json"), auth);
    }
}
#[test]
fn cc04_codex_login_legacy_stash_recovers_only_existing_users_workspaces() {
    let f = Fixture::new();
    let plus = login("alice", "plus", "latest");
    let team = login("alice", "team", "old");
    let gone = login("bob", "other", "logged-out");
    f.write(STASH_FILENAME, &json!({"last":"sub:alice","logins":{"sub:alice":plus}}));
    let rows = [team.clone(), gone.clone()];
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared = prepare(
        &guard,
        LoginInput {
            official_logins: &rows,
            ..f.input(AuthTarget::Official { row_auth: &team }, None)
        },
    )
    .unwrap();
    f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &prepared.changes(),
        PendingTarget::default(),
    )
    .unwrap();
    assert_eq!(f.read("auth.json"), team);
    let stash = f.read(STASH_FILENAME);
    assert_eq!(stash["version"], 1);
    assert_eq!(stash["logins"]["sub:alice|account:plus"], plus);
    assert!(stash["logins"].get("sub:bob|account:other").is_none());
}
#[test]
fn cc04_codex_login_managed_target_saves_native_and_never_stashes_managed() {
    let f = Fixture::new();
    let native = login("alice", "plus", "latest");
    let managed = login("managed", "team", "latest");
    f.write("auth.json", &native);
    f.apply(AuthTarget::Managed { auth: &managed }, None);
    assert_eq!(f.read("auth.json"), managed);
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared = prepare(
        &guard,
        LoginInput {
            live_is_managed: true,
            ..f.input(AuthTarget::Official { row_auth: &json!({}) }, None)
        },
    )
    .unwrap();
    f.tx.commit(
        &TenantScope::new("p", "c").unwrap(),
        &mut guard,
        &prepared.changes(),
        PendingTarget::default(),
    )
    .unwrap();
    assert_eq!(f.read("auth.json"), native);
    assert_eq!(f.read(STASH_FILENAME)["logins"], json!({}));
}
#[test]
fn cc04_codex_login_metadata_does_not_claim_signed_in_status() {
    for auth in [
        json!({"last_refresh":"metadata","tokens":{"account_id":"alice"}}),
        json!({"auth_mode":"bedrockApiKey","bedrock_api_key":"fake-key","OPENAI_API_KEY":"stale"}),
        json!({"auth_mode":"headers","tokens":{"access_token":"fake"}}),
    ] {
        let f = Fixture::new();
        f.write("auth.json", &auth);
        let guard = lock_app(AppId::Codex, f.live.path()).unwrap();
        let prepared =
            prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: true }, None)).unwrap();
        assert!(!prepared.login_on_disk);
        assert_eq!(f.read("auth.json"), auth, "preparation is read only");
        assert!(!f.live.path().join(STASH_FILENAME).exists());
    }
}
