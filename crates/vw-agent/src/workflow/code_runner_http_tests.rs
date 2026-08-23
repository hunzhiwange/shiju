//! 验证代码运行器 HTTP 桥接的鉴权、代理参数、错误结构与敏感信息脱敏。

use serde_json::json;
use std::collections::BTreeMap;

use anyhow::Result;
use axum::body::Bytes;
use axum::extract::OriginalUri;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json as AxumJson, Router, routing::post};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::{
    HttpBridge, append_query_params, debug_body_text, debug_redacted_value, http_method,
    query_param_pairs, redact_form_text, redact_string_map, redact_url_for_log,
    truncate_debug_text,
};

async fn echo_http_bridge_request(
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let request = json!({
        "query": uri.query().unwrap_or_default(),
        "x_test": headers
            .get("x-test")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        "content_type": headers
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        "body": String::from_utf8_lossy(&body),
    });
    (StatusCode::CREATED, [("x-response-token", "visible")], AxumJson(request))
}

async fn start_upstream_server() -> Result<(String, oneshot::Sender<()>, JoinHandle<()>)> {
    let app = Router::new().route("/echo", post(echo_http_bridge_request));
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let (shutdown, shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
        let _ = server.await;
    });
    Ok((format!("http://{addr}"), shutdown, task))
}

async fn stop_upstream_server(shutdown: oneshot::Sender<()>, task: JoinHandle<()>) {
    let _ = shutdown.send(());
    let _ = task.await;
}

#[test]
fn http_bridge_debug_redacts_url_query_credentials() {
    let url = redact_url_for_log(
        "https://user:pass@example.test/RestfulApi/goods?dhb_skey=abc123&page=1&token=secret",
    );

    assert!(!url.contains("user:pass"));
    assert!(url.contains("dhb_skey=%5BREDACTED%5D"));
    assert!(url.contains("token=%5BREDACTED%5D"));
    assert!(url.contains("page=1"));
}

#[test]
fn http_bridge_debug_redacts_headers_and_json_values() {
    let headers = BTreeMap::from([
        ("Authorization".to_string(), "Bearer secret".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ]);
    let redacted_headers = redact_string_map(&headers);

    assert_eq!(redacted_headers.get("Authorization").map(String::as_str), Some("[REDACTED]"));
    assert_eq!(redacted_headers.get("Content-Type").map(String::as_str), Some("application/json"));

    let value = json!({
        "params": {
            "dhb_skey": "abc123",
            "kw": "手机"
        }
    });
    let debug = debug_redacted_value(&value);

    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("abc123"));
    assert!(debug.contains("手机"));
}

#[test]
fn http_bridge_debug_redacts_form_body_values() {
    let body = redact_form_text("dhb_skey=abc123&page=1&password=secret");

    assert_eq!(body, "dhb_skey=[REDACTED]&page=1&password=[REDACTED]");
}

