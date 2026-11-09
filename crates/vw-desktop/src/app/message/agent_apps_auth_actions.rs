//! Review and execute account operations through the scoped Gateway SDK.

use super::{Event, load_service, message, read_error_label};
use crate::app::{
    App, Message,
    state::agent_apps_auth::{AccountAction, AccountOperation, ActionPhase, Connection, Scope},
};
use iced::Task;
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthError, AgentAppsAuthLogout, GatewayClient,
    GatewayEndpoint,
    vw_api_types::agent_apps_auth::{AuthAccount, AuthCredentialState, AuthErrorCode, AuthService},
};

#[derive(Clone)]
pub enum ActionEvent {
    Open {
        rows_generation: u64,
        operation: AccountOperation,
        service: AuthService,
        account_id: String,
    },
    Review(u64),
    Confirm(u64),
    Close(u64),
    Reviewed {
        scope: Scope,
        generation: u64,
        phase: ActionPhase,
    },
    Saved {
        scope: Scope,
        generation: u64,
        result: Outcome,
    },
}

#[derive(Clone)]
pub enum Outcome {
    Confirmed,
    RemoteUnconfirmed,
    Unconfirmed,
    Failed(&'static str),
}

impl AccountOperation {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::SetDefault => "切换默认账号",
            Self::Delete => "删除账号",
            Self::Logout => "本地登出",
            Self::Revoke => "撤销授权",
        }
    }

    fn success(self) -> &'static str {
        match self {
            Self::SetDefault => "已切换默认授权账号",
            Self::Delete => "已删除授权账号",
            Self::Logout => "账号已在执行主机登出",
            Self::Revoke => "上游已确认撤销授权",
        }
    }

    fn unavailable(self, account: &AuthAccount) -> Option<&'static str> {
        match self {
            Self::SetDefault if account.is_default => Some("此账号已经是默认账号"),
            Self::SetDefault if account.credential_state != AuthCredentialState::Ready => {
                Some("此账号需要重新授权后才能设为默认")
            }
            Self::Revoke if account.service != AuthService::DshAccount => {
                Some("此服务不支持上游撤销，可单独执行本地登出")
            }
            Self::Logout | Self::Revoke
                if account.credential_state == AuthCredentialState::SignedOut =>
            {
                Some("此账号已在执行主机登出")
            }
            _ => None,
        }
    }
}

pub(super) fn update(
    app: &mut App,
    event: ActionEvent,
    endpoint: GatewayEndpoint,
    scope: Scope,
    visible: bool,
) -> Task<Message> {
    let state = &mut app.agent_apps_auth;
    if !state.loopback {
        return Task::none();
    }
    let Some(connection) = state.connection.as_ref().filter(|connection| connection.valid()) else {
        return Task::none();
    };

    if let ActionEvent::Saved { scope: saved_scope, generation, result } = event {
        if saved_scope != scope {
            return Task::none();
        }
        return saved(app, endpoint, scope, generation, result, visible);
    }
    if !visible
        || state.pending.is_some()
        || state.binding.is_some()
        || state.login.is_some()
        || state.form.is_some()
    {
        return Task::none();
    }

    let generation = match event {
        ActionEvent::Open { rows_generation, operation, service, ref account_id } => {
            if state.account_action.is_some() {
                return Task::none();
            }
            let account = state
                .rows
                .as_ref()
                .filter(|_| state.rows_generation() == rows_generation)
                .and_then(|rows| rows.iter().find(|row| row.service == service))
                .and_then(|row| row.accounts.as_ref().ok())
                .and_then(|accounts| {
                    accounts.iter().find(|account| {
                        account.service == service && &account.account_id == account_id
                    })
                });
            let Some(account) = account.cloned() else {
                return app.show_error_toast("账号状态已变化，请刷新后重试");
            };
            state.action_generation = state.action_generation.wrapping_add(1);
            let phase = match operation.unavailable(&account) {
                Some(reason) => ActionPhase::Blocked(reason),
                None if operation == AccountOperation::Logout => ActionPhase::Ready(Vec::new()),
                None => ActionPhase::Checking,
            };
            let generation = state.action_generation;
            state.account_action = Some(AccountAction { generation, operation, account, phase });
            generation
        }
        ActionEvent::Review(generation)
        | ActionEvent::Confirm(generation)
        | ActionEvent::Close(generation)
        | ActionEvent::Reviewed { generation, .. } => generation,
        ActionEvent::Saved { .. } => unreachable!("saved events handled above"),
    };

    let Some(action) =
        state.account_action.as_mut().filter(|action| action.generation == generation)
    else {
        return Task::none();
    };
    let operation = action.operation;
    let account = &action.account;
    match event {
        ActionEvent::Close(_) if !matches!(action.phase, ActionPhase::Saving) => {
            state.account_action = None;
        }
        ActionEvent::Open { .. } if matches!(action.phase, ActionPhase::Checking) => {
            return check(endpoint, scope, connection.clone(), action);
        }
        ActionEvent::Review(_) if action.phase.can_review() => {
            return check(endpoint, scope, connection.clone(), action);
        }
        ActionEvent::Reviewed { scope: reviewed_scope, phase, .. }
            if reviewed_scope == scope && matches!(action.phase, ActionPhase::Checking) =>
        {
            action.phase = phase;
            if let ActionPhase::CheckFailed(reason) = action.phase {
                return app.show_error_toast(format!("{}失败。{reason}", operation.label()));
            }
        }
        ActionEvent::Confirm(_) if action.phase.can_confirm() => {
            let Ok(client) = GatewayClient::new(endpoint) else {
                return app.show_error_toast("无法建立管理连接，请重试");
            };
            let connection = connection.clone();
            let service = account.service;
            let account_id = account.account_id.clone();
            action.phase = ActionPhase::Saving;
            return Task::perform(
                async move { mutate(client, connection, operation, service, &account_id).await },
                move |result| {
                    message(Event::AccountAction(ActionEvent::Saved { scope, generation, result }))
                },
            );
        }
        _ => {}
    }
    Task::none()
}

