use super::{
    body_limits::MAX_JSON_BYTES,
    protocol_errors::{Protocol, RouteError},
    timeouts_tests::{Fixture, GUARD, STREAM_HEAD, chunk},
};
use crate::app_routing::config_apply::AppTimeouts;
use axum::{
    body::Body,
    http::{StatusCode, header},
};
use flate2::{Compression, write::GzEncoder};
use futures_util::StreamExt;
use serde_json::Value;
use std::{
    fmt::{self, Write as _},
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, time::timeout};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    metadata::LevelFilter,
    span::{Attributes, Id, Record},
};

const SECRET: &str = "never-return-this-fixture-secret";
const OUTPUT: &[u8] = b"data: {\"choices\":[{\"delta\":{\"content\":\"model output\"}}]}\n\n";

#[tokio::test]
async fn cc_route_024_protocol_errors_expected_behavior() {
    let fixture = Fixture::start().await;
    let client = reqwest::Client::builder().no_proxy().timeout(GUARD).build().unwrap();
    for (path, pointer, expected) in [
        ("/v1/messages", "/error/type", "authentication_error"),
        ("/claude-desktop/v1/messages", "/error/type", "authentication_error"),
        ("/v1/responses", "/error/code", "invalid_api_key"),
        ("/codex/v1/responses", "/error/code", "invalid_api_key"),
        ("/v1/chat/completions", "/error/code", "invalid_api_key"),
        ("/v1beta/models/fixture:generateContent", "/error/status", "UNAUTHENTICATED"),
    ] {
        let response = client
            .post(format!("http://{}{path}?key={SECRET}", fixture.address))
            .bearer_auth(SECRET)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json", "{path}");
        let body = response.text().await.unwrap();
        assert!(!body.contains(SECRET));
        let body: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body.pointer(pointer).and_then(Value::as_str), Some(expected), "{path}");
        if path.ends_with("/messages") {
            assert_eq!(body["type"], "error");
        }
        if path.starts_with("/v1beta/") {
            assert_eq!(body["error"]["code"], 401);
        }
    }

    let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
    gzip.write_all(&vec![b'x'; MAX_JSON_BYTES + 1]).unwrap();
    let response = client
        .post(format!("http://{}/v1/chat/completions", fixture.address))
        .bearer_auth(&fixture.token)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_ENCODING, "gzip")
        .body(gzip.finish().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "local_request_too_large");
    assert!(timeout(Duration::from_millis(50), fixture.upstream.accept()).await.is_err());
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_024_upstream_rejections_keep_status_and_hide_bodies() {
    let fixture = Fixture::start().await;
    for (stream, status, code) in [
        (false, StatusCode::PAYLOAD_TOO_LARGE, "upstream_request_too_large"),
        (true, StatusCode::UNAUTHORIZED, "upstream_authentication_failed"),
    ] {
        let client = fixture.request(stream, false);
        let mut upstream = fixture.upstream_request(stream).await;
        let payload = format!("<html>{SECRET}</html>");
        let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
        gzip.write_all(payload.as_bytes()).unwrap();
        let payload = gzip.finish().unwrap();
        upstream.write_all(format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nX-Upstream-Secret: {SECRET}\r\n\r\n",
            payload.len()
        ).as_bytes()).await.unwrap();
        upstream.write_all(&payload).await.unwrap();
        let response = timeout(GUARD, client).await.unwrap().unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        assert!(!response.headers().contains_key("x-upstream-secret"));
        let bytes = axum::body::to_bytes(Body::new(response.into_body()), 4096).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(SECRET));
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], code);
    }

    fixture.save(AppTimeouts { first_byte_timeout_secs: 1, ..AppTimeouts::default() });
    let client = fixture.request(true, false);
    let _upstream = fixture.upstream_request(true).await;
    let response = timeout(GUARD, client).await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    let bytes = axum::body::to_bytes(Body::new(response.into_body()), 4096).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "upstream_timeout");
    fixture.listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_024_started_sse_ends_with_a_sanitized_error_frame() {
    let logs = CapturedLogs::default();
    let _guard = tracing::subscriber::set_default(logs.clone());
    let fixture = Fixture::start_with_logging(true).await;
    fixture.save(AppTimeouts { idle_timeout_secs: 1, ..AppTimeouts::default() });

    for expected in ["upstream_timeout", "upstream_stream_failed"] {
        let client = fixture.request(true, false);
        let mut upstream = fixture.upstream_request(true).await;
        upstream.write_all(STREAM_HEAD).await.unwrap();
        chunk(&mut upstream, OUTPUT).await.unwrap();

        let response = timeout(GUARD, client).await.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let mut stream = Body::new(response.into_body()).into_data_stream();
        assert_eq!(timeout(GUARD, stream.next()).await.unwrap().unwrap().unwrap(), OUTPUT);

        let partial = format!("data: {{\"error\":{{\"message\":\"{SECRET}");
        chunk(&mut upstream, partial.as_bytes()).await.unwrap();
        if expected == "upstream_timeout" {
            // Hold the connection open with an incomplete frame until the idle deadline.
            assert_stream_error(&mut stream, expected).await;
        } else {
            chunk(&mut upstream, b"\"}}\n\n").await.unwrap();
            assert_stream_error(&mut stream, expected).await;
        }
    }

    let client = fixture.request(true, true);
    let mut upstream = fixture.upstream_request(true).await;
    upstream.write_all(STREAM_HEAD).await.unwrap();
    chunk(&mut upstream, OUTPUT).await.unwrap();
    let response = timeout(GUARD, client).await.unwrap().unwrap();
    let mut stream = Body::new(response.into_body()).into_data_stream();
    assert_eq!(timeout(GUARD, stream.next()).await.unwrap().unwrap().unwrap(), OUTPUT);
    drop(upstream); // Truncated HTTP response with no terminal event.
    assert_stream_error(&mut stream, "upstream_stream_failed").await;
    fixture.listener.stop().await.unwrap();
    let logs = logs.0.lock().unwrap();
    assert!(logs.contains("Authorized application routing request"), "no routing logs captured");
    assert!(!logs.contains(SECRET));
}

