//! Shared snapshot and manual refresh for supplier and bound-account cards.

use std::{collections::HashMap, time::Duration};

use iced::Task;
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthReadError, AppsUsageReadError, AppsUsageWriteError,
    GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps::AppId,
        agent_apps_auth::{AuthCredentialState, AuthErrorCode, AuthService},
        apps_usage::{UsageQueryKind, UsageSnapshotQuery, UsageSnapshotResponse},
        id::ProjectId,
        task::TaskStatus,
    },
};

use super::system_settings::SystemTab;
#[path = "apps_usage_cards/view.rs"]
mod view;
pub(crate) use view::{account_view, provider_view};

use crate::app::{
    App, Message,
    message::app_providers::list::{ListMessage, message},
    state::agent_apps_auth::{Connection, Scope as AuthScope},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Provider(AppId, String),
    Account(AuthService, String),
}

#[derive(Clone, Debug)]
pub enum Event {
    Read(Target),
    Refresh(Target, String),
    Finished(Target, Scope, u64, CardResult),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    endpoint: AuthScope,
    project: ProjectId,
    host: String,
    provider_generation: u64,
    catalog_generation: u64,
    auth_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Denied,
    Unsupported,
    Changed,
    Reauth,
    Unavailable,
    InvalidResponse,
    RefreshFailed,
}

impl Failure {
    fn label(self) -> &'static str {
        match self {
            Self::Denied => "当前连接没有额度权限",
            Self::Unsupported => "执行主机暂不支持此额度查询",
            Self::Changed => "供应商或绑定账号已变化，请重新读取",
            Self::Reauth => "账号需要重新授权",
            Self::Unavailable => "额度服务暂不可用",
            Self::InvalidResponse => "网关返回了无效的额度响应",
            Self::RefreshFailed => "额度刷新未完成，请重试",
        }
    }

    fn gateway(status: u16, code: Option<AuthErrorCode>) -> Self {
        match (status, code) {
            (_, Some(AuthErrorCode::ReauthRequired)) => Self::Reauth,
            (401 | 403, _) => Self::Denied,
            (404 | 409, _) => Self::Changed,
            (501, _) => Self::Unsupported,
            _ => Self::Unavailable,
        }
    }
}

type CardResult = Result<Vec<CardSnapshot>, Failure>;

#[derive(Clone, Debug, PartialEq)]
pub struct CardSnapshot {
    provider: String,
    snapshot: Result<UsageSnapshotResponse, Failure>,
    refresh_failure: Option<Failure>,
}

#[derive(Clone, Debug, Default)]
struct Entry {
    pending: Option<u64>,
    refreshing: bool,
    result: Option<CardResult>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Cards {
    scope: Option<Scope>,
    entries: HashMap<Target, Entry>,
    sequence: u64,
}

impl Cards {
    pub(crate) fn clear(&mut self) {
        self.scope = None;
        self.entries.clear();
    }

    fn begin(&mut self, target: Target) -> u64 {
        self.sequence = self.sequence.wrapping_add(1);
        let entry = self.entries.entry(target).or_default();
        entry.pending = Some(self.sequence);
        self.sequence
    }

