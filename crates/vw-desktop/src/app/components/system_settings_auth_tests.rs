use crate::app::{
    message::agent_apps_auth::{SERVICES, load},
    state::agent_apps_auth::{Connection, Scope, State},
};
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    time::Duration,
};
use vw_gateway_client::{
    AgentAppsAuthCredentials, GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps_auth::{AuthCredentialState, AuthService},
        id::TenantScope,
    },
};

const PRINCIPAL: &str = "fixture-agent-apps-auth-principal-token";

const CREDENTIALS: AgentAppsAuthCredentials<'_> =
    AgentAppsAuthCredentials { username: "admin", password: "fixture", principal_token: PRINCIPAL };

fn endpoint(port: u16, company: &str) -> GatewayEndpoint {
    GatewayEndpoint::new("127.0.0.1", port)
        .with_tenant_scope(TenantScope::new("fixture-platform", company).unwrap())
}

fn gateway(
    company: &'static str,
    denied: bool,
) -> (GatewayEndpoint, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let endpoint = endpoint(listener.local_addr().unwrap().port(), company);
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in SERVICES {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut head = BufReader::new(&socket);
            let mut request = String::new();
            head.read_line(&mut request).unwrap();
            let uri = request.split_whitespace().nth(1).unwrap();
            let service = uri.split("service=").nth(1).unwrap();
            assert_eq!(
                uri,
                format!("/v1/agent-apps/auth/accounts?host_id=fixture-host&service={service}")
            );
            assert!(request.starts_with("GET "));
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                if head.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                headers.push_str(&line);
            }
            for header in [
                "x-platform-id: fixture-platform".to_string(),
                format!("x-company-id: {company}"),
                format!("x-agent-apps-token: {PRINCIPAL}"),
                "cache-control: no-cache, no-store".to_string(),
            ] {
                assert!(headers.lines().any(|line| line.eq_ignore_ascii_case(&header)));
            }
            assert!(
                headers.lines().any(|line| line == "authorization: Basic YWRtaW46Zml4dHVyZQ==")
            );
            requests.push(service.to_owned());
            let (status, body) = if denied {
                (
                    "403 Forbidden",
                    json!({"code": "forbidden", "message": "fixture-private-detail", "correlation_id": "fixture-id"}),
                )
            } else if service == "agy_oauth" {
                (
                    "501 Not Implemented",
                    json!({"code": "execution_channel_unavailable", "message": "fixture-private-detail", "correlation_id": "fixture-id"}),
                )
            } else if service == "codex_oauth" {
                (
                    "200 OK",
                    json!({"accounts": [
                        {"service": service, "account_id": "opaque-1", "label": "账***号", "credential_state": "ready", "is_default": true},
                        {"service": service, "account_id": "opaque-2", "label": "另***户", "credential_state": "reauth_required", "is_default": false}
                    ]}),
                )
            } else {
                ("200 OK", json!({"accounts": []}))
            };
            let body = body.to_string();
            write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    (endpoint, server)
}

#[tokio::test]
async fn auth6_ui_list_commits_expected_scoped_result() {
    let (endpoint, server) = gateway("company-a", false);
    let scope = Scope::from(&endpoint);
    let mut state = State::default();
    state.select(scope.clone());
    let generation = state.begin();
    let rows = load(&GatewayClient::new(endpoint).unwrap(), "fixture-host", &CREDENTIALS).await;
    assert!(state.finish(&scope, generation, rows));
    let rows = state.rows.unwrap();
    assert_eq!(rows.iter().map(|row| row.service).collect::<Vec<_>>(), SERVICES);
    let codex = rows.iter().find(|row| row.service == AuthService::CodexOauth).unwrap();
    let accounts = codex.accounts.as_ref().expect("authorized Codex accounts must load");
    assert_eq!(accounts[0].label, "账***号");
    assert!(accounts[0].is_default);
    assert_eq!(accounts[0].credential_state, AuthCredentialState::Ready);
    assert_eq!(accounts[1].credential_state, AuthCredentialState::ReauthRequired);
    let agy = rows.iter().find(|row| row.service == AuthService::AgyOauth).unwrap();
    assert_eq!(agy.accounts, Err("执行主机没有可用的授权通道"));
    assert!(rows[0].accounts.as_ref().unwrap().is_empty());
    assert_eq!(server.join().unwrap().len(), 7);
}

#[tokio::test]
async fn auth6_ui_list_rejects_invalid_or_stale_input() {
    let (endpoint_b, server) = gateway("company-b", true);
    let scope_a = Scope::from(&endpoint(endpoint_b.port, "company-a"));
    let scope_b = Scope::from(&endpoint_b);
    let mut state = State::default();
    state.select(scope_a.clone());
    let old = state.begin();
    assert!(state.finish(&scope_a, old, Vec::new()));
    state.connection =
        Some(Connection { token: "company-a-principal".into(), ..Connection::default() });
    let in_flight = state.begin();
    state.select(scope_b.clone());
    assert!(state.rows.is_none());
    assert!(state.connection.is_none());
    let current = state.begin();
    assert!(!state.finish(&scope_a, in_flight, Vec::new()));
    let rows = load(&GatewayClient::new(endpoint_b).unwrap(), "fixture-host", &CREDENTIALS).await;
    assert!(state.finish(&scope_b, current, rows));
    assert!(
        state
            .rows
            .as_ref()
            .unwrap()
            .iter()
            .all(|row| row.accounts == Err("当前主体没有此服务的账号读取权限"))
    );
    state.select(scope_a.clone());
    state.begin();
    assert!(!state.finish(&scope_a, in_flight, Vec::new()));
    assert_eq!(server.join().unwrap().len(), 7);

    let remote = GatewayClient::new(GatewayEndpoint::new("192.0.2.1", 80)).unwrap();
    let rows = load(&remote, "fixture-host", &CREDENTIALS).await;
    assert!(rows.iter().all(|row| row.accounts == Err("授权中心需要本机回环管理连接")));

    let unconfigured = GatewayClient::new(endpoint(1, "company-a")).unwrap();
    let invalid = AgentAppsAuthCredentials { principal_token: "invalid\nheader", ..CREDENTIALS };
    let rows = load(&unconfigured, "fixture-host", &invalid).await;
    assert!(rows.iter().all(|row| row.accounts == Err("主体令牌格式无效")));
}
