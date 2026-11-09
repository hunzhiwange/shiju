use crate::app::{
    message::agent_apps_auth::flow::{Update, run},
    state::agent_apps_auth::{Connection, LoginPhase, LoginTarget, Scope, State},
};
use iced::futures::{StreamExt, channel::mpsc};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint,
    vw_api_types::{TimestampMs, agent_apps_auth::AuthRegion, id::TenantScope},
};

const FLOW: &str = "743b63aa-a8b8-4c1d-8c99-40324b431d92";
const NEXT: &str = "7cb10748-4eb9-4424-a67d-d4ec08509645";
const PRINCIPAL: &str = "fixture-agent-apps-auth-principal-token";
const DEADLINE: i64 = 7_100;

pub(in crate::app) struct Exchange {
    method: &'static str,
    suffix: String,
    request: Option<Value>,
    response: Option<Value>,
    status: &'static str,
    pause: Option<(oneshot::Sender<()>, oneshot::Receiver<()>)>,
}

impl Exchange {
    fn start(flow: &str, region: Option<&str>, verification_url: &str) -> Self {
        let mut request = json!({"method": "device_code"});
        if let Some(region) = region {
            request["region"] = json!(region);
        }
        Self {
            method: "POST",
            suffix: String::new(),
            request: Some(request),
            response: Some(json!({
                "state": "pending",
                "flow_id": flow,
                "expires_at": DEADLINE,
                "retry_after_seconds": 1,
                "user_code": "ABCD-2345",
                "verification_url": verification_url,
            })),
            status: "201 Created",
            pause: None,
        }
    }

    pub(in crate::app) fn claude_start(
        flow: &str,
        method: &str,
        url: &str,
        expires_at: i64,
    ) -> Self {
        Self {
            method: "POST",
            suffix: String::new(),
            request: Some(json!({"method": method})),
            response: Some(json!({
                "state": "pending", "flow_id": flow, "expires_at": expires_at,
                "retry_after_seconds": 1, "verification_url": url, "can_paste": true,
            })),
            status: "201 Created",
            pause: None,
        }
    }

    pub(in crate::app) fn paste(flow: &str, pasted: &str, accepted: bool) -> Self {
        Self {
            method: "POST",
            suffix: format!("/{flow}/paste"),
            request: Some(json!({"pasted": pasted})),
            response: (!accepted).then(|| json!({
                "code": "invalid_input", "message": "fixture upstream diagnostic", "correlation_id": "fixture-correlation"
            })),
            status: if accepted { "202 Accepted" } else { "400 Bad Request" },
            pause: None,
        }
    }

    pub(in crate::app) fn poll(response: Value) -> Self {
        Self {
            method: "GET",
            suffix: format!("/{FLOW}"),
            request: None,
            response: Some(response),
            status: "200 OK",
            pause: None,
        }
    }

    pub(in crate::app) fn cancel(flow: &'static str) -> Self {
        Self {
            method: "DELETE",
            suffix: format!("/{flow}"),
            request: None,
            response: None,
            status: "204 No Content",
            pause: None,
        }
    }
}

pub(in crate::app) fn gateway(
    service: &'static str,
    exchanges: Vec<Exchange>,
) -> (GatewayClient, Connection, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_tenant_scope(TenantScope::new("fixture-platform", "fixture-company").unwrap());
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        for exchange in exchanges {
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("expected {} gateway request: {error}", exchange.method),
                }
            };
            socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut reader = BufReader::new(&socket);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let suffix = exchange.suffix;
            assert_eq!(
                line.trim_end(),
                format!(
                    "{} /v1/agent-apps/auth/flows{suffix}?host_id=fixture-host&service={service} HTTP/1.1",
                    exchange.method
                ),
            );
            let mut headers = String::new();
            loop {
                line.clear();
                assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line.to_ascii_lowercase());
            }
            for header in [
                "x-platform-id: fixture-platform".to_owned(),
                "x-company-id: fixture-company".to_owned(),
                format!("x-agent-apps-token: {PRINCIPAL}"),
                "authorization: basic ywrtaw46zml4dhvyzq==".to_owned(),
                "cache-control: no-cache, no-store".to_owned(),
            ] {
                assert!(headers.lines().any(|line| line == header));
            }
            let length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .map_or(0, |value| value.parse().unwrap());
            let mut payload = vec![0; length];
            reader.read_exact(&mut payload).unwrap();
            if let Some(expected) = exchange.request {
                assert_eq!(serde_json::from_slice::<Value>(&payload).unwrap(), expected);
            } else {
                assert!(payload.is_empty());
            }
            if let Some((received, resume)) = exchange.pause {
                received.send(()).unwrap();
                resume.blocking_recv().unwrap();
            }
            let body = exchange.response.map_or(String::new(), |value| value.to_string());
            write!(
                socket,
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                exchange.status,
                body.len()
            )
            .unwrap();
        }
    });
    let connection = Connection {
        host: "fixture-host".into(),
        username: "admin".into(),
        password: "fixture".into(),
        token: PRINCIPAL.into(),
    };
    (GatewayClient::new(endpoint).unwrap(), connection, server)
}

