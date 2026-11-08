use super::{
    codex_browser::CodexBrowserStart,
    copilot_tests::oauth_server,
    credential_file::{AccountCredential, CredentialFileStore},
    flow::{AuthAccountScope, AuthFlows},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, StatusCode, Url};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
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
        method: AuthLoginMethod::Browser,
        github_domain: None,
        region: None,
        target_account_id: None,
    }
}

fn start(flows: &AuthFlows, scope: AuthAccountScope<'_>) -> CodexBrowserStart {
    let result =
        CodexBrowserStart::start_on(flows, scope, request(), true, 0, Duration::from_secs(60));
    assert_eq!(result.as_ref().err(), None, "local browser must start a real callback listener");
    result.unwrap()
}

fn client() -> Client {
    Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap()
}

fn parameters(start: &CodexBrowserStart) -> HashMap<String, String> {
    let AuthStartResponse::Pending { verification_url, user_code, can_paste, .. } = &start.response;
    assert!(user_code.is_none() && can_paste.is_none());
    let url = Url::parse(verification_url).unwrap();
    assert_eq!(url.origin().ascii_serialization(), "https://auth.openai.com");
    assert_eq!(url.path(), "/oauth/authorize");
    url.query_pairs().into_owned().collect()
}

fn callback_url(query: &HashMap<String, String>) -> Url {
    let mut url = Url::parse(&query["redirect_uri"]).unwrap();
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host_str(), Some("localhost"));
    assert_eq!(url.path(), "/auth/callback");
    url.set_host(Some("127.0.0.1")).unwrap();
    url
}

fn with_query(mut url: Url, pairs: [(&str, &str); 2]) -> Url {
    url.query_pairs_mut().extend_pairs(pairs);
    url
}

async fn raw_callback(url: &Url, request: &[u8]) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection =
            tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap())).await.unwrap();
        connection.write_all(request).await.unwrap();
        let mut response = String::new();
        connection.read_to_string(&mut response).await.unwrap();
        response
    })
    .await
    .expect("loopback request must finish within the test deadline")
}

