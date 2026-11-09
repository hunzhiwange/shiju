use super::{Event, actions::ActionEvent, update_at};
use crate::app::{
    App, Message,
    components::system_settings::SystemTab,
    message::SettingsMessage,
    state::{
        ToastKind,
        agent_apps_auth::{AccountOperation, ActionPhase, Connection, ServiceAccounts},
    },
};
use iced::{Task, futures::StreamExt};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
use vw_gateway_client::{
    GatewayEndpoint,
    vw_api_types::{
        agent_apps_auth::{AuthAccount, AuthCredentialState, AuthService},
        id::TenantScope,
    },
};

const TIMEOUT: Duration = Duration::from_secs(5);
const BINDINGS: &str = "GET /v1/agent-apps/auth/bindings?host_id=fixture-mac&service=codex_oauth&account_id=account-a HTTP/1.1";
const DELETE: &str = "DELETE /v1/agent-apps/auth/accounts/account-a?host_id=fixture-mac&service=codex_oauth HTTP/1.1";
const CODEX_ACCOUNTS: &str =
    "GET /v1/agent-apps/auth/accounts?host_id=fixture-mac&service=codex_oauth HTTP/1.1";
const DSH_ACCOUNTS: &str =
    "GET /v1/agent-apps/auth/accounts?host_id=fixture-mac&service=dsh_account HTTP/1.1";
const DSH_STATUS: &str =
    "GET /v1/agent-apps/auth/status?host_id=fixture-mac&service=dsh_account HTTP/1.1";
const DSH_REVOKE: &str = "POST /v1/agent-apps/auth/accounts/account-a/revoke?host_id=fixture-mac&service=dsh_account HTTP/1.1";

fn fixture(app: &mut App) -> (GatewayEndpoint, TcpListener) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_tenant_scope(TenantScope::new("platform-a", "company-a").unwrap());
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Auth;
    let state = &mut app.agent_apps_auth;
    state.select_endpoint(&endpoint);
    state.connection = Some(Connection {
        host: "fixture-mac".into(),
        username: "admin".into(),
        password: "fixture".into(),
        token: "fixture-auth-actions-token".into(),
    });
    state.rows = Some(
        [AuthService::CodexOauth, AuthService::DshAccount]
            .map(|service| ServiceAccounts { service, accounts: Ok(vec![account(service)]) })
            .to_vec(),
    );
    (endpoint, listener)
}

fn account(service: AuthService) -> AuthAccount {
    AuthAccount {
        service,
        account_id: "account-a".into(),
        label: "托管中文账号".into(),
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: None,
        login_expires_at: None,
        github_domain: None,
        region: None,
    }
}

fn status(service: AuthService, can_set_default: bool, can_revoke: bool) -> Value {
    json!({"service":service,"default_account_id":null,"capabilities":{
        "login_methods":[], "can_set_default":can_set_default, "can_bind":false, "can_revoke":can_revoke
    }})
}

fn response(
    listener: &TcpListener,
    request: &str,
    request_body: Option<Value>,
    status: &str,
    body: Value,
) {
    let deadline = Instant::now() + TIMEOUT;
    let mut socket = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("missing request {request}: {error}"),
        }
    };
    socket.set_nonblocking(false).unwrap();
    socket.set_read_timeout(Some(TIMEOUT)).unwrap();
    let mut reader = BufReader::new(&mut socket);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert_eq!(line.trim_end(), request);
    let mut headers = Vec::new();
    loop {
        line.clear();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        if line == "\r\n" {
            break;
        }
        headers.push(line.trim_end().to_ascii_lowercase());
    }
    for header in [
        "x-platform-id: platform-a",
        "x-company-id: company-a",
        "x-agent-apps-token: fixture-auth-actions-token",
        "authorization: basic ywrtaw46zml4dhvyzq==",
    ] {
        assert!(headers.iter().any(|actual| actual == header), "missing fixture header {header}");
    }
    if let Some(expected) = request_body {
        let len = headers.iter().find_map(|h| h.strip_prefix("content-length: ")).unwrap();
        let mut input = vec![0; len.parse().unwrap()];
        reader.read_exact(&mut input).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&input).unwrap(), expected);
    }
    if status == "disconnect" {
        return;
    }
    let body = if status == "204 No Content" { String::new() } else { body.to_string() };
    write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}

