use super::{ORIGIN, client_builder, query_at, query_novita};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};
use vw_api_types::apps_usage::UsageQueryErrorCode;

const KEY: &str = "fixture-novita-key";

async fn serve(status: &str, content_length: usize, body: &str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        content_length
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "GET /openapi/v1/billing/balance/detail HTTP/1.1\r\n");

        let mut authorization = None;
        let mut accept = None;
        let mut content_type = None;
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
            } else if name.eq_ignore_ascii_case("content-type") {
                content_type = Some(value.trim().to_owned());
            }
        }
        assert_eq!(authorization.as_deref(), Some("Bearer fixture-novita-key"));
        assert_eq!(accept.as_deref(), Some("application/json"));
        assert_eq!(content_type.as_deref(), Some("application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    (origin, server)
}

#[tokio::test]
async fn usage_balances_novita_returns_expected_result() {
    assert_eq!(ORIGIN, "https://api.novita.ai");
    let client = client_builder().no_proxy().build().unwrap();
    for (body, remaining, topped_up) in [
        (
            r#"{"availableBalance":"255000","cashBalance":80001,"creditLimit":"175000","outstandingInvoices":"0"}"#,
            25.5,
            Some(8.0001),
        ),
        (r#"{"availableBalance":0,"cashBalance":"0"}"#, 0.0, Some(0.0)),
        (r#"{"availableBalance":-50000}"#, -5.0, None),
        (r#"{"availableBalance":"1","cashBalance":null}"#, 0.0001, None),
    ] {
        let (origin, server) = serve("200 OK", body.len(), body).await;
        let wallet = query_at(&client, &origin, KEY).await.unwrap();
        server.await.unwrap();

        assert_eq!(wallet.currency, "USD");
        assert_eq!(wallet.remaining.map(f64::from), Some(remaining));
        assert_eq!(wallet.topped_up.map(f64::from), topped_up);
        assert_eq!(wallet.granted, None);
        assert_eq!(wallet.total, None);
        assert_eq!(wallet.used, None);
    }
}

#[tokio::test]
async fn usage_balances_novita_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for key in ["", "bad\r\nkey"] {
        assert_eq!(query_novita(key).await, Err(CredentialUnavailable));
    }

    let client = client_builder().no_proxy().build().unwrap();
    for body in [
        "{",
        r#"{"cashBalance":"10000"}"#,
        r#"{"availableBalance":null}"#,
        r#"{"availableBalance":"NaN"}"#,
        r#"{"availableBalance":true}"#,
        r#"{"availableBalance":"10000","cashBalance":"inf"}"#,
    ] {
        let (origin, server) = serve("200 OK", body.len(), body).await;
        assert_eq!(query_at(&client, &origin, KEY).await, Err(InvalidResponse));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn usage_balances_novita_distinguishes_transport_and_upstream_status() {
    use UsageQueryErrorCode::*;

    let client = client_builder().no_proxy().build().unwrap();
    // A syntactically incomplete body with the promised length missing is a read failure.
    let (origin, server) = serve("200 OK", 64, "{").await;
    assert_eq!(query_at(&client, &origin, KEY).await, Err(Transport));
    server.await.unwrap();

    for (status, expected) in [
        ("401 Unauthorized", UpstreamUnauthorized),
        ("403 Forbidden", UpstreamUnauthorized),
        ("429 Too Many Requests", UpstreamRateLimited),
        ("503 Service Unavailable", UpstreamUnavailable),
    ] {
        let (origin, server) = serve(status, 1, "{").await;
        assert_eq!(query_at(&client, &origin, KEY).await, Err(expected));
        server.await.unwrap();
    }
}
