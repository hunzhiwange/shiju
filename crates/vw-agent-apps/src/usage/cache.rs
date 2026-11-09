//! Short-lived fallback for completed usage queries.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    apps_usage::{
        UsageCredentialState, UsageQueryErrorCode, UsageQueryKind, UsageQueryOutcome,
        UsageQueryResult,
    },
    id::ProjectScope,
};

use super::binding::UsageAccount;

const SUCCESS_TTL: Duration = Duration::from_secs(10 * 60);

/// An authorized query's resolved identity. Versions are opaque generations
/// supplied by the caller. The key contains neither credential material nor URLs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UsageCacheKey {
    pub project: ProjectScope,
    pub principal: String,
    pub host: String,
    pub app: AppId,
    pub provider: String,
    pub kind: UsageQueryKind,
    /// Queries that use provider credentials without an account use `None`.
    pub account: Option<(AuthService, UsageAccount)>,
    pub credential_version: u64,
    pub configuration_version: u64,
}

#[derive(Debug, PartialEq)]
pub enum UsageCacheResult {
    Current(UsageQueryResult),
    /// Display the original data and success time alongside the current failure.
    Stale {
        last_success: Arc<UsageQueryResult>,
        failure: UsageQueryResult,
    },
}

#[derive(Default)]
pub struct UsageCache {
    last_successes: HashMap<UsageCacheKey, (Instant, Arc<UsageQueryResult>)>,
}

impl UsageCache {
    /// Record a fresh execution result at its monotonic completion time.
    /// The caller must recheck authorization, host availability and the resolved
    /// key before publishing, and serialize refreshes of the same key.
    pub fn record(
        &mut self,
        key: &UsageCacheKey,
        result: UsageQueryResult,
        now: Instant,
    ) -> UsageCacheResult {
        use UsageQueryErrorCode::{Timeout, Transport, UpstreamRateLimited, UpstreamUnavailable};

        match (&result.credential_state, &result.outcome) {
            (UsageCredentialState::Valid, UsageQueryOutcome::Success { .. })
                if result.queried_at.is_some() =>
            {
                self.last_successes
                    .retain(|_, (at, _)| now.saturating_duration_since(*at) < SUCCESS_TTL);
                self.last_successes.insert(key.clone(), (now, Arc::new(result.clone())));
            }
            (UsageCredentialState::Valid, UsageQueryOutcome::Failure { error, .. })
                if matches!(
                    error.code,
                    Timeout | Transport | UpstreamRateLimited | UpstreamUnavailable
                ) =>
            {
                if let Some((at, last_success)) = self.last_successes.get(key)
                    && now.checked_duration_since(*at).is_some_and(|age| age < SUCCESS_TTL)
                {
                    return UsageCacheResult::Stale {
                        last_success: last_success.clone(),
                        failure: result,
                    };
                }
                self.invalidate(key);
            }
            _ => self.invalidate(key),
        }
        UsageCacheResult::Current(result)
    }

    /// Drop the previous key when a binding, credential or query setting changes.
    pub fn invalidate(&mut self, key: &UsageCacheKey) {
        self.last_successes.remove(key);
    }
}
