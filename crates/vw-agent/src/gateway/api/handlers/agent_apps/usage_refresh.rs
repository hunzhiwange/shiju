//! Gateway-owned polling for explicitly authorized provider quota queries.

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash, RandomState},
    path::{Path, PathBuf},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};

use futures_util::{
    FutureExt,
    future::{BoxFuture, Shared},
    stream::{self, StreamExt},
};
use parking_lot::Mutex;
use reqwest::Url;
use vw_agent_apps::{
    apps::lifecycle::operations::Operation,
    providers::{update::profile, validate_profile},
    usage::{
        cache::{UsageCache, UsageCacheKey, UsageCacheResult},
        detect::{NativeUsageCapability, ZhipuPlanSelection, detect_native_usage},
        scripts::{
            config::{PreparedUsage, ProviderDefaults, ScriptParameters, UsageTemplate},
            network::NetworkError,
            run::RunError,
        },
        subscriptions::codex::ManagedCodexUsage,
    },
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::AuthErrorCode,
    app_providers::AppProviderWrite,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryError, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageSnapshot, UsageSnapshotResponse, UsageSource,
    },
};

use super::{
    access::{UsageRefreshTarget, usage_refresh_targets, usage_targets},
    usage_codex::CodexScope,
    usage_native::NativeQuery,
};
use crate::{
    agent_apps::usage_settings::{UsageCredentialRefs, UsageSettingsScope, UsageSettingsStore},
    app_providers::AppProviderStore,
    global_database::GlobalDatabase,
    project::store::ProjectStore,
};

const POLL: Duration = Duration::from_secs(60);
pub(super) const CONCURRENCY: usize = 4;
type RefreshResult = Arc<Result<Publication, RefreshError>>;
type RefreshFuture = BoxFuture<'static, RefreshResult>;
#[cfg(test)]
pub(super) type NativeTransport = Arc<dyn Fn() -> reqwest::ClientBuilder + Send + Sync>;

#[derive(Debug, PartialEq)]
pub(super) struct Publication {
    pub(super) snapshot: UsageCacheResult,
    pub(super) version: uuid::Uuid,
}

struct Flight {
    request: Shared<RefreshFuture>,
    operation: Operation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RefreshError {
    Revoked,
    Unavailable,
    InvalidConfiguration,
    NotFound,
    Account(AuthErrorCode),
}

#[derive(Debug, PartialEq)]
pub(super) struct RefreshItem {
    provider: String,
    pub(super) result: RefreshResult,
}

pub(super) struct Query {
    key: UsageCacheKey,
    source: QuerySource,
    interval: Option<Duration>,
}

enum QuerySource {
    General { origin: Url, base_url: String, api_key: String },
    Codex(serde_json::Value),
    Native { query: NativeQuery, api_key: String },
}

#[derive(Default)]
struct State {
    attempts: HashMap<UsageCacheKey, Instant>,
    in_flight: HashMap<UsageCacheKey, Weak<Flight>>,
    cache: UsageCache,
}

pub(crate) struct UsageRefresh {
    pub(super) database: GlobalDatabase,
    pub(super) settings: UsageSettingsStore,
    pub(super) projects: ProjectStore,
    pub(super) operations: super::apps_usage_refresh::Operations,
    // Process-local, keyed generations. Never store raw credentials in cache keys.
    generations: RandomState,
    state: Mutex<State>,
    pub(super) query_slots: tokio::sync::Semaphore,
    #[cfg(test)]
    pub(super) codex_client: Option<ManagedCodexUsage>,
    #[cfg(test)]
    pub(super) native_transport: Option<NativeTransport>,
}

impl UsageRefresh {
    pub(super) fn new(database: GlobalDatabase) -> Result<Arc<Self>, RefreshError> {
        Ok(Arc::new(Self {
            settings: UsageSettingsStore::new(database.clone())
                .map_err(|_| RefreshError::Unavailable)?,
            projects: ProjectStore::new(database.clone()).map_err(|_| RefreshError::Unavailable)?,
            database,
            operations: Default::default(),
            generations: RandomState::new(),
            state: Mutex::default(),
            query_slots: tokio::sync::Semaphore::new(CONCURRENCY),
            #[cfg(test)]
            codex_client: None,
            #[cfg(test)]
            native_transport: None,
        }))
    }

    pub(super) async fn tick(self: &Arc<Self>, policy: &Path, now: Instant) -> Vec<RefreshItem> {
        self.refresh_all(policy, Some(now)).await
    }

