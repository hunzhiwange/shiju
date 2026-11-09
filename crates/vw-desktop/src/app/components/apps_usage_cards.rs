//! Shared snapshot and manual refresh for supplier and bound-account cards.

use std::collections::{HashMap, HashSet};

use iced::Task;
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthReadError, GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps::AppId,
        agent_apps_auth::{AuthCredentialState, AuthErrorCode, AuthService},
        apps_usage::{UsageQueryKind, UsageSnapshotQuery, UsageSnapshotResponse},
        id::ProjectId,
    },
};

use super::system_settings::SystemTab;
#[path = "apps_usage_cards/batch.rs"]
mod batch;
#[path = "apps_usage_cards/view.rs"]
mod view;
pub(crate) use view::{account_view, provider_view};

use crate::app::{
    App, Message,
    message::{
        app_providers::list::{ListMessage, message},
        settings::apps_usage_events::{read_snapshot, refresh_one},
    },
    state::agent_apps_auth::{Connection, Scope as AuthScope},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Provider(AppId, String),
    Account(AuthService, String),
}

impl Target {
    fn includes(&self, selection: &UsageSnapshotQuery) -> bool {
        match self {
            Self::Provider(app, id) => selection.app_id == *app && selection.provider_id == *id,
            Self::Account(service, id) => {
                binding_app(*service) == Some(selection.app_id)
                    && selection.account_id.as_ref() == Some(id)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Read(Target),
    Refresh(Target, String),
    ToggleDetails(Target, String),
    Finished(Target, Scope, u64, CardResult),
    RefreshAll,
    BatchFinished(Scope, u64, CardResult),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    endpoint: AuthScope,
    pub(super) project: ProjectId,
    pub(super) host: String,
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

    pub(crate) fn gateway(status: u16, code: Option<AuthErrorCode>) -> Self {
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

#[derive(Clone, Debug)]
pub struct CardSnapshot {
    selection: UsageSnapshotQuery,
    pub(crate) snapshot: Result<UsageSnapshotResponse, Failure>,
    pub(crate) refresh_failure: Option<Failure>,
}

impl CardSnapshot {
    pub(crate) fn new(
        mut selection: UsageSnapshotQuery,
        snapshot: Result<UsageSnapshotResponse, Failure>,
        refresh_failure: Option<Failure>,
    ) -> Self {
        if selection.kind.is_none()
            && let Ok(response) = &snapshot
            && let [kind] = response.capabilities.as_slice()
        {
            selection.kind = Some(*kind);
        }
        Self { selection, snapshot, refresh_failure }
    }

    fn provider(&self) -> &str {
        &self.selection.provider_id
    }

    fn matches(&self, other: &Self) -> bool {
        self.selection.project_id == other.selection.project_id
            && self.selection.target_host_id == other.selection.target_host_id
            && self.selection.app_id == other.selection.app_id
            && self.selection.provider_id == other.selection.provider_id
            && self.selection.account_id == other.selection.account_id
            && self.selection.kind == other.selection.kind
    }
}

#[derive(Clone, Debug, Default)]
struct Entry {
    pending: Option<u64>,
    refreshing: Option<String>,
    result: Option<CardResult>,
    expanded: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Cards {
    scope: Option<Scope>,
    entries: HashMap<Target, Entry>,
    pub(super) settings: super::apps_usage_settings::Editor,
    sequence: u64,
    batch_pending: Option<u64>,
    batch_result: Option<CardResult>,
}

impl Cards {
    pub(crate) fn clear(&mut self) {
        self.scope = None;
        self.entries.clear();
        self.settings.clear();
        self.batch_pending = None;
        self.batch_result = None;
    }

    fn select_scope(&mut self, scope: &Scope) {
        if self.scope.as_ref() != Some(scope) {
            self.clear();
            self.scope = Some(scope.clone());
        }
    }

    fn refreshing(&self, scope: &Scope) -> bool {
        self.scope.as_ref() == Some(scope)
            && (self.batch_pending.is_some()
                || self.entries.values().any(|entry| entry.refreshing.is_some()))
    }

    fn apply_updates(&mut self, scope: &Scope, updates: &[CardSnapshot]) {
        if self.scope.as_ref() != Some(scope) {
            return;
        }
        for (target, entry) in &mut self.entries {
            if updates.iter().any(|update| target.includes(&update.selection)) {
                // An in-flight read may have started before the service published the update.
                entry.pending = None;
            }
            if let Some(Ok(result)) = &mut entry.result {
                for snapshot in result.iter_mut() {
                    if let Some(update) = updates.iter().find(|update| snapshot.matches(update)) {
                        *snapshot = update.clone();
                    }
                }
                entry.expanded.retain(|provider| has_snapshot(result, provider));
            }
        }
    }

    fn next_ticket(&mut self) -> u64 {
        self.sequence = self.sequence.wrapping_add(1);
        self.sequence
    }

    fn begin(&mut self, target: Target) -> u64 {
        let ticket = self.next_ticket();
        let entry = self.entries.entry(target).or_default();
        entry.pending = Some(ticket);
        ticket
    }

    fn finish(&mut self, target: &Target, ticket: u64, result: CardResult) -> bool {
        let Some(entry) = self.entries.get_mut(target) else { return false };
        if entry.pending != Some(ticket) {
            return false;
        }
        entry.pending = None;
        entry.refreshing = None;
        entry
            .expanded
            .retain(|provider| result.as_ref().is_ok_and(|items| has_snapshot(items, provider)));
        entry.result = Some(result);
        true
    }
}

fn has_snapshot(result: &[CardSnapshot], provider: &str) -> bool {
    result.iter().any(|entry| {
        entry.provider() == provider
            && entry.snapshot.as_ref().is_ok_and(|response| response.snapshot.is_some())
    })
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
            refresh_failure = refresh_one(client, &selection, &context.token).await.err();
        }
        let snapshot = read_snapshot(client, &selection, &context.token).await;
        result.push(CardSnapshot::new(selection, snapshot, refresh_failure));
    }
    Ok(result)
}

pub(super) fn scope(app: &App, endpoint: &GatewayEndpoint, account: bool) -> Option<Scope> {
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

impl Scope {
    fn same_context(&self, other: &Self) -> bool {
        self.endpoint == other.endpoint
            && self.project == other.project
            && self.host == other.host
            && self.auth_generation == other.auth_generation
    }
}

fn linked_scope(
    app: &App,
    endpoint: &GatewayEndpoint,
    current: &Scope,
    account: bool,
) -> Option<Scope> {
    let connection = app.agent_apps_auth.connection.as_ref()?;
    if !endpoint.is_loopback()
        || !connection.valid()
        || connection.token.trim() != app.apps_catalog.token.trim()
    {
        return None;
    }
    scope(app, endpoint, !account).filter(|other| current.same_context(other))
}

/// Reopening the provider list resolves current bindings before displaying linked results.
pub(crate) fn read_linked_providers(app: &mut App, app_id: AppId) -> Task<Message> {
    if !app.show_system_settings || app.system_settings_tab != SystemTab::Apps {
        return Task::none();
    }
    let endpoint = crate::app::gateway_client_endpoint();
    let Some(scope) = scope(app, &endpoint, false) else { return Task::none() };
    let auth_cards = &app.agent_apps_auth.usage;
    if linked_scope(app, &endpoint, &scope, false).as_ref() != auth_cards.scope.as_ref()
        || auth_cards.scope.is_none()
    {
        return Task::none();
    }
    let providers: HashSet<_> = auth_cards
        .entries
        .values()
        .filter_map(|entry| entry.result.as_ref())
        .chain(auth_cards.batch_result.as_ref())
        .filter_map(|result| result.as_ref().ok())
        .flatten()
        .filter(|item| item.selection.app_id == app_id && item.snapshot.is_ok())
        .map(|item| item.provider().to_owned())
        .collect();
    let cards = &app.app_providers.usage;
    let mut reads = Vec::new();
    for id in providers {
        let target = Target::Provider(app_id, id);
        if cards.scope.as_ref() != Some(&scope) || !cards.entries.contains_key(&target) {
            reads.push(target);
        }
    }
    Task::batch(
        reads.into_iter().map(|target| update_at(app, Event::Read(target), endpoint.clone())),
    )
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    update_at(app, event, crate::app::gateway_client_endpoint())
}

fn update_at(app: &mut App, event: Event, endpoint: GatewayEndpoint) -> Task<Message> {
    let target = match &event {
        Event::Read(target)
        | Event::Refresh(target, _)
        | Event::ToggleDetails(target, _)
        | Event::Finished(target, ..) => target,
        Event::RefreshAll | Event::BatchFinished(..) => return batch::update(app, event, endpoint),
    };
    let is_account = matches!(target, Target::Account(..));
    let expected_tab = if is_account { SystemTab::Auth } else { SystemTab::Apps };
    let completed = matches!(event, Event::Finished(..));
    if !completed && (!app.show_system_settings || app.system_settings_tab != expected_tab) {
        return Task::none();
    }
    let Some(scope) = scope(app, &endpoint, is_account) else {
        return if matches!(event, Event::Read(_) | Event::Refresh(..)) {
            app.show_error_toast("请先选择项目并连接执行主机")
        } else {
            Task::none()
        };
    };
    let linked = linked_scope(app, &endpoint, &scope, is_account);
    let other = if is_account { &app.app_providers.usage } else { &app.agent_apps_auth.usage };
    let linked_busy = linked.as_ref().is_some_and(|scope| other.refreshing(scope));
    let auth = &mut app.agent_apps_auth;
    let connection = auth.connection.as_ref().filter(|connection| {
        endpoint.is_loopback()
            && auth.scope.as_ref() == Some(&scope.endpoint)
            && connection.valid()
            && connection.host.trim() == scope.host
    });
    let (cards, token) = match target {
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
    let (target, refresh) = match event {
        Event::Read(target) => (target, None),
        Event::Refresh(target, provider) => (target, Some(provider)),
        Event::Finished(target, expected, ticket, result) => {
            if scope == expected && cards.scope.as_ref() == Some(&scope) {
                let refresh = cards.entries.get(&target).and_then(|entry| entry.refreshing.clone());
                if cards.finish(&target, ticket, result)
                    && let (Some(provider), Some(linked)) = (refresh, linked)
                    && let Some(Ok(items)) = &cards.entries[&target].result
                    && let Some(item) = items.iter().find(|item| item.provider() == provider)
                {
                    let item = item.clone();
                    let other =
                        if is_account { &mut app.app_providers.usage } else { &mut auth.usage };
                    other.apply_updates(&linked, std::slice::from_ref(&item));
                }
            }
            return Task::none();
        }
        Event::ToggleDetails(target, provider) => {
            if cards.scope.as_ref() == Some(&scope)
                && let Some(entry) = cards.entries.get_mut(&target)
                && matches!(&entry.result, Some(Ok(items)) if has_snapshot(items, &provider))
            {
                if entry.expanded.remove(&provider) {
                    return Task::none();
                }
                entry.expanded.insert(provider);
            }
            return Task::none();
        }
        Event::RefreshAll | Event::BatchFinished(..) => unreachable!("batch events handled above"),
    };
    if token.is_empty() {
        return app.show_error_toast("请先连接执行主机");
    }
    if cards.refreshing(&scope) || linked_busy {
        return Task::none();
    }
    let Ok(client) = GatewayClient::new(endpoint) else {
        return app.show_error_toast("无法建立额度查询连接");
    };
    cards.select_scope(&scope);
    let ticket = cards.begin(target.clone());
    if let Some(entry) = cards.entries.get_mut(&target) {
        entry.refreshing.clone_from(&refresh);
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

pub(crate) fn message_event(event: Event) -> Message {
    message(ListMessage::Usage(event))
}

pub(crate) fn current_scope(app: &App, account: bool) -> Option<Scope> {
    scope(app, &crate::app::gateway_client_endpoint(), account)
}

#[cfg(test)]
#[path = "apps_usage_cards_tests.rs"]
pub(in crate::app) mod tests;
