use std::{sync::atomic::Ordering, time::Instant};

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use super::{usage_read, usage_refresh::UsageRefresh, usage_refresh_tests::Fixture};
use crate::gateway::middleware::tenant_scope_middleware;

const TOKEN: &str = "test-usage-reader-token-of-valid-length";
const SELECTOR: &str = "projectId=project-a&targetHostId=local&appId=codex&providerId=ok";

fn router(fixture: &mut Fixture, refresh: std::sync::Arc<UsageRefresh>) -> Router {
    fixture.policy["principals"][0]["token_sha256"] = json!(hex::encode(Sha256::digest(TOKEN)));
    fixture.policy["principals"][0]["grants"][0]["usage_read"] =
        json!([{"project_id": "project-a", "app_id": "codex", "provider_id": "ok"}]);
    fixture.write_policy();
    Router::new()
        .nest("/v1", usage_read::router_with_policy(Some(fixture.path.clone())))
        .layer(Extension(refresh))
        .layer(middleware::from_fn(tenant_scope_middleware))
}

async fn read(router: &Router, company: &str, query: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/apps/usage/snapshot?{query}"))
                .header("x-platform-id", "test-platform")
                .header("x-company-id", company)
                .header("x-agent-apps-token", TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let status = response.status();
    let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    assert!(!String::from_utf8_lossy(&body).contains("fixture-only-key"));
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn usage_usage_read_returns_expected_result() {
    let mut fixture = Fixture::new().await;
    let refresh = UsageRefresh::new(fixture.database.clone()).unwrap();
    let router = router(&mut fixture, refresh.clone());

    let (status, body) = read(&router, "test-company", SELECTOR).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["capabilities"], json!(["custom_script"]));
    assert!(body["snapshot"].is_null());
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);

    assert_eq!(refresh.tick(&fixture.path, Instant::now()).await.len(), 2);
    let (status, body) = read(&router, "test-company", SELECTOR).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["snapshot"]["result"]["outcome"]["data"]["value"][0]["remaining"].as_f64(),
        Some(17.0)
    );
    assert_eq!(body["snapshot"]["stale"], false);
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 2);

    fixture.save("ok", true, 2);
    let (status, body) = read(&router, "test-company", SELECTOR).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["snapshot"].is_null());
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn usage_usage_read_rejects_invalid_input() {
    let mut fixture = Fixture::new().await;
    let refresh = UsageRefresh::new(fixture.database.clone()).unwrap();
    let router = router(&mut fixture, refresh);

    for (company, query, expected) in [
        ("another-company", SELECTOR.into(), StatusCode::FORBIDDEN),
        (
            "test-company",
            SELECTOR.replace("targetHostId=local", "targetHostId=foreign"),
            StatusCode::FORBIDDEN,
        ),
        ("test-company", SELECTOR.replace("project-a", "project-b"), StatusCode::FORBIDDEN),
        (
            "test-company",
            SELECTOR.replace("providerId=ok", "providerId=limited"),
            StatusCode::FORBIDDEN,
        ),
        ("test-company", format!("{SELECTOR}&accountId=forged"), StatusCode::FORBIDDEN),
        ("test-company", format!("{SELECTOR}&apiKey=bogus"), StatusCode::BAD_REQUEST),
    ] {
        let (status, body) = read(&router, company, &query).await;
        assert_eq!(status, expected);
        assert!(body["correlation_id"].is_string());
    }
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn usage_usage_read_requires_current_scoped_provider_binding() {
    use std::os::unix::fs::PermissionsExt;

    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use vw_agent_apps::{
        auth::credential_file::{AccountCredential, CredentialFileStore},
        providers::codex_catalog,
    };
    use vw_api_types::{
        TimestampMs, agent_apps::AppId, agent_apps_auth::AuthService, id::TenantScope,
    };

    use crate::{
        agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
        app_providers::AppProviderStore,
    };

    let mut fixture = Fixture::new().await;
    let refresh = UsageRefresh::new(fixture.database.clone()).unwrap();
    let router = router(&mut fixture, refresh);
    let root = fixture.path.with_file_name("credentials");
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    fixture.policy["hosts"][0]["credential_root"] = json!(root);
    fixture.policy["principals"][0]["grants"][0]["usage_read"] = json!([
        {"project_id": "project-a", "app_id": "codex", "provider_id": "official", "account_id": "managed"},
        {"project_id": "project-a", "app_id": "codex", "provider_id": "official", "account_id": "other"}
    ]);
    fixture.write_policy();

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
    accounts.insert_metadata(&scope, "other", &metadata).unwrap();
    let id_token = format!(
        "{}.{}.signature",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#),
        URL_SAFE_NO_PAD.encode(br#"{"sub":"subject","chatgpt_account_id":"workspace"}"#)
    );
    let credential = AccountCredential::CodexAuth(json!({
        "auth_mode": "chatgpt",
        "tokens": {"access_token": "fixture-only-key", "refresh_token": "fixture-refresh",
                   "account_id": "workspace", "id_token": id_token}
    }));
    CredentialFileStore::new(&root)
        .unwrap()
        .for_owner(scope.principal)
        .unwrap()
        .lease(&tenant, scope.host, AppId::Codex, "managed", true)
        .unwrap()
        .save(&credential)
        .unwrap();
    let providers = AppProviderStore::new(fixture.database.clone()).unwrap();
    let profile = codex_catalog::import(
        Some(b"model_provider='openai'\nmodel='fixture-model'\n"),
        Some(b"{}"),
    )
    .unwrap();
    providers.codex_save(&tenant, "official", &profile, None, None).unwrap();
    providers.bind_codex_account(scope, &root, "official", None, Some("managed")).unwrap();

    let selector = SELECTOR.replace("providerId=ok", "providerId=official");
    let managed = format!("{selector}&accountId=managed&kind=official_subscription");
    let (status, body) = read(&router, "test-company", &managed).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["capabilities"], json!(["official_subscription"]));
    assert!(body["snapshot"].is_null());
    assert_eq!(
        read(&router, "test-company", &format!("{selector}&accountId=other")).await.0,
        StatusCode::NOT_FOUND,
    );
    providers.bind_codex_account(scope, &root, "official", Some("managed"), None).unwrap();
    assert_eq!(read(&router, "test-company", &managed).await.0, StatusCode::NOT_FOUND);
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);
}
