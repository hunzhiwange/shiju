use crate::app::{
    App, Message,
    components::system_settings::SystemTab,
    message::{
        SettingsMessage,
        agent_apps_auth::{Event, update_at},
    },
    state::{
        AgentAppsContext, ToastKind,
        agent_apps_auth::Connection,
        app_providers::{
            claude::Draft,
            edit::{EditForm, Mode},
            list::Scope,
        },
    },
};
use iced::{Task, futures::StreamExt};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};
use vw_gateway_client::{
    GatewayEndpoint,
    vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthService, id::TenantScope},
};

const HOST: &str = "fixture-mac";
const PROVIDER: &str = "official-claude";
const TOKEN: &str = "fixture-auth-binding-token";
const TIMEOUT: Duration = Duration::from_secs(5);
const LOAD: &str =
    "GET /v1/agent-apps/auth/accounts?host_id=fixture-mac&service=claude_official HTTP/1.1";
const SAVE: &str = "PUT /v1/agent-apps/auth/bindings/official-claude?host_id=fixture-mac&service=claude_official HTTP/1.1";

fn prepare(app: &mut App, endpoint: &GatewayEndpoint) {
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Apps;
    app.apps_catalog.host = HOST.into();
    app.apps_catalog.token = TOKEN.into();
    app.app_providers.scope = Some(Scope {
        context: AgentAppsContext {
            gateway: endpoint.base_url(),
            tenant: endpoint.tenant_scope().clone(),
            project: app.project_path.clone(),
        },
        host: HOST.into(),
        app: AppId::Claude,
    });
    let mut form = EditForm::new(
        Mode::Update,
        PROVIDER.into(),
        serde_json::from_value(json!({"name":"官方供应商 · 中文"})).unwrap(),
        None,
    );
    form.claude = Some(Draft::new());
    app.app_providers.form = Some(form);
    app.agent_apps_auth.select_endpoint(endpoint);
    app.agent_apps_auth.connection = Some(Connection {
        host: HOST.into(),
        username: "admin".into(),
        password: "fixture".into(),
        token: TOKEN.into(),
    });
}

fn drive(app: &mut App, endpoint: &GatewayEndpoint, task: Task<Message>) {
    let mut stream = iced_runtime::task::into_stream(task).expect("a scoped gateway task");
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
                    panic!("missing authorization result");
                })
                .await
                .unwrap()
            },
        );
    let _ = update_at(app, event, endpoint.clone());
}

fn serve(socket: &mut TcpStream, expected: &str, status: &str, body: &str) {
    socket.set_read_timeout(Some(TIMEOUT)).unwrap();
    let mut reader = BufReader::new(&mut *socket);
    let mut request = String::new();
    reader.read_line(&mut request).unwrap();
    assert_eq!(request.trim_end(), expected);
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        if line == "\r\n" {
            break;
        }
        headers.push(line.trim_end().to_ascii_lowercase());
    }
    for header in [
        "x-platform-id: platform-a",
        "x-company-id: company-a",
        "x-agent-apps-token: fixture-auth-binding-token",
        "authorization: basic ywrtaw46zml4dhvyzq==",
    ] {
        assert!(headers.iter().any(|actual| actual == header), "missing fixture header {header}");
    }
    if expected == SAVE {
        let len = headers.iter().find_map(|h| h.strip_prefix("content-length: ")).unwrap();
        let mut input = vec![0; len.parse().unwrap()];
        reader.read_exact(&mut input).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&input).unwrap(), json!({"account_id":"ready"}));
    }
    write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}

fn gateway() -> (GatewayEndpoint, TcpListener) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_tenant_scope(TenantScope::new("platform-a", "company-a").unwrap());
    (endpoint, listener)
}

