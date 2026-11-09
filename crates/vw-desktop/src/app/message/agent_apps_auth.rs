//! Authorization center requests through the scoped management SDK.

use crate::app::{
    App, Message,
    message::SettingsMessage,
    state::agent_apps_auth::{Connection, Scope, ServiceAccounts},
};
use iced::Task;
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthReadError, GatewayClient,
    vw_api_types::agent_apps_auth::{AuthErrorCode, AuthService},
};

pub(crate) const SERVICES: [AuthService; 7] = [
    AuthService::ClaudeOfficial,
    AuthService::CodexOauth,
    AuthService::GithubCopilot,
    AuthService::XaiOauth,
    AuthService::KimiOauth,
    AuthService::AgyOauth,
    AuthService::DshAccount,
];

#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Host(String),
    Username(String),
    Password(String),
    Token(String),
    Connect,
    Refresh,
    Loaded { scope: Scope, generation: u64, rows: Vec<ServiceAccounts> },
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentAppsAuthEvent")
    }
}

pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::AgentAppsAuth(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let endpoint = crate::app::gateway_client_endpoint();
    let scope = Scope::from(&endpoint);
    let state = &mut app.agent_apps_auth;
    state.select_endpoint(&endpoint);

    if let Event::Loaded { scope: loaded_scope, generation, rows } = event {
        let failed = rows.iter().all(|row| row.accounts.is_err());
        if state.finish(&loaded_scope, generation, rows)
            && failed
            && app.show_system_settings
            && app.system_settings_tab == crate::app::components::system_settings::SystemTab::Auth
        {
            return app.show_error_toast("授权账号读取失败，请检查各服务的提示");
        }
        return Task::none();
    }

    // GatewayEndpoint currently uses HTTP. Keep management credentials on the direct loopback.
    if !state.loopback {
        return Task::none();
    }

    match event {
        Event::Open => {
            if state.pending.is_none() && state.form.is_none() {
                let mut form = state.connection.clone().unwrap_or_else(|| Connection {
                    username: "vibewindow".into(),
                    ..Connection::default()
                });
                let catalog = &app.apps_catalog;
                if state.connection.is_none()
                    && catalog.context.as_ref().is_some_and(|context| {
                        context.gateway == scope.gateway && context.tenant == scope.tenant
                    })
                {
                    form.host.clone_from(&catalog.host);
                    form.token.clone_from(&catalog.token);
                }
                state.form = Some(form);
            }
        }
        Event::Close => state.form = None,
        Event::Host(value) => {
            if let Some(form) = &mut state.form {
                form.host = value;
            }
        }
        Event::Username(value) => {
            if let Some(form) = &mut state.form {
                form.username = value;
            }
        }
        Event::Password(value) => {
            if let Some(form) = &mut state.form {
                form.password = value;
            }
        }
        Event::Token(value) => {
            if let Some(form) = &mut state.form {
                form.token = value;
            }
        }
        Event::Connect | Event::Refresh => {
            if state.pending.is_some() {
                return Task::none();
            }
            if matches!(event, Event::Connect) {
                if !state.form.as_ref().is_some_and(Connection::valid) {
                    return Task::none();
                }
                state.connection = state.form.take();
            }
            let Some(connection) = state.connection.clone() else {
                return Task::none();
            };
            let Ok(client) = GatewayClient::new(endpoint) else {
                return app.show_error_toast("无法建立管理连接，请重试");
            };
            let generation = state.begin();
            return Task::perform(
                async move {
                    let credentials = AgentAppsAuthCredentials {
                        username: connection.username.trim(),
                        password: &connection.password,
                        principal_token: connection.token.trim(),
                    };
                    load(&client, connection.host.trim(), &credentials).await
                },
                move |rows| message(Event::Loaded { scope, generation, rows }),
            );
        }
        Event::Loaded { .. } => unreachable!("loaded results handled above"),
    }
    Task::none()
}

pub(crate) async fn load(
    client: &GatewayClient,
    host: &str,
    credentials: &AgentAppsAuthCredentials<'_>,
) -> Vec<ServiceAccounts> {
    iced::futures::future::join_all(SERVICES.map(|service| async move {
        let accounts = if client.endpoint().is_loopback() {
            client
                .agent_apps_auth_accounts(host, service, credentials)
                .await
                .map_err(read_error_label)
                .and_then(|accounts| {
                    if accounts.iter().any(|account| account.service != service) {
                        Err("授权账号响应与所选服务不符")
                    } else {
                        Ok(accounts)
                    }
                })
        } else {
            Err("授权中心需要本机回环管理连接")
        };
        ServiceAccounts { service, accounts }
    }))
    .await
}

fn read_error_label(error: AgentAppsAuthReadError) -> &'static str {
    use AuthErrorCode::*;
    match error {
        AgentAppsAuthReadError::Http { error: Some(error), .. } => match error.code {
            InvalidInput => "执行主机或授权查询参数无效",
            Unauthenticated => "管理凭据或主体令牌无效",
            Forbidden => "当前主体没有此服务的账号读取权限",
            NotFound => "找不到所选执行主机或授权资源",
            Unsupported => "执行主机不支持此服务的账号读取",
            ExecutionChannelUnavailable => "执行主机没有可用的授权通道",
            ExecutionHostUnavailable => "授权执行主机不可用",
            KeychainUnavailable => "执行主机的钥匙串不可用",
            StorageUnavailable => "执行主机的授权存储不可用",
            AccountInUse => "账号正在被执行主机使用，请稍后重试",
            _ => "此服务的账号状态暂不可用",
        },
        AgentAppsAuthReadError::Http { status: 401 | 403, .. } => "管理认证被拒绝，请检查连接凭据",
        AgentAppsAuthReadError::Http { .. } => "网关拒绝了账号查询，请检查管理连接",
        AgentAppsAuthReadError::InvalidPrincipalToken => "主体令牌格式无效",
        AgentAppsAuthReadError::Decode => "网关返回了无效的授权账号响应",
        AgentAppsAuthReadError::Transport => "无法连接管理网关，请重试",
    }
}
