//! Gateway-owned polling for explicitly authorized provider quota queries.

use std::{
    collections::HashMap,
    hash::{BuildHasher, RandomState},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use futures_util::{
    FutureExt,
    future::{BoxFuture, Shared, WeakShared},
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

use super::access::{UsageRefreshTarget, usage_refresh_targets};
use crate::{
    agent_apps::usage_settings::{UsageCredentialRefs, UsageSettingsScope, UsageSettingsStore},
    app_providers::AppProviderStore,
    global_database::GlobalDatabase,
    project::store::ProjectStore,
};

const POLL: Duration = Duration::from_secs(60);
const CONCURRENCY: usize = 4;
type RefreshFuture = BoxFuture<'static, Arc<Result<UsageCacheResult, RefreshError>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RefreshError {
    Revoked,
    Unavailable,
    InvalidConfiguration,
}

#[derive(Debug, PartialEq)]
pub(super) struct RefreshItem {
    provider: String,
    pub(super) result: Arc<Result<UsageCacheResult, RefreshError>>,
}

struct Query {
    key: UsageCacheKey,
    origin: Url,
    base_url: String,
    api_key: String,
    interval: Option<Duration>,
}

#[derive(Default)]
struct State {
    attempts: HashMap<UsageCacheKey, Instant>,
    in_flight: HashMap<UsageCacheKey, WeakShared<RefreshFuture>>,
    cache: UsageCache,
}

pub(crate) struct UsageRefresh {
    pub(super) database: GlobalDatabase,
    settings: UsageSettingsStore,
    pub(super) projects: ProjectStore,
    // Process-local, keyed generations. Never store raw credentials in cache keys.
    generations: RandomState,
    state: Mutex<State>,
}

impl UsageRefresh {
    pub(super) fn new(database: GlobalDatabase) -> Result<Arc<Self>, RefreshError> {
        Ok(Arc::new(Self {
            settings: UsageSettingsStore::new(database.clone())
                .map_err(|_| RefreshError::Unavailable)?,
            projects: ProjectStore::new(database.clone()).map_err(|_| RefreshError::Unavailable)?,
            database,
            generations: RandomState::new(),
            state: Mutex::default(),
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
                    Ok(query) => self.refresh(target, query, scheduled).await?,
                    Err(error) => Arc::new(Err(error)),
                };
                Some(RefreshItem { provider, result })
            })
            .buffered(CONCURRENCY)
            .filter_map(std::future::ready)
            .collect()
            .await
    }

    fn prepare(&self, target: &UsageRefreshTarget) -> Result<Option<Query>, RefreshError> {
        let scope = UsageSettingsScope {
            project: &target.project,
            principal: &target.principal,
            target_host: &target.host,
            app: target.app,
            provider: &target.provider,
            account: None,
        };
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
        if self.projects.get(&target.project).map_err(|_| RefreshError::Unavailable)?.is_none() {
            return Ok(None);
        }
        let record = AppProviderStore::preview_record(
            &self.database,
            target.project.tenant_scope(),
            target.app,
            &target.provider,
        )
        .map_err(|_| RefreshError::Unavailable)?;
        let profile = profile(&record).map_err(|_| RefreshError::InvalidConfiguration)?;
        validate_profile(target.app, &profile).map_err(|_| RefreshError::InvalidConfiguration)?;
        let base = Url::parse(&profile.base_url).map_err(|_| RefreshError::InvalidConfiguration)?;
        if base.origin() != target.origin.origin() {
            return Err(RefreshError::InvalidConfiguration);
        }
        let key = self.general_key(scope, &profile, settings.refresh_interval_minutes);
        let interval = (settings.refresh_interval_minutes > 0)
            .then(|| Duration::from_secs(u64::from(settings.refresh_interval_minutes) * 60));
        Ok(Some(Query {
            key,
            origin: target.origin.clone(),
            base_url: profile.base_url,
            api_key: profile.api_key,
            interval,
        }))
    }

    fn general_key(
        &self,
        scope: UsageSettingsScope<'_>,
        profile: &AppProviderWrite,
        interval: u16,
    ) -> UsageCacheKey {
        UsageCacheKey {
            project: scope.project.clone(),
            principal: scope.principal.to_owned(),
            host: scope.target_host.to_owned(),
            app: scope.app,
            provider: scope.provider.to_owned(),
            kind: UsageQueryKind::CustomScript,
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
                capabilities.push(match capability {
                    NativeUsageCapability::Balance(_) => UsageQueryKind::Balance,
                    NativeUsageCapability::CodingPlan(_) => UsageQueryKind::CodingPlan,
                });
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
                    let key = self.general_key(scope, &profile, settings.refresh_interval_minutes);
                    snapshot =
                        self.state.lock().cache.snapshot(&key, now).map(|result| match result {
                            UsageCacheResult::Current(result) => {
                                UsageSnapshot { result, stale: false, refresh_failure: None }
                            }
                            UsageCacheResult::Stale { last_success, failure } => UsageSnapshot {
                                result: last_success.as_ref().clone(),
                                stale: true,
                                refresh_failure: Some(failure),
                            },
                        });
                }
            }
        }
        Ok(UsageSnapshotResponse { capabilities, snapshot })
    }

    async fn refresh(
        self: &Arc<Self>,
        target: UsageRefreshTarget,
        query: Query,
        scheduled: Option<Instant>,
    ) -> Option<Arc<Result<UsageCacheResult, RefreshError>>> {
        let request: Shared<RefreshFuture> = {
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
            state.in_flight.retain(|_, pending| pending.upgrade().is_some());
            if let Some(in_flight) = state.in_flight.get(&query.key).and_then(WeakShared::upgrade) {
                in_flight
            } else {
                let key = query.key.clone();
                let service = Arc::clone(self);
                let request = async move {
                    Arc::new(service.execute(target, query, scheduled.is_some()).await)
                }
                .boxed()
                .shared();
                state.in_flight.insert(key, request.downgrade().expect("new request is pending"));
                request
            }
        };
        Some(request.await)
    }

    async fn recheck(
        self: &Arc<Self>,
        target: UsageRefreshTarget,
        key: UsageCacheKey,
        automatic: bool,
    ) -> Result<(), RefreshError> {
        let service = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let result = if usage_refresh_targets(&target.policy).contains(&target) {
                service.prepare(&target)
            } else {
                Ok(None)
            };
            match result {
                Ok(Some(query)) if query.key == key && (!automatic || query.interval.is_some()) => {
                    Ok(())
                }
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
    ) -> Result<UsageCacheResult, RefreshError> {
        self.recheck(target.clone(), query.key.clone(), automatic).await?;
        let PreparedUsage::Script(script) = UsageTemplate::General
            .prepare(
                None,
                ScriptParameters::default(),
                ProviderDefaults { api_key: Some(&query.api_key), base_url: Some(&query.base_url) },
            )
            .map_err(|_| RefreshError::InvalidConfiguration)?
        else {
            return Err(RefreshError::InvalidConfiguration);
        };
        let operation = Operation::new();
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let response = script.run(15, std::slice::from_ref(&query.origin), &operation);
        tokio::pin!(response);
        let mut authorization_checks = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_secs(1),
            Duration::from_secs(1),
        );
        authorization_checks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let response = loop {
            tokio::select! {
                result = &mut response => break result,
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
                        kind: query.key.kind,
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
        let result = UsageQueryResult {
            source: UsageSource::Script("general".into()),
            queried_at: Some(TimestampMs(chrono::Utc::now().timestamp_millis())),
            credential_state,
            credential_message: None,
            outcome,
        };
        Ok(self.state.lock().cache.record(&query.key, result, Instant::now()))
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
            let failed = match item.result.as_ref() {
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