#[tokio::test]
async fn http_bridge_start_rejects_bad_token_and_proxies_json_and_body_requests() -> Result<()> {
    let bridge = HttpBridge::start().await.map_err(anyhow::Error::msg)?;
    let (upstream, upstream_shutdown, upstream_task) = start_upstream_server().await?;
    let client = reqwest::Client::new();

    let rejected: serde_json::Value = client
        .post(bridge.endpoint())
        .json(&json!({
            "bridge_token": "wrong-token",
            "method": "GET",
            "url": format!("{upstream}/echo"),
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["error"], "workflow HTTP bridge token mismatch");

    let json_response: serde_json::Value = client
        .post(bridge.endpoint())
        .json(&json!({
            "bridge_token": bridge.token(),
            "method": "post",
            "url": format!("{upstream}/echo?existing=1"),
            "headers": {
                "x-test": "json-request",
                "authorization": "Bearer secret"
            },
            "params": {
                "tag": ["a", "b"],
                "enabled": true
            },
            "timeout_secs": 3.0,
            "json": {
                "hello": "world"
            }
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(json_response["ok"], true);
    assert_eq!(json_response["status_code"], 201);
    assert_eq!(json_response["headers"]["x-response-token"], "visible");
    let json_text: serde_json::Value =
        serde_json::from_str(json_response["text"].as_str().expect("json response text"))?;
    assert!(json_text["query"].as_str().expect("query").contains("existing=1"));
    assert!(json_text["query"].as_str().expect("query").contains("tag=a"));
    assert!(json_text["query"].as_str().expect("query").contains("tag=b"));
    assert_eq!(json_text["x_test"], "json-request");
    assert!(json_text["content_type"].as_str().expect("content type").contains("application/json"));
    assert_eq!(json_text["body"], r#"{"hello":"world"}"#);

    let body_response: serde_json::Value = client
        .post(bridge.endpoint())
        .json(&json!({
            "bridge_token": bridge.token(),
            "method": "POST",
            "url": format!("{upstream}/echo"),
            "headers": {
                "x-test": "body-request"
            },
            "params": [
                ["shape", {"nested": true}],
                ["page", 2]
            ],
            "body": "token=secret&page=1"
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(body_response["ok"], true);
    let body_text: serde_json::Value =
        serde_json::from_str(body_response["text"].as_str().expect("body response text"))?;
    assert!(body_text["query"].as_str().expect("query").contains("page=2"));
    assert!(
        body_text["query"].as_str().expect("query").contains("shape=%7B%22nested%22%3Atrue%7D")
    );
    assert_eq!(body_text["x_test"], "body-request");
    assert_eq!(body_text["body"], "token=secret&page=1");

    bridge.stop().await;
    stop_upstream_server(upstream_shutdown, upstream_task).await;
    Ok(())
}

#[tokio::test]
async fn http_bridge_returns_structured_errors_for_invalid_timeout_and_failed_upstream()
-> Result<()> {
    let bridge = HttpBridge::start().await.map_err(anyhow::Error::msg)?;
    let client = reqwest::Client::new();

    let invalid_timeout: serde_json::Value = client
        .post(bridge.endpoint())
        .json(&json!({
            "bridge_token": bridge.token(),
            "method": "GET",
            "url": "http://127.0.0.1/",
            "timeout_secs": 0.0
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(invalid_timeout["ok"], false);
    assert_eq!(invalid_timeout["error"], "HTTP timeout must be a positive number");
    assert!(invalid_timeout.get("timeout").is_none());

    let closed_listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let closed_addr = closed_listener.local_addr()?;
    drop(closed_listener);
    let failed_upstream: serde_json::Value = client
        .post(bridge.endpoint())
        .json(&json!({
            "bridge_token": bridge.token(),
            "method": "GET",
            "url": format!("http://{closed_addr}/missing"),
            "timeout_secs": 0.5
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(failed_upstream["ok"], false);
    assert!(failed_upstream["error"].as_str().expect("upstream error").contains("error"));

    bridge.stop().await;
    Ok(())
}

#[test]
fn http_bridge_methods_and_query_params_cover_supported_shapes() {
    for method in ["get", "POST", "put", "DELETE", "patch", "HEAD", "options"] {
        assert!(http_method(method).is_ok(), "{method}");
    }
    assert!(http_method("trace").is_err());

    let url = append_query_params(
        "https://example.test/path?existing=1",
        Some(&json!({"tag": ["a", "b"], "enabled": true, "missing": null})),
    )
    .expect("object params");
    assert!(url.contains("existing=1"));
    assert!(url.contains("tag=a"));
    assert!(url.contains("tag=b"));
    assert!(url.contains("enabled=true"));
    assert!(url.contains("missing=None"));
}

#[test]
fn http_bridge_query_params_reject_malformed_inputs() {
    assert_eq!(append_query_params("not a url", None).expect("unchanged"), "not a url");
    assert!(append_query_params("not a url", Some(&json!({"a": 1}))).is_err());
    assert!(query_param_pairs(&json!("bad")).is_err());
    assert!(query_param_pairs(&json!([["only-key"]])).is_err());
    assert!(query_param_pairs(&json!([[1, "value"]])).is_err());
}

#[test]
fn http_bridge_debug_handles_plain_and_truncated_text() {
    assert_eq!(redact_form_text("plain text"), "plain text");
    assert_eq!(redact_form_text("flag&token=secret"), "flag&token=[REDACTED]");
    assert!(debug_body_text(r#"{"session_id":"secret","ok":true}"#).contains("[REDACTED]"));
    assert!(truncate_debug_text("界".repeat(4_001)).ends_with("..."));
    assert_eq!(redact_url_for_log("not a url"), "not a url");
}