    pub(super) async fn refresh_all(
        self: &Arc<Self>,
        policy: &Path,
        scheduled: Option<Instant>,
    ) -> Vec<RefreshItem> {
        let policy = policy.to_owned();
        let service = Arc::clone(self);
        let items = tokio::task::spawn_blocking(move || {
            let mut ready = Vec::new();
            let mut current = std::collections::HashSet::new();
            for target in usage_refresh_targets(&policy) {
                match service.prepare(&target) {
                    Ok(Some(query)) => {
                        if scheduled.is_none() || query.interval.is_some() {
                            current.insert(query.key.clone());
                            ready.push((target, Ok(query)));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => ready.push((target, Err(error))),
                }
            }
            if scheduled.is_some() {
                let mut state = service.state.lock();
                let expired: Vec<_> =
                    state.attempts.keys().filter(|key| !current.contains(*key)).cloned().collect();
                for key in expired {
                    state.attempts.remove(&key);
                    state.cache.invalidate(&key);
                }
            }
            ready
        })
        .await
        .unwrap_or_default();

        stream::iter(items)
            .map(|(target, query)| async move {
                let provider = target.provider.clone();
                let result = match query {
                    Ok(query) => self.refresh(target, query, scheduled, None).await?,
                    Err(error) => Arc::new(Err(error)),
                };
                Some(RefreshItem { provider, result })
            })
            .buffered(CONCURRENCY)
            .filter_map(std::future::ready)
            .collect()
            .await
    }

    pub(super) fn generation(&self, value: impl Hash) -> u64 {
        self.generations.hash_one(value)
    }

    pub(super) fn codex_scope<'a>(&self, target: &'a UsageRefreshTarget) -> Option<CodexScope<'a>> {
        let codex = target.codex.as_ref()?;
        Some(CodexScope {
            selection: UsageSettingsScope {
                project: &target.project,
                principal: &target.principal,
                target_host: &target.host,
                app: target.app,
                provider: &target.provider,
                account: Some(&codex.account),
            },
            credential_root: &codex.credential_root,
            account: &codex.account,
        })
    }

    pub(super) fn prepare(
        &self,
        target: &UsageRefreshTarget,
    ) -> Result<Option<Query>, RefreshError> {
        if self.projects.get(&target.project).map_err(|_| RefreshError::Unavailable)?.is_none() {
            return Ok(None);
        }
        if let Some(scope) = self.codex_scope(target) {
            let (key, auth) = scope.prepare(self).map_err(RefreshError::Account)?;
            return Ok(Some(Query { key, source: QuerySource::Codex(auth), interval: None }));
        }
        let scope = UsageSettingsScope {
            project: &target.project,
            principal: &target.principal,
            target_host: &target.host,
            app: target.app,
            provider: &target.provider,
            account: None,
        };
        let settings = if target.kind == UsageQueryKind::CustomScript {
            let Some(settings) = self.settings.get(scope).map_err(|_| RefreshError::Unavailable)?
            else {
                return Ok(None);
            };
            if !settings.enabled {
                return Ok(None);
            }
            if settings.template != UsageTemplate::General
                || settings.credentials != UsageCredentialRefs::default()
            {
                return Err(RefreshError::InvalidConfiguration);
            }
            Some(settings)
        } else {
            None
        };
        let record = AppProviderStore::usage_record(
            &self.database,
            target.project.tenant_scope(),
            target.app,
            &target.provider,
        )
        .map_err(|error| match error {
            AuthErrorCode::NotFound => RefreshError::NotFound,
            _ => RefreshError::Unavailable,
        })?;
        let profile = profile(&record).map_err(|_| RefreshError::InvalidConfiguration)?;
        validate_profile(target.app, &profile).map_err(|_| RefreshError::InvalidConfiguration)?;
        let base = Url::parse(&profile.base_url).map_err(|_| RefreshError::InvalidConfiguration)?;
        if base.origin() != target.origin.origin() {
            return Err(RefreshError::InvalidConfiguration);
        }
        let Some(settings) = settings else {
            let query = NativeQuery::select(&base, target.kind)
                .ok_or(RefreshError::Account(AuthErrorCode::Unsupported))?;
            return Ok(Some(Query {
                key: self.provider_key(scope, &profile, target.kind, 0),
                source: QuerySource::Native { query, api_key: profile.api_key },
                interval: None,
            }));
        };
        let key =
            self.provider_key(scope, &profile, target.kind, settings.refresh_interval_minutes);
        let interval = (settings.refresh_interval_minutes > 0)
            .then(|| Duration::from_secs(u64::from(settings.refresh_interval_minutes) * 60));
        Ok(Some(Query {
            key,
            source: QuerySource::General {
                origin: target.origin.clone(),
                base_url: profile.base_url,
                api_key: profile.api_key,
            },
            interval,
        }))
    }

