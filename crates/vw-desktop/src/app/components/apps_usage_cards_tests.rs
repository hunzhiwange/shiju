use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};

const BINDING: &str = r#"{"bindings":[{"provider_id":"provider-a","service":"codex_oauth","account_id":"account-a"}]}"#;
const SNAPSHOT: &str = r#"{
    "capabilities":["official_subscription"],
    "snapshot":{"result":{
        "source":{"kind":"official","id":"codex"},
        "credentialState":"valid",
        "outcome":{"status":"success","data":{"kind":"official_subscription","value":{
            "subscriptionType":"Pro","windows":[{"name":"5 小时","utilization":25}]
        }}}
    },"stale":false}
}"#;
const EMPTY: &str = r#"{"capabilities":["official_subscription"],"snapshot":null}"#;
const SELECTOR: &str = r#""appId":"codex","providerId":"provider-a","accountId":"account-a","kind":"official_subscription""#;

fn context() -> QueryContext {
    QueryContext {
        project: "project-a".into(),
        host: "local".into(),
        token: "fixture-token".into(),
        connection: Some(Connection {
            host: "local".into(),
            username: "fixture".into(),
            password: "fixture-password".into(),
            token: "fixture-token".into(),
        }),
    }
}

fn gateway() -> (TcpListener, GatewayClient) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    (listener, GatewayClient::new(endpoint).unwrap())
}

