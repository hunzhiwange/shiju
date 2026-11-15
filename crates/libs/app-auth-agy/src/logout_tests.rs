use crate::{flow::AuthFlows, logout::LogoutSelection, test_support::oauth_server};
use serde_json::json;
use vw_api_app_auth::AuthErrorCode;

#[tokio::test]
async fn auth6_logout_agy_keychain_clears_owned_credentials_and_default() {
    use super::{
        agy_account::logout_accounts_in,
        agy_account_tests::{Fixture, credential, scope},
    };

    let mut fixture = Fixture::new();
    let first = fixture.managed("first", &credential("first", 1), "ready");
    let spare = fixture.managed("spare", &credential("spare", 1), "ready");
    let mut other = Fixture::new();
    let foreign = other.managed("spare", &credential("spare", 1), "ready");
    fixture.switch(&first).unwrap();
    let scope = scope(&fixture.tenant);
    let flows = AuthFlows::default();
    let entry = fixture.entry();
    let default = LogoutSelection::Default;
    assert_eq!(
        logout_accounts_in(&flows, scope, default, fixture.target(None), &entry),
        Err(AuthErrorCode::Forbidden)
    );
    assert!(entry.read().unwrap().is_some());
    let target = fixture.target(Some(scope));
    let (mut server, request) =
        oauth_server("200 OK", "", json!({"access_token": "late-agy-access", "expires_in": 3600}))
            .await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let refresh = super::agy_refresh::refresh_with(scope, &first, &client, |outbound| {
        assert_eq!(outbound.url().as_str(), "https://oauth2.googleapis.com/token");
        // The adapter has captured its keychain generation and dropped the lock.
        assert_eq!(logout_accounts_in(&flows, scope, default, target, &entry), Ok(()));
        server.set_path(outbound.url().path());
        *outbound.url_mut() = server;
    })
    .await;
    assert_eq!(refresh.as_ref().err(), Some(&AuthErrorCode::Conflict));
    assert!(request.await.unwrap().starts_with("POST /token HTTP/1.1"));
    assert!(entry.read().unwrap().is_none());
    let record = fixture.record(&first);
    assert_eq!(record["account"]["credential_state"], "signed_out");
    assert_eq!(record["account"]["account_id"], first);
    assert!(record["credential"].is_null());
    assert_eq!(fixture.record(&spare)["account"]["credential_state"], "ready");
    assert_eq!(
        logout_accounts_in(&flows, scope, default, target, &entry),
        Err(AuthErrorCode::NotFound)
    );
    assert_eq!(logout_accounts_in(&flows, scope, LogoutSelection::All, target, &entry), Ok(()));
    assert_eq!(fixture.record(&spare)["account"]["credential_state"], "signed_out");
    assert_eq!(other.record(&foreign)["account"]["credential_state"], "ready");
}
