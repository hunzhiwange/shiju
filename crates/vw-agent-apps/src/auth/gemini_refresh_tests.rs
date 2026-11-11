use super::{
    copilot_tests::oauth_server,
    credential_file::{AccountCredential, CredentialFileStore, GeminiCredential},
    gemini_refresh::{GeminiRefresh, GeminiScope},
};
use serde_json::json;
use vw_api_types::{
    TimestampMs, agent_apps::AppId, agent_apps_auth::AuthErrorCode, id::TenantScope,
};
#[tokio::test]
async fn cc_route_031_gemini_refresh_and_invalid_grant() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::default();
    let scope = GeminiScope { tenant: &tenant, principal: "fixture-owner", host: "fixture-host" };
    let owner = store.for_owner(scope.principal).unwrap();
    let refresh = GeminiRefresh::new().unwrap();
    for (status, body, expected) in [
        (
            "200 OK",
            json!({"access_token":"fixture-new", "expires_in":3600}),
            Ok("fixture-new".to_string()),
        ),
        ("400 Bad Request", json!({"error":"invalid_grant"}), Err(AuthErrorCode::ReauthRequired)),
    ] {
        let lease =
            owner.lease(&tenant, scope.host, AppId::Gemini, "fixture-account", true).unwrap();
        lease
            .save(&AccountCredential::GeminiOauth(GeminiCredential {
                access_token: "fixture-old".into(),
                refresh_token: "fixture-refresh".into(),
                client_id: "fixture-client".into(),
                client_secret: "fixture-secret".into(),
                expires_at: TimestampMs(1),
                requires_reauth: false,
            }))
            .unwrap();
        drop(lease);
        let (route, captured) = oauth_server(status, "", body).await;
        let result = refresh
            .access_token_with(&store, scope, "fixture-account", |outbound| {
                assert_eq!(outbound.url().as_str(), "https://oauth2.googleapis.com/token");
                *outbound.url_mut() = route.clone();
            })
            .await;
        assert_eq!(result, expected);
        let request = captured.await.unwrap();
        assert!(request.contains("grant_type=refresh_token"));
        assert!(request.contains("client_id=fixture-client"));
        if expected.is_err() {
            assert_eq!(
                refresh
                    .access_token_with(&store, scope, "fixture-account", |_| panic!(
                        "invalidated grant must not issue another request"
                    ))
                    .await,
                Err(AuthErrorCode::ReauthRequired)
            );
        }
    }
}
