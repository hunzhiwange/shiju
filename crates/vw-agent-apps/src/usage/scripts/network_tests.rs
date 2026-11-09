use std::time::Duration;

use reqwest::{Client, Url};
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
    time::timeout,
};
use vw_api_types::apps_usage::UsageQueryErrorCode;

use super::{NetworkError, build_http_request};
use crate::usage::scripts::config::{
    PreparedUsage, PreparedUsageScript, ProviderDefaults, ScriptParameters, UsageTemplate,
};

fn prepare(
    template: UsageTemplate,
    request: serde_json::Value,
    base: &str,
) -> PreparedUsageScript<'_> {
    let source = format!(
        "({{request: {request}, extractor: (response) => ({{remaining: response.balance, unit: 'USD'}})}})"
    );
    let PreparedUsage::Script(script) = template
        .prepare(
            Some(&source),
            ScriptParameters::default(),
            ProviderDefaults { base_url: Some(base), ..ProviderDefaults::default() },
        )
        .unwrap()
    else {
        panic!("expected a prepared script");
    };
    script
}

async fn fixture(response: String) -> (Url, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let received = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut data = Vec::new();
        let mut chunk = [0; 1024];
        loop {
            let count = stream.read(&mut chunk).await.unwrap();
            assert_ne!(count, 0);
            data.extend_from_slice(&chunk[..count]);
            if let Some(headers_end) = data.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&data[..headers_end]).unwrap();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or_default();
                if data.len() >= headers_end + 4 + length {
                    break;
                }
            }
        }
        stream.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(data).unwrap()
    });
    (base, received)
}

fn response(status: &str, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    )
}

#[tokio::test]
async fn usage_scripts_network_returns_expected_result() {
    let (base, received) = fixture(response("200 OK", "", r#"{"balance":12}"#)).await;
    let script = prepare(
        UsageTemplate::General,
        json!({
            "url": base.join("/user/balance?currency=USD").unwrap().as_str(),
            "method": "PATCH",
            "headers": {"Authorization": "Bearer fixture-token", "Content-Type": "application/json"},
            "body": "{\"currency\":\"USD\"}",
        }),
        base.as_str(),
    );

    let body = script.send_request(10, std::slice::from_ref(&base)).await.unwrap();
    assert_eq!(script.extract(&body).await.unwrap(), json!({"remaining": 12, "unit": "USD"}));
    let request = received.await.unwrap();
    assert!(request.starts_with("PATCH /user/balance?currency=USD HTTP/1.1\r\n"));
    assert!(request.to_ascii_lowercase().contains("\r\nauthorization: bearer fixture-token\r\n"));
    assert!(request.to_ascii_lowercase().contains("\r\ncontent-type: application/json\r\n"));
    assert!(request.ends_with("\r\n\r\n{\"currency\":\"USD\"}"));
}

#[tokio::test]
async fn usage_scripts_network_rejects_invalid_input() {
    let denied = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let denied_url =
        Url::parse(&format!("http://{}/secret", denied.local_addr().unwrap())).unwrap();
    let (base, received) = fixture(response(
        "307 Temporary Redirect",
        &format!("Location: {denied_url}\r\n"),
        "fixture-private-body",
    ))
    .await;
    let custom = |url: &Url| {
        prepare(
            UsageTemplate::Custom,
            json!({
                "url": url.as_str(), "method": "GET", "headers": {"Authorization": "fixture-token"}
            }),
            "http://unused.example:8090",
        )
    };
    assert_eq!(
        custom(&denied_url).send_request(2, std::slice::from_ref(&base)).await,
        Err(NetworkError::DestinationDenied)
    );
    // Even an independently authorized redirect target must receive no request.
    assert_eq!(
        custom(&base).send_request(2, &[base.clone(), denied_url]).await,
        Err(NetworkError::Redirect)
    );
    assert!(received.await.unwrap().starts_with("GET / HTTP/1.1\r\n"));
    assert!(timeout(Duration::from_millis(50), denied.accept()).await.is_err());
}

#[tokio::test]
async fn usage_scripts_network_enforces_template_origin_and_valid_request_fields() {
    let base = Url::parse("https://example.test").unwrap();
    let other = Url::parse("https://example.test:8443").unwrap();
    let script = prepare(
        UsageTemplate::General,
        json!({"url": other.as_str(), "method": "GET"}),
        base.as_str(),
    );
    assert_eq!(script.send_request(2, &[other]).await, Err(NetworkError::DestinationDenied));

    for request in [
        json!({"url": "file:///private/fixture", "method": "GET"}),
        json!({"url": "https://secret@example.test", "method": "GET"}),
        json!({"url": "https://example.test", "method": "BAD METHOD"}),
        json!({"url": "https://example.test", "method": "GET", "headers": {"Host": "other.test"}}),
    ] {
        let script = prepare(UsageTemplate::Custom, request, "");
        assert_eq!(
            script.send_request(2, std::slice::from_ref(&base)).await,
            Err(NetworkError::InvalidRequest)
        );
    }
}

#[tokio::test]
async fn usage_scripts_network_clamps_request_timeout() {
    let base = Url::parse("https://example.test").unwrap();
    let script = prepare(UsageTemplate::Custom, json!({"url": base.as_str(), "method": "GET"}), "");
    let client = Client::new();
    for (input, expected) in [(0, 2), (2, 2), (10, 10), (30, 30), (u64::MAX, 30)] {
        let request = build_http_request(
            &client,
            &script,
            script.build_request().await.unwrap(),
            input,
            std::slice::from_ref(&base),
        )
        .unwrap();
        assert_eq!(request.timeout(), Some(&Duration::from_secs(expected)));
    }
}

#[tokio::test]
async fn usage_scripts_network_classifies_upstream_denial_without_response_body() {
    let (base, received) = fixture(response("403 Forbidden", "", "fixture-private-body")).await;
    let script = prepare(UsageTemplate::Custom, json!({"url": base.as_str(), "method": "GET"}), "");
    assert_eq!(
        script.send_request(2, &[base]).await,
        Err(NetworkError::Upstream(UsageQueryErrorCode::UpstreamUnauthorized))
    );
    received.await.unwrap();
}

#[tokio::test]
async fn usage_scripts_network_timeout_covers_response_body() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            assert_ne!(stream.read_buf(&mut request).await.unwrap(), 0);
        }
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n{").await.unwrap();
        // Keep the response open until the client times out and closes it.
        assert_eq!(stream.read(&mut [0]).await.unwrap(), 0);
    });
    let script = prepare(UsageTemplate::Custom, json!({"url": base.as_str(), "method": "GET"}), "");
    assert_eq!(
        timeout(Duration::from_secs(5), script.send_request(0, &[base])).await.unwrap(),
        Err(NetworkError::Upstream(UsageQueryErrorCode::Timeout))
    );
    timeout(Duration::from_secs(1), server).await.unwrap().unwrap();
}
