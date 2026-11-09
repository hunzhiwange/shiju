use std::{sync::atomic::Ordering, time::Duration};

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use super::{apps_usage_script, usage_refresh::UsageRefresh, usage_refresh_tests::Fixture};
use crate::gateway::middleware::tenant_scope_middleware;

const TOKEN: &str = "test-usage-script-owner-token-with-valid-length";
const SELECTOR: &str = "projectId=project-a&targetHostId=local&appId=codex&providerId=ok";
const KEY_REF: &str = "c3ed9a3e-98a5-466d-a607-1bbf5e5345ad";

fn selection() -> Value {
    json!({"projectId": "project-a", "targetHostId": "local", "appId": "codex", "providerId": "ok"})
}

fn router(fixture: &mut Fixture) -> Router {
    let principal = &mut fixture.policy["principals"][0];
    principal["token_sha256"] = json!(hex::encode(Sha256::digest(TOKEN)));
    let grant = &mut principal["grants"][0];
    let resource = json!({"project_id": "project-a", "app_id": "codex", "provider_id": "ok"});
    let credentialed = json!({"resource": resource, "api_key_ref": KEY_REF});
    let mut trial = credentialed.clone();
    trial["origin"] = grant["usage_refresh"][0]["origin"].clone();
    grant["usage_read"] = json!([resource.clone()]);
    grant["usage_settings_read"] = json!([resource]);
    grant["usage_settings_write"] = json!([credentialed]);
    grant["usage_script_test"] = json!([trial]);
    fixture.write_policy();

    let service = UsageRefresh::new(fixture.database.clone()).unwrap();
    Router::new()
        .nest(
            "/v1",
            apps_usage_script::router_with_policy(Some(fixture.path.clone()))
                .merge(super::usage_read::router_with_policy(Some(fixture.path.clone()))),
        )
        .layer(Extension(service))
        .layer(middleware::from_fn(tenant_scope_middleware))
}

async fn request(
    router: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/v1/apps/usage/{path}"))
        .header("x-platform-id", "test-platform")
        .header("x-company-id", "test-company")
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("x-agent-apps-token", token);
    }
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        router.clone().oneshot(
            request
                .body(if body.is_null() { Body::empty() } else { Body::from(body.to_string()) })
                .unwrap(),
        ),
    )
    .await
    .expect("usage request timed out")
    .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    assert!(!String::from_utf8_lossy(&body).contains("fixture-only-key"));
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn owner(router: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    request(router, method, path, Some(TOKEN), body).await
}

fn settings(minutes: u16) -> Value {
    json!({"selection": selection(), "settings": {
        "enabled": true, "template": "general", "refreshIntervalMinutes": minutes,
        "display": "used", "credentials": {"apiKeyRef": KEY_REF}
    }})
}

fn trial() -> Value {
    json!({"selection": selection(), "template": "general", "credentials": {"apiKeyRef": KEY_REF}})
}

