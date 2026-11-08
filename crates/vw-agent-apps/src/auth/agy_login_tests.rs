use super::{
    agy_login::{AgyLogin, MANAGED_SERVICE},
    copilot_tests::oauth_server,
    flow::{AuthAccountScope, AuthFlows},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, StatusCode, Url};
use security_framework::os::macos::keychain::{SecKeychain, SecPreferencesDomain};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use uuid::Uuid;
use vw_api_types::{
    agent_apps_auth::{
        AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod, AuthService, AuthStartRequest,
        AuthStartResponse,
    },
    id::TenantScope,
};

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "test-host",
        service: AuthService::AgyOauth,
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

fn start(flows: &AuthFlows, scope: AuthAccountScope<'_>) -> AgyLogin {
    let result = AgyLogin::start(flows, scope, request(), true);
    assert_eq!(result.as_ref().err(), None, "local AGY login must start a loopback listener");
    result.unwrap()
}

fn parameters(started: &AgyLogin) -> (&AuthFlowId, HashMap<String, String>) {
    let AuthStartResponse::Pending { flow_id, verification_url, user_code, can_paste, .. } =
        &started.response;
    assert!(user_code.is_none() && can_paste.is_none());
    let url = Url::parse(verification_url).unwrap();
    assert_eq!(url.origin().ascii_serialization(), "https://accounts.google.com");
    assert_eq!(url.path(), "/o/oauth2/v2/auth");
    (flow_id, url.query_pairs().into_owned().collect())
}

fn callback(query: &HashMap<String, String>) -> Url {
    let url = Url::parse(&query["redirect_uri"]).unwrap();
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(url.path(), "/oauth-callback");
    assert!(url.port().is_some());
    url
}

fn with_query(mut url: Url, pairs: &[(&str, &str)]) -> Url {
    url.query_pairs_mut().extend_pairs(pairs.iter().copied());
    url
}

fn client() -> Client {
    Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap()
}

struct TestEntry {
    keychain: SecKeychain,
    id: String,
}

impl TestEntry {
    fn new(scope: AuthAccountScope<'_>, subject: &str) -> Self {
        let encoded = serde_json::to_vec(&(
            scope.tenant,
            scope.principal,
            scope.host,
            scope.service,
            subject,
        ))
        .unwrap();
        Self {
            keychain: SecKeychain::default_for_domain(SecPreferencesDomain::User).unwrap(),
            id: URL_SAFE_NO_PAD.encode(Sha256::digest(encoded)),
        }
    }

    fn assert_missing(&self) {
        assert_eq!(
            self.keychain.find_generic_password(MANAGED_SERVICE, &self.id).err().unwrap().code(),
            -25300,
        );
    }
}

impl Drop for TestEntry {
    fn drop(&mut self) {
        if let Ok((_, item)) = self.keychain.find_generic_password(MANAGED_SERVICE, &self.id) {
            item.delete();
        }
    }
}

