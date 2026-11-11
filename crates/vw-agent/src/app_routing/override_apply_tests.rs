use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{RoutingListener, SharedTransport, dispatch::prepare_request};
use axum::{
    Json, Router,
    extract::Path,
    http::{HeaderMap, StatusCode, header},
    routing::post,
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc, time::timeout};
use vw_agent_apps::providers::request_options;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::validation::ProviderRequestOptions,
    id::{ProjectScope, TenantScope},
};

fn binding() -> VerifiedRouteBinding {
    VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Codex,
        account: "account".into(),
        account_generation: 1,
    }
}

fn target(url: &str, authorization: &'static str, options: ProviderRequestOptions) -> JsonTarget {
    JsonTarget {
        #[cfg(target_os = "macos")]
        managed: None,
        url: url.parse().unwrap(),
        authorization: authorization.parse().unwrap(),
        transport: SharedTransport::new().unwrap(),
        request_options: options,
    }
}

fn fixture_options() -> ProviderRequestOptions {
    let options = serde_json::from_value(json!({
        "localProxyRequestOverrides": {
            "headers": {" X-Provider-Flag ": "provider-only"},
            "body": {
                "model": "provider-model",
                "metadata": {"changed": null, "_private": "must not reach upstream", "stream": true},
                "stop": ["provider-stop"]
            }
        }
    }))
    .unwrap();
    let stored = request_options::update(AppId::Codex, &json!({"codex": {}}), &options).unwrap();
    request_options::read(AppId::Codex, &stored).unwrap()
}

#[tokio::test]
async fn cc_route_015_override_apply_expected_behavior() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_base = format!("http://{}", upstream.local_addr().unwrap());
    let (requests, mut received) = mpsc::unbounded_channel();
    let app = Router::new().route(
        "/{provider}",
        post(move |Path(provider): Path<String>, headers: HeaderMap, Json(body): Json<Value>| async move {
            let status = if provider == "first" {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::OK
            };
            requests.send((provider, headers, body)).unwrap();
            (status, Json(json!({"id": "fixture"})))
        }),
    );
    let server = tokio::spawn(async move { axum::serve(upstream, app).await.unwrap() });
    let listener = RoutingListener::default();
    let chat = format!("http://{}/v1/chat/completions", listener.start(0).unwrap());
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let input = json!({
        "model": "client-model",
        "metadata": {"changed": true, "keep": 1},
        "stop": ["client-stop", "other-stop"],
        "stream": false
    });

    let first = listener
        .auth
        .issue_with_target(
            binding(),
            Some(target(
                &format!("{upstream_base}/first"),
                "Bearer first-fixture",
                fixture_options(),
            )),
        )
        .unwrap();
    let request = |token: &str| {
        client
            .post(&chat)
            .header("x-api-key", token)
            .header("x-provider-flag", "client-flag")
            .header("chatgpt-account-id", "client-account")
            .json(&input)
    };
    assert_eq!(request(&first.token).send().await.unwrap().status(), StatusCode::BAD_GATEWAY);
    let (provider, headers, body) =
        timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
    assert_eq!(provider, "first");
    assert_eq!(headers[header::AUTHORIZATION], "Bearer first-fixture");
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    assert!(!headers.contains_key("x-api-key"));
    assert!(!headers.contains_key("chatgpt-account-id"));
    assert_eq!(
        body,
        json!({
            "model": "provider-model",
            "metadata": {"changed": null, "keep": 1, "stream": true},
            "stop": ["provider-stop"],
            "stream": false
        })
    );
    assert_eq!(headers["x-provider-flag"], "provider-only");

    let second = listener
        .auth
        .issue_with_target(
            binding(),
            Some(target(
                &format!("{upstream_base}/second"),
                "Bearer second-fixture",
                ProviderRequestOptions::default(),
            )),
        )
        .unwrap();
    assert_eq!(request(&first.token).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
    assert_eq!(request(&second.token).send().await.unwrap().status(), StatusCode::OK);
    let (provider, headers, body) =
        timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
    assert_eq!(provider, "second");
    assert_eq!(headers[header::AUTHORIZATION], "Bearer second-fixture");
    assert!(!headers.contains_key("x-provider-flag"));
    assert_eq!(body, input);
    assert!(received.try_recv().is_err());
    listener.stop().await.unwrap();
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}