fn drive(app: &mut App, endpoint: &GatewayEndpoint, task: Task<Message>) -> Task<Message> {
    let mut stream = iced_runtime::task::into_stream(task).expect("a scoped account action task");
    let event =
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
            async {
                tokio::time::timeout(TIMEOUT, async {
                    while let Some(action) = stream.next().await {
                        if let iced_runtime::Action::Output(Message::Settings(
                            SettingsMessage::AgentAppsAuth(event),
                        )) = action
                        {
                            return event;
                        }
                    }
                    panic!("missing scoped account action result");
                })
                .await
                .unwrap()
            },
        );
    update_at(app, event, endpoint.clone())
}

fn update(app: &mut App, endpoint: &GatewayEndpoint, event: ActionEvent) -> Task<Message> {
    update_at(app, Event::AccountAction(event), endpoint.clone())
}

fn open(
    app: &mut App,
    endpoint: &GatewayEndpoint,
    service: AuthService,
    operation: AccountOperation,
) -> Task<Message> {
    update(
        app,
        endpoint,
        ActionEvent::Open {
            rows_generation: app.agent_apps_auth.rows_generation(),
            operation,
            service,
            account_id: "account-a".into(),
        },
    )
}

fn generation(app: &App) -> u64 {
    app.agent_apps_auth
        .account_action
        .as_ref()
        .expect("account action opens a scoped dialog")
        .generation
}

fn toast(app: &App, expected: &str, success: bool) {
    let toast = app.active_toast.as_ref().expect("account action shows a desktop toast");
    assert_eq!(toast.message, expected);
    assert_eq!(matches!(toast.kind, ToastKind::Success), success);
}

fn confirm(app: &mut App, endpoint: &GatewayEndpoint) -> Task<Message> {
    update(app, endpoint, ActionEvent::Confirm(generation(app)))
}

fn complete(app: &mut App, endpoint: &GatewayEndpoint, expected: &str, success: bool) {
    let action = confirm(app, endpoint);
    let refresh = drive(app, endpoint, action);
    toast(app, expected, success);
    assert!(app.agent_apps_auth.account_action.is_none());
    let _ = drive(app, endpoint, refresh);
}

fn scoped_account(app: &App, service: AuthService) -> Option<&AuthAccount> {
    app.agent_apps_auth
        .rows
        .as_ref()?
        .iter()
        .find(|row| row.service == service)?
        .accounts
        .as_ref()
        .ok()?
        .first()
}