    fn provider_key(
        &self,
        scope: UsageSettingsScope<'_>,
        profile: &AppProviderWrite,
        kind: UsageQueryKind,
        interval: u16,
    ) -> UsageCacheKey {
        UsageCacheKey {
            project: scope.project.clone(),
            principal: scope.principal.to_owned(),
            host: scope.target_host.to_owned(),
            app: scope.app,
            provider: scope.provider.to_owned(),
            kind,
            account: None,
            credential_version: self.generations.hash_one(&profile.api_key),
            configuration_version: self.generations.hash_one((&profile.base_url, interval)),
        }
    }

    pub(super) fn provider_snapshot(
        &self,
        scope: UsageSettingsScope<'_>,
        kind: Option<UsageQueryKind>,
        now: Instant,
    ) -> Result<UsageSnapshotResponse, AuthErrorCode> {
        let record = AppProviderStore::usage_record(
            &self.database,
            scope.project.tenant_scope(),
            scope.app,
            scope.provider,
        )?;
        let mut capabilities = Vec::new();
        let mut snapshot = None;
        if let Ok(profile) = profile(&record) {
            if let Ok(Some(capability)) =
                detect_native_usage(&profile.base_url, ZhipuPlanSelection::Auto)
            {
                let native_kind = match capability {
                    NativeUsageCapability::Balance(_) => UsageQueryKind::Balance,
                    NativeUsageCapability::CodingPlan(_) => UsageQueryKind::CodingPlan,
                };
                capabilities.push(native_kind);
                if kind.is_none_or(|kind| kind == native_kind)
                    && validate_profile(scope.app, &profile).is_ok()
                    && Url::parse(&profile.base_url)
                        .is_ok_and(|base| NativeQuery::select(&base, native_kind).is_some())
                {
                    let key = self.provider_key(scope, &profile, native_kind, 0);
                    snapshot = self.snapshot(&key, now);
                }
            }
            let settings =
                self.settings.get(scope).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if let Some(settings) = settings
                && settings.enabled
                && settings.template == UsageTemplate::General
                && settings.credentials == UsageCredentialRefs::default()
                && validate_profile(scope.app, &profile).is_ok()
            {
                capabilities.push(UsageQueryKind::CustomScript);
                if kind.is_none_or(|kind| kind == UsageQueryKind::CustomScript) {
                    let key = self.provider_key(
                        scope,
                        &profile,
                        UsageQueryKind::CustomScript,
                        settings.refresh_interval_minutes,
                    );
                    snapshot = self.snapshot(&key, now).or(snapshot);
                }
            }
        }
        Ok(UsageSnapshotResponse { capabilities, snapshot })
    }

    pub(super) fn snapshot(&self, key: &UsageCacheKey, now: Instant) -> Option<UsageSnapshot> {
        self.state.lock().cache.snapshot(key, now).map(|result| match result {
            UsageCacheResult::Current(result) => {
                UsageSnapshot { result, stale: false, refresh_failure: None }
            }
            UsageCacheResult::Stale { last_success, failure } => UsageSnapshot {
                result: last_success.as_ref().clone(),
                stale: true,
                refresh_failure: Some(failure),
            },
        })
    }

