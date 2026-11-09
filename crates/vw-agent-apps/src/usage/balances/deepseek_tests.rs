use super::{client_builder, query_at};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};
use vw_api_types::apps_usage::{UsageQueryErrorCode, UsageWallet};

const KEY: &str = "fixture-secret";
const BALANCE: &str = r#"{
    "is_available": false,
    "balance_infos": [
        {"currency":"CNY","total_balance":"-0.50","granted_balance":"0","topped_up_balance":-0.5},
        {"currency":"USD","total_balance":2.5,"granted_balance":"1.25","topped_up_balance":"1.25"},
        {"currency":"EUR","total_balance":"0"}
    ]
}"#;

async fn serve(status: u16, headers: &str, body: &str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut request = String::new();
        loop {
            let mut line = String::new();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            request.push_str(&line);
        }

        let request = request.to_ascii_lowercase();
        assert!(request.starts_with("get /user/balance http/1.1\r\n"));
        assert!(request.lines().any(|line| line == format!("authorization: bearer {KEY}")));
        assert!(request.lines().any(|line| line == "accept: application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    (origin, server)
}

fn amounts(wallet: &UsageWallet) -> (Option<f64>, Option<f64>, Option<f64>) {
    (
        wallet.remaining.map(Into::into),
        wallet.granted.map(Into::into),
        wallet.topped_up.map(Into::into),
    )
}

#[tokio::test]
async fn usage_balances_deepseek_returns_expected_result() {
    let (origin, server) = serve(200, "", BALANCE).await;
    let client = client_builder().no_proxy().build().unwrap();

    let result = query_at(&client, &origin, KEY).await.unwrap();
    server.await.unwrap();
    assert_eq!(result.is_available, Some(false));
    let [cny, usd, eur] = result.wallets.as_slice() else {
        panic!("expected all three currency wallets");
    };
    assert_eq!(cny.currency, "CNY");
    assert_eq!(amounts(cny), (Some(-0.5), Some(0.0), Some(-0.5)));
    assert_eq!(usd.currency, "USD");
    assert_eq!(amounts(usd), (Some(2.5), Some(1.25), Some(1.25)));
    assert_eq!(eur.currency, "EUR");
    assert_eq!(amounts(eur), (Some(0.0), None, None));
}

#[tokio::test]
async fn usage_balances_deepseek_rejects_invalid_input() {
    let client = client_builder().no_proxy().build().unwrap();
    for status in [401, 403] {
        let (origin, server) = serve(status, "", BALANCE).await;
        let result = query_at(&client, &origin, KEY).await;
        server.await.unwrap();
        assert_eq!(result.unwrap_err(), UsageQueryErrorCode::UpstreamUnauthorized);
    }
}

#[tokio::test]
async fn usage_balances_deepseek_rejects_invalid_responses() {
    let client = client_builder().no_proxy().build().unwrap();
    for body in [
        r#"{}"#,
        r#"{"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"NaN"}]}"#,
    ] {
        let (origin, server) = serve(200, "", body).await;
        let result = query_at(&client, &origin, KEY).await;
        server.await.unwrap();
        assert_eq!(result.unwrap_err(), UsageQueryErrorCode::InvalidResponse);
    }
}

#[tokio::test]
async fn usage_balances_deepseek_rejects_redirects() {
    let client = client_builder().no_proxy().build().unwrap();
    let (origin, server) = serve(302, "Location: /unexpected\r\n", "").await;
    let result = query_at(&client, &origin, KEY).await;
    server.await.unwrap();
    assert_eq!(result.unwrap_err(), UsageQueryErrorCode::UpstreamRejected);
}
