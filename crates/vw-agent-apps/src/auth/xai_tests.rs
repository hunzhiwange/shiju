use super::{
    copilot_tests::oauth_server,
    credential_file::{AccountCredential, CredentialFileStore},
    flow::{AuthAccountScope, AuthFlows},
    xai::{XaiDeviceAuth, XaiDeviceStart},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Request, Url};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};
use vw_api_types::{
    agent_apps_auth::{
        AuthErrorCode, AuthFlow, AuthLoginMethod, AuthService, AuthStartRequest, AuthStartResponse,
    },
    id::TenantScope,
};

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "fixture-host",
        service: AuthService::XaiOauth,
    }
}

fn request() -> AuthStartRequest {
    AuthStartRequest {
        method: AuthLoginMethod::DeviceCode,
        github_domain: None,
        region: None,
        target_account_id: None,
    }
}

fn discovery() -> Value {
    json!({
        "issuer":"https://auth.x.ai",
        "token_endpoint":"https://auth.x.ai/oauth2/token",
        "device_authorization_endpoint":"https://auth.x.ai/oauth2/device/code"
    })
}

fn route_to(outbound: &mut Request, path: &str, mut route: Url) {
    assert_eq!(outbound.url().as_str(), format!("https://auth.x.ai{path}"));
    route.set_path(path);
    *outbound.url_mut() = route;
}