fn binding_case(app: &mut App, endpoint: &GatewayEndpoint, listener: TcpListener, reject: bool) {
    let provider_generation = app.app_providers.generation;
    let open = update_at(
        app,
        Event::OpenBinding { provider_id: PROVIDER.into(), provider_generation },
        endpoint.clone(),
    );
    assert_eq!(
        app.system_settings_tab,
        SystemTab::Auth,
        "supplier editor must open the authorization center"
    );
    let binding = app.agent_apps_auth.binding.as_ref().expect("supplier must open the picker");
    assert_eq!(binding.service, AuthService::ClaudeOfficial);
    let generation = binding.generation;
    let server = std::thread::spawn(move || {
        let accounts = json!({"accounts":[
            {"service":"claude_official","account_id":"ready","label":"可用中文账号","credential_state":"ready","is_default":false},
            {"service":"claude_official","account_id":"expired","label":"需重登","credential_state":"reauth_required","is_default":false}
        ]}).to_string();
        serve(&mut listener.accept().unwrap().0, LOAD, "200 OK", &accounts);
        let (status, response) = if reject {
            (
                "409 Conflict",
                r#"{"code":"reauth_required","message":"private-upstream-body","correlation_id":"private-correlation-id"}"#,
            )
        } else {
            ("204 No Content", "")
        };
        serve(&mut listener.accept().unwrap().0, SAVE, status, response);
    });
    drive(app, endpoint, open);
    let binding = app.agent_apps_auth.binding.as_ref().unwrap();
    assert_eq!(
        binding.accounts.iter().map(|a| a.account_id.as_str()).collect::<Vec<_>>(),
        ["ready"]
    );
    for invalid in ["expired", "foreign-account"] {
        let _ = update_at(app, Event::SelectBinding(generation, invalid.into()), endpoint.clone());
        assert!(app.agent_apps_auth.binding.as_ref().unwrap().selected.is_none());
        let invalid_save = update_at(app, Event::SaveBinding(generation), endpoint.clone());
        assert!(iced_runtime::task::into_stream(invalid_save).is_none());
    }
    let _ = update_at(app, Event::SelectBinding(generation, "ready".into()), endpoint.clone());
    let save = update_at(app, Event::SaveBinding(generation), endpoint.clone());
    drive(app, endpoint, save);
    server.join().unwrap();

    if reject {
        let binding = app.agent_apps_auth.binding.as_ref().expect("rejection keeps picker open");
        assert!(!binding.saving);
        assert_eq!(binding.error, Some("保存失败：所选账号需要重新授权"));
        let toast = app.active_toast.as_ref().expect("save errors use the desktop toast");
        assert!(matches!(toast.kind, ToastKind::Error));
        assert_eq!(toast.message, "保存失败：所选账号需要重新授权");
        let another_company = endpoint
            .clone()
            .with_tenant_scope(TenantScope::new("platform-a", "company-b").unwrap());
        let stale = update_at(app, Event::SaveBinding(generation), another_company);
        assert!(iced_runtime::task::into_stream(stale).is_none());
        assert!(app.agent_apps_auth.binding.is_none());
    } else {
        assert!(app.agent_apps_auth.binding.is_none());
        assert!(matches!(app.active_toast.as_ref().unwrap().kind, ToastKind::Success));
    }
}

#[test]
fn auth6_ui_binding_commits_expected_scoped_result() {
    crate::app::config::with_editor_settings_gateway(|mut app, _, _| {
        let (endpoint, listener) = gateway();
        prepare(&mut app, &endpoint);
        binding_case(&mut app, &endpoint, listener, false);
    });
}

#[test]
fn auth6_ui_binding_rejects_invalid_or_stale_input() {
    crate::app::config::with_editor_settings_gateway(|mut app, _, _| {
        let (endpoint, listener) = gateway();
        prepare(&mut app, &endpoint);
        let _ = update_at(
            &mut app,
            Event::OpenBinding { provider_id: "stale-provider".into(), provider_generation: 0 },
            endpoint.clone(),
        );
        assert_eq!(app.system_settings_tab, SystemTab::Apps);
        assert!(matches!(
            app.active_toast.as_ref().expect("stale supplier must be rejected explicitly").kind,
            ToastKind::Error
        ));
        binding_case(&mut app, &endpoint, listener, true);
    });
}