    pub(super) async fn refresh(
        self: &Arc<Self>,
        target: UsageRefreshTarget,
        query: Query,
        scheduled: Option<Instant>,
        consumer: Option<&Operation>,
    ) -> Option<RefreshResult> {
        if consumer.is_some_and(Operation::is_cancelled) {
            return None;
        }
        let flight = {
            let mut state = self.state.lock();
            if let Some(now) = scheduled {
                let interval = query.interval?;
                if state
                    .attempts
                    .get(&query.key)
                    .is_some_and(|at| now.saturating_duration_since(*at) < interval)
                {
                    return None;
                }
                state.attempts.insert(query.key.clone(), now);
            }
            state.in_flight.retain(|_, pending| pending.strong_count() != 0);
            if let Some(in_flight) = state.in_flight.get(&query.key).and_then(Weak::upgrade) {
                in_flight
            } else {
                let service = Arc::clone(self);
                let operation = Operation::new();
                let worker = operation.clone();
                let key = query.key.clone();
                let request = async move {
                    Arc::new(service.execute(target, query, scheduled.is_some(), worker).await)
                }
                .boxed()
                .shared();
                let flight = Arc::new(Flight { request, operation });
                state.in_flight.insert(key, Arc::downgrade(&flight));
                flight
            }
        };
        let Some(consumer) = consumer else {
            return Some(flight.request.clone().await);
        };
        tokio::select! {
            biased;
            result = flight.request.clone() => Some(result),
            _ = consumer.cancelled() => {
                // Admission and the last subscriber's cancellation use the same lock.
                // Detach without interrupting another authorized consumer.
                let cleanup = {
                    let mut state = self.state.lock();
                    if Arc::strong_count(&flight) == 1 {
                        flight.operation.cancel();
                        let weak = Arc::downgrade(&flight);
                        state.in_flight.retain(|_, pending| !pending.ptr_eq(&weak));
                        Some(flight)
                    } else {
                        drop(flight);
                        None
                    }
                };
                if let Some(flight) = cleanup {
                    // QuickJS may be on a blocking thread. Wait for its exit before
                    // reporting the caller's cancellation as terminal.
                    let _ = flight.request.clone().await;
                }
                None
            }
        }
    }