pub(in crate::app) async fn next(updates: &mut mpsc::Receiver<Update>) -> Update {
    tokio::time::timeout(Duration::from_secs(8), updates.next())
        .await
        .expect("device update timed out")
        .expect("missing device update")
}

#[tokio::test]
async fn auth6_ui_login_commits_expected_scoped_result() {
    let (client, connection, server) = gateway(
        "github_copilot",
        vec![
            Exchange::start(FLOW, None, "https://github.example:8443/login/device"),
            Exchange::poll(json!({
                "state": "pending", "flow_id": FLOW, "expires_at": DEADLINE,
                "retry_after_seconds": 1,
            })),
            Exchange::poll(json!({
                "state": "approved", "flow_id": FLOW, "expires_at": DEADLINE,
                "account": {"account_id": "opaque", "service": "github_copilot",
                    "label": "账***号", "credential_state": "ready", "is_default": true},
            })),
        ],
    );
    let mut state = State::default();
    state.select_endpoint(client.endpoint());
    let (generation, cancelled, pastes) = state.begin_login(LoginTarget::Copilot);
    let (sender, mut updates) = mpsc::channel(4);
    let task = tokio::spawn(run(
        client,
        connection,
        LoginTarget::Copilot,
        false,
        cancelled,
        pastes,
        sender,
    ));
    let Update::Started(challenge) = next(&mut updates).await else {
        panic!("expected a device challenge");
    };
    assert_eq!(challenge.user_code.as_deref(), Some("ABCD-2345"));
    assert_eq!(challenge.detail(), ("授权域", "github.example:8443"));
    assert_eq!(challenge.remaining_seconds(TimestampMs(3_200)), 4);
    assert_eq!(challenge.remaining_seconds(TimestampMs(7_101)), 0);
    state.login_mut(generation).unwrap().phase = LoginPhase::Pending(challenge);
    assert!(matches!(next(&mut updates).await, Update::Pending(TimestampMs(DEADLINE))));
    assert!(matches!(next(&mut updates).await, Update::Approved));
    task.await.unwrap();
    server.join().unwrap();
}

#[tokio::test]
async fn auth6_ui_login_rejects_invalid_or_stale_input() {
    let (received_tx, received) = oneshot::channel();
    let (resume, resume_rx) = oneshot::channel();
    let mut delayed = Exchange::start(FLOW, Some("mainland-cn"), "https://kimi.example/device");
    delayed.pause = Some((received_tx, resume_rx));
    let (client, connection, server) = gateway(
        "kimi_oauth",
        vec![
            delayed,
            Exchange::cancel(FLOW),
            Exchange::start(NEXT, Some("mainland-cn"), "http://kimi.example/device"),
            Exchange::cancel(NEXT),
        ],
    );
    let mut state = State::default();
    state.select_endpoint(client.endpoint());
    let scope = Scope::from(client.endpoint());
    let target = LoginTarget::Kimi(AuthRegion::MainlandCn);
    let (old_generation, old_cancelled, old_pastes) = state.begin_login(target);
    let (sender, mut updates) = mpsc::channel(4);
    let old_task = tokio::spawn(run(
        client.clone(),
        connection.clone(),
        target,
        false,
        old_cancelled,
        old_pastes,
        sender,
    ));
    tokio::time::timeout(Duration::from_secs(5), received).await.unwrap().unwrap();

    state.login = None;
    let (generation, cancelled, pastes) = state.begin_login(target);
    resume.send(()).unwrap();
    let Update::Started(old_challenge) = next(&mut updates).await else {
        panic!("expected delayed challenge");
    };
    assert_eq!(old_challenge.detail(), ("授权地区", "中国大陆"));
    assert!(!state.is_latest_login_update(&scope, old_generation));
    assert!(state.login_mut(old_generation).is_none());
    assert!(matches!(state.login_mut(generation).unwrap().phase, LoginPhase::Starting));
    old_task.await.unwrap();

    let (sender, mut updates) = mpsc::channel(4);
    let task = tokio::spawn(run(client, connection, target, false, cancelled, pastes, sender));
    assert!(matches!(next(&mut updates).await, Update::Failed("网关返回了无效的设备码或验证地址")));
    task.await.unwrap();
    server.join().unwrap();
}
