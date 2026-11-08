use super::{
    copilot::utc_now,
    copilot_tests::oauth_server,
    credential_file::{AccountCredential, CredentialFileStore, XaiCredential},
    flow::AuthAccountScope,
    xai_refresh::XaiRefresh,
};
use reqwest::{Request, Url};
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, time::Duration};
use tokio::{net::TcpListener, time::timeout};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

pub(super) const ACCOUNT: &str = "fixture-account";
const OTHER_ACCOUNT: &str = "other-account";

pub(super) fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "fixture-host",
        service: AuthService::XaiOauth,
    }
}

pub(super) fn store() -> (tempfile::TempDir, CredentialFileStore) {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    (root, store)
}

pub(super) fn seed(
    store: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    id: &str,
    expires_at: i64,
) {
    // The batch 19 credential shape, also used for an already installed account.
    let credential = serde_json::from_value(json!({
        "access_token": format!("access-{id}"),
        "refresh_token": format!("refresh-{id}"),
        "expires_at": expires_at,
    }))
    .unwrap();
    store
        .lease_xai(scope, id, true)
        .unwrap()
        .save(&AccountCredential::XaiOauth(credential))
        .unwrap();
}

pub(super) fn saved(
    store: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    id: &str,
) -> XaiCredential {
    let AccountCredential::XaiOauth(tokens) =
        store.lease_xai(scope, id, false).unwrap().read().unwrap()
    else {
        panic!("wrong credential kind");
    };
    tokens
}

pub(super) async fn discovery() -> (Url, tokio::task::JoinHandle<String>) {
    oauth_server(
        "200 OK",
        "",
        json!({
            "issuer": "https://auth.x.ai",
            "token_endpoint": "https://auth.x.ai/oauth2/token",
            "device_authorization_endpoint": "https://auth.x.ai/oauth2/device/code",
        }),
    )
    .await
}

pub(super) fn route(request: &mut Request, discovery: &Url, token: &Url) {
    let base = match request.url().as_str() {
        "https://auth.x.ai/.well-known/openid-configuration" => discovery,
        "https://auth.x.ai/oauth2/token" => token,
        _ => panic!("unexpected xAI endpoint"),
    };
    let mut url = base.clone();
    url.set_path(request.url().path());
    *request.url_mut() = url;
}

async fn exchange(
    refresh: &XaiRefresh,
    store: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    status: &str,
    body: Value,
) -> Result<String, AuthErrorCode> {
    let (discover, discovered) = discovery().await;
    let (token, posted) = oauth_server(status, "", body).await;
    let result = refresh
        .access_token_with(store, scope, ACCOUNT, |outbound| {
            let lease = store.lease_xai(scope, ACCOUNT, false).unwrap();
            assert!(lease.lock_refresh().is_err());
            route(outbound, &discover, &token);
        })
        .await;
    assert!(
        timeout(Duration::from_secs(5), discovered)
            .await
            .unwrap()
            .unwrap()
            .starts_with("GET /.well-known/openid-configuration HTTP/1.1\r\n")
    );
    let request = timeout(Duration::from_secs(5), posted).await.unwrap().unwrap();
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    assert!(headers.starts_with("POST /oauth2/token HTTP/1.1\r\n"));
    assert!(headers.contains("\r\nuser-agent: cc-switch-xai-oauth\r\n"));
    assert!(headers.contains("\r\ncontent-type: application/x-www-form-urlencoded"));
    assert_eq!(
        body,
        "grant_type=refresh_token&client_id=b1a00492-073a-47ea-816f-4c329264a828&refresh_token=refresh-fixture-account&scope=openid+profile+email+offline_access+grok-cli%3Aaccess+api%3Aaccess"
    );
    result
}

