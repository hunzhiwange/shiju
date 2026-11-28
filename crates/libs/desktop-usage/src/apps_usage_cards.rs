//! Shared snapshot and manual refresh for supplier and authorization cards.

use std::collections::{HashMap, HashSet};

use {
    vw_gateway_client::GatewayClient,
    vw_api_app_identity::AppId,
    vw_api_app_auth::AuthErrorCode,
    vw_api_app_auth::AuthService,
    vw_api_apps_usage::apps_usage::UsageSnapshotQuery,
    vw_api_apps_usage::apps_usage::UsageSnapshotResponse,
    vw_api_transport::id::ProjectId,
};

use desktop_settings_account::types::{Connection, Scope as AuthScope};
#[cfg(not(target_arch = "wasm32"))]
use {
    crate::refresh::{read_snapshot, refresh_one},
    gateway_app_auth::{AgentAppsAuthCredentials, AgentAppsAuthReadError, GatewayAppAuthClient},
    vw_api_apps_usage::apps_usage::{UsageQueryKind, UsageSnapshot},
};
#[path = "apps_usage_cards/view.rs"]
mod view;
pub use view::{account_view, provider_view};
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Provider(AppId, String),
    Account(AuthService, String),
}

impl Target {
    pub(super) fn is_codex_account(&self) -> bool {
        matches!(self, Self::Account(AuthService::CodexOauth, _))
    }

