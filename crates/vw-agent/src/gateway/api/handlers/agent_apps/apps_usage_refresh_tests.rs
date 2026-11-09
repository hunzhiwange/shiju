use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use super::{
    apps_usage_refresh, lifecycle, usage_read, usage_refresh::UsageRefresh,
    usage_refresh_tests::Fixture,
};
use crate::gateway::middleware::tenant_scope_middleware;

const TOKEN: &str = "test-usage-refresh-owner-token-with-valid-length";
const OTHER_TOKEN: &str = "test-usage-refresh-other-token-with-valid-length";
const SELECTOR: &str = "projectId=project-a&targetHostId=local&appId=codex&providerId=ok";

fn selector() -> Value {
    json!({"projectId": "project-a", "targetHostId": "local", "appId": "codex",
           "providerId": "ok", "kind": "custom_script"})
}

fn router(fixture: &mut Fixture) -> Router {
    router_with_codex(fixture, None)
}

fn router_with_codex(fixture: &mut Fixture, codex: Option<reqwest::ClientBuilder>) -> Router {
    let principal = &mut fixture.policy["principals"][0];
    principal["token_sha256"] = json!(hex::encode(Sha256::digest(TOKEN)));
    let grant = &mut principal["grants"][0];
    grant["usage_read"] = json!([
        {"project_id": "project-a", "app_id": "codex", "provider_id": "ok"},
        {"project_id": "project-a", "app_id": "codex", "provider_id": "limited"}
    ]);
    if codex.is_some() {
        grant["usage_read"].as_array_mut().unwrap().push(json!({
            "project_id": "project-a", "app_id": "codex", "provider_id": "official",
            "account_id": "managed"
        }));
        grant["usage_refresh"].as_array_mut().unwrap().push(json!({
            "project_id": "project-a", "app_id": "codex", "provider_id": "official",
            "account_id": "managed", "origin": "https://chatgpt.com"
        }));
    }
    grant["usage_cancel"] = grant["usage_read"].clone();
    grant["usage_refresh_all"] = json!(["project-a"]);
    let mut other = principal.clone();
    other["id"] = json!("other");
    other["token_sha256"] = json!(hex::encode(Sha256::digest(OTHER_TOKEN)));
    fixture.policy["principals"].as_array_mut().unwrap().push(other);
    fixture.write_policy();

    let policy = Some(fixture.path.clone());
    let mut service = UsageRefresh::new(fixture.database.clone()).unwrap();
    if let Some(codex) = codex {
        Arc::get_mut(&mut service).unwrap().codex_client = Some(
            vw_agent_apps::usage::subscriptions::codex::ManagedCodexUsage::new(codex).unwrap(),
        );
    }
    Router::new()
        .nest(
            "/v1",
            apps_usage_refresh::router_with_policy(policy.clone())
                .merge(lifecycle::router_with_policy(policy.clone()))
                .merge(usage_read::router_with_policy(policy)),
        )
        .layer(Extension(service))
        .layer(middleware::from_fn(tenant_scope_middleware))
}

async fn request(
    router: &Router,
    method: &str,
    path: &str,
    company: &str,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        router.clone().oneshot(
            Request::builder()
                .method(method)
                .uri(format!("/v1{path}"))
                .header("x-platform-id", "test-platform")
                .header("x-company-id", company)
                .header("x-agent-apps-token", token)
                .header("content-type", "application/json")
                .body(if body.is_null() { Body::empty() } else { Body::from(body.to_string()) })
                .unwrap(),
        ),
    )
    .await
    .expect("refresh request timed out")
    .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 32 * 1024).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(!text.contains("fixture-only-key") && !text.contains("fixture-oauth-"));
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn owner(router: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    request(router, method, path, "test-company", TOKEN, body).await
}

