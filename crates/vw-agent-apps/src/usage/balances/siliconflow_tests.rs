use std::time::Duration;

use super::{client_builder, endpoint, query_at, query_siliconflow};
use crate::usage::detect::BalanceProvider::{DeepSeek, SiliconFlowCn, SiliconFlowGlobal};
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
        assert_eq!(line, "GET /v1/user/info HTTP/1.1\r\n");

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
async fn usage_balances_siliconflow_returns_expected_result() {
    for (provider, expected_endpoint, expected_currency, body, remaining, granted, topped_up) in [
        (
            SiliconFlowCn,
            "https://api.siliconflow.cn/v1/user/info",
            "CNY",
            r#"{"code":20000,"status":true,"data":{"totalBalance":"88.88","balance":"0.88","chargeBalance":"88.00"}}"#,
            Some(88.88),
            Some(0.88),
            Some(88.0),
        ),
        (
            SiliconFlowGlobal,
            "https://api.siliconflow.com/v1/user/info",
            "USD",
            r#"{"code":20000,"status":true,"data":{"totalBalance":-0.25,"chargeBalance":0}}"#,
            Some(-0.25),
            None,
            Some(0.0),
        ),
        (
            SiliconFlowGlobal,
            "https://api.siliconflow.com/v1/user/info",
            "USD",
            r#"{"code":20000,"status":true,"data":{"balance":0,"totalBalance":null}}"#,
            None,
            Some(0.0),
            None,
        ),
    ] {
        let (url, currency) = endpoint(provider).unwrap();
        assert_eq!(url, expected_endpoint);
        let (origin, server) = serve("200 OK", "", body).await;
        let (_, path) = url.trim_start_matches("https://").split_once('/').unwrap();
        let wallet =
            query_at(client_builder().no_proxy(), &format!("{origin}/{path}"), currency, KEY)
                .await
                .unwrap();
        server.await.unwrap();
        assert_eq!(wallet.currency, expected_currency);
        assert_eq!(wallet.remaining.map(f64::from), remaining);
        assert_eq!(wallet.granted.map(f64::from), granted);
        assert_eq!(wallet.topped_up.map(f64::from), topped_up);
        assert_eq!(wallet.total, None);
        assert_eq!(wallet.used, None);
    }
}

#[tokio::test]
async fn usage_balances_siliconflow_rejects_invalid_input() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/user/info", listener.local_addr().unwrap());

    for key in ["", "   ", "invalid\nkey"] {
        assert_eq!(
            query_at(client_builder().no_proxy(), &url, "CNY", key).await.unwrap_err(),
            UsageQueryErrorCode::CredentialUnavailable
        );
    }
    assert!(tokio::time::timeout(Duration::from_millis(50), listener.accept()).await.is_err());
    assert_eq!(
        query_siliconflow(DeepSeek, KEY).await.unwrap_err(),
        UsageQueryErrorCode::Unsupported
    );
}

#[tokio::test]
async fn usage_balances_siliconflow_rejects_cross_origin_redirect() {
    let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!(
        "Location: http://localhost:{}/v1/user/info\r\n",
        target.local_addr().unwrap().port()
    );
    let (origin, server) = serve("302 Found", &location, "").await;
    let result =
        query_at(client_builder().no_proxy(), &format!("{origin}/v1/user/info"), "CNY", KEY).await;
    server.await.unwrap();
    assert_eq!(result.unwrap_err(), UsageQueryErrorCode::UpstreamRejected);
    assert!(tokio::time::timeout(Duration::from_millis(50), target.accept()).await.is_err());
}

#[tokio::test]
async fn usage_balances_siliconflow_classifies_upstream_failures() {
    use UsageQueryErrorCode::*;

    for (status, body, expected) in [
        ("401 Unauthorized", "", UpstreamUnauthorized),
        ("403 Forbidden", "", UpstreamUnauthorized),
        ("429 Too Many Requests", "", UpstreamRateLimited),
        ("503 Service Unavailable", "", UpstreamUnavailable),
        ("200 OK", r#"{"code":20000,"status":false,"data":{"totalBalance":10}}"#, UpstreamRejected),
        ("200 OK", r#"{"code":40000,"status":true}"#, UpstreamRejected),
        ("200 OK", r#"{"code":20000,"status":true}"#, InvalidResponse),
        ("200 OK", r#"{"data":{}}"#, InvalidResponse),
        ("200 OK", r#"{"data":{"totalBalance":"NaN"}}"#, InvalidResponse),
        ("200 OK", r#"{"data":{"totalBalance":10,"balance":false}}"#, InvalidResponse),
    ] {
        let (origin, server) = serve(status, "", body).await;
        let result =
            query_at(client_builder().no_proxy(), &format!("{origin}/v1/user/info"), "USD", KEY)
                .await;
        server.await.unwrap();
        assert_eq!(result.unwrap_err(), expected);
    }
}