async fn assert_stream_error(stream: &mut axum::body::BodyDataStream, code: &str) {
    let mut terminal = Vec::new();
    timeout(GUARD, async {
        while let Some(bytes) = stream.next().await {
            terminal.extend_from_slice(&bytes.unwrap());
        }
    })
    .await
    .unwrap();
    let terminal = String::from_utf8(terminal).unwrap();
    assert!(!terminal.contains(SECRET));
    let data = terminal.strip_prefix("data: ").unwrap().strip_suffix("\n\n").unwrap();
    let body: Value = serde_json::from_str(data).unwrap();
    assert_eq!(body["error"]["code"], code);
}

#[tokio::test]
async fn cc_route_024_sse_error_before_output_uses_http_and_done_ends_the_stream() {
    let fixture = Fixture::start().await;
    let error = format!("event: error\r\ndata: {{\"message\":\"{SECRET}\"}}\r\n\r\n");

    let client = fixture.request(true, true);
    let mut upstream = fixture.upstream_request(true).await;
    upstream.write_all(STREAM_HEAD).await.unwrap();
    chunk(&mut upstream, error.as_bytes()).await.unwrap();
    let response = timeout(GUARD, client).await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    let bytes = axum::body::to_bytes(Body::new(response.into_body()), 4096).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains(SECRET));
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "upstream_stream_failed");

    let client = fixture.request(true, false);
    let mut upstream = fixture.upstream_request(true).await;
    upstream.write_all(STREAM_HEAD).await.unwrap();
    let mut complete = OUTPUT.to_vec();
    complete.extend_from_slice(b"data: [DONE]\n\n");
    let mut upstream_chunk = complete.clone();
    upstream_chunk.extend_from_slice(error.as_bytes());
    chunk(&mut upstream, &upstream_chunk).await.unwrap();
    let response = timeout(GUARD, client).await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(Body::new(response.into_body()), 4096).await.unwrap();
    assert_eq!(&bytes[..], &complete);
    fixture.listener.stop().await.unwrap();
}

#[test]
fn cc_route_024_terminal_frames_match_client_protocols() {
    let timeout = RouteError::from(StatusCode::GATEWAY_TIMEOUT);
    for (protocol, prefix, pointer, expected) in [
        (Protocol::Messages, "event: error\ndata: ", "/error/type", "api_error"),
        (
            Protocol::Responses,
            "event: response.failed\ndata: ",
            "/response/error/code",
            "upstream_timeout",
        ),
        (Protocol::Chat, "data: ", "/error/code", "upstream_timeout"),
        (Protocol::Gemini, "data: ", "/error/status", "DEADLINE_EXCEEDED"),
    ] {
        let frame = protocol.terminal(timeout, 8);
        let data = frame.strip_prefix(prefix).unwrap().strip_suffix("\n\n").unwrap();
        let event: Value = serde_json::from_str(data).unwrap();
        assert_eq!(event.pointer(pointer).and_then(Value::as_str), Some(expected));
        match protocol {
            Protocol::Messages => assert_eq!(event["type"], "error"),
            Protocol::Responses => {
                assert_eq!(event["type"], "response.failed");
                assert_eq!(event["response"]["status"], "failed");
                assert_eq!(event["sequence_number"], 8);
            }
            Protocol::Gemini => assert_eq!(event["error"]["code"], 504),
            Protocol::Chat => {}
        }
    }
}

#[derive(Clone, Default)]
struct CapturedLogs(Arc<Mutex<String>>);

impl Subscriber for CapturedLogs {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target().starts_with("vw_agent::app_routing")
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::TRACE)
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        struct Fields<'a>(&'a mut String);
        impl Visit for Fields<'_> {
            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                writeln!(self.0, "{}={value:?}", field.name()).unwrap();
            }
        }
        event.record(&mut Fields(&mut self.0.lock().unwrap()));
    }
}
