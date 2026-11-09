use super::{client_builder, query_at, query_zhipu_balance};
use crate::usage::detect::CodingPlanProvider;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
    time::timeout,
};
use vw_api_types::apps_usage::UsageQueryErrorCode;

const KEY: &str = "fixture-zhipu-key";
const PLAN: &str = "/api/monitor/usage/quota/limit";
const BALANCE: &str = "/api/biz/account/query-customer-account-report";
const NO_PLAN: &str = r#"{"code":500,"msg":"当前用户不存在coding plan","success":false}"#;

struct Reply {
    path: &'static str,
    status: &'static str,
    body: &'static str,
    truncated: bool,
}

fn reply(path: &'static str, status: &'static str, body: &'static str) -> Reply {
    Reply { path, status, body, truncated: false }
}

async fn serve(replies: Vec<Reply>) -> (String, JoinHandle<TcpListener>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for reply in replies {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert_eq!(line, format!("GET {} HTTP/1.1\r\n", reply.path));

            let mut authorization = None;
            let mut language = None;
            let mut json_header = None;
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap();
                match (name.to_ascii_lowercase().as_str(), reply.path) {
                    ("authorization", _) => authorization = Some(value.trim().to_owned()),
                    ("accept-language", _) => language = Some(value.trim().to_owned()),
                    ("content-type", PLAN) | ("accept", BALANCE) => {
                        json_header = Some(value.trim().to_owned());
                    }
                    _ => {}
                }
            }
            assert_eq!(authorization.as_deref(), Some(KEY));
            assert_eq!(language.as_deref(), Some("en-US,en"));
            assert_eq!(json_header.as_deref(), Some("application/json"));
            let length = reply.body.len() + usize::from(reply.truncated);
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{}",
                reply.status, reply.body,
            );
            stream.get_mut().write_all(response.as_bytes()).await.unwrap();
        }
        listener
    });
    (origin, server)
}

async fn assert_requests_complete(server: JoinHandle<TcpListener>) {
    let listener = timeout(Duration::from_secs(1), server).await.unwrap().unwrap();
    assert!(timeout(Duration::from_millis(20), listener.accept()).await.is_err());
}

#[tokio::test]
async fn usage_balances_zhipu_returns_expected_result() {
    let client = client_builder().no_proxy().build().unwrap();
    for (currency, plan, balance, expected) in [
        (
            "CNY",
            NO_PLAN,
            r#"{"code":200,"success":true,"data":{"availableBalance":0,"balance":15}}"#,
            0.0,
        ),
        (
            "USD",
            r#"{"code":500,"msg":"The current user does not have a coding plan","success":false}"#,
            r#"{"success":true,"data":{"balance":"-3.5"}}"#,
            -3.5,
        ),
    ] {
        let (origin, server) =
            serve(vec![reply(PLAN, "200 OK", plan), reply(BALANCE, "200 OK", balance)]).await;
        let wallet = query_at(&client, &origin, currency, KEY).await.unwrap().unwrap();
        assert_requests_complete(server).await;
        assert_eq!(wallet.currency, currency);
        assert_eq!(wallet.remaining.map(f64::from), Some(expected));
    }
}

#[tokio::test]
async fn usage_balances_zhipu_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for key in ["", "bad\r\nkey"] {
        assert_eq!(
            query_zhipu_balance(CodingPlanProvider::ZhipuCn, key).await,
            Err(CredentialUnavailable),
        );
    }
    assert_eq!(query_zhipu_balance(CodingPlanProvider::ZhipuTeam, KEY).await, Err(Unsupported),);

    let client = client_builder().no_proxy().build().unwrap();
    for (status, body, truncated, expected) in [
        ("401 Unauthorized", NO_PLAN, false, UpstreamUnauthorized),
        ("403 Forbidden", NO_PLAN, false, UpstreamUnauthorized),
        ("429 Too Many Requests", NO_PLAN, false, UpstreamRateLimited),
        ("500 Internal Server Error", NO_PLAN, false, UpstreamUnavailable),
        ("200 OK", NO_PLAN, true, Transport),
        ("200 OK", "{", false, InvalidResponse),
        (
            "200 OK",
            r#"{"code":1001,"success":false,"msg":"当前用户不存在coding plan"}"#,
            false,
            UpstreamRejected,
        ),
        (
            "200 OK",
            r#"{"code":500,"success":false,"msg":"Coding plan service not found"}"#,
            false,
            UpstreamRejected,
        ),
    ] {
        let (origin, server) = serve(vec![Reply { truncated, ..reply(PLAN, status, body) }]).await;
        assert_eq!(query_at(&client, &origin, "CNY", KEY).await, Err(expected));
        assert_requests_complete(server).await;
    }
}

#[tokio::test]
async fn usage_balances_zhipu_leaves_existing_plans_alone() {
    let client = client_builder().no_proxy().build().unwrap();
    for body in [
        r#"{"success":true,"data":{"limits":[{"type":"TOKENS_LIMIT","percentage":12}]}}"#,
        r#"{"success":true,"data":{"limits":[]}}"#,
    ] {
        let (origin, server) = serve(vec![reply(PLAN, "200 OK", body)]).await;
        assert_eq!(query_at(&client, &origin, "CNY", KEY).await, Ok(None));
        assert_requests_complete(server).await;
    }
}

#[tokio::test]
async fn usage_balances_zhipu_rejects_invalid_balance_response() {
    use UsageQueryErrorCode::*;

    let client = client_builder().no_proxy().build().unwrap();
    for (status, body, expected) in [
        ("403 Forbidden", "{}", UpstreamUnauthorized),
        ("200 OK", r#"{"success":false,"data":{"balance":15}}"#, UpstreamRejected),
        ("200 OK", r#"{"success":true,"data":null}"#, InvalidResponse),
        ("200 OK", r#"{"data":{"availableBalance":"inf","balance":15}}"#, InvalidResponse),
    ] {
        let (origin, server) =
            serve(vec![reply(PLAN, "200 OK", NO_PLAN), reply(BALANCE, status, body)]).await;
        assert_eq!(query_at(&client, &origin, "CNY", KEY).await, Err(expected));
        assert_requests_complete(server).await;
    }
}
