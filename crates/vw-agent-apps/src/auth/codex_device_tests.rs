use super::{
    codex_device::{CodexDeviceAuth, CodexDeviceStart},
    copilot_tests::oauth_server,
    credential_file::{AccountCredential, CredentialFileStore},
    flow::{AuthAccountScope, AuthFlows},
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
    agent_apps::AppId,
    agent_apps_auth::{
        AuthErrorCode, AuthFlow, AuthLoginMethod, AuthService, AuthStartRequest, AuthStartResponse,
    },
    id::TenantScope,
};

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "host",
        service: AuthService::CodexOauth,
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

fn challenge() -> Value {
    json!({"device_auth_id":"fixture-private-device","user_code":"ABCD-EFGH","interval":"5"})
}

fn jwt(claims: Value) -> String {
    format!(
        "{}.{}.fixture-signature",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256"}"#),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    )
}

fn route_to(outbound: &mut Request, path: &str, mut route: Url) {
    assert_eq!(outbound.url().as_str(), format!("https://auth.openai.com{path}"));
    route.set_path(path);
    *outbound.url_mut() = route;
}

async fn start(
    auth: &CodexDeviceAuth,
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
) -> CodexDeviceStart {
    let (route, received) = oauth_server("200 OK", "", challenge()).await;
    let result = auth
        .start_with(flows, scope, request(), |outbound| {
            route_to(outbound, "/api/accounts/deviceauth/usercode", route.clone());
        })
        .await;
    assert_eq!(result.as_ref().err(), None, "a valid Codex device challenge should start");
    let sent = received.await.unwrap();
    assert!(sent.starts_with("POST /api/accounts/deviceauth/usercode HTTP/1.1\r\n"));
    assert!(sent.contains("\r\ncontent-type: application/json\r\n"));
    assert_eq!(
        serde_json::from_str::<Value>(sent.split_once("\r\n\r\n").unwrap().1).unwrap(),
        json!({"client_id": "app_EMoamEEZ73f0CkXaXp7hrann"})
    );
    result.unwrap()
}

#[tokio::test]
async fn auth6_codex_device_commits_expected_scoped_result() {
    let auth = CodexDeviceAuth::new().unwrap();
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
    assert_eq!(verification_url, "https://auth.openai.com/codex/device");
    assert_eq!(user_code.as_deref(), Some("ABCD-EFGH"));
    assert_eq!(*retry_after_seconds, 8);
    assert!(!serde_json::to_string(&started.response).unwrap().contains("fixture-private-device"));

    let early = auth
        .poll_with(
            &flows,
            scope,
            &started,
            Instant::now,
            |_| panic!("early poll sent HTTP"),
            |_, _, _, _| panic!("early poll persisted"),
        )
        .await
        .unwrap();
    assert!(matches!(early, AuthFlow::Pending { .. }));

    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let due = Instant::now() + Duration::from_secs(9);
    for (status, now) in [("403 Forbidden", due), ("404 Not Found", due + Duration::from_secs(8))] {
        let (route, seen) = oauth_server(status, "", json!({})).await;
        let pending = auth
            .poll_with(
                &flows,
                scope,
                &started,
                || now,
                |outbound| {
                    route_to(outbound, "/api/accounts/deviceauth/token", route.clone());
                },
                |_, _, _, _| panic!("pending authorization attempted to persist"),
            )
            .await
            .unwrap();
        assert!(matches!(pending, AuthFlow::Pending { .. }));
        let sent = seen.await.unwrap();
        let (headers, body) = sent.split_once("\r\n\r\n").unwrap();
        assert!(headers.starts_with("POST /api/accounts/deviceauth/token HTTP/1.1\r\n"));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!({"device_auth_id":"fixture-private-device","user_code":"ABCD-EFGH"})
        );
        assert!(std::fs::read_dir(root.path()).unwrap().next().is_none());
    }

    let id_token = jwt(json!({"sub":"fixture-user"}));
    let access =
        jwt(json!({"https://api.openai.com/auth":{"chatgpt_account_id":"fixture-workspace"}}));
    let (poll, polled) = oauth_server(
        "200 OK",
        "",
        json!({"authorization_code":"fixture-code","code_verifier":"fixture-verifier"}),
    )
    .await;
    let (exchange, exchanged) = oauth_server("200 OK", "", json!({"access_token":access,"refresh_token":"fixture-refresh","id_token":id_token,"expires_in":3600})).await;
    let mut routes =
        VecDeque::from([("/api/accounts/deviceauth/token", poll), ("/oauth/token", exchange)]);
    let outcome = auth
        .poll_with(
            &flows,
            scope,
            &started,
            || due + Duration::from_secs(16),
            |outbound| {
                let (path, route) = routes.pop_front().unwrap();
                route_to(outbound, path, route);
            },
            |bound, account, identity, material| {
                assert!(bound == scope);
                assert_eq!(identity, r#"["fixture-workspace","fixture-user"]"#);
                assert!(!account.is_default);
                store
                    .for_owner(bound.principal)
                    .unwrap()
                    .save(bound.tenant, bound.host, AppId::Codex, &account.account_id, material)
                    .unwrap();
                Ok(())
            },
        )
        .await
        .unwrap();
    assert!(routes.is_empty());
    polled.await.unwrap();
    let sent = exchanged.await.unwrap();
    assert!(sent.starts_with("POST /oauth/token HTTP/1.1\r\n"));
    assert!(sent.contains("\r\ncontent-type: application/x-www-form-urlencoded\r\n"));
    let body = sent.split_once("\r\n\r\n").unwrap().1;
    assert_eq!(
        body,
        "grant_type=authorization_code&code=fixture-code&redirect_uri=https%3A%2F%2Fauth.openai.com%2Fdeviceauth%2Fcallback&client_id=app_EMoamEEZ73f0CkXaXp7hrann&code_verifier=fixture-verifier"
    );

    let projected = serde_json::to_string(&outcome).unwrap();
    for secret in [
        "fixture-private-device",
        "fixture-verifier",
        "fixture-code",
        "fixture-refresh",
        "fixture-workspace",
        &id_token,
        &access,
    ] {
        assert!(!projected.contains(secret));
    }
    let AuthFlow::Approved { account, .. } = &outcome else { panic!("expected approval") };
    let owner = store.for_owner(scope.principal).unwrap();
    let AccountCredential::CodexAuth(saved) =
        owner.read(&tenant, scope.host, AppId::Codex, &account.account_id).unwrap()
    else {
        panic!("expected Codex credential")
    };
    assert_eq!(saved["auth_mode"], "chatgpt");
    assert_eq!(
        saved["tokens"],
        json!({"access_token":access,"refresh_token":"fixture-refresh","id_token":id_token,"account_id":"fixture-workspace"})
    );
    assert!(saved["last_refresh"].as_str().is_some_and(|timestamp| timestamp.ends_with('Z')));
    assert!(
        store
            .for_owner("oidc:other")
            .unwrap()
            .read(&tenant, scope.host, AppId::Codex, &account.account_id)
            .is_err()
    );
    assert_eq!(flows.snapshot(scope, flow_id, Instant::now), Ok(outcome));
}

#[tokio::test]
async fn auth6_codex_device_rejects_invalid_or_stale_input() {
    let auth = CodexDeviceAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let (route, _) =
        oauth_server("200 OK", "", json!({"device_auth_id":"", "user_code":"ABCD-EFGH"})).await;
    let invalid = auth
        .start_with(&flows, scope, request(), |outbound| {
            route_to(outbound, "/api/accounts/deviceauth/usercode", route.clone());
        })
        .await;
    assert_eq!(
        invalid.err(),
        Some(AuthErrorCode::UpstreamFailure),
        "empty device grant must be rejected"
    );
    for request in [
        AuthStartRequest { method: AuthLoginMethod::Browser, ..request() },
        AuthStartRequest { target_account_id: Some("existing".into()), ..request() },
    ] {
        let unsupported = auth
            .start_with(&flows, scope, request, |_| panic!("unsupported request sent HTTP"))
            .await;
        assert_eq!(unsupported.err(), Some(AuthErrorCode::Unsupported));
    }

    let started = start(&auth, &flows, scope).await;
    let due = Instant::now() + Duration::from_secs(9);
    let foreign = AuthAccountScope { host: "other-host", ..scope };
    let result = auth
        .poll_with(
            &flows,
            foreign,
            &started,
            || due,
            |_| panic!("foreign poll sent HTTP"),
            |_, _, _, _| panic!("foreign poll persisted"),
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::NotFound));

    let (route, seen) = oauth_server(
        "403 Forbidden",
        "",
        json!({"error":"access_denied","error_description":"fixture-private-error"}),
    )
    .await;
    let denied = auth
        .poll_with(
            &flows,
            scope,
            &started,
            || due,
            |outbound| {
                route_to(outbound, "/api/accounts/deviceauth/token", route.clone());
            },
            |_, _, _, _| panic!("denial persisted"),
        )
        .await
        .unwrap();
    seen.await.unwrap();
    assert!(
        matches!(&denied, AuthFlow::Failed { error, .. } if error.code == AuthErrorCode::AuthorizationDenied)
    );
    assert!(!serde_json::to_string(&denied).unwrap().contains("fixture-private-error"));
    assert_eq!(
        auth.poll_with(
            &flows,
            scope,
            &started,
            || due,
            |_| panic!("terminal poll sent HTTP"),
            |_, _, _, _| panic!("terminal poll persisted")
        )
        .await,
        Ok(denied)
    );

    let started = start(&auth, &flows, scope).await;
    let (route, seen) = oauth_server("410 Gone", "", json!({})).await;
    let expired = auth
        .poll_with(
            &flows,
            scope,
            &started,
            || due + Duration::from_secs(8),
            |outbound| {
                route_to(outbound, "/api/accounts/deviceauth/token", route.clone());
            },
            |_, _, _, _| panic!("expired poll persisted"),
        )
        .await
        .unwrap();
    seen.await.unwrap();
    assert!(matches!(expired, AuthFlow::Expired { .. }));

    let valid_tokens = json!({
        "access_token": "fixture-access",
        "refresh_token": "fixture-refresh",
        "id_token": jwt(json!({"sub":"fixture-user", "chatgpt_account_id":"fixture-workspace"})),
    });
    let mut missing_refresh = valid_tokens.clone();
    missing_refresh.as_object_mut().unwrap().remove("refresh_token");
    let mut missing_subject = valid_tokens.clone();
    missing_subject["id_token"] = json!(jwt(json!({"chatgpt_account_id":"fixture-workspace"})));
    for (tokens, cancel) in
        [(missing_refresh, false), (missing_subject, false), (valid_tokens, true)]
    {
        let started = start(&auth, &flows, scope).await;
        let AuthStartResponse::Pending { flow_id, .. } = &started.response;
        let (poll, polled) = oauth_server(
            "200 OK",
            "",
            json!({"authorization_code":"fixture-code","code_verifier":"fixture-verifier"}),
        )
        .await;
        let (exchange, exchanged) = oauth_server("200 OK", "", tokens).await;
        let mut routes =
            VecDeque::from([("/api/accounts/deviceauth/token", poll), ("/oauth/token", exchange)]);
        let due = Instant::now() + Duration::from_secs(9);
        let result = auth
            .poll_with(
                &flows,
                scope,
                &started,
                || due,
                |outbound| {
                    let (path, route) = routes.pop_front().unwrap();
                    if cancel && path == "/oauth/token" {
                        flows.cancel(scope, flow_id, || due).unwrap();
                    }
                    route_to(outbound, path, route);
                },
                |_, _, _, _| panic!("incomplete or cancelled exchange persisted"),
            )
            .await
            .unwrap();
        polled.await.unwrap();
        exchanged.await.unwrap();
        if cancel {
            assert!(matches!(result, AuthFlow::Cancelled { .. }));
        } else {
            assert!(
                matches!(result, AuthFlow::Failed { error, .. } if error.code == AuthErrorCode::UpstreamFailure)
            );
        }
    }
}