    fn includes(&self, selection: &UsageSnapshotQuery) -> bool {
        match self {
            Self::Provider(app, id) => selection.app_id == *app && selection.provider_id == *id,
            Self::Account(service, id) => {
                (*service == AuthService::CodexOauth) == selection.provider_id.is_empty()
                    && binding_app(*service) == Some(selection.app_id)
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub endpoint: AuthScope,
    pub project: ProjectId,
    pub host: String,
    pub provider_generation: u64,
    pub catalog_generation: u64,
    pub auth_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Denied,
    Unsupported,
    GatewayUpgradeRequired,
    Changed,
    Reauth,
    Unavailable,
    InvalidResponse,
    RefreshFailed,
}

impl Failure {
    pub fn label(self) -> &'static str {
        match self {
            Self::Denied => "当前连接没有额度权限",
            Self::Unsupported => "当前环境暂不支持此额度查询",
            Self::GatewayUpgradeRequired => "当前网关版本缺少 Codex 额度接口，请更新网关",
            Self::Changed => "供应商或绑定账号已变化，请重新读取",
            Self::Reauth => "账号需要重新授权",
            Self::Unavailable => "额度服务暂不可用",
            Self::InvalidResponse => "网关返回了无效的额度响应",
            Self::RefreshFailed => "额度刷新未完成，请重试",
        }
    }

    pub fn gateway(status: u16, code: Option<AuthErrorCode>) -> Self {
        match (status, code) {
            (_, Some(AuthErrorCode::ReauthRequired)) => Self::Reauth,
            (401 | 403, _) => Self::Denied,
            (404 | 409, _) => Self::Changed,
            (501, _) => Self::Unsupported,
            _ => Self::Unavailable,
        }
    }
}

pub type CardResult = Result<Vec<CardSnapshot>, Failure>;

#[derive(Clone, Debug)]
pub struct CardSnapshot {
    pub selection: UsageSnapshotQuery,
    pub snapshot: Result<UsageSnapshotResponse, Failure>,
    pub refresh_failure: Option<Failure>,
}

impl CardSnapshot {
    pub fn new(
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

    pub fn provider(&self) -> &str {
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
pub struct Entry {
    pub pending: Option<u64>,
    pub refreshing: Option<String>,
    pub result: Option<CardResult>,
    expanded: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Cards {
    pub scope: Option<Scope>,
    pub entries: HashMap<Target, Entry>,
    sequence: u64,
}

impl Cards {
    pub fn toggle_details(&mut self, scope: &Scope, target: &Target, provider: String) {
        if self.scope.as_ref() == Some(scope)
            && let Some(entry) = self.entries.get_mut(target)
            && matches!(&entry.result, Some(Ok(items)) if has_snapshot(items, &provider))
            && !entry.expanded.remove(&provider) { entry.expanded.insert(provider); }
    }
    pub fn clear(&mut self) {
        self.scope = None;
        self.entries.clear();
    }

    #[cfg(test)]
    fn select_scope(&mut self, scope: &Scope) {
        if self.scope.as_ref() != Some(scope) {
            self.clear();
            self.scope = Some(scope.clone());
        }
    }

    pub fn refreshing(&self, scope: &Scope) -> bool {
        self.scope.as_ref() == Some(scope)
            && self.entries.iter().any(|(target, entry)| {
                entry.refreshing.is_some() || target.is_codex_account() && entry.pending.is_some()
            })
    }

    pub fn apply_updates(&mut self, scope: &Scope, updates: &[CardSnapshot]) {
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

    pub fn begin(&mut self, target: Target) -> u64 {
        let ticket = self.next_ticket();
        let entry = self.entries.entry(target).or_default();
        entry.pending = Some(ticket);
        ticket
    }

    pub fn finish(&mut self, target: &Target, ticket: u64, result: CardResult) -> bool {
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

pub fn has_snapshot(result: &[CardSnapshot], provider: &str) -> bool {
    result.iter().any(|entry| {
        entry.provider() == provider
            && entry.snapshot.as_ref().is_ok_and(|response| response.snapshot.is_some())
    })
}

pub fn binding_service(app: AppId) -> Option<AuthService> {
    match app {
        AppId::Claude => Some(AuthService::ClaudeOfficial),
        AppId::Codex => Some(AuthService::CodexOauth),
        AppId::Agy => Some(AuthService::AgyOauth),
        AppId::Kimi => Some(AuthService::KimiOauth),
        _ => None,
    }
}

pub fn binding_app(service: AuthService) -> Option<AppId> {
    match service {
        AuthService::ClaudeOfficial => Some(AppId::Claude),
        AuthService::CodexOauth => Some(AppId::Codex),
        AuthService::AgyOauth => Some(AppId::Agy),
        AuthService::KimiOauth => Some(AppId::Kimi),
        _ => None,
    }
}

pub struct QueryContext {
    pub project: ProjectId,
    pub host: String,
    pub token: String,
    pub connection: Option<Connection>,
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read(
    client: &GatewayClient,
    context: &QueryContext,
    target: &Target,
    refresh: Option<&str>,
) -> CardResult {
    if let Target::Account(AuthService::CodexOauth, account) = target {
        if context.connection.is_none() {
            return Err(Failure::Denied);
        }
        let result = GatewayAppAuthClient::new(client.transport().clone())
            .codex_account_quota(&context.host, account)
            .await
            .map_err(|error| match error {
                AgentAppsAuthReadError::Http { status: 404 | 405, error: None } => {
                    Failure::GatewayUpgradeRequired
                }
                error => auth_failure(error),
            })?;
        let kind = UsageQueryKind::OfficialSubscription;
        return Ok(vec![CardSnapshot::new(
            UsageSnapshotQuery {
                project_id: context.project.clone(),
                target_host_id: context.host.clone(),
                app_id: AppId::Codex,
                // An account-only result has no supplier to link or refresh.
                provider_id: String::new(),
                account_id: Some(account.clone()),
                kind: Some(kind),
            },
            Ok(UsageSnapshotResponse {
                capabilities: vec![kind],
                snapshot: Some(UsageSnapshot { result, stale: false, refresh_failure: None }),
            }),
            None,
        )]);
    }

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
            .map_err(auth_failure)?;
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
                && let Ok(initial) = gateway_app_usage::GatewayAppUsageClient::new(client.transport().clone()).apps_usage_snapshot(&selection, &context.token).await
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


#[cfg(not(target_arch = "wasm32"))]
fn auth_failure(error: AgentAppsAuthReadError) -> Failure {
    match error {
        AgentAppsAuthReadError::Http { status, error } => {
            Failure::gateway(status, error.map(|error| error.code))
        }
        AgentAppsAuthReadError::InvalidPrincipalToken => Failure::Denied,
        AgentAppsAuthReadError::Decode => Failure::InvalidResponse,
        AgentAppsAuthReadError::Transport => Failure::Unavailable,
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn read(
    _client: &GatewayClient,
    _context: &QueryContext,
    _target: &Target,
    _refresh: Option<&str>,
) -> CardResult {
    Err(Failure::Unsupported)
}

impl Scope { pub fn same_context(&self, other: &Self) -> bool { self.endpoint == other.endpoint && self.project == other.project && self.host == other.host && self.auth_generation == other.auth_generation } }
#[cfg(test)]
#[path="apps_usage_cards_tests.rs"]
mod tests;

pub use view::Message as ViewMessage;
