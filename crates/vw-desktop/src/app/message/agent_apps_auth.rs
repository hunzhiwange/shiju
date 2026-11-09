//! Authorization center requests through the scoped management SDK.

#[path = "agent_apps_auth_actions.rs"]
pub(in crate::app) mod actions;
#[cfg(test)]
#[path = "agent_apps_auth_actions_tests.rs"]
mod actions_tests;
pub(in crate::app) mod binding;
#[path = "agent_apps_auth_browser.rs"]
pub(in crate::app) mod browser;
#[cfg(test)]
#[path = "agent_apps_auth_browser_tests.rs"]
mod browser_tests;
pub(in crate::app) mod flow;

use crate::app::{
    App, Message,
    components::system_settings::SystemTab,
    message::SettingsMessage,
    state::agent_apps_auth::{Connection, LoginPhase, LoginTarget, Scope, ServiceAccounts, now},
};
use browser::LinkAction;
use iced::Task;
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthReadError, GatewayClient,
    vw_api_types::agent_apps_auth::{AuthAccount, AuthErrorCode, AuthService},
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
    AccountAction(actions::ActionEvent),
    OpenBinding { provider_id: String, provider_generation: u64 },
    LoadBinding(u64),
    BindingLoaded(u64, Result<Vec<AuthAccount>, &'static str>),
    SelectBinding(u64, String),
    SaveBinding(u64),
    BindingSaved(u64, Result<(), &'static str>),
    CloseBinding(u64),
    Open,
    Close,
    Host(String),
    Username(String),
    Password(String),
    Token(String),
    Connect,
    Refresh,
    OpenBrowserPreference(bool),
    StartLogin(LoginTarget),
    RetryLogin,
    CloseLogin,
    LoginTick,
    LoginLink(u64, LinkAction),
    PasteInput(u64, String),
    PasteSubmit(u64),
    LoginUpdated { scope: Scope, generation: u64, update: flow::Update },
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
    update_at(app, event, endpoint)
}

pub(in crate::app) fn update_at(
    app: &mut App,
    event: Event,
    endpoint: vw_gateway_client::GatewayEndpoint,
) -> Task<Message> {
    let scope = Scope::from(&endpoint);
    let visible = app.show_system_settings && app.system_settings_tab == SystemTab::Auth;
    let state = &mut app.agent_apps_auth;
    state.select_endpoint(&endpoint);

    if let Event::AccountAction(action) = event {
        return actions::update(app, action, endpoint, scope, visible);
    }

    if matches!(
        event,
        Event::OpenBinding { .. }
            | Event::LoadBinding(_)
            | Event::BindingLoaded(..)
            | Event::SelectBinding(..)
            | Event::SaveBinding(_)
            | Event::BindingSaved(..)
            | Event::CloseBinding(_)
    ) {
        if state.account_action.is_some() {
            return Task::none();
        }
        return binding::update(app, event, endpoint);
    }

    if let Event::Loaded { scope: loaded_scope, generation, rows } = event {
        let failed = rows.iter().all(|row| row.accounts.is_err());
        if state.finish(&loaded_scope, generation, rows) && failed && visible {
            return app.show_error_toast("授权账号读取失败，请检查各服务的提示");
        }
        return Task::none();
    }

    if let Event::OpenBrowserPreference(open) = event {
        if visible {
            state.copy_login_link = !open;
            if let Err(error) = browser::store_preference(open) {
                return app.show_error_toast(error);
            }
        }
        return Task::none();
    }

    // GatewayEndpoint currently uses HTTP. Keep management credentials on the direct loopback.
    if !state.loopback {
        return Task::none();
    }

    match event {
        Event::AccountAction(_) => unreachable!("account action events handled above"),
        Event::OpenBinding { .. }
        | Event::LoadBinding(_)
        | Event::BindingLoaded(..)
        | Event::SelectBinding(..)
        | Event::SaveBinding(_)
        | Event::BindingSaved(..)
        | Event::CloseBinding(_) => unreachable!("binding events handled above"),
        Event::Open => {
            if visible
                && (state.pending.is_none() || state.binding.is_some())
                && state.form.is_none()
                && state.login.is_none()
                && state.account_action.is_none()
            {
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
                if let Some(binding) = &state.binding {
                    form.host.clone_from(&binding.host);
                    form.token.clone_from(&catalog.token);
                }
                state.form = Some(form);
            }
        }
        Event::Close => state.form = None,
        Event::Host(value) => {
            if state.binding.is_none()
                && let Some(form) = &mut state.form
            {
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
            if state.binding.is_none()
                && let Some(form) = &mut state.form
            {
                form.token = value;
            }
        }
        Event::Connect | Event::Refresh => {
            if !visible || state.account_action.is_some() {
                return Task::none();
            }
            if let Some(binding) = &state.binding {
                let generation = binding.generation;
                if matches!(event, Event::Connect)
                    && !binding.loading
                    && !binding.saving
                    && state.form.as_ref().is_some_and(|form| {
                        binding::connection_matches(binding, form, &app.apps_catalog.token)
                    })
                {
                    state.connection = state.form.take();
                    state.usage.clear();
                    state.rows = None;
                    state.pending = None;
                    return binding::update(app, Event::LoadBinding(generation), endpoint);
                }
                return Task::none();
            }
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
        Event::StartLogin(_) | Event::RetryLogin => {
            if !visible
                || state.form.is_some()
                || state.pending.is_some()
                || state.binding.is_some()
                || state.account_action.is_some()
            {
                return Task::none();
            }
            let target = match (&state.login, event) {
                (None, Event::StartLogin(target)) => target,
                (Some(login), Event::RetryLogin)
                    if matches!(login.phase, LoginPhase::Failed(_)) =>
                {
                    login.target
                }
                _ => return Task::none(),
            };
            let Some(connection) = state.connection.clone() else { return Task::none() };
            let Ok(client) = GatewayClient::new(endpoint) else {
                return app.show_error_toast("无法建立管理连接，请重试");
            };
            return flow::start(state, scope, client, connection, target);
        }
        Event::CloseLogin => state.login = None,
        Event::LoginTick => {}
        Event::LoginLink(generation, action) if visible => {
            if let Some(login) = state.login_mut(generation)
                && let LoginPhase::Pending(challenge) = &login.phase
                && challenge.expires_at.0 > now().0
            {
                let url = challenge.verification_url.clone();
                return deliver_link(app, action, url);
            }
        }
        Event::PasteInput(generation, input) if visible => {
            browser::edit_paste(state, generation, input);
        }
        Event::PasteSubmit(generation) if visible => {
            if let Err(error) = browser::submit_paste(state, generation) {
                return app.show_error_toast(error);
            }
        }
        Event::LoginLink(..) | Event::PasteInput(..) | Event::PasteSubmit(_) => {}
        Event::LoginUpdated { scope, generation, update } => {
            if !visible || !state.is_latest_login_update(&scope, generation) {
                return Task::none();
            }
            if matches!(update, flow::Update::CancelFailed) {
                return app.show_error_toast("执行主机未确认取消授权登录");
            }
            let action = LinkAction::for_preference(state.copy_login_link);
            let Some(login) = state.login_mut(generation) else { return Task::none() };
            match update {
                flow::Update::Started(challenge) => {
                    if matches!(login.phase, LoginPhase::Starting) {
                        let active = challenge.expires_at.0 > now().0;
                        let url = challenge.verification_url.clone();
                        login.phase = LoginPhase::Pending(challenge);
                        if active {
                            return deliver_link(app, action, url);
                        }
                    }
                }
                flow::Update::Pending(expires_at) => {
                    if let LoginPhase::Pending(challenge) = &mut login.phase {
                        challenge.expires_at = expires_at;
                    }
                }
                flow::Update::Approved => {
                    let label = if login.target.is_device() {
                        "设备码登录成功"
                    } else {
                        "授权登录成功"
                    };
                    state.login = None;
                    return Task::batch([
                        app.show_success_toast(label),
                        Task::done(message(Event::Refresh)),
                    ]);
                }
                flow::Update::Failed(reason) => {
                    login.paste_input.clear();
                    login.phase = LoginPhase::Failed(reason);
                    return app.show_error_toast(reason);
                }
                flow::Update::PasteFinished(result) => {
                    login.paste_submitting = false;
                    return match result {
                        Ok(()) => app.show_success_toast("已提交回填，正在等待执行主机确认登录"),
                        Err(error) => app.show_error_toast(error),
                    };
                }
                flow::Update::CancelFailed => unreachable!("cancellation errors handled above"),
            }
        }
        Event::Loaded { .. } | Event::OpenBrowserPreference(_) => {
            unreachable!("local events handled above")
        }
    }
    Task::none()
}

fn deliver_link(app: &mut App, action: LinkAction, url: reqwest::Url) -> Task<Message> {
    match action.deliver(&url) {
        Ok(()) if matches!(action, LinkAction::Copy) => app.show_success_toast("已复制授权链接"),
        Ok(()) => Task::none(),
        Err(error) => app.show_error_toast(error),
    }
}

pub(crate) async fn load(
    client: &GatewayClient,
    host: &str,
    credentials: &AgentAppsAuthCredentials<'_>,
) -> Vec<ServiceAccounts> {
    iced::futures::future::join_all(
        SERVICES.map(|service| load_service(client, host, credentials, service)),
    )
    .await
}

async fn load_service(
    client: &GatewayClient,
    host: &str,
    credentials: &AgentAppsAuthCredentials<'_>,
    service: AuthService,
) -> ServiceAccounts {
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
