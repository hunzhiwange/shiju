use super::transport::credential;
use crate::{auth, provider::provider};
use axum::http::StatusCode;

#[test]
fn acp_model_source_relay_chat_oauth_expiry_and_revocation_never_use_cached_keys() {
    let info = provider::Info {
        id: "relay-oauth-fixture".into(),
        name: "fixture".into(),
        source: provider::ProviderSource::Api,
        env: Vec::new(),
        key: Some("stale-api-secret".into()),
        options: Default::default(),
        models: Default::default(),
    };
    let oauth = |expires| {
        Some(auth::Info::Oauth(auth::OauthInfo {
            access: "fresh-access".into(),
            refresh: "refresh-not-a-bearer".into(),
            expires,
            account_id: None,
            enterprise_url: None,
        }))
    };
    assert_eq!(
        credential(&info, oauth(chrono::Utc::now().timestamp_millis() + 60_000)).unwrap(),
        "fresh-access"
    );
    assert_eq!(credential(&info, oauth(0)).unwrap_err(), StatusCode::UNAUTHORIZED);
    assert_eq!(credential(&info, None).unwrap_err(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        credential(
            &info,
            Some(auth::Info::Api(auth::ApiInfo { key: "rotated-api-secret".into() }))
        )
        .unwrap(),
        "rotated-api-secret"
    );
}