#[test]
fn auth6_ui_actions_commits_expected_scoped_result() {
    crate::app::config::with_editor_settings_gateway(|mut app, _, _| {
        let (endpoint, listener) = fixture(&mut app);
        let code = AuthService::CodexOauth;
        let dsh = AuthService::DshAccount;
        let task = open(&mut app, &endpoint, code, AccountOperation::SetDefault);
        assert!(
            matches!(
                app.agent_apps_auth.account_action.as_ref().map(|action| &action.phase),
                Some(ActionPhase::Checking)
            ),
            "setting the default must check the execution host's capability"
        );

        let server = thread::spawn(move || {
            let reply =
                |request, body, status, value| response(&listener, request, body, status, value);
            reply(
                "GET /v1/agent-apps/auth/status?host_id=fixture-mac&service=codex_oauth HTTP/1.1",
                None,
                "200 OK",
                status(code, true, false),
            );
            reply(
                "PUT /v1/agent-apps/auth/accounts/account-a/default?host_id=fixture-mac&service=codex_oauth HTTP/1.1",
                None,
                "204 No Content",
                Value::Null,
            );
            let mut codex = account(code);
            codex.is_default = true;
            reply(CODEX_ACCOUNTS, None, "200 OK", json!({"accounts":[codex]}));

            reply(
                "POST /v1/agent-apps/auth/logout?host_id=fixture-mac&service=codex_oauth HTTP/1.1",
                Some(json!({"selection":"account","account_id":"account-a"})),
                "204 No Content",
                Value::Null,
            );
            codex.is_default = false;
            codex.credential_state = AuthCredentialState::SignedOut;
            reply(CODEX_ACCOUNTS, None, "200 OK", json!({"accounts":[codex]}));

            reply(BINDINGS, None, "200 OK", json!({"bindings":[]}));
            reply(DELETE, None, "204 No Content", Value::Null);
            reply(CODEX_ACCOUNTS, None, "200 OK", json!({"accounts":[]}));

            reply(DSH_STATUS, None, "200 OK", status(dsh, false, true));
            reply(DSH_REVOKE, None, "204 No Content", Value::Null);
            let mut dsh_account = account(dsh);
            dsh_account.credential_state = AuthCredentialState::SignedOut;
            reply(DSH_ACCOUNTS, None, "200 OK", json!({"accounts":[dsh_account]}));
        });

        let _ = drive(&mut app, &endpoint, task);
        complete(&mut app, &endpoint, "已切换默认授权账号", true);
        assert!(scoped_account(&app, code).unwrap().is_default);

        let _ = open(&mut app, &endpoint, code, AccountOperation::Logout);
        complete(&mut app, &endpoint, "账号已在执行主机登出", true);
        assert_eq!(
            scoped_account(&app, code).unwrap().credential_state,
            AuthCredentialState::SignedOut
        );

        let check = open(&mut app, &endpoint, code, AccountOperation::Delete);
        let _ = drive(&mut app, &endpoint, check);
        complete(&mut app, &endpoint, "已删除授权账号", true);
        assert!(scoped_account(&app, code).is_none());

        let check = open(&mut app, &endpoint, dsh, AccountOperation::Revoke);
        let _ = drive(&mut app, &endpoint, check);
        complete(&mut app, &endpoint, "上游已确认撤销授权", true);
        assert_eq!(
            scoped_account(&app, dsh).unwrap().credential_state,
            AuthCredentialState::SignedOut
        );
        server.join().unwrap();
    });
}