#[tokio::test]
async fn usage_usage_script_returns_expected_result() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    let settings_path = format!("settings?{SELECTOR}");
    let (status, templates) =
        owner(&router, "GET", &format!("templates?{SELECTOR}"), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let general =
        templates.as_array().unwrap().iter().find(|row| row["template"] == "general").unwrap();
    assert!(general["source"].as_str().unwrap().contains("{{apiKey}}"));
    let (status, initial) = owner(&router, "GET", &settings_path, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(initial["refreshIntervalMinutes"], 1);
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);
    let (status, tested) = owner(&router, "POST", "script-test", trial()).await;
    assert_eq!(status, StatusCode::OK, "{tested}");
    assert_eq!(tested[0]["remaining"].as_f64(), Some(17.0));
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 1);
    assert_eq!(owner(&router, "GET", &settings_path, Value::Null).await.1, initial);
    let (status, snapshot) =
        owner(&router, "GET", &format!("snapshot?{SELECTOR}"), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(snapshot, json!({"capabilities": ["custom_script"], "snapshot": null}));

    assert_eq!(owner(&router, "PUT", "settings", settings(22)).await.0, StatusCode::NO_CONTENT);
    let (status, saved) = owner(&router, "GET", &settings_path, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved, settings(22)["settings"]);
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn usage_usage_script_rejects_invalid_input() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    for (method, path, body) in [
        ("GET", format!("templates?{SELECTOR}"), Value::Null),
        ("GET", format!("settings?{SELECTOR}"), Value::Null),
        ("PUT", "settings".into(), settings(22)),
        ("POST", "script-test".into(), trial()),
    ] {
        assert_eq!(request(&router, method, &path, None, body).await.0, StatusCode::UNAUTHORIZED);
    }

    assert_eq!(owner(&router, "PUT", "settings", settings(1441)).await.0, StatusCode::BAD_REQUEST);
    let mut forged = settings(22);
    forged["settings"]["credentials"]["apiKeyRef"] = json!("751fdd6c-fb8a-4c4c-a9c8-81fd9e67557b");
    assert_eq!(owner(&router, "PUT", "settings", forged).await.0, StatusCode::FORBIDDEN);
    let mut command = trial();
    command["command"] = json!("get_provider_api_key");
    assert_eq!(owner(&router, "POST", "script-test", command).await.0, StatusCode::BAD_REQUEST);
    let mut raw_key = trial();
    raw_key["credentials"]["apiKey"] = json!("fixture-only-key");
    assert_eq!(owner(&router, "POST", "script-test", raw_key).await.0, StatusCode::BAD_REQUEST);
    let mut draft = trial();
    draft["source"] =
        json!("({ request: {}, extractor: () => ({ planName: '{{apiKey}}', remaining: 0 }) })");
    assert_eq!(owner(&router, "POST", "script-test", draft).await.0, StatusCode::BAD_REQUEST);

    let mut other = trial();
    other["selection"]["providerId"] = json!("limited");
    assert_eq!(owner(&router, "POST", "script-test", other).await.0, StatusCode::FORBIDDEN);

    let mut denied = trial();
    denied["template"] = json!("custom");
    denied.as_object_mut().unwrap().remove("credentials");
    denied["source"] = json!(
        "({ request: {url: 'http://127.0.0.1:1', method: 'GET'}, extractor: () => ({remaining: 0}) })"
    );
    assert_eq!(owner(&router, "POST", "script-test", denied).await.0, StatusCode::FORBIDDEN);

    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);
    let (_, unchanged) = owner(&router, "GET", &format!("settings?{SELECTOR}"), Value::Null).await;
    assert_eq!(unchanged["refreshIntervalMinutes"], 1);
}

#[tokio::test]
async fn usage_usage_script_runs_custom_draft_without_provider_credentials() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    fixture.policy["principals"][0]["grants"][0]["usage_script_test"][0]["origin"] = json!(origin);
    fixture.write_policy();
    let upstream = tokio::spawn(async move {
        let app = Router::new().route(
            "/draft",
            axum::routing::post(|headers: axum::http::HeaderMap, body: String| async move {
                assert!(!headers.contains_key("authorization"));
                assert_eq!(headers["x-fixture"], "draft");
                assert_eq!(body, "check");
                axum::Json(json!({"balance": 8, "unused": "raw-fixture-response"}))
            }),
        );
        axum::serve(listener, app).await.unwrap();
    });
    let (status, result) = owner(
        &router,
        "POST",
        "script-test",
        json!({
            "selection": selection(),
            "template": "custom",
            "source": format!(
                "({{request: {{url: '{origin}/draft', method: 'POST', headers: {{'X-Fixture': 'draft'}}, body: 'check'}}, extractor: r => ({{remaining: r.balance}})}})"
            ),
        }),
    )
    .await;
    upstream.abort();
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result[0]["remaining"].as_f64(), Some(8.0));
    assert!(!result.to_string().contains("raw-fixture-response"));
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn usage_usage_script_revokes_in_flight_trial_without_revoking_settings_read() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    let upstream_gate = fixture.release.acquire().await.unwrap();
    let pending = tokio::spawn({
        let router = router.clone();
        async move { owner(&router, "POST", "script-test", trial()).await }
    });
    tokio::time::timeout(Duration::from_secs(3), fixture.started.notified())
        .await
        .expect("trial never contacted upstream");
    fixture.policy["principals"][0]["grants"][0]["usage_script_test"] = json!([]);
    fixture.policy["principals"][0]["grants"][0]["usage_settings_write"] = json!([]);
    fixture.write_policy();
    assert_eq!(pending.await.unwrap().0, StatusCode::FORBIDDEN);
    drop(upstream_gate);

    assert_eq!(owner(&router, "PUT", "settings", settings(22)).await.0, StatusCode::FORBIDDEN);
    assert_eq!(owner(&router, "POST", "script-test", trial()).await.0, StatusCode::FORBIDDEN);
    let (status, saved) = owner(&router, "GET", &format!("settings?{SELECTOR}"), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["refreshIntervalMinutes"], 1);
}