#[tokio::test]
async fn auth6_agy_login_commits_expected_scoped_result() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", format!("agy-{}", Uuid::new_v4().simple())).unwrap();
    let scope = scope(&tenant);
    let entry = TestEntry::new(scope, "fixture-user");
    entry.assert_missing();
    let started = start(&flows, scope);
    let (flow_id, query) = parameters(&started);
    assert_ne!(flow_id.0, query["state"]);
    assert_eq!(
        query["client_id"],
        "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
    );
    assert_eq!(
        query["scope"],
        concat!(
            "openid https://www.googleapis.com/auth/cloud-platform ",
            "https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile ",
            "https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs ",
            "https://www.googleapis.com/auth/aicode"
        )
    );
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["access_type"], "offline");
    assert_eq!(query["prompt"], "select_account consent");
    assert_eq!(query["code_challenge_method"], "S256");
    assert!(!query.contains_key("code_verifier"));
    assert!(!query.contains_key("client_secret"));

    let redirect = callback(&query);
    let browser = client();
    let state = query["state"].as_str();
    let valid = with_query(redirect.clone(), &[("state", state), ("code", "fixture+code")]);
    let response = browser.get(valid.clone()).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert!(!response.text().await.unwrap().contains("fixture+code"));
    assert_eq!(browser.get(valid).send().await.unwrap().status(), StatusCode::CONFLICT);

    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    assert_eq!(
        started.poll_with(&flows, foreign, |_| panic!("foreign poll exchanged a code")).await,
        Err(AuthErrorCode::NotFound),
    );

    let claims = URL_SAFE_NO_PAD.encode(br#"{"sub":"fixture-user","email":"fixture@example.com"}"#);
    let signature = "s".repeat(350);
    let id_token = format!("e30.{claims}.{signature}");
    let access = format!("fixture-access-{}", "a".repeat(2100));
    let (mut endpoint, received) = oauth_server("200 OK", "", json!({
        "access_token": access, "refresh_token": "fixture-refresh", "id_token": id_token, "expires_in": 3599,
    })).await;
    endpoint.set_path("/token");
    let result = started
        .poll_with(&flows, scope, |outbound| {
            assert_eq!(outbound.url().as_str(), "https://oauth2.googleapis.com/token");
            *outbound.url_mut() = endpoint;
        })
        .await
        .unwrap();
    let AuthFlow::Approved { account, .. } = &result else {
        panic!("expected approval: {result:?}")
    };
    assert_eq!(account.account_id, entry.id);
    TestEntry::new(foreign, "fixture-user").assert_missing();
    assert_eq!(account.service, AuthService::AgyOauth);
    assert_eq!(account.label, "fixture@example.com");
    assert!(!account.is_default);
    let (saved, _) = entry.keychain.find_generic_password(MANAGED_SERVICE, &entry.id).unwrap();
    let saved: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(saved["account"]["account_id"], entry.id);
    assert_eq!(saved["credential"]["auth_method"], "consumer");
    assert_eq!(saved["credential"]["id_token"], id_token);
    assert_eq!(saved["credential"]["token"]["access_token"], access);
    assert_eq!(saved["credential"]["token"]["refresh_token"], "fixture-refresh");
    assert_eq!(saved["credential"]["token"]["token_type"], "Bearer");
    chrono::DateTime::parse_from_rfc3339(saved["credential"]["token"]["expiry"].as_str().unwrap())
        .unwrap();

    let sent = received.await.unwrap();
    let (headers, body) = sent.split_once("\r\n\r\n").unwrap();
    assert!(headers.starts_with("POST /token HTTP/1.1"));
    assert!(headers.contains("\r\ncontent-type: application/x-www-form-urlencoded\r\n"));
    let form: HashMap<_, _> = Url::parse(&format!("https://fixture.invalid/?{body}"))
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect();
    assert_eq!(form.len(), 6);
    assert_eq!(form["code"], "fixture+code");
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["client_id"], query["client_id"]);
    assert_eq!(form["redirect_uri"], query["redirect_uri"]);
    assert!(
        URL_SAFE_NO_PAD.encode(Sha256::digest(form["client_secret"].as_bytes()))
            == "HS8EEJP9laqJlaA4xxHVCnlg2gmlBTgcCadF1q0OzGA",
        "exchange uses the source installed-app client credential"
    );
    assert_eq!(
        URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes())),
        query["code_challenge"]
    );
    assert!((43..=128).contains(&form["code_verifier"].len()));
    let projected = serde_json::to_string(&result).unwrap();
    for secret in
        ["fixture+code", "fixture-access", "fixture-refresh", &id_token, &form["code_verifier"]]
    {
        assert!(!projected.contains(secret));
    }
    assert!(browser.get(redirect).send().await.is_err());
    assert_eq!(started.poll_with(&flows, scope, |_| panic!("replayed exchange")).await, Ok(result),);
    let id = entry.id.clone();
    drop(entry);
    assert_eq!(
        SecKeychain::default_for_domain(SecPreferencesDomain::User)
            .unwrap()
            .find_generic_password(MANAGED_SERVICE, &id)
            .err()
            .unwrap()
            .code(),
        -25300
    );
}