#[tokio::test]
async fn auth6_xai_refresh_commits_expected_scoped_result() {
    let (root, store) = store();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    seed(&store, scope, ACCOUNT, 1);
    seed(&store, scope, OTHER_ACCOUNT, i64::MAX);
    let refresh = XaiRefresh::new().unwrap();
    let result = exchange(
        &refresh,
        &store,
        scope,
        "200 OK",
        json!({"access_token":"refreshed-access","refresh_token":"rotated-refresh","expires_in":3600}),
    )
    .await;
    assert_eq!(result.as_deref(), Ok("refreshed-access"), "expired xAI credentials should rotate");

    let reopened = CredentialFileStore::new(root.path()).unwrap();
    let tokens = saved(&reopened, scope, ACCOUNT);
    assert_eq!(tokens.refresh_token, "rotated-refresh");
    assert!(tokens.expires_at.0 > 1);
    assert_eq!(
        XaiRefresh::new()
            .unwrap()
            .access_token_with(&reopened, scope, ACCOUNT, |_| panic!(
                "cached credential should be fresh"
            ))
            .await
            .as_deref(),
        Ok("refreshed-access")
    );
    assert_eq!(saved(&reopened, scope, OTHER_ACCOUNT).refresh_token, "refresh-other-account");
}

#[tokio::test]
async fn auth6_xai_refresh_rejects_invalid_or_stale_input() {
    let (root, store) = store();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let expires_at = utc_now().unwrap().0 + 45_000;
    seed(&store, scope, ACCOUNT, expires_at);
    seed(&store, scope, OTHER_ACCOUNT, i64::MAX);
    let refresh = XaiRefresh::new().unwrap();

    let (discover, discovered) = discovery().await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let closed = tokio::spawn(async move {
        drop(listener.accept().await.unwrap());
    });
    let unavailable = refresh
        .access_token_with(&store, scope, ACCOUNT, |outbound| route(outbound, &discover, &token))
        .await;
    assert_eq!(unavailable, Err(AuthErrorCode::UpstreamFailure), "network failure stays retryable");
    timeout(Duration::from_secs(5), discovered).await.unwrap().unwrap();
    timeout(Duration::from_secs(5), closed).await.unwrap().unwrap();

    assert_eq!(
        exchange(
            &refresh,
            &store,
            scope,
            "503 Service Unavailable",
            json!({"error":"temporarily_unavailable"})
        )
        .await,
        Err(AuthErrorCode::UpstreamFailure)
    );
    assert!(!saved(&store, scope, ACCOUNT).requires_reauth);
    assert_eq!(
        exchange(
            &refresh,
            &store,
            scope,
            "400 Bad Request",
            json!({"error":"invalid_grant","error_description":"private fixture text"})
        )
        .await,
        Err(AuthErrorCode::ReauthRequired)
    );

    let reopened = CredentialFileStore::new(root.path()).unwrap();
    let tokens = saved(&reopened, scope, ACCOUNT);
    assert!(tokens.requires_reauth);
    assert_eq!(tokens.expires_at.0, expires_at);
    assert_eq!(
        XaiRefresh::new()
            .unwrap()
            .access_token_with(&reopened, scope, ACCOUNT, |_| panic!(
                "must not retry a rejected grant"
            ))
            .await,
        Err(AuthErrorCode::ReauthRequired)
    );
    assert_eq!(
        refresh
            .access_token_with(&reopened, scope, OTHER_ACCOUNT, |_| panic!(
                "unaffected account should be fresh"
            ))
            .await
            .as_deref(),
        Ok("access-other-account")
    );
    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    assert_eq!(
        refresh
            .access_token_with(&reopened, foreign, ACCOUNT, |_| panic!(
                "foreign scope must not reach OAuth"
            ))
            .await,
        Err(AuthErrorCode::StorageUnavailable)
    );

    seed(&reopened, scope, ACCOUNT, i64::MAX);
    assert_eq!(
        refresh
            .access_token_with(&reopened, scope, ACCOUNT, |_| panic!(
                "replacement credential should be fresh"
            ))
            .await
            .as_deref(),
        Ok("access-fixture-account")
    );
}
