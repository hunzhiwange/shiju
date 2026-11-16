use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{RoutingListener, SharedTransport};
use axum::{Json, Router, http::StatusCode, routing::post};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc};
use {
    vw_api_app_identity::AppId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};

#[tokio::test]
async fn aggregation_t11_aggregation_routing_preserves_contract() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/upstream", upstream.local_addr().unwrap());
    let (record, mut requests) = mpsc::unbounded_channel();
    let router = Router::new().route(
        "/upstream",
        post(move |Json(body): Json<Value>| async move {
            record.send(body).unwrap();
            Json(json!({"id": "default-provider"}))
        }),
    );
    let upstream_task = tokio::spawn(async move { axum::serve(upstream, router).await.unwrap() });

    let listener = crate::test_listener();
    let address = listener.start(0).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let binding = VerifiedRouteBinding::new(
        ProjectScope::new(tenant, "project".into()).unwrap(),
        "host".into(),
        AppId::Codex,
        "account".into(),
        1,
    );
    let token = listener
        .auth
        .issue_test_token(
            binding,
            JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: url.parse().unwrap(),
                authorization: "Bearer fixture".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            },
        )
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let send = |model: &str| {
        client
            .post(format!("http://{address}/v1/chat/completions"))
            .bearer_auth(&token)
            .json(&json!({"model": model}))
            .send()
    };

    let response = send("shared-model").await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"id": "default-provider"}));
    assert_eq!(requests.try_recv().unwrap(), json!({"model": "shared-model"}));

    let inactive = send("ccs-member/shared-model").await.unwrap().status();
    let malformed = send("ccs-member").await.unwrap().status();
    let extra_request = requests.try_recv().ok();

    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
    assert_eq!(inactive, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(malformed, StatusCode::BAD_REQUEST);
    assert!(extra_request.is_none(), "reserved IDs must never reach the default provider");
}

// Host lifecycle publication integration is covered by vw-agent app_routing tests.