#[tokio::test]
async fn auth6_agy_login_rejects_invalid_or_stale_input() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", format!("agy-{}", Uuid::new_v4().simple())).unwrap();
    let scope = scope(&tenant);
    assert_eq!(
        AgyLogin::start(&flows, scope, request(), false).err(),
        Some(AuthErrorCode::CallbackUnreachable)
    );
    let started = start(&flows, scope);
    let (flow_id, query) = parameters(&started);
    let redirect = callback(&query);
    let browser = client();
    let state = query["state"].as_str();
    for pairs in [
        &[("state", "wrong"), ("code", "bogus")][..],
        &[("state", state), ("state", state), ("code", "bogus")],
        &[("state", state), ("code", "bogus"), ("error", "access_denied")],
    ] {
        assert_eq!(
            browser.get(with_query(redirect.clone(), pairs)).send().await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut wrong_path = redirect.clone();
    wrong_path.set_path("/favicon.ico");
    assert_eq!(browser.get(wrong_path).send().await.unwrap().status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        started.poll_with(&flows, scope, |_| panic!("invalid callback exchanged")).await,
        Ok(AuthFlow::Pending { .. })
    ));

    let valid = with_query(redirect.clone(), &[("state", state), ("code", "late-code")]);
    assert_eq!(browser.get(valid.clone()).send().await.unwrap().status(), StatusCode::OK);
    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    assert_eq!(flows.cancel(foreign, flow_id, Instant::now), Err(AuthErrorCode::NotFound));
    flows.cancel(scope, flow_id, Instant::now).unwrap();
    assert!(browser.get(valid).send().await.is_err());
    assert!(matches!(
        started.poll_with(&flows, scope, |_| panic!("cancelled callback exchanged")).await,
        Ok(AuthFlow::Cancelled { .. })
    ));

    let denied = start(&flows, scope);
    let (_, query) = parameters(&denied);
    let denied_url =
        with_query(callback(&query), &[("state", &query["state"]), ("error", "access_denied")]);
    assert_eq!(browser.get(denied_url).send().await.unwrap().status(), StatusCode::OK);
    assert!(matches!(
        denied.poll_with(&flows, scope, |_| panic!("denied callback exchanged")).await,
        Ok(AuthFlow::Failed { error, .. }) if error.code == AuthErrorCode::AuthorizationDenied
    ));

    let missing_refresh = start(&flows, scope);
    let (_, query) = parameters(&missing_refresh);
    let url = with_query(callback(&query), &[("state", &query["state"]), ("code", "no-refresh")]);
    assert_eq!(browser.get(url).send().await.unwrap().status(), StatusCode::OK);
    let claims = URL_SAFE_NO_PAD.encode(br#"{"sub":"partial-grant-user"}"#);
    let (endpoint, received) = oauth_server(
        "200 OK",
        "",
        json!({
            "access_token": "short-lived", "id_token": format!("e30.{claims}.signature"),
        }),
    )
    .await;
    let entry = TestEntry::new(scope, "partial-grant-user");
    assert!(matches!(
        missing_refresh.poll_with(&flows, scope, |outbound| *outbound.url_mut() = endpoint).await,
        Ok(AuthFlow::Failed { error, .. }) if error.code == AuthErrorCode::ReauthRequired
    ));
    assert!(received.await.unwrap().contains("code=no-refresh"));
    entry.assert_missing();

    let replaced = start(&flows, scope);
    let (replaced_id, replaced_query) = parameters(&replaced);
    let other = start(&flows, foreign);
    let (other_id, _) = parameters(&other);
    let exchanging = start(&flows, scope);
    let (exchanging_id, query) = parameters(&exchanging);
    assert!(matches!(
        flows.snapshot(scope, replaced_id, Instant::now),
        Ok(AuthFlow::Cancelled { .. })
    ));
    assert!(matches!(
        flows.snapshot(foreign, other_id, Instant::now),
        Ok(AuthFlow::Pending { .. })
    ));
    assert!(browser.get(callback(&replaced_query)).send().await.is_err());
    let valid =
        with_query(callback(&query), &[("state", &query["state"]), ("code", "late-exchange")]);
    assert_eq!(browser.get(valid).send().await.unwrap().status(), StatusCode::OK);
    let claims = URL_SAFE_NO_PAD.encode(br#"{"sub":"cancelled-user"}"#);
    let tokens = json!({
        "access_token": "discard-access", "refresh_token": "discard-refresh",
        "id_token": format!("e30.{claims}.signature"),
    })
    .to_string();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = Url::parse(&format!("http://{}/token", listener.local_addr().unwrap())).unwrap();
    let pending_entry = TestEntry::new(scope, "cancelled-user");
    let exchange = exchanging.poll_with(&flows, scope, |outbound| *outbound.url_mut() = endpoint);
    let server = async {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(stream.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(headers).unwrap();
        let size = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .unwrap()
            .parse()
            .unwrap();
        let mut form = vec![0; size];
        stream.read_exact(&mut form).await.unwrap();
        assert!(String::from_utf8(form).unwrap().contains("code=late-exchange"));
        flows.cancel(scope, exchanging_id, Instant::now).unwrap();
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{tokens}",
            tokens.len(),
        );
        stream.write_all(reply.as_bytes()).await.unwrap();
    };
    let (result, ()) =
        tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(exchange, server) })
            .await
            .expect("exchange and cancellation must finish");
    assert!(matches!(result, Ok(AuthFlow::Cancelled { .. })));
    pending_entry.assert_missing();
    flows.cancel(foreign, other_id, Instant::now).unwrap();

    let expired =
        AgyLogin::start_with_lifetime(&flows, scope, request(), true, Duration::from_millis(200))
            .unwrap();
    let (_, query) = parameters(&expired);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(browser.get(callback(&query)).send().await.is_err());
    assert!(matches!(
        expired.poll_with(&flows, scope, |_| panic!("expired callback exchanged")).await,
        Ok(AuthFlow::Expired { .. })
    ));
}