fn check(
    endpoint: GatewayEndpoint,
    scope: Scope,
    connection: Connection,
    action: &mut AccountAction,
) -> Task<Message> {
    let generation = action.generation;
    let operation = action.operation;
    let service = action.account.service;
    let account_id = action.account.account_id.clone();
    action.phase = ActionPhase::Checking;
    Task::perform(
        async move {
            let Ok(client) = GatewayClient::new(endpoint) else {
                return ActionPhase::CheckFailed("无法建立管理连接，请重试");
            };
            match review(&client, &connection, operation, service, &account_id).await {
                Ok(phase) => phase,
                Err(reason) => ActionPhase::CheckFailed(reason),
            }
        },
        move |phase| {
            message(Event::AccountAction(ActionEvent::Reviewed { scope, generation, phase }))
        },
    )
}

async fn review(
    client: &GatewayClient,
    connection: &Connection,
    operation: AccountOperation,
    service: AuthService,
    account_id: &str,
) -> Result<ActionPhase, &'static str> {
    let credentials = credentials(connection);
    let host = connection.host.trim();
    if operation == AccountOperation::Delete {
        let bindings = client
            .agent_apps_auth_bindings(host, service, Some(account_id), &credentials)
            .await
            .map_err(|error| match error {
                vw_gateway_client::AgentAppsAuthReadError::Http { status: 403, .. } => {
                    "当前主体没有此服务的绑定读取权限"
                }
                _ => read_error_label(error),
            })?;
        if bindings.iter().any(|binding| {
            binding.service != service
                || binding.account_id != account_id
                || binding.provider_id.trim().is_empty()
        }) {
            return Err("网关返回的绑定与所选账号不符");
        }
        let mut users: Vec<_> = bindings.into_iter().map(|binding| binding.provider_id).collect();
        users.sort_unstable();
        users.dedup();
        return Ok(ActionPhase::Ready(users));
    }

    let status = client
        .agent_apps_auth_status(host, service, &credentials)
        .await
        .map_err(read_error_label)?;
    if status.service != service {
        return Err("网关返回的能力与所选服务不符");
    }
    Ok(match operation {
        AccountOperation::SetDefault if !status.capabilities.can_set_default => {
            ActionPhase::Blocked("当前执行主机或主体无法切换此服务的默认账号")
        }
        AccountOperation::Revoke if !status.capabilities.can_revoke => {
            ActionPhase::Blocked("当前执行主机或主体无法执行上游撤销，可单独尝试本地登出")
        }
        _ => ActionPhase::Ready(Vec::new()),
    })
}

async fn mutate(
    client: GatewayClient,
    connection: Connection,
    operation: AccountOperation,
    service: AuthService,
    account_id: &str,
) -> Outcome {
    let host = connection.host.trim();
    let credentials = credentials(&connection);
    let result = match operation {
        AccountOperation::SetDefault => {
            client.agent_apps_auth_set_default(host, service, &credentials, account_id).await
        }
        AccountOperation::Delete => {
            client.agent_apps_auth_delete(host, service, &credentials, account_id).await
        }
        AccountOperation::Logout => {
            client
                .agent_apps_auth_logout(
                    host,
                    service,
                    &credentials,
                    AgentAppsAuthLogout::Account { account_id },
                )
                .await
        }
        AccountOperation::Revoke => {
            client.agent_apps_auth_revoke(host, service, &credentials, account_id).await
        }
    };
    match result {
        Ok(()) => Outcome::Confirmed,
        Err(AgentAppsAuthError::Http {
            code: Some(AuthErrorCode::RemoteRevocationUnconfirmed),
            ..
        }) if operation == AccountOperation::Revoke => Outcome::RemoteUnconfirmed,
        Err(AgentAppsAuthError::Transport | AgentAppsAuthError::Decode)
        | Err(AgentAppsAuthError::Http { status: 200..=299, .. }) => Outcome::Unconfirmed,
        Err(error) => Outcome::Failed(error_label(error)),
    }
}

