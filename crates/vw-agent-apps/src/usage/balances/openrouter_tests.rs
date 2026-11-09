use super::{ORIGIN, client_builder, query_at, query_openrouter};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};
use vw_api_types::apps_usage::UsageQueryErrorCode;

const KEY: &str = "fixture-management-key";

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
        assert_eq!(line, "GET /api/v1/credits HTTP/1.1\r\n");

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
        assert_eq!(authorization.as_deref(), Some("Bearer fixture-management-key"));
        assert_eq!(accept.as_deref(), Some("application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    (origin, server)
}

#[tokio::test]
async fn usage_balances_openrouter_returns_expected_result() {
    assert_eq!(ORIGIN, "https://openrouter.ai");
    let client = client_builder().no_proxy().build().unwrap();
    for (body, total, used, remaining) in [
        (r#"{"data":{"total_credits":100.5,"total_usage":25.75}}"#, 100.5, 25.75, 74.75),
        (r#"{"data":{"total_credits":0,"total_usage":0}}"#, 0.0, 0.0, 0.0),
        (r#"{"total_credits":"100.00","total_usage":"105.50"}"#, 100.0, 105.5, -5.5),
    ] {
        let (origin, server) = serve("200 OK", "", body).await;
        let wallet = query_at(&client, &origin, KEY).await.unwrap();
        server.await.unwrap();

        assert_eq!(wallet.currency, "USD");
        assert_eq!(wallet.total.map(f64::from), Some(total));
        assert_eq!(wallet.used.map(f64::from), Some(used));
        assert_eq!(wallet.remaining.map(f64::from), Some(remaining));
        assert_eq!(wallet.granted, None);
        assert_eq!(wallet.topped_up, None);
    }
}

#[tokio::test]
async fn usage_balances_openrouter_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for key in ["", "bad\r\nkey"] {
        assert_eq!(query_openrouter(key).await, Err(CredentialUnavailable));
    }

    let client = client_builder().no_proxy().build().unwrap();
    for body in [
        r#"{"data":{"total_credits":100}}"#,
        r#"{"data":{"total_usage":0}}"#,
        r#"{"data":{"total_credits":null,"total_usage":0}}"#,
        r#"{"data":{"limit":100,"limit_remaining":100}}"#,
        r#"{"data":{"total_credits":"NaN","total_usage":0}}"#,
        r#"{"data":{"total_credits":100,"total_usage":"invalid"}}"#,
        r#"{"data":{"total_credits":1e308,"total_usage":-1e308}}"#,
        r#"{"data":null,"total_credits":100,"total_usage":0}"#,
    ] {
        let (origin, server) = serve("200 OK", "", body).await;
        assert_eq!(query_at(&client, &origin, KEY).await, Err(InvalidResponse), "{body}");
        server.await.unwrap();
    }
}

#[tokio::test]
async fn usage_balances_openrouter_classifies_upstream_rejections() {
    use UsageQueryErrorCode::*;

    let client = client_builder().no_proxy().build().unwrap();
    for (status, headers, expected) in [
        ("401 Unauthorized", "", UpstreamUnauthorized),
        ("403 Forbidden", "", UpstreamUnauthorized),
        ("429 Too Many Requests", "", UpstreamRateLimited),
        ("503 Service Unavailable", "", UpstreamUnavailable),
        ("302 Found", "Location: http://127.0.0.1:1/\r\n", UpstreamRejected),
    ] {
        let (origin, server) = serve(status, headers, r#"{"error":{"message":"fixture"}}"#).await;
        assert_eq!(query_at(&client, &origin, KEY).await, Err(expected));
        server.await.unwrap();
    }
}