async fn set_native_provider(fixture: &Fixture, base_url: &str) {
    let providers = crate::gateway::api::handlers::app_providers::router()
        .layer(Extension(fixture.database.clone()))
        .layer(middleware::from_fn(tenant_scope_middleware));
    let request = Request::builder()
        .method("PUT")
        .uri("/app-providers/codex/ok")
        .header("content-type", "application/json")
        .header("x-platform-id", "test-platform")
        .header("x-company-id", "test-company")
        .body(Body::from(
            json!({
                "name": "Native fixture",
                "protocol": "openai_responses",
                "apiKey": "fixture-only-key",
                "baseUrl": base_url,
                "model": "fixture-model",
            })
            .to_string(),
        ))
        .unwrap();
    assert_eq!(providers.oneshot(request).await.unwrap().status(), StatusCode::OK);
}

#[tokio::test]
async fn usage_usage_script_round_trips_native_settings_with_scoped_signing_refs() {
    let mut fixture = Fixture::new().await;
    let router = router(&mut fixture);
    let settings_path = format!("settings?{SELECTOR}");
    set_native_provider(&fixture, "https://open.bigmodel.cn/api/paas/v4").await;
    let mut team = json!({"selection": selection(), "settings": {
        "template": "token_plan",
        "enabled": true,
        "refreshIntervalMinutes": 15,
        "display": "used",
        "credentials": {},
        "codingPlan": {"kind": "zhipu_team", "organizationId": "org-a", "projectId": ""}
    }});
    assert_eq!(owner(&router, "PUT", "settings", team.clone()).await.0, StatusCode::BAD_REQUEST);
    team["settings"]["codingPlan"]["projectId"] = json!("project-team");
    assert_eq!(owner(&router, "PUT", "settings", team.clone()).await.0, StatusCode::NO_CONTENT);
    assert_eq!(owner(&router, "GET", &settings_path, Value::Null).await.1, team["settings"]);

    set_native_provider(&fixture, "https://ark.cn-beijing.volces.com/api/plan/v3").await;
    let mut volcengine = json!({"selection": selection(), "settings": {
        "template": "token_plan",
        "enabled": true,
        "refreshIntervalMinutes": 0,
        "display": "left",
        "credentials": {},
        "codingPlan": {"kind": "volcengine"}
    }});
    assert_eq!(
        owner(&router, "PUT", "settings", volcengine.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    volcengine["settings"]["enabled"] = json!(false);
    assert_eq!(
        owner(&router, "PUT", "settings", volcengine.clone()).await.0,
        StatusCode::NO_CONTENT
    );

    let signing = json!({
        "accessKeyIdRef": "a0a0a0a0-0000-4000-8000-000000000001",
        "secretAccessKeyRef": "a0a0a0a0-0000-4000-8000-000000000002"
    });
    volcengine["settings"]["credentials"]["signing"] = signing.clone();
    volcengine["settings"]["enabled"] = json!(true);
    assert_eq!(
        owner(&router, "PUT", "settings", volcengine.clone()).await.0,
        StatusCode::FORBIDDEN
    );

    let permit = &mut fixture.policy["principals"][0]["grants"][0]["usage_settings_write"][0];
    permit["signing"] = signing;
    fixture.write_policy();
    let mut mixed = volcengine.clone();
    mixed["settings"]["credentials"]["signing"]["secretAccessKeyRef"] =
        json!("a0a0a0a0-0000-4000-8000-000000000003");
    assert_eq!(owner(&router, "PUT", "settings", mixed).await.0, StatusCode::FORBIDDEN);
    assert_eq!(
        owner(&router, "PUT", "settings", volcengine.clone()).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(owner(&router, "GET", &settings_path, Value::Null).await.1, volcengine["settings"]);
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);
}