fn saved(
    app: &mut App,
    endpoint: GatewayEndpoint,
    scope: Scope,
    generation: u64,
    result: Outcome,
    visible: bool,
) -> Task<Message> {
    let state = &mut app.agent_apps_auth;
    let Some(action) = state.account_action.as_mut().filter(|action| {
        action.generation == generation && matches!(action.phase, ActionPhase::Saving)
    }) else {
        return Task::none();
    };
    let operation = action.operation;
    if let Outcome::Failed(reason) = result {
        if !visible {
            state.account_action = None;
            return Task::none();
        }
        action.phase = if operation == AccountOperation::Delete {
            ActionPhase::CheckFailed(reason)
        } else {
            ActionPhase::SaveFailed(reason)
        };
        return app.show_error_toast(format!("{}失败。{reason}", operation.label()));
    }
    let Some(connection) = state.connection.clone() else {
        return Task::none();
    };
    let service = action.account.service;
    state.account_action = None;
    let mut rows = state.rows.take().unwrap_or_default();
    let load_generation = state.begin();
    let reload = Task::perform(
        async move {
            let refreshed = match GatewayClient::new(endpoint) {
                Ok(client) => {
                    load_service(
                        &client,
                        connection.host.trim(),
                        &credentials(&connection),
                        service,
                    )
                    .await
                }
                Err(_) => crate::app::state::agent_apps_auth::ServiceAccounts {
                    service,
                    accounts: Err("无法建立管理连接，请重试"),
                },
            };
            if let Some(row) = rows.iter_mut().find(|row| row.service == service) {
                *row = refreshed;
            } else {
                rows.push(refreshed);
            }
            rows
        },
        move |rows| message(Event::Loaded { scope, generation: load_generation, rows }),
    );
    let toast = if visible {
        match result {
            Outcome::Confirmed => app.show_success_toast(operation.success()),
            Outcome::RemoteUnconfirmed => {
                app.show_error_toast("账号已在执行主机登出，上游未确认撤销")
            }
            Outcome::Unconfirmed => app.show_error_toast(format!(
                "未收到{}的确认，请查看刷新后的账号状态",
                operation.label()
            )),
            Outcome::Failed(_) => unreachable!("failed operations handled above"),
        }
    } else {
        Task::none()
    };
    Task::batch([toast, reload])
}

fn credentials(connection: &Connection) -> AgentAppsAuthCredentials<'_> {
    AgentAppsAuthCredentials {
        username: connection.username.trim(),
        password: &connection.password,
        principal_token: connection.token.trim(),
    }
}

fn error_label(error: AgentAppsAuthError) -> &'static str {
    use AuthErrorCode::*;
    match error {
        AgentAppsAuthError::Request(code) | AgentAppsAuthError::Http { code: Some(code), .. } => {
            match code {
                InvalidInput => "账号操作参数无效",
                Unauthenticated => "管理凭据或主体令牌无效",
                Forbidden => "当前主体没有执行此操作的权限",
                NotFound => "找不到所选账号，请刷新授权账号",
                Unsupported => "执行主机不支持此操作",
                AccountInUse => "账号仍被供应商引用，请解除绑定后重试",
                Conflict => "账号状态已变化，请刷新后重试",
                ReauthRequired => "账号需要重新授权",
                ExecutionChannelUnavailable => "执行主机没有可用的授权通道",
                ExecutionHostUnavailable => "授权执行主机不可用",
                KeychainUnavailable => "执行主机的钥匙串不可用",
                StorageUnavailable => "执行主机的授权存储不可用",
                _ => "执行主机未确认操作，请重试",
            }
        }
        AgentAppsAuthError::Http { status: 401 | 403, .. } => "管理认证被拒绝，请检查连接凭据",
        AgentAppsAuthError::InvalidPrincipalToken => "主体令牌格式无效",
        AgentAppsAuthError::Transport | AgentAppsAuthError::Decode => {
            unreachable!("unconfirmed operations handled above")
        }
        AgentAppsAuthError::Http { .. } => "网关拒绝了账号操作",
    }
}
