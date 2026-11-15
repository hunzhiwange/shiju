use crate::agy_account_tests::{Fixture, credential, encode, scope as agy_scope};
use vw_api_app_auth::AuthErrorCode;

#[test]
fn auth6_default_commits_expected_scoped_result() {
    let mut fixture = Fixture::new();
    let previous = credential("first", 1);
    let next = credential("second", 2);
    let first = fixture.managed("first", &previous, "ready");
    let second = fixture.managed("second", &next, "ready");
    assert_eq!(fixture.switch(&first), Ok(()));
    assert_eq!(fixture.switch(&second), Ok(()));
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&next).into_bytes()));
    fixture.remove_entries();
}

#[test]
fn auth6_default_rejects_invalid_or_stale_input() {
    let mut fixture = Fixture::new();
    let previous = credential("first", 1);
    let mut oversized = credential("second", 2);
    oversized["unknown"]["payload"] = serde_json::json!("x".repeat(4096));
    let first = fixture.managed("first", &previous, "ready");
    let second = fixture.managed("second", &oversized, "ready");
    assert_eq!(fixture.switch(&first), Ok(()));
    let native_scope = agy_scope(&fixture.tenant);
    assert_eq!(
        super::agy_account::switch_account_in(
            native_scope,
            &second,
            fixture.target(None),
            &fixture.entry()
        ),
        Err(AuthErrorCode::Forbidden)
    );
    assert_eq!(fixture.switch(&second), Err(AuthErrorCode::KeychainUnavailable));
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&previous).into_bytes()));
    fixture.remove_entries();
}

#[test]
fn auth6_remove_agy_preserves_live_and_referenced_accounts() {
    use super::agy_account::remove_managed_account_in;
    let mut fixture = Fixture::new();
    let active_credential = credential("live", 1);
    let inactive_credential = credential("inactive", 2);
    let active = fixture.managed("live", &active_credential, "ready");
    let inactive = fixture.managed("inactive", &inactive_credential, "reauth_required");
    fixture.switch(&active).unwrap();
    let scope = agy_scope(&fixture.tenant);
    let native = fixture.target(Some(scope));
    let entry = fixture.entry();

    assert_eq!(
        remove_managed_account_in(scope, &active, native, &entry, || panic!("live slot is in use")),
        Err(AuthErrorCode::AccountInUse)
    );
    assert_eq!(
        remove_managed_account_in(scope, &inactive, native, &entry, || Err(
            AuthErrorCode::AccountInUse
        )),
        Err(AuthErrorCode::AccountInUse)
    );
    assert_eq!(fixture.record(&inactive)["credential"], inactive_credential);
    assert_eq!(remove_managed_account_in(scope, &inactive, native, &entry, || Ok(())), Ok(()));
    assert_eq!(
        remove_managed_account_in(scope, &inactive, native, &entry, || Ok(())),
        Err(AuthErrorCode::NotFound)
    );
    assert_eq!(entry.read().unwrap(), Some(encode(&active_credential).into_bytes()));
    assert_eq!(fixture.record(&active)["credential"], active_credential);
    fixture.remove_entries();
}