    async fn recheck(
        self: &Arc<Self>,
        target: UsageRefreshTarget,
        key: UsageCacheKey,
        automatic: bool,
    ) -> Result<(), RefreshError> {
        let service = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let current = (|| {
                if !usage_targets(&target.policy, target.batch)
                    .is_ok_and(|targets| targets.contains(&target))
                {
                    return Ok(false);
                }
                if let Some(scope) = service.codex_scope(&target) {
                    let project = service
                        .projects
                        .get(&target.project)
                        .map_err(|_| RefreshError::Unavailable)?;
                    return Ok(project.is_some()
                        && scope.current_key(&service).map_err(RefreshError::Account)?.as_ref()
                            == Some(&key));
                }
                Ok(service.prepare(&target)?.is_some_and(|query| {
                    query.key == key && (!automatic || query.interval.is_some())
                }))
            })();
            match current {
                Ok(true) => Ok(()),
                invalid => {
                    service.state.lock().cache.invalidate(&key);
                    Err(invalid.err().unwrap_or(RefreshError::Revoked))
                }
            }
        })
        .await
        .map_err(|_| RefreshError::Unavailable)?
    }

    async fn execute(
        self: &Arc<Self>,
        target: UsageRefreshTarget,
        query: Query,
        automatic: bool,
        operation: Operation,
    ) -> Result<Publication, RefreshError> {
        let _permit = tokio::select! {
            biased;
            _ = operation.cancelled() => return Err(RefreshError::Revoked),
            permit = self.query_slots.acquire() => permit.map_err(|_| RefreshError::Unavailable)?,
        };
        self.recheck(target.clone(), query.key.clone(), automatic).await?;
        let response = self.query(query.source, &operation);
        tokio::pin!(response);
        let mut authorization_checks = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_secs(1),
            Duration::from_secs(1),
        );
        authorization_checks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let result = loop {
            tokio::select! {
                result = &mut response => break result?,
                _ = authorization_checks.tick() => {
                    if let Err(error) = self.recheck(target.clone(), query.key.clone(), automatic).await {
                        operation.cancel();
                        let _ = response.await;
                        return Err(error);
                    }
                }
            }
        };
        self.recheck(target, query.key.clone(), automatic).await?;
        let mut state = self.state.lock();
        if operation.is_cancelled() {
            return Err(RefreshError::Revoked);
        }
        if let Some((vw_api_types::agent_apps_auth::AuthService::CodexOauth,
            vw_agent_apps::usage::binding::UsageAccount::Managed(account))) = &query.key.account
        {
            crate::model_relay::quota_gate::publish(
                crate::model_relay::quota_gate::Account(
                    query.key.project.tenant_scope().clone(), query.key.principal.clone(),
                    query.key.host.clone(), account.clone()), &result);
        }
        let snapshot = state.cache.record(&query.key, result, Instant::now());
        Ok(Publication { snapshot, version: uuid::Uuid::new_v4() })
    }

    async fn query(
        &self,
        source: QuerySource,
        operation: &Operation,
    ) -> Result<UsageQueryResult, RefreshError> {
        let now = TimestampMs(chrono::Utc::now().timestamp_millis());
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let (origin, base_url, api_key) = match source {
            QuerySource::General { origin, base_url, api_key } => (origin, base_url, api_key),
            QuerySource::Native { query, api_key } => {
                #[cfg(test)]
                let transport = self
                    .native_transport
                    .as_ref()
                    .map_or_else(reqwest::Client::builder, |fixture| fixture());
                #[cfg(not(test))]
                let transport = reqwest::Client::builder();
                return tokio::select! {
                    biased;
                    _ = operation.cancelled() => Err(RefreshError::Revoked),
                    result = query.run(transport.no_proxy(), &api_key, now, &correlation_id) => Ok(result),
                };
            }
            QuerySource::Codex(auth) => {
                #[cfg(test)]
                let fixture = self.codex_client.as_ref();
                #[cfg(not(test))]
                let fixture: Option<&ManagedCodexUsage> = None;
                let real;
                let client = if let Some(fixture) = fixture {
                    fixture
                } else {
                    real = ManagedCodexUsage::new(reqwest::Client::builder())
                        .map_err(RefreshError::Account)?;
                    &real
                };
                return tokio::select! {
                    biased;
                    _ = operation.cancelled() => Err(RefreshError::Revoked),
                    result = client.query(&auth, now, &correlation_id) => Ok(result),
                };
            }
        };
        let PreparedUsage::Script(script) = UsageTemplate::General
            .prepare(
                None,
                ScriptParameters::default(),
                ProviderDefaults { api_key: Some(&api_key), base_url: Some(&base_url) },
            )
            .map_err(|_| RefreshError::InvalidConfiguration)?
        else {
            return Err(RefreshError::InvalidConfiguration);
        };
        let response = script.run(15, std::slice::from_ref(&origin), operation).await;
        let (outcome, credential_state) = match response {
            Ok(packages) => (
                UsageQueryOutcome::Success { data: UsageQueryData::CustomScript(packages) },
                UsageCredentialState::Valid,
            ),
            Err(error) => {
                let code = match error {
                    RunError::Network(NetworkError::Upstream(code)) => code,
                    RunError::Network(_) => UsageQueryErrorCode::MissingConfiguration,
                    RunError::Extract(_) => UsageQueryErrorCode::InvalidResponse,
                    RunError::Cancelled => return Err(RefreshError::Revoked),
                };
                let credential_state = if code == UsageQueryErrorCode::UpstreamUnauthorized {
                    UsageCredentialState::Expired
                } else {
                    UsageCredentialState::Valid
                };
                (
                    UsageQueryOutcome::Failure {
                        kind: UsageQueryKind::CustomScript,
                        error: UsageQueryError {
                            code,
                            message: "Usage query failed".into(),
                            correlation_id,
                        },
                    },
                    credential_state,
                )
            }
        };
        Ok(UsageQueryResult {
            source: UsageSource::Script("general".into()),
            queried_at: Some(TimestampMs(chrono::Utc::now().timestamp_millis())),
            credential_state,
            credential_message: None,
            outcome,
        })
    }
}

pub(crate) async fn init() -> Option<Arc<UsageRefresh>> {
    let refresh = tokio::task::spawn_blocking(|| {
        let database = GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| RefreshError::Unavailable)?;
        UsageRefresh::new(database)
    })
    .await;
    let Ok(Ok(refresh)) = refresh else {
        tracing::warn!("Gateway usage refresh storage unavailable");
        return None;
    };
    Some(refresh)
}

pub(crate) async fn run(refresh: Arc<UsageRefresh>, policy: PathBuf) {
    let mut ticks = tokio::time::interval(POLL);
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let now = ticks.tick().await.into_std();
        for item in refresh.tick(&policy, now).await {
            let failed =
                match item.result.as_ref().as_ref().map(|publication| &publication.snapshot) {
                    Err(_) | Ok(UsageCacheResult::Stale { .. }) => true,
                    Ok(UsageCacheResult::Current(result)) => {
                        matches!(result.outcome, UsageQueryOutcome::Failure { .. })
                    }
                };
            if failed {
                tracing::debug!(provider = %item.provider, "Scheduled usage refresh failed");
            }
        }
    }
}