fn tokens() -> Value {
    let payload =
        URL_SAFE_NO_PAD.encode(br#"{"sub":"fixture-subject","email":"owner@example.test"}"#);
    json!({
        "access_token":"fixture-private-access",
        "refresh_token":"fixture-private-refresh",
        "id_token": format!("e30.{payload}.fixture"),
        "expires_in":3600,
    })
}

async fn start(
    auth: &XaiDeviceAuth,
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
) -> XaiDeviceStart {
    let (discover, discovered) = oauth_server("200 OK", "", discovery()).await;
    let (device, requested) = oauth_server(
        "200 OK",
        "",
        json!({
            "device_code":"fixture-private-grant",
            "user_code":"ABCD-EFGH",
            "verification_uri":"https://auth.x.ai/oauth2/device",
            "verification_uri_complete":"https://auth.x.ai/oauth2/device?user_code=ABCD-EFGH",
            "expires_in":120,
            "interval":2,
        }),
    )
    .await;
    let mut routes = VecDeque::from([
        ("/.well-known/openid-configuration", discover),
        ("/oauth2/device/code", device),
    ]);
    let result = auth
        .start_with(flows, scope, request(), |outbound| {
            let (path, url) = routes.pop_front().unwrap();
            route_to(outbound, path, url);
        })
        .await;
    assert_eq!(result.as_ref().err(), None, "valid xAI discovery and device challenge must start");
    assert!(routes.is_empty());
    assert!(
        discovered.await.unwrap().starts_with("GET /.well-known/openid-configuration HTTP/1.1\r\n")
    );
    let sent = requested.await.unwrap();
    assert!(sent.starts_with("POST /oauth2/device/code HTTP/1.1\r\n"));
    assert!(sent.contains("\r\nuser-agent: cc-switch-xai-oauth\r\n"));
    assert!(sent.contains("\r\ncontent-type: application/x-www-form-urlencoded\r\n"));
    assert!(sent.ends_with("client_id=b1a00492-073a-47ea-816f-4c329264a828&scope=openid+profile+email+offline_access+grok-cli%3Aaccess+api%3Aaccess"));
    result.unwrap()
}

#[tokio::test]
async fn auth6_xai_device_commits_expected_scoped_result() {
    let auth = XaiDeviceAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let started = start(&auth, &flows, scope).await;
    let AuthStartResponse::Pending {
        flow_id,
        verification_url,
        user_code,
        retry_after_seconds,
        ..
    } = &started.response;
    assert_eq!(verification_url, "https://auth.x.ai/oauth2/device?user_code=ABCD-EFGH");
    assert_eq!(user_code.as_deref(), Some("ABCD-EFGH"));
    assert_eq!(*retry_after_seconds, 5);
    assert!(!serde_json::to_string(&started.response).unwrap().contains("fixture-private-grant"));

    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let due = Instant::now() + Duration::from_secs(6);
    let mut at = due;
    for (error, wait) in [("authorization_pending", 5), ("slow_down", 10)] {
        let (url, sent) = oauth_server("400 Bad Request", "", json!({"error":error})).await;
        let pending = auth
            .poll_with(
                &flows,
                scope,
                &started,
                || at,
                |outbound| {
                    route_to(outbound, "/oauth2/token", url.clone());
                },
                |_, _, _, _| panic!("pending authorization persisted"),
            )
            .await
            .unwrap();
        assert!(
            matches!(pending, AuthFlow::Pending { retry_after_seconds, .. } if retry_after_seconds == wait)
        );
        let sent = sent.await.unwrap();
        assert!(sent.starts_with("POST /oauth2/token HTTP/1.1\r\n"));
        assert!(sent.ends_with("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&client_id=b1a00492-073a-47ea-816f-4c329264a828&device_code=fixture-private-grant"));
        let early = auth
            .poll_with(
                &flows,
                scope,
                &started,
                || at + Duration::from_secs(wait - 1),
                |_| panic!("early poll reached HTTP"),
                |_, _, _, _| panic!("early poll persisted"),
            )
            .await
            .unwrap();
        assert_eq!(early, pending);
        at += Duration::from_secs(wait);
    }
    let mut response = tokens();
    response["padding"] = json!("x".repeat(20 * 1024));
    let (url, sent) = oauth_server("200 OK", "", response).await;
    let approved = auth
        .poll_with(
            &flows,
            scope,
            &started,
            || at,
            |outbound| {
                route_to(outbound, "/oauth2/token", url.clone());
            },
            |bound, account, identity, credential| {
                assert!(bound == scope);
                assert_eq!(account.service, AuthService::XaiOauth);
                assert_eq!(account.label, "owner@example.test");
                assert_eq!(identity, "fixture-subject");
                assert_eq!(
                    credential.expires_at.0 - account.authenticated_at.unwrap().0,
                    3_600_000
                );
                store
                    .lease_xai(bound, &account.account_id, true)
                    .unwrap()
                    .save(&AccountCredential::XaiOauth(credential))
                    .unwrap();
                Ok(())
            },
        )
        .await
        .unwrap();
    sent.await.unwrap();
    let AuthFlow::Approved { account, .. } = &approved else { panic!("expected approval") };
    assert!(!account.is_default);
    let AccountCredential::XaiOauth(saved) =
        store.lease_xai(scope, &account.account_id, false).unwrap().read().unwrap()
    else {
        panic!("expected private xAI credential")
    };
    assert_eq!(saved.access_token, "fixture-private-access");
    assert_eq!(saved.refresh_token, "fixture-private-refresh");
    for foreign in [
        AuthAccountScope { principal: "oidc:other", ..scope },
        AuthAccountScope { host: "other", ..scope },
        AuthAccountScope { service: AuthService::GithubCopilot, ..scope },
    ] {
        assert!(store.lease_xai(foreign, &account.account_id, false).is_err());
    }
    let projected = serde_json::to_string(&approved).unwrap();
    assert!(!projected.contains("fixture-private"));
    assert!(!projected.contains("fixture-subject"));
    assert_eq!(flows.snapshot(scope, flow_id, Instant::now), Ok(approved));
}

#[tokio::test]
async fn auth6_xai_device_rejects_invalid_or_stale_input() {
    let auth = XaiDeviceAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let unsupported = auth
        .start_with(
            &flows,
            scope,
            AuthStartRequest { method: AuthLoginMethod::Browser, ..request() },
            |_| panic!("browser login reached HTTP"),
        )
        .await;
    assert_eq!(unsupported.err(), Some(AuthErrorCode::Unsupported));

    let targeted = auth
        .start_with(
            &flows,
            scope,
            AuthStartRequest { target_account_id: Some("existing-account".into()), ..request() },
            |_| panic!("targeted login reached HTTP"),
        )
        .await;
    assert_eq!(targeted.err(), Some(AuthErrorCode::InvalidInput));

    let mut invalid = discovery();
    invalid["token_endpoint"] = json!("https://auth.x.ai.attacker.test/token");
    let (url, sent) = oauth_server("200 OK", "", invalid).await;
    let result = auth
        .start_with(&flows, scope, request(), |outbound| {
            route_to(outbound, "/.well-known/openid-configuration", url.clone());
        })
        .await;
    assert_eq!(
        result.err(),
        Some(AuthErrorCode::UpstreamFailure),
        "discovery must reject a foreign token host"
    );
    sent.await.unwrap();

    let elapsed = start(&auth, &flows, scope).await;
    let expired = auth
        .poll_with(
            &flows,
            scope,
            &elapsed,
            || Instant::now() + Duration::from_secs(121),
            |_| panic!("elapsed grant reached HTTP"),
            |_, _, _, _| panic!("elapsed grant persisted"),
        )
        .await
        .unwrap();
    assert!(matches!(expired, AuthFlow::Expired { .. }));

    let mut missing_refresh = tokens();
    missing_refresh.as_object_mut().unwrap().remove("refresh_token");
    for (status, payload, expected, cancel) in [
        (
            "403 Forbidden",
            json!({"error":"access_denied","error_description":"fixture-private-detail"}),
            Some(AuthErrorCode::AuthorizationDenied),
            false,
        ),
        ("400 Bad Request", json!({"error":"expired_token"}), None, false),
        (
            "200 OK",
            json!({"access_token":"fixture-private-access","refresh_token":"fixture-private-refresh"}),
            Some(AuthErrorCode::UpstreamFailure),
            false,
        ),
        ("200 OK", missing_refresh, Some(AuthErrorCode::UpstreamFailure), false),
        ("200 OK", tokens(), None, true),
    ] {
        let started = start(&auth, &flows, scope).await;
        let AuthStartResponse::Pending { flow_id, .. } = &started.response;
        let due = Instant::now() + Duration::from_secs(6);
        let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
        assert_eq!(
            auth.poll_with(
                &flows,
                foreign,
                &started,
                || due,
                |_| panic!("foreign flow reached HTTP"),
                |_, _, _, _| panic!("foreign flow persisted"),
            )
            .await,
            Err(AuthErrorCode::NotFound)
        );
        let (url, sent) = oauth_server(status, "", payload).await;
        let outcome = auth
            .poll_with(
                &flows,
                scope,
                &started,
                || due,
                |outbound| {
                    if cancel {
                        flows.cancel(scope, flow_id, || due).unwrap();
                    }
                    route_to(outbound, "/oauth2/token", url.clone());
                },
                |_, _, _, _| panic!("failed or cancelled attempt persisted"),
            )
            .await
            .unwrap();
        sent.await.unwrap();
        if cancel {
            assert!(matches!(outcome, AuthFlow::Cancelled { .. }));
        } else if let Some(expected) = expected {
            assert!(matches!(&outcome, AuthFlow::Failed { error, .. } if error.code == expected));
            assert!(!serde_json::to_string(&outcome).unwrap().contains("fixture-private-detail"));
        } else {
            assert!(matches!(outcome, AuthFlow::Expired { .. }));
        }
        assert_eq!(
            auth.poll_with(
                &flows,
                scope,
                &started,
                || due,
                |_| panic!("terminal flow reached HTTP"),
                |_, _, _, _| panic!("terminal flow persisted"),
            )
            .await,
            Ok(outcome)
        );
    }
}