    fn finish(&mut self, target: &Target, ticket: u64, result: CardResult) -> bool {
        let Some(entry) = self.entries.get_mut(target) else { return false };
        if entry.pending != Some(ticket) {
            return false;
        }
        entry.pending = None;
        entry.refreshing = false;
        entry.result = Some(result);
        true
    }
}

pub(crate) fn binding_service(app: AppId) -> Option<AuthService> {
    match app {
        AppId::Claude => Some(AuthService::ClaudeOfficial),
        AppId::Codex => Some(AuthService::CodexOauth),
        AppId::Agy => Some(AuthService::AgyOauth),
        AppId::Kimi => Some(AuthService::KimiOauth),
        _ => None,
    }
}

pub(crate) fn binding_app(service: AuthService) -> Option<AppId> {
    match service {
        AuthService::ClaudeOfficial => Some(AppId::Claude),
        AuthService::CodexOauth => Some(AppId::Codex),
        AuthService::AgyOauth => Some(AppId::Agy),
        AuthService::KimiOauth => Some(AppId::Kimi),
        _ => None,
    }
}

struct QueryContext {
    project: ProjectId,
    host: String,
    token: String,
    connection: Option<Connection>,
}

async fn read(
    client: &GatewayClient,
    context: &QueryContext,
    target: &Target,
    refresh: Option<&str>,
) -> CardResult {
    let (app, service, provider, account) = match target {
        Target::Provider(app, provider) => (*app, binding_service(*app), Some(provider), None),
        Target::Account(service, account) => (
            binding_app(*service).ok_or(Failure::Unsupported)?,
            Some(*service),
            None,
            Some(account),
        ),
    };
    let mut selections = Vec::new();
    if let (Some(service), Some(connection)) = (service, &context.connection) {
        let credentials = AgentAppsAuthCredentials {
            username: connection.username.trim(),
            password: &connection.password,
            principal_token: &context.token,
        };
        let bindings = client
            .agent_apps_auth_bindings(
                &context.host,
                service,
                account.map(String::as_str),
                &credentials,
            )
            .await
            .map_err(|error| match error {
                AgentAppsAuthReadError::Http { status, error } => {
                    Failure::gateway(status, error.map(|error| error.code))
                }
                AgentAppsAuthReadError::InvalidPrincipalToken => Failure::Denied,
                AgentAppsAuthReadError::Decode => Failure::InvalidResponse,
                AgentAppsAuthReadError::Transport => Failure::Unavailable,
            })?;
        for binding in bindings {
            if binding.service != service
                || account.is_some_and(|account| account != &binding.account_id)
                || provider.is_some_and(|provider| provider != &binding.provider_id)
            {
                continue;
            }
            selections.push((binding.provider_id, Some(binding.account_id)));
        }
    }
    if selections.is_empty()
        && let Some(provider) = provider
    {
        selections.push((provider.clone(), None));
    }
    selections.sort_unstable();
    selections.dedup();
    if provider.is_some() && selections.len() > 1 {
        return Err(Failure::Changed);
    }
    if refresh.is_some_and(|id| !selections.iter().any(|(provider, _)| provider == id)) {
        return Err(Failure::Changed);
    }

    let mut result = Vec::with_capacity(selections.len());
    for (provider_id, account_id) in selections {
        let mut selection = UsageSnapshotQuery {
            project_id: context.project.clone(),
            target_host_id: context.host.clone(),
            app_id: app,
            provider_id,
            kind: account_id.as_ref().map(|_| UsageQueryKind::OfficialSubscription),
            account_id,
        };
        let mut refresh_failure = None;
        if refresh == Some(selection.provider_id.as_str()) {
            if selection.kind.is_none()
                && let Ok(initial) = client.apps_usage_snapshot(&selection, &context.token).await
                && let [kind] = initial.capabilities.as_slice()
            {
                selection.kind = Some(*kind);
            }
            refresh_failure = refresh_snapshot(client, &selection, &context.token).await.err();
        }
        let snapshot = client.apps_usage_snapshot(&selection, &context.token).await.map_err(
            |error| match error {
                AppsUsageReadError::Http { status, error } => {
                    Failure::gateway(status, error.map(|error| error.code))
                }
                AppsUsageReadError::InvalidPrincipalToken => Failure::Denied,
                AppsUsageReadError::Decode => Failure::InvalidResponse,
                AppsUsageReadError::Transport => Failure::Unavailable,
            },
        );
        result.push(CardSnapshot { provider: selection.provider_id, snapshot, refresh_failure });
    }
    Ok(result)
}

async fn refresh_snapshot(
    client: &GatewayClient,
    selection: &UsageSnapshotQuery,
    token: &str,
) -> Result<(), Failure> {
    let map_error = |error| match error {
        AppsUsageWriteError::Http { status, error } => {
            Failure::gateway(status, error.map(|error| error.code))
        }
        AppsUsageWriteError::InvalidPrincipalToken => Failure::Denied,
        AppsUsageWriteError::Decode | AppsUsageWriteError::InvalidOperationId => {
            Failure::InvalidResponse
        }
        _ => Failure::Unavailable,
    };
    let operation = client.apps_usage_refresh(selection, token).await.map_err(map_error)?;
    tokio::time::timeout(Duration::from_secs(90), async {
        let mut status = client
            .apps_usage_operation(&selection.target_host_id, &operation.operation_id, token)
            .await
            .map_err(map_error)?;
        loop {
            if status.project_id != selection.project_id
                || (!status.items.is_empty()
                    && !matches!(status.items.as_slice(), [item]
                    if item.app_id == selection.app_id && item.provider_id == selection.provider_id
                        && item.account_id == selection.account_id
                        && selection.kind.is_none_or(|kind| item.kind == kind)))
            {
                return Err(Failure::InvalidResponse);
            }
            match status.operation.status {
                TaskStatus::Completed if status.items.len() == 1 => return Ok(()),
                TaskStatus::Completed => return Err(Failure::InvalidResponse),
                TaskStatus::Failed | TaskStatus::Cancelled => return Err(Failure::RefreshFailed),
                TaskStatus::Queued | TaskStatus::Running => {}
            }
            status = match client
                .apps_usage_operation_events(
                    &selection.target_host_id,
                    &operation.operation_id,
                    token,
                )
                .await
            {
                Ok(status) => status,
                Err(AppsUsageWriteError::Transport) => client
                    .apps_usage_operation(&selection.target_host_id, &operation.operation_id, token)
                    .await
                    .map_err(map_error)?,
                Err(error) => return Err(map_error(error)),
            };
        }
    })
    .await
    .map_err(|_| Failure::RefreshFailed)?
}

fn scope(app: &App, endpoint: &GatewayEndpoint, account: bool) -> Option<Scope> {
    let endpoint = AuthScope::from(endpoint);
    let project = app.project_id.as_deref().filter(|id| !id.trim().is_empty())?.into();
    let host = if account {
        if app.agent_apps_auth.scope.as_ref() != Some(&endpoint) {
            return None;
        }
        app.agent_apps_auth.connection.as_ref()?.host.trim().to_owned()
    } else {
        let origin = app.app_providers.scope.as_ref()?;
        if origin.context.gateway != endpoint.gateway
            || origin.context.tenant != endpoint.tenant
            || origin.context.project != app.project_path
            || origin.host != app.apps_catalog.host
        {
            return None;
        }
        origin.host.clone()
    };
    Some(Scope {
        endpoint,
        project,
        host,
        provider_generation: if account { 0 } else { app.app_providers.generation },
        catalog_generation: if account { 0 } else { app.apps_catalog.generation },
        auth_generation: app.agent_apps_auth.rows_generation(),
    })
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let (target, refresh, completed) = match event {
        Event::Read(target) => (target, None, None),
        Event::Refresh(target, provider) => (target, Some(provider), None),
        Event::Finished(target, scope, ticket, result) => {
            (target, None, Some((scope, ticket, result)))
        }
    };
    let is_account = matches!(target, Target::Account(..));
    let expected_tab = if is_account { SystemTab::Auth } else { SystemTab::Apps };
    if completed.is_none() && (!app.show_system_settings || app.system_settings_tab != expected_tab)
    {
        return Task::none();
    }
    let endpoint = crate::app::gateway_client_endpoint();
    let Some(scope) = scope(app, &endpoint, is_account) else {
        return if completed.is_none() {
            app.show_error_toast("请先选择项目并连接执行主机")
        } else {
            Task::none()
        };
    };
    let auth = &mut app.agent_apps_auth;
    let connection = auth.connection.as_ref().filter(|connection| {
        endpoint.is_loopback()
            && auth.scope.as_ref() == Some(&scope.endpoint)
            && connection.valid()
            && connection.host.trim() == scope.host
    });
    let (cards, token) = match &target {
        Target::Provider(app_id, id) => {
            if app.app_providers.scope.as_ref().is_none_or(|scope| scope.app != *app_id)
                || !app.app_providers.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.catalog.rows.iter().any(|row| row.provider.id == *id)
                })
            {
                return Task::none();
            }
            (&mut app.app_providers.usage, app.apps_catalog.token.trim())
        }
        Target::Account(service, id) => {
            if !auth.rows.as_ref().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row.service == *service
                        && row.accounts.as_ref().is_ok_and(|accounts| {
                            accounts.iter().any(|account| {
                                account.account_id == *id
                                    && account.credential_state == AuthCredentialState::Ready
                            })
                        })
                })
            }) {
                return Task::none();
            }
            let Some(connection) = connection else { return Task::none() };
            (&mut auth.usage, connection.token.trim())
        }
    };
    if let Some((expected, ticket, result)) = completed {
        if scope == expected && cards.scope.as_ref() == Some(&scope) {
            cards.finish(&target, ticket, result);
        }
        return Task::none();
    }
    if token.is_empty() {
        return app.show_error_toast("请先连接执行主机");
    }
    if cards.scope.as_ref() != Some(&scope) {
        cards.clear();
        cards.scope = Some(scope.clone());
    }
    if cards.entries.get(&target).is_some_and(|entry| entry.refreshing) {
        return Task::none();
    }
    let Ok(client) = GatewayClient::new(endpoint) else {
        return app.show_error_toast("无法建立额度查询连接");
    };
    let ticket = cards.begin(target.clone());
    if let Some(entry) = cards.entries.get_mut(&target) {
        entry.refreshing = refresh.is_some();
    }
    let context = QueryContext {
        project: scope.project.clone(),
        host: scope.host.clone(),
        token: token.into(),
        connection: connection.filter(|connection| connection.token.trim() == token).cloned(),
    };
    let requested = target.clone();
    Task::perform(
        async move { read(&client, &context, &requested, refresh.as_deref()).await },
        move |result| {
            message(ListMessage::Usage(Event::Finished(
                target.clone(),
                scope.clone(),
                ticket,
                result,
            )))
        },
    )
}

pub(crate) fn current_scope(app: &App, account: bool) -> Option<Scope> {
    scope(app, &crate::app::gateway_client_endpoint(), account)
}

#[cfg(test)]
#[path = "apps_usage_cards_tests.rs"]
mod tests;
