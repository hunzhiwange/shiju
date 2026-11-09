use std::time::Duration;

use super::{client_builder, query_at, query_stepfun};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};
use vw_api_types::apps_usage::UsageQueryErrorCode;

const KEY: &str = "fixture-secret";

async fn serve(status: &str, headers: &str, body: &str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "GET /v1/accounts HTTP/1.1\r\n");

        let mut authorization = None;
        let mut accept = None;
        loop {
            line.clear();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            if name.eq_ignore_ascii_case("authorization") {
                authorization = Some(value.trim().to_owned());
            } else if name.eq_ignore_ascii_case("accept") {
                accept = Some(value.trim().to_owned());
            }
        }
        assert_eq!(authorization.as_deref(), Some("Bearer fixture-secret"));
        assert_eq!(accept.as_deref(), Some("application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    (origin, server)
}

#[tokio::test]
async fn usage_balances_stepfun_returns_expected_result() {
    for (body, expected) in [
        (
            r#"{"object":"account","balance":42.75,"total_cash_balance":40,"total_voucher_balance":2.75}"#,
            42.75,
        ),
        (r#"{"balance":"-0.25"}"#, -0.25),
        (r#"{"balance":0}"#, 0.0),
    ] {
        let (origin, server) = serve("200 OK", "", body).await;
        let wallet = query_at(client_builder().no_proxy(), &origin, KEY).await.unwrap();
        server.await.unwrap();
        assert_eq!(wallet.currency, "CNY");
        assert_eq!(wallet.remaining.map(f64::from), Some(expected));
    }
}

#[tokio::test]
async fn usage_balances_stepfun_rejects_invalid_input() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());

    for key in ["", "   ", "invalid\nkey"] {
        assert_eq!(
            query_at(client_builder().no_proxy(), &origin, key).await.unwrap_err(),
            UsageQueryErrorCode::CredentialUnavailable
        );
    }
    assert!(tokio::time::timeout(Duration::from_millis(50), listener.accept()).await.is_err());
    assert_eq!(query_stepfun("").await.unwrap_err(), UsageQueryErrorCode::CredentialUnavailable);
}

#[tokio::test]
async fn usage_balances_stepfun_rejects_missing_or_invalid_balance() {
    for body in [
        r#"{"total_cash_balance":10}"#,
        r#"{"balance":null}"#,
        r#"{"balance":true}"#,
        r#"{"balance":"NaN"}"#,
    ] {
        let (origin, server) = serve("200 OK", "", body).await;
        let result = query_at(client_builder().no_proxy(), &origin, KEY).await;
        server.await.unwrap();
        assert_eq!(result.unwrap_err(), UsageQueryErrorCode::InvalidResponse);
    }
}

#[tokio::test]
async fn usage_balances_stepfun_classifies_http_rejections() {
    for (status, headers, expected) in [
        ("401 Unauthorized", "", UsageQueryErrorCode::UpstreamUnauthorized),
        ("403 Forbidden", "", UsageQueryErrorCode::UpstreamUnauthorized),
        ("429 Too Many Requests", "", UsageQueryErrorCode::UpstreamRateLimited),
        ("503 Service Unavailable", "", UsageQueryErrorCode::UpstreamUnavailable),
        ("302 Found", "Location: /unexpected\r\n", UsageQueryErrorCode::UpstreamRejected),
    ] {
        let (origin, server) = serve(status, headers, r#"{"balance":10}"#).await;
        let result = query_at(client_builder().no_proxy(), &origin, KEY).await;
        server.await.unwrap();
        assert_eq!(result.unwrap_err(), expected);
    }
}