#[test]
fn auth6_ui_actions_rejects_invalid_or_stale_input() {
    crate::app::config::with_editor_settings_gateway(|mut app, _, _| {
        let (endpoint, listener) = fixture(&mut app);
        let code = AuthService::CodexOauth;
        let dsh = AuthService::DshAccount;
        let invalid = ActionEvent::Open {
            rows_generation: app.agent_apps_auth.rows_generation(),
            operation: AccountOperation::Delete,
            service: code,
            account_id: "foreign-account".into(),
        };
        let _ = update(&mut app, &endpoint, invalid);
        toast(&app, "账号状态已变化，请刷新后重试", false);
        assert!(app.agent_apps_auth.account_action.is_none());

        let _ = open(&mut app, &endpoint, code, AccountOperation::Revoke);
        assert!(matches!(
            app.agent_apps_auth.account_action.as_ref().map(|action| &action.phase),
            Some(ActionPhase::Blocked("此服务不支持上游撤销，可单独执行本地登出"))
        ));
        let blocked = confirm(&mut app, &endpoint);
        assert!(iced_runtime::task::into_stream(blocked).is_none());
        let close = ActionEvent::Close(generation(&app));
        let _ = update(&mut app, &endpoint, close);

        let server = thread::spawn(move || {
            let reply = |request, status, value| response(&listener, request, None, status, value);
            response(
                &listener,
                "POST /v1/agent-apps/auth/logout?host_id=fixture-mac&service=codex_oauth HTTP/1.1",
                Some(json!({"selection":"account","account_id":"account-a"})),
                "disconnect",
                Value::Null,
            );
            reply(CODEX_ACCOUNTS, "200 OK", json!({"accounts":[account(code)]}));
            reply(
                BINDINGS,
                "200 OK",
                json!({"bindings":[{"provider_id":"供应商-中文","service":code,"account_id":"account-a"}]}),
            );
            reply(BINDINGS, "200 OK", json!({"bindings":[]}));
            reply(
                DELETE,
                "409 Conflict",
                json!({"code":"account_in_use","message":"private-upstream-body","correlation_id":"private-correlation-id"}),
            );
            reply(BINDINGS, "200 OK", json!({"bindings":[]}));
            reply(DELETE, "204 No Content", Value::Null);
            reply(CODEX_ACCOUNTS, "200 OK", json!({"accounts":[]}));

            reply(DSH_STATUS, "200 OK", status(dsh, false, true));
            reply(
                DSH_REVOKE,
                "502 Bad Gateway",
                json!({"code":"remote_revocation_unconfirmed","message":"private-upstream-body","correlation_id":"private-correlation-id"}),
            );
            reply(DSH_ACCOUNTS, "200 OK", json!({"accounts":[]}));
        });
        let old_rows = app.agent_apps_auth.rows_generation();
        let _ = open(&mut app, &endpoint, code, AccountOperation::Logout);
        complete(&mut app, &endpoint, "未收到本地登出的确认，请查看刷新后的账号状态", false);
        assert_eq!(
            scoped_account(&app, code).unwrap().credential_state,
            AuthCredentialState::Ready
        );
        let _ = update(
            &mut app,
            &endpoint,
            ActionEvent::Open {
                rows_generation: old_rows,
                operation: AccountOperation::Delete,
                service: code,
                account_id: "account-a".into(),
            },
        );
        toast(&app, "账号状态已变化，请刷新后重试", false);
        assert!(app.agent_apps_auth.account_action.is_none());

        let check = open(&mut app, &endpoint, code, AccountOperation::Delete);
        let _ = drive(&mut app, &endpoint, check);
        assert!(matches!(
            &app.agent_apps_auth.account_action.as_ref().unwrap().phase,
            ActionPhase::Ready(users) if users == &["供应商-中文"]
        ));
        let bound = confirm(&mut app, &endpoint);
        assert!(iced_runtime::task::into_stream(bound).is_none());

        let id = generation(&app);
        let review = update(&mut app, &endpoint, ActionEvent::Review(id));
        let _ = drive(&mut app, &endpoint, review);
        let remove = confirm(&mut app, &endpoint);
        let _ = drive(&mut app, &endpoint, remove);
        toast(&app, "删除账号失败。账号仍被供应商引用，请解除绑定后重试", false);
        assert!(matches!(
            app.agent_apps_auth.account_action.as_ref().map(|action| &action.phase),
            Some(ActionPhase::CheckFailed(_))
        ));
        let review = update(&mut app, &endpoint, ActionEvent::Review(id));
        let _ = drive(&mut app, &endpoint, review);
        complete(&mut app, &endpoint, "已删除授权账号", true);

        let check = open(&mut app, &endpoint, dsh, AccountOperation::Revoke);
        let _ = drive(&mut app, &endpoint, check);
        complete(&mut app, &endpoint, "账号已在执行主机登出，上游未确认撤销", false);
        server.join().unwrap();

        let other_company =
            endpoint.with_tenant_scope(TenantScope::new("platform-a", "company-b").unwrap());
        let stale = update(&mut app, &other_company, ActionEvent::Confirm(id));
        assert!(iced_runtime::task::into_stream(stale).is_none());
        assert!(app.agent_apps_auth.rows.is_none());
        assert!(app.agent_apps_auth.account_action.is_none());
    });
}