fn request(listener: &TcpListener, expected: &str) -> (TcpStream, String, String) {
    let (mut stream, _) = listener.accept().unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with(expected), "{header}");
    let lower = header.to_ascii_lowercase();
    assert!(lower.contains("x-agent-apps-token: fixture-token\r\n"));
    let len = lower
        .lines()
        .find_map(|line| {
            line.strip_prefix("content-length:").map(|value| value.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; len];
    stream.read_exact(&mut body).unwrap();
    (stream, header, String::from_utf8(body).unwrap())
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}

fn assert_snapshot_selection(header: &str) {
    for field in [
        "projectId=project-a",
        "targetHostId=local",
        "appId=codex",
        "providerId=provider-a",
        "accountId=account-a",
        "kind=official_subscription",
    ] {
        assert!(header.contains(field), "missing {field}");
    }
}

#[tokio::test]
async fn usage_usage_cards_returns_expected_result() {
    let (listener, client) = gateway();
    let server = std::thread::spawn(move || {
        for account_view in [false, true] {
            let (mut stream, header, _) = request(&listener, "GET /v1/agent-apps/auth/bindings?");
            assert_eq!(header.contains("account_id=account-a"), account_view);
            respond(&mut stream, "200 OK", BINDING);
            let (mut stream, header, _) = request(&listener, "GET /v1/apps/usage/snapshot?");
            assert_snapshot_selection(&header);
            respond(&mut stream, "200 OK", SNAPSHOT);
        }
    });
    let context = context();
    let provider =
        read(&client, &context, &Target::Provider(AppId::Codex, "provider-a".into()), None)
            .await
            .unwrap();
    let account = read(
        &client,
        &context,
        &Target::Account(AuthService::CodexOauth, "account-a".into()),
        None,
    )
    .await
    .unwrap();
    assert_eq!(provider, account);
    assert_eq!(view::summary(&account[0].snapshot), ["官方额度", "Pro", "5 小时 · 剩余 75%"]);
    server.join().unwrap();
}

#[test]
fn usage_usage_cards_rejects_invalid_input() {
    let first = Target::Account(AuthService::CodexOauth, "account-a".into());
    let second = Target::Account(AuthService::CodexOauth, "account-b".into());
    let mut cards = Cards::default();
    let old = cards.begin(first.clone());
    cards.clear();
    let current = cards.begin(second.clone());
    assert!(!cards.finish(&first, old, Err(Failure::Unavailable)));
    assert!(!cards.finish(&second, old, Err(Failure::Unavailable)));
    assert!(cards.finish(&second, current, Ok(Vec::new())));
    assert!(cards.entries[&second].result.as_ref().unwrap().is_ok());
    assert!(!cards.entries.contains_key(&first));
}

#[tokio::test]
async fn usage_usage_cards_refreshes_empty_snapshot_after_gateway_completion() {
    let (listener, client) = gateway();
    let server = std::thread::spawn(move || {
        let (mut stream, _, _) = request(&listener, "GET /v1/agent-apps/auth/bindings?");
        respond(&mut stream, "200 OK", BINDING);
        let (mut stream, _, _) = request(&listener, "GET /v1/apps/usage/snapshot?");
        respond(&mut stream, "200 OK", EMPTY);

        let (mut stream, _, _) = request(&listener, "GET /v1/agent-apps/auth/bindings?");
        respond(&mut stream, "200 OK", BINDING);
        let (mut stream, _, body) = request(&listener, "POST /v1/apps/usage/refresh ");
        assert!(body.contains(SELECTOR));
        respond(&mut stream, "202 Accepted", r#"{"operationId":"op-1","status":"queued"}"#);
        let (mut stream, _, _) = request(&listener, "GET /v1/agent-apps/operations/op-1?");
        respond(
            &mut stream,
            "200 OK",
            r#"{"operationId":"op-1","status":"running","phase":"executing","projectId":"project-a","targetHostId":"local","correlationId":"op-1","cancelRequested":false,"items":[]}"#,
        );
        let (mut stream, _, _) = request(&listener, "GET /v1/agent-apps/operations/op-1/events?");
        respond(
            &mut stream,
            "200 OK",
            r#"{"operationId":"op-1","status":"completed","phase":"terminal","projectId":"project-a","targetHostId":"local","correlationId":"op-1","cancelRequested":false,"items":[{"appId":"codex","providerId":"provider-a","accountId":"account-a","kind":"official_subscription","status":"completed"}]}"#,
        );
        let (mut stream, header, _) = request(&listener, "GET /v1/apps/usage/snapshot?");
        assert_snapshot_selection(&header);
        respond(&mut stream, "200 OK", SNAPSHOT);
    });
    let context = context();
    let target = Target::Account(AuthService::CodexOauth, "account-a".into());
    let empty = read(&client, &context, &target, None).await.unwrap();
    assert_eq!(view::summary(&empty[0].snapshot), ["暂无额度快照"]);
    let refreshed = read(&client, &context, &target, Some("provider-a")).await.unwrap();
    assert_eq!(refreshed[0].refresh_failure, None);
    assert_eq!(view::summary(&refreshed[0].snapshot), ["官方额度", "Pro", "5 小时 · 剩余 75%"]);
    server.join().unwrap();
}

#[tokio::test]
async fn usage_usage_cards_distinguishes_unsupported_refresh_from_empty_snapshot() {
    let (listener, client) = gateway();
    let server = std::thread::spawn(move || {
        let (mut stream, header, _) = request(&listener, "GET /v1/agent-apps/auth/bindings?");
        assert!(header.contains("service=kimi_oauth"));
        respond(
            &mut stream,
            "200 OK",
            r#"{"bindings":[{"provider_id":"provider-a","service":"kimi_oauth","account_id":"account-a"}]}"#,
        );
        let (mut stream, _, body) = request(&listener, "POST /v1/apps/usage/refresh ");
        assert!(
            body.contains(r#""appId":"kimi","providerId":"provider-a","accountId":"account-a""#)
        );
        respond(
            &mut stream,
            "501 Not Implemented",
            r#"{"code":"unsupported","message":"diagnostic body","correlation_id":"fixture"}"#,
        );
        let (mut stream, header, _) = request(&listener, "GET /v1/apps/usage/snapshot?");
        assert!(header.contains("appId=kimi"));
        respond(&mut stream, "200 OK", EMPTY);
    });
    let result = read(
        &client,
        &context(),
        &Target::Account(AuthService::KimiOauth, "account-a".into()),
        Some("provider-a"),
    )
    .await
    .unwrap();
    assert_eq!(result[0].refresh_failure, Some(Failure::Unsupported));
    assert_eq!(view::summary(&result[0].snapshot), ["暂无额度快照"]);
    server.join().unwrap();
}
