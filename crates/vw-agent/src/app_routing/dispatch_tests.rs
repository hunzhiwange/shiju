use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{RoutingListener, SharedTransport};
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::post,
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc, time::timeout};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn cc_route_014_listener_filters_the_authorized_outgoing_json() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/fixture", upstream.local_addr().unwrap()).parse().unwrap();
    let (requests, mut received) = mpsc::unbounded_channel();
    let upstream_app = Router::new().route(
        "/fixture",
        post(move |headers: HeaderMap, Json(body): Json<Value>| async move {
            requests.send((headers, body)).unwrap();
            ([(header::SET_COOKIE, "fixture=private")], Json(json!({"id": "upstream-response"})))
                .into_response()
        }),
    );
    let upstream_task = tokio::spawn(async move {
        axum::serve(upstream, upstream_app).await.unwrap();
    });

    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let base = format!("http://{address}");
    let chat = format!("{base}/v1/chat/completions");
    let binding = VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Codex,
        account: "account".into(),
        account_generation: 1,
    };
    let unready = listener.auth.issue(binding.clone()).unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    assert_eq!(
        client
            .post(&chat)
            .bearer_auth(&unready.token)
            .body("not parsed")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE,
    );

    let active = listener
        .auth
        .issue_with_target(
            binding,
            Some(JsonTarget {
                url: target,
                authorization: "Bearer provider-fixture".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            }),
        )
        .unwrap();
    assert_eq!(
        client.post(&chat).bearer_auth(&unready.token).send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED,
    );
    for (builder, status) in [
        (client.get(&chat).bearer_auth(&active.token), StatusCode::METHOD_NOT_ALLOWED),
        (
            client.post(format!("{base}/unsupported")).bearer_auth(&active.token),
            StatusCode::NOT_FOUND,
        ),
        (
            client.post(&chat).bearer_auth(&active.token).json(&json!({"stream": "true"})),
            StatusCode::BAD_REQUEST,
        ),
        (
            client.post(&chat).bearer_auth(&active.token).body("untyped"),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ] {
        assert_eq!(builder.send().await.unwrap().status(), status);
    }
    assert!(received.try_recv().is_err());

    let response = client
        .post(&chat)
        .bearer_auth(&active.token)
        .header("x-upstream-url", "http://example.invalid")
        .json(&json!({
            "model": "selected",
            "_credential": "fixture",
            "metadata": {"properties": {"_private": true}},
            "tools": [{"type": "function", "function": {"name": "lookup", "parameters": {
                "properties": {"_id": {"type": "string", "_note": true}}
            }}}]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(header::SET_COOKIE));
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"id": "upstream-response"}));
    let (headers, body) = timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
    assert_eq!(headers[header::AUTHORIZATION], "Bearer provider-fixture");
    assert!(!headers.contains_key("x-upstream-url"));
    assert_eq!(
        body,
        json!({
            "model": "selected",
            "metadata": {"properties": {}},
            "tools": [{"type": "function", "function": {"name": "lookup", "parameters": {
                "properties": {"_id": {"type": "string"}}
            }}}]
        }),
    );
    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
}