#[test]
fn cc_route_015_preparation_leaves_the_cached_request_intact() {
    // Native Chat uses the parsed client document as its converted document.
    // Both attempts consume the very same parsed value.
    let cached = json!({"model": "original", "metadata": {"_private": true}});
    let first = target("http://127.0.0.1:1", "Bearer first-fixture", fixture_options());
    let second =
        target("http://127.0.0.1:1", "Bearer second-fixture", ProviderRequestOptions::default());
    let body = |target| {
        let request = prepare_request(&cached, target, None).unwrap();
        serde_json::from_slice::<Value>(request.body().unwrap().as_bytes().unwrap()).unwrap()
    };
    assert_eq!(body(&first)["model"], "provider-model");
    assert_eq!(body(&second), json!({"model": "original", "metadata": {}}));
    assert_eq!(cached, json!({"model": "original", "metadata": {"_private": true}}));
}

#[test]
fn cc_route_015_rejects_unsafe_options_without_rotating_the_active_route() {
    let listener = RoutingListener::default();
    let active = listener.auth.issue(binding()).unwrap();
    for overrides in [
        json!({"headers": {" Authorization ": "Bearer unsafe-fixture"}}),
        json!({"headers": {"content-type": "text/plain"}}),
        json!({"headers": {"X-Fixture": "first", "x-fixture": "second"}}),
        json!({"body": {"stream": true}}),
    ] {
        let options =
            serde_json::from_value(json!({"localProxyRequestOverrides": overrides})).unwrap();
        let rejected = listener.auth.issue_with_target(
            binding(),
            Some(target("http://127.0.0.1:1", "Bearer fixture", options)),
        );
        assert!(rejected.is_err());
    }
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", active.token.parse().unwrap());
    assert!(listener.auth.authorize(&headers).is_some());
}

#[tokio::test]
async fn cc_route_016_user_agent_reaches_authorized_upstream() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/fixture", upstream.local_addr().unwrap());
    let (requests, mut received) = mpsc::unbounded_channel();
    let app = Router::new().route(
        "/fixture",
        post(move |headers: HeaderMap| async move {
            requests.send(headers).unwrap();
            Json(json!({"id": "fixture"}))
        }),
    );
    let server = tokio::spawn(async move { axum::serve(upstream, app).await.unwrap() });
    let listener = RoutingListener::default();
    let chat = format!("http://{}/v1/chat/completions", listener.start(0).unwrap());
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();

    for (raw, expected) in [
        ("\u{3000}\t ", None),
        ("\u{3000}fixture/1.0\t中文\u{3000}", Some("fixture/1.0\t中文".as_bytes())),
    ] {
        // Exercise already persisted request options, including legacy blank values.
        let row = json!({
            "codex": {},
            "requestOptions": {
                "localProxyRequestOverrides": {"headers": {" User-Agent ": raw}},
            },
        });
        let options = request_options::read(AppId::Codex, &row).unwrap();
        let grant = listener
            .auth
            .issue_with_target(binding(), Some(target(&url, "Bearer fixture", options)))
            .unwrap();
        let mut request =
            client.post(&chat).header("x-api-key", grant.token).json(&json!({"model": "fixture"}));
        if expected.is_some() {
            request = request.header(header::USER_AGENT, "incoming-fixture");
        }
        let response = request.send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let headers = timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
        let mut values = headers.get_all(header::USER_AGENT).iter();
        assert_eq!(values.next().map(|value| value.as_bytes()), expected);
        assert!(values.next().is_none());
    }

    listener.stop().await.unwrap();
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}