#[tokio::test]
async fn auth6_codex_browser_commits_expected_scoped_result() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let started = start(&flows, scope);
    let query = parameters(&started);
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["originator"], "codex_cli_rs");
    assert_eq!(query["client_id"], "app_EMoamEEZ73f0CkXaXp7hrann");
    assert_eq!(
        query["scope"],
        "openid profile email offline_access api.connectors.read api.connectors.invoke"
    );
    assert!(!query.contains_key("code_verifier"));
    let client = client();
    let callback = callback_url(&query);
    let response = client
        .get(with_query(
            callback.clone(),
            [("state", query["state"].as_str()), ("code", "fixture+code")],
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert!(!response.text().await.unwrap().contains("fixture+code"));

    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    assert_eq!(
        started
            .poll_with(
                &flows,
                foreign,
                |_| panic!("foreign poll exchanged a code"),
                |_, _, _, _, _| panic!("foreign poll persisted"),
            )
            .await,
        Err(AuthErrorCode::NotFound)
    );

    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let claims = URL_SAFE_NO_PAD
        .encode(br#"{"sub":"fixture-user","chatgpt_account_id":"fixture-workspace"}"#);
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256"}"#);
    let id_token = format!("{header}.{claims}.signature");
    let (endpoint, received) = oauth_server("200 OK", "", json!({
        "access_token": "fixture-access", "refresh_token": "fixture-refresh", "id_token": id_token,
    })).await;
    let result = started
        .poll_with(
            &flows,
            scope,
            |outbound| {
                assert_eq!(outbound.url().as_str(), "https://auth.openai.com/oauth/token");
                let mut endpoint = endpoint.clone();
                endpoint.set_path("/oauth/token");
                *outbound.url_mut() = endpoint;
            },
            |bound, target_account_id, account, identity, material| {
                assert!(bound == scope);
                assert_eq!(target_account_id, None);
                assert_eq!(identity, r#"["fixture-workspace","fixture-user"]"#);
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

    let sent = received.await.unwrap();
    let (headers, body) = sent.split_once("\r\n\r\n").unwrap();
    assert!(headers.starts_with("POST /oauth/token HTTP/1.1"));
    assert!(headers.contains("\r\ncontent-type: application/x-www-form-urlencoded\r\n"));
    let form: HashMap<_, _> = Url::parse(&format!("https://fixture.invalid/?{body}"))
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect();
    assert_eq!(form.len(), 5);
    assert_eq!(form["code"], "fixture+code");
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["redirect_uri"], query["redirect_uri"]);
    assert_eq!(form["client_id"], query["client_id"]);
    assert_eq!(
        URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes())),
        query["code_challenge"]
    );
    assert!((43..=128).contains(&form["code_verifier"].len()));
    assert!(!serde_json::to_string(&started.response).unwrap().contains(&form["code_verifier"]));

    let AuthFlow::Approved { account, .. } = &result else { panic!("expected approval") };
    let AccountCredential::CodexAuth(saved) = store
        .for_owner(scope.principal)
        .unwrap()
        .read(&tenant, scope.host, AppId::Codex, &account.account_id)
        .unwrap()
    else {
        panic!("expected Codex credentials")
    };
    assert_eq!(saved["tokens"]["refresh_token"], "fixture-refresh");
    assert!(
        store
            .for_owner(foreign.principal)
            .unwrap()
            .read(&tenant, scope.host, AppId::Codex, &account.account_id)
            .is_err()
    );
    let projected = serde_json::to_string(&result).unwrap();
    for secret in
        ["fixture+code", "fixture-access", "fixture-refresh", &id_token, &form["code_verifier"]]
    {
        assert!(!projected.contains(secret));
    }
    assert!(client.get(callback).send().await.is_err());
    assert_eq!(
        started
            .poll_with(
                &flows,
                scope,
                |_| panic!("replayed exchange"),
                |_, _, _, _, _| { panic!("replayed commit") }
            )
            .await,
        Ok(result)
    );
}

#[tokio::test]
async fn auth6_codex_browser_rejects_invalid_or_stale_input() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = occupied.local_addr().unwrap().port();
    let lifetime = Duration::from_secs(60);
    assert_eq!(
        CodexBrowserStart::start_on(&flows, scope, request(), false, port, lifetime).err(),
        Some(AuthErrorCode::CallbackUnreachable),
        "a browser on another host must be rejected before binding"
    );
    assert_eq!(
        CodexBrowserStart::start_on(&flows, scope, request(), true, port, lifetime).err(),
        Some(AuthErrorCode::Conflict),
    );

    let started = start(&flows, scope);
    let query = parameters(&started);
    let client = client();
    let base = callback_url(&query);
    let state = &query["state"];
    for query in [
        "code=c".into(),
        "state=wrong&code=c".into(),
        format!("state={state}&state={state}&code=c"),
        format!("state={state}&code=c&code=d"),
        format!("state={state}&code=c&error=denied"),
        format!("state={state}&code="),
        format!("state={state}&code=%xx"),
    ] {
        let mut callback = base.clone();
        callback.set_query(Some(&query));
        assert_eq!(client.get(callback).send().await.unwrap().status(), StatusCode::BAD_REQUEST);
    }
    let mut wrong_path = base.clone();
    wrong_path.set_path("/other");
    assert_eq!(
        client
            .get(with_query(wrong_path, [("state", state.as_str()), ("code", "c")]))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .post(with_query(base.clone(), [("state", state.as_str()), ("code", "c")]))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::METHOD_NOT_ALLOWED,
    );
    let partial = b"GET /auth/callback HTTP/1.1\r\nHost: localhost\r\nX-Test: ";
    let mut oversized = partial.to_vec();
    oversized.resize(8192, b'x');
    assert!(raw_callback(&base, &oversized).await.starts_with("HTTP/1.1 431 "));
    assert!(raw_callback(&base, partial).await.starts_with("HTTP/1.1 408 "));
    let pending = started
        .poll_with(
            &flows,
            scope,
            |_| panic!("invalid callback exchanged a code"),
            |_, _, _, _, _| panic!("invalid callback persisted"),
        )
        .await
        .unwrap();
    assert!(matches!(pending, AuthFlow::Pending { .. }));

    let denied = client
        .get(with_query(
            base.clone(),
            [("state", state.as_str()), ("error", "fixture-private-denial")],
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::OK);
    assert!(!denied.text().await.unwrap().contains("fixture-private-denial"));
    assert_eq!(
        client
            .get(with_query(base.clone(), [("state", state.as_str()), ("code", "c")]))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let failed = started
        .poll_with(
            &flows,
            scope,
            |_| panic!("denial exchanged a code"),
            |_, _, _, _, _| panic!("denial persisted"),
        )
        .await
        .unwrap();
    assert!(
        matches!(failed, AuthFlow::Failed { error, .. } if error.code == AuthErrorCode::AuthorizationDenied)
    );

    let started =
        CodexBrowserStart::start_on(&flows, scope, request(), true, base.port().unwrap(), lifetime)
            .unwrap();
    let query = parameters(&started);
    assert_ne!(query["state"], *state);
    assert_eq!(
        client
            .get(with_query(base.clone(), [("state", state.as_str()), ("code", "c")]))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let AuthStartResponse::Pending { flow_id, .. } = &started.response;
    flows.cancel(scope, flow_id, Instant::now).unwrap();
    assert!(client.get(base).send().await.is_err());
    let cancelled = started
        .poll_with(
            &flows,
            scope,
            |_| panic!("cancelled flow exchanged a code"),
            |_, _, _, _, _| panic!("cancelled flow persisted"),
        )
        .await
        .unwrap();
    assert!(matches!(cancelled, AuthFlow::Cancelled { .. }));

    let expired =
        CodexBrowserStart::start_on(&flows, scope, request(), true, 0, Duration::from_millis(100))
            .unwrap();
    let expired_port = callback_url(&parameters(&expired)).port().unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if std::net::TcpListener::bind(("127.0.0.1", expired_port)).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("expired browser attempt must release its listener without a poll");
    let expired = expired
        .poll_with(
            &flows,
            scope,
            |_| panic!("expired flow exchanged a code"),
            |_, _, _, _, _| panic!("expired flow persisted"),
        )
        .await
        .unwrap();
    assert!(matches!(expired, AuthFlow::Expired { .. }));
}