async fn start(router: &Router, path: &str, body: Value) -> String {
    let (status, body) = owner(router, "POST", path, body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    body["operationId"].as_str().expect("opaque operation ID").into()
}

fn operation(id: &str, suffix: &str) -> String {
    format!("/agent-apps/operations/{id}{suffix}?targetHostId=local")
}

async fn terminal(router: &Router, id: &str) -> Value {
    for _ in 0..5 {
        let (status, body) = owner(router, "GET", &operation(id, "/events"), Value::Null).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        if body["phase"] == "terminal" {
            return body;
        }
    }
    panic!("refresh did not reach a terminal phase")
}

#[tokio::test]
async fn usage_usage_refresh_returns_expected_result() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    let id = start(&router, "/apps/usage/refresh", selector()).await;
    let (status, _) = owner(&router, "GET", &operation(&id, ""), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let event = terminal(&router, &id).await;
    assert_eq!(event["status"], "completed");
    assert_eq!(event["operationId"], id);
    assert_eq!(event["correlationId"], id);
    assert!(event["items"][0]["snapshotVersion"].is_string());
    assert_eq!(
        owner(&router, "DELETE", &operation(&id, ""), Value::Null).await.1["status"],
        "completed"
    );
    let (_, snapshot) =
        owner(&router, "GET", &format!("/apps/usage/snapshot?{SELECTOR}"), Value::Null).await;
    assert_eq!(
        snapshot["snapshot"]["result"]["outcome"]["data"]["value"][0]["remaining"].as_f64(),
        Some(17.0)
    );
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 1);
    let id = start(
        &router,
        "/apps/usage/refresh-all",
        json!({"projectId": "project-a", "targetHostId": "local"}),
    )
    .await;
    let event = terminal(&router, &id).await;
    assert_eq!(event["items"].as_array().unwrap().len(), 2);
    assert!(
        event["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["providerId"] == "limited" && item["status"] == "failed")
    );
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn usage_usage_refresh_rejects_invalid_input() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    let mut input = selector();
    input["providerId"] = json!("ungranted");
    assert_eq!(owner(&router, "POST", "/apps/usage/refresh", input).await.0, StatusCode::FORBIDDEN);

    let permit = fixture.release.acquire().await.unwrap();
    let id = start(&router, "/apps/usage/refresh", selector()).await;
    tokio::time::timeout(Duration::from_secs(5), fixture.started.notified()).await.unwrap();
    for (method, suffix) in [("GET", ""), ("GET", "/events"), ("DELETE", "")] {
        for (company, token) in [("foreign-company", TOKEN), ("test-company", OTHER_TOKEN)] {
            let (status, body) =
                request(&router, method, &operation(&id, suffix), company, token, Value::Null)
                    .await;
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert!(body.is_null());
        }
    }
    for _ in 0..2 {
        let (status, result) = owner(&router, "DELETE", &operation(&id, ""), Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(result["status"], "cancelled");
        assert_eq!(result["phase"], "terminal");
    }
    drop(permit);
    let (_, snapshot) =
        owner(&router, "GET", &format!("/apps/usage/snapshot?{SELECTOR}"), Value::Null).await;
    assert!(snapshot["snapshot"].is_null());
}

#[tokio::test]
async fn usage_usage_refresh_rechecks_read_permission_before_event_delivery() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    let permit = fixture.release.acquire().await.unwrap();
    let id = start(&router, "/apps/usage/refresh", selector()).await;
    tokio::time::timeout(Duration::from_secs(5), fixture.started.notified()).await.unwrap();

    fixture.policy["principals"][0]["grants"][0]["usage_cancel"] = json!([]);
    fixture.write_policy();
    assert_eq!(
        owner(&router, "DELETE", &operation(&id, ""), Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    let path = operation(&id, "/events");
    let poll = owner(&router, "GET", &path, Value::Null);
    tokio::pin!(poll);
    assert!(tokio::time::timeout(Duration::from_millis(150), &mut poll).await.is_err());

    fixture.policy["principals"][0]["grants"][0]["usage_read"] = json!([]);
    fixture.write_policy();
    drop(permit);
    let (status, body) = poll.await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.get("items").is_none());
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn usage_usage_refresh_publishes_only_current_managed_codex_credentials() {
    use std::os::unix::fs::DirBuilderExt;

    use crate::{
        agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
        app_providers::AppProviderStore,
    };
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use vw_agent_apps::{
        auth::credential_file::{AccountCredential, CredentialFileStore},
        providers::codex_catalog,
    };
    use vw_api_types::{
        TimestampMs, agent_apps::AppId, agent_apps_auth::AuthService, id::TenantScope,
    };

    let mut fixture = Fixture::new().await;
    let root = fixture.path.with_file_name("credentials");
    std::fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    fixture.policy["hosts"][0]["credential_root"] = json!(root);
    let tenant = TenantScope::new("test-platform", "test-company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "local",
        service: AuthService::CodexOauth,
    };
    let accounts = AuthAccountStore::new(fixture.database.clone(), &root).unwrap();
    let metadata = AuthAccountMetadata::new(r#"["workspace","subject"]"#, TimestampMs(1)).unwrap();
    accounts.insert_metadata(&scope, "managed", &metadata).unwrap();
    let id_token = format!(
        "{}.{}.signature",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#),
        URL_SAFE_NO_PAD.encode(br#"{"sub":"subject","chatgpt_account_id":"workspace"}"#)
    );
    let credentials = CredentialFileStore::new(&root).unwrap().for_owner("owner").unwrap();
    let save = |token: &str| {
        credentials
            .save(
                &tenant,
                "local",
                AppId::Codex,
                "managed",
                &AccountCredential::CodexAuth(json!({"auth_mode": "chatgpt",
                    "tokens": {"access_token": token, "refresh_token": "fixture-oauth-refresh",
                               "account_id": "workspace", "id_token": id_token}
                })),
            )
            .unwrap()
    };
    save("fixture-oauth-original");
    let providers = AppProviderStore::new(fixture.database.clone()).unwrap();
    let profile = codex_catalog::import(
        Some(b"model_provider='openai'\nmodel='fixture-model'\n"),
        Some(b"{}"),
    )
    .unwrap();
    providers.codex_save(&tenant, "official", &profile, None, None).unwrap();
    providers.bind_codex_account(scope, &root, "official", None, Some("managed")).unwrap();

    let (seen, mut requests) = tokio::sync::mpsc::unbounded_channel();
    let permits = Arc::new(tokio::sync::Semaphore::new(1));
    let router =
        router_with_codex(&mut fixture, Some(codex_http_fixture(seen, permits.clone()).await));
    let selector = json!({"projectId": "project-a", "targetHostId": "local", "appId": "codex",
        "providerId": "official", "accountId": "managed", "kind": "official_subscription"});
    let path = "/apps/usage/snapshot?projectId=project-a&targetHostId=local&appId=codex&providerId=official&accountId=managed&kind=official_subscription";
    let mut forged = selector.clone();
    forged["accountId"] = json!("another");
    assert_eq!(
        owner(&router, "POST", "/apps/usage/refresh", forged).await.0,
        StatusCode::FORBIDDEN
    );

    let id = start(&router, "/apps/usage/refresh", selector.clone()).await;
    let event = terminal(&router, &id).await;
    assert_eq!(event["status"], "completed", "{event}");
    assert_eq!(event["items"][0]["accountId"], "managed");
    let (_, snapshot) = owner(&router, "GET", path, Value::Null).await;
    assert_eq!(
        snapshot["snapshot"]["result"]["outcome"]["data"]["value"]["creditsBalance"].as_f64(),
        Some(42.5)
    );

    save("fixture-oauth-rotated");
    assert!(owner(&router, "GET", path, Value::Null).await.1["snapshot"].is_null());

    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), requests.recv()).await.unwrap().as_deref(),
        Some("Bearer fixture-oauth-original")
    );
    let permit = permits.acquire().await.unwrap();
    let id = start(&router, "/apps/usage/refresh", selector.clone()).await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), requests.recv()).await.unwrap().as_deref(),
        Some("Bearer fixture-oauth-rotated")
    );
    save("fixture-oauth-newer");
    drop(permit);
    let event = terminal(&router, &id).await;
    assert_eq!(event["status"], "failed");
    assert!(event["items"][0]["snapshotVersion"].is_null());
    assert!(owner(&router, "GET", path, Value::Null).await.1["snapshot"].is_null());

    let permit = permits.acquire().await.unwrap();
    let id = start(&router, "/apps/usage/refresh", selector).await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), requests.recv()).await.unwrap().as_deref(),
        Some("Bearer fixture-oauth-newer")
    );
    let (status, event) = owner(&router, "DELETE", &operation(&id, ""), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(event["status"], "cancelled");
    drop(permit);
    assert!(owner(&router, "GET", path, Value::Null).await.1["snapshot"].is_null());
    let id = start(
        &router,
        "/apps/usage/refresh-all",
        json!({"projectId": "project-a", "targetHostId": "local"}),
    )
    .await;
    let event = terminal(&router, &id).await;
    assert_eq!(event["items"].as_array().unwrap().len(), 3);
    assert!(
        event["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item["accountId"] == "managed" && item["status"] == "completed" })
    );
}

async fn codex_http_fixture(
    seen: tokio::sync::mpsc::UnboundedSender<String>,
    permits: Arc<tokio::sync::Semaphore>,
) -> reqwest::ClientBuilder {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    // Self-contained loopback TLS fixture. The private key is test material.
    let ca = STANDARD.decode(FIXTURE_CA).unwrap();
    let leaf = STANDARD.decode(FIXTURE_CERT).unwrap();
    let key = STANDARD.decode(FIXTURE_KEY).unwrap();
    let server = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![leaf.into()], rustls_pki_types::PrivateKeyDer::try_from(key).unwrap())
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let (acceptor, seen, permits) = (acceptor.clone(), seen.clone(), permits.clone());
            tokio::spawn(async move {
                let Ok(stream) = acceptor.accept(stream).await else { return };
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                stream.read_line(&mut line).await.unwrap();
                let quota = match line.as_str() {
                    "GET /backend-api/wham/usage HTTP/1.1\r\n" => true,
                    "GET /backend-api/wham/rate-limit-reset-credits HTTP/1.1\r\n" => false,
                    _ => panic!("unexpected Codex fixture request"),
                };
                let mut headers = std::collections::HashMap::new();
                loop {
                    line.clear();
                    if stream.read_line(&mut line).await.unwrap() == 0 {
                        return;
                    }
                    if line == "\r\n" {
                        break;
                    }
                    let (name, value) = line.split_once(':').unwrap();
                    headers.insert(name.to_ascii_lowercase(), value.trim().to_string());
                }
                assert!(headers["authorization"].starts_with("Bearer fixture-oauth-"));
                assert_eq!(headers["chatgpt-account-id"], "workspace");
                let body = if quota {
                    seen.send(headers["authorization"].clone()).unwrap();
                    let _permit = permits.acquire().await.unwrap();
                    r#"{"credits":{"has_credits":true,"unlimited":false,"balance":"42.5"}}"#
                } else {
                    r#"{"credits":[]}"#
                };
                let _ = stream.write_all(format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                ).as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    reqwest::Client::builder()
        .resolve("chatgpt.com", addr)
        .tls_certs_only([reqwest::Certificate::from_der(&ca).unwrap()])
}

const FIXTURE_CA: &str = concat!(
    "MIIBajCCARCgAwIBAgIUasVLE9HIy3BYqTfsGEUL+fGf5FgwCgYIKoZIzj0EAwIwGzEZMBcGA1UEAwwQVXNhZ2Ug",
    "Rml4dHVyZSBDQTAeFw0yNjExMDkyMTAwNTVaFw00NjExMDQyMTAwNTVaMBsxGTAXBgNVBAMMEFVzYWdlIEZpeHR1",
    "cmUgQ0EwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAARdpiUEDBkd5gK8wFSTu7x4mdVfnZUhOg/WqnQTnF+5WYRe",
    "HcVU+Gdq9ORcx+v5Z6vI8iRpiEXts8ZyMzzWf5OjozIwMDAdBgNVHQ4EFgQUMgbsEBot3AEf+W4R8tH0UBuRPMsw",
    "DwYDVR0TAQH/BAUwAwEB/zAKBggqhkjOPQQDAgNIADBFAiASrHqk1EPCvdd0x554ZRgpvp/z9Kwyg3ayVKQGzeRI",
    "lQIhAN4kPFQWuxuYtVwToQCz2KNghdaSceoKMbTC0Ff4tzOk",
);

const FIXTURE_CERT: &str = concat!(
    "MIIBbDCCAROgAwIBAgIUeKNLvlk9PPV9eA2nBYDQ+gcz6HEwCgYIKoZIzj0EAwIwGzEZMBcGA1UEAwwQVXNhZ2Ug",
    "Rml4dHVyZSBDQTAeFw0yNjExMDkyMTAwNTVaFw00NjExMDQyMTAwNTVaMBYxFDASBgNVBAMMC2NoYXRncHQuY29t",
    "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEvMy/uykRaUmLTE+iEC/WIvgBWR317uU6roMQeBly9a6tPlrc77GT",
    "jcd9AXAp+jHjSTZ8BJZCrbL7eTsNiuDp2aM6MDgwFgYDVR0RBA8wDYILY2hhdGdwdC5jb20wEwYDVR0lBAwwCgYI",
    "KwYBBQUHAwEwCQYDVR0TBAIwADAKBggqhkjOPQQDAgNHADBEAiAMlYFXxPIcSoxnFwgnngEnM+z8ydpFbvxNoI8x",
    "O/jTaAIgN/Q8VMCxXDJgciSUBSu58gSFonw97VxqC4ElCiX/tcE=",
);

const FIXTURE_KEY: &str = concat!(
    "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgsQFZ9KZSHniyxsr38ITt3FMXpOMAwu9/vlomqDsX",
    "CfChRANCAAS8zL+7KRFpSYtMT6IQL9Yi+AFZHfXu5TqugxB4GXL1rq0+WtzvsZONx30BcCn6MeNJNnwElkKtsvt5",
    "Ow2K4OnZ",
);
