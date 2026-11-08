//! Execution-host registry for scoped authorization attempts.

use crate::providers::validate_identity;
use std::{
    collections::{HashMap, hash_map::Entry},
    future::Future,
    sync::{Arc, Mutex, MutexGuard, TryLockError},
    time::{Duration, Instant},
};
use uuid::Uuid;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthError, AuthErrorCode, AuthFlow, AuthFlowId,
        AuthLoginMethod, AuthService, AuthStartRequest,
    },
    id::TenantScope,
};

/// Resolve this scope from authorized tenant membership, principal and host policy.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AuthAccountScope<'a> {
    pub tenant: &'a TenantScope,
    pub principal: &'a str,
    pub host: &'a str,
    pub service: AuthService,
}

impl AuthAccountScope<'_> {
    pub fn validate(self) -> Result<(), AuthErrorCode> {
        if self.principal.trim().is_empty() || self.principal.chars().any(char::is_control) {
            return Err(AuthErrorCode::InvalidInput);
        }
        validate_identity(self.host).map_err(|_| AuthErrorCode::InvalidInput)
    }
}

/// Host-derived timing. The UTC timestamp is for display; eligibility uses the
/// monotonic deadline so changes to the system clock cannot extend a flow.
pub struct AuthFlowTiming {
    pub expires_at: TimestampMs,
    pub deadline: Instant,
    pub retry_after_seconds: u64,
}

/// The host adapter translates the upstream response before returning it.
/// A successful exchange may carry private data for the scoped commit only.
pub enum AuthDevicePoll<T> {
    Pending,
    SlowDown,
    SlowDownCapped(u64),
    Expired,
    Complete(Result<(AuthAccount, T), AuthErrorCode>),
}

type FlowCleanup = Box<dyn FnMut() -> Result<(), AuthErrorCode> + Send>;

struct FlowState {
    snapshot: AuthFlow,
    cancelling: bool,
    polling: bool,
    next_poll: Instant,
}

impl FlowState {
    fn expire(&mut self) {
        if let AuthFlow::Pending { flow_id, expires_at, .. } = &self.snapshot {
            self.snapshot = AuthFlow::Expired { flow_id: flow_id.clone(), expires_at: *expires_at };
        }
    }
}

struct FlowRecord {
    tenant: TenantScope,
    principal: String,
    host: String,
    service: AuthService,
    request: AuthStartRequest,
    deadline: Instant,
    state: Mutex<FlowState>,
    cleanup: Mutex<Option<FlowCleanup>>,
}

impl FlowRecord {
    fn scope(&self) -> AuthAccountScope<'_> {
        AuthAccountScope {
            tenant: &self.tenant,
            principal: &self.principal,
            host: &self.host,
            service: self.service,
        }
    }

    fn state_at(
        &self,
        clock: impl FnOnce() -> Instant,
    ) -> Result<(MutexGuard<'_, FlowState>, Instant), AuthErrorCode> {
        let mut state = self.state.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let now = clock();
        if !state.cancelling && now >= self.deadline {
            state.expire();
        }
        Ok((state, now))
    }

    fn reap_expired(&self, now: Instant) -> Result<(), AuthErrorCode> {
        {
            let (state, _) = self.state_at(|| now)?;
            if !(matches!(state.snapshot, AuthFlow::Expired { .. })
                || state.cancelling && now >= self.deadline)
            {
                return Ok(());
            }
        }
        // Another sweep or explicit cancellation can be stopping this flow.
        // Do not wait for that work before processing other records.
        let mut cleanup = match self.cleanup.try_lock() {
            Ok(cleanup) => cleanup,
            Err(TryLockError::WouldBlock) => return Ok(()),
            Err(TryLockError::Poisoned(_)) => return Err(AuthErrorCode::ExecutionHostUnavailable),
        };
        // No state lock while stopping a worker that might still return a result.
        if let Some(stop) = cleanup.as_mut() {
            stop()?;
        }
        *cleanup = None;

        let mut state = self.state.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        if state.cancelling
            && let AuthFlow::Pending { flow_id, expires_at, .. } = &state.snapshot
        {
            state.snapshot =
                AuthFlow::Cancelled { flow_id: flow_id.clone(), expires_at: *expires_at };
            state.cancelling = false;
        }
        Ok(())
    }

    fn validate_account(&self, account: &AuthAccount) -> Result<(), AuthErrorCode> {
        validate_identity(&account.account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        if account.credential_state != AuthCredentialState::Ready {
            return Err(AuthErrorCode::InvalidInput);
        }
        if account.service != self.service
            || self.request.target_account_id.as_ref().is_some_and(|id| *id != account.account_id)
            || self.request.region.is_some_and(|region| Some(region) != account.region)
            || self
                .request
                .github_domain
                .as_ref()
                .is_some_and(|domain| Some(domain) != account.github_domain.as_ref())
        {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        Ok(())
    }

    fn commit(
        &self,
        state: &mut FlowState,
        flow_id: &AuthFlowId,
        result: Result<AuthAccount, AuthErrorCode>,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthStartRequest,
            &AuthAccount,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        if state.cancelling {
            return Err(AuthErrorCode::Conflict);
        }
        let AuthFlow::Pending { expires_at, .. } = state.snapshot else {
            return Err(AuthErrorCode::Conflict);
        };
        state.snapshot = match result.and_then(|account| {
            self.validate_account(&account)?;
            persist(self.scope(), &self.request, &account)?;
            Ok(account)
        }) {
            Ok(account) => AuthFlow::Approved { flow_id: flow_id.clone(), expires_at, account },
            Err(code) => AuthFlow::Failed {
                flow_id: flow_id.clone(),
                expires_at,
                error: AuthError {
                    code,
                    message: "Authorization could not be completed.".into(),
                    correlation_id: Uuid::new_v4().to_string(),
                },
            },
        };
        Ok(state.snapshot.clone())
    }
}

struct InFlightPoll<'a>(&'a FlowRecord);

impl Drop for InFlightPoll<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.polling = false;
        }
    }
}

fn next_poll(now: Instant, interval: u64, deadline: Instant) -> Instant {
    now.checked_add(Duration::from_secs(interval)).map_or(deadline, |next| next.min(deadline))
}

/// One registry per host runtime. Challenges and upstream secrets stay in
/// their host service adapter; only scoped, redacted state lives here.
#[derive(Default)]
pub struct AuthFlows {
    records: Mutex<HashMap<String, Arc<FlowRecord>>>,
    #[cfg(target_os = "macos")]
    replacement: Mutex<()>,
}

impl AuthFlows {
    /// Run from the execution host's maintenance loop, independent of client
    /// requests. Keep terminal snapshots readable and retry failed cleanup on
    /// the next tick. Cleanup can block, so do not call on an async worker.
    pub fn reap_expired(&self, now: Instant) -> Result<(), AuthErrorCode> {
        let records: Vec<_> = self
            .records
            .lock()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
            .values()
            .cloned()
            .collect();
        let mut first_error = None;
        for record in records {
            if let Err(error) = record.reap_expired(now) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Advance a due device exchange on the execution host. Status reads use
    /// `snapshot` and never send a request. Do not hold locks across HTTP;
    /// check cancellation and expiry again at the scoped commit.
    pub async fn poll_device<T, F: Future<Output = AuthDevicePoll<T>>>(
        &self,
        scope: AuthAccountScope<'_>,
        flow_id: &AuthFlowId,
        clock: impl Fn() -> Instant,
        exchange: impl FnOnce() -> F,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthStartRequest,
            &AuthAccount,
            T,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let record = self.find(scope, flow_id)?;
        if record.request.method != AuthLoginMethod::DeviceCode {
            return Err(AuthErrorCode::Unsupported);
        }
        {
            let (mut state, now) = record.state_at(&clock)?;
            let AuthFlow::Pending { retry_after_seconds, .. } = state.snapshot else {
                return Ok(state.snapshot.clone());
            };
            if state.cancelling || state.polling || now < state.next_poll {
                return Ok(state.snapshot.clone());
            }
            state.next_poll = next_poll(now, retry_after_seconds, record.deadline);
            state.polling = true;
        }
        let _in_flight = InFlightPoll(&record);
        let outcome = exchange().await;
        let (mut state, now) = record.state_at(clock)?;
        if state.cancelling {
            return Err(AuthErrorCode::Conflict);
        }
        if let AuthFlow::Pending { retry_after_seconds, .. } = &mut state.snapshot {
            match outcome {
                AuthDevicePoll::Pending => {}
                outcome @ (AuthDevicePoll::SlowDown | AuthDevicePoll::SlowDownCapped(_)) => {
                    let limit = match outcome {
                        AuthDevicePoll::SlowDownCapped(limit) => limit,
                        _ => u64::MAX,
                    };
                    *retry_after_seconds = retry_after_seconds.saturating_add(5).min(limit);
                    state.next_poll = next_poll(now, *retry_after_seconds, record.deadline);
                }
                AuthDevicePoll::Expired => state.expire(),
                AuthDevicePoll::Complete(Ok((account, payload))) => {
                    return record.commit(
                        &mut state,
                        flow_id,
                        Ok(account),
                        |scope, request, account| persist(scope, request, account, payload),
                    );
                }
                AuthDevicePoll::Complete(Err(error)) => {
                    return record.commit(&mut state, flow_id, Err(error), |_, _, _| Ok(()));
                }
            }
        }
        Ok(state.snapshot.clone())
    }

    /// Register a start with no local resources to stop on cancellation.
    pub fn start(
        &self,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        timing: AuthFlowTiming,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.register(scope, request, timing, None)
    }

    /// Register a start after the service validates its challenge and policy
    /// (including any normalized domain or region, and the target's ownership).
    ///
    /// Bind the adapter's local resource handles to this attempt. Cleanup must
    /// stop the listener or process and wait for it to exit before returning
    /// success. It may be retried after an error and must not call `cancel` on
    /// the same flow. The host also runs it when reaping expired attempts.
    pub fn start_with_cleanup(
        &self,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        timing: AuthFlowTiming,
        cleanup: impl FnMut() -> Result<(), AuthErrorCode> + Send + 'static,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.register(scope, request, timing, Some(Box::new(cleanup)))
    }

    /// Preserve providers' single pending attempt within the authorized scope.
    #[cfg(target_os = "macos")]
    pub(super) fn replace_with_cleanup(
        &self,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        timing: AuthFlowTiming,
        cleanup: impl FnMut() -> Result<(), AuthErrorCode> + Send + 'static,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let _replace =
            self.replacement.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let candidates: Vec<_> = self
            .records
            .lock()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
            .iter()
            .filter(|(_, record)| record.scope() == scope)
            .map(|(id, record)| (AuthFlowId(id.clone()), Arc::clone(record)))
            .collect();
        for (id, record) in candidates {
            let pending =
                matches!(record.state_at(Instant::now)?.0.snapshot, AuthFlow::Pending { .. });
            if pending {
                match self.cancel(scope, &id, Instant::now) {
                    Ok(()) | Err(AuthErrorCode::Conflict) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        self.start_with_cleanup(scope, request, timing, cleanup)
    }

    fn register(
        &self,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        timing: AuthFlowTiming,
        cleanup: Option<FlowCleanup>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        scope.validate()?;
        request.validate_for(scope.service)?;
        if let Some(id) = &request.target_account_id {
            validate_identity(id).map_err(|_| AuthErrorCode::InvalidInput)?;
        }
        let now = Instant::now();
        if timing.deadline <= now
            || timing.expires_at.0 <= 0
            || request.method == AuthLoginMethod::DeviceCode && timing.retry_after_seconds == 0
        {
            return Err(AuthErrorCode::InvalidInput);
        }

        let mut records =
            self.records.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        loop {
            let flow_id = AuthFlowId(Uuid::new_v4().to_string());
            if let Entry::Vacant(entry) = records.entry(flow_id.0.clone()) {
                let state = AuthFlow::Pending {
                    flow_id,
                    expires_at: timing.expires_at,
                    retry_after_seconds: timing.retry_after_seconds,
                };
                entry.insert(Arc::new(FlowRecord {
                    tenant: scope.tenant.clone(),
                    principal: scope.principal.into(),
                    host: scope.host.into(),
                    service: scope.service,
                    request,
                    deadline: timing.deadline,
                    state: Mutex::new(FlowState {
                        snapshot: state.clone(),
                        cancelling: false,
                        polling: false,
                        next_poll: next_poll(now, timing.retry_after_seconds, timing.deadline),
                    }),
                    cleanup: Mutex::new(cleanup),
                }));
                return Ok(state);
            }
        }
    }

    fn find(
        &self,
        scope: AuthAccountScope<'_>,
        flow_id: &AuthFlowId,
    ) -> Result<Arc<FlowRecord>, AuthErrorCode> {
        let records = self.records.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        records
            .get(&flow_id.0)
            .filter(|record| record.scope() == scope)
            .cloned()
            .ok_or(AuthErrorCode::NotFound)
    }

    pub fn snapshot(
        &self,
        scope: AuthAccountScope<'_>,
        flow_id: &AuthFlowId,
        clock: impl FnOnce() -> Instant,
    ) -> Result<AuthFlow, AuthErrorCode> {
        Ok(self.find(scope, flow_id)?.state_at(clock)?.0.snapshot.clone())
    }

    /// Invalidate the attempt before teardown and report cancellation only
    /// after local cleanup succeeds. A failed teardown leaves the attempt
    /// ineligible for completion and allows cancellation to be retried.
    pub fn cancel(
        &self,
        scope: AuthAccountScope<'_>,
        flow_id: &AuthFlowId,
        clock: impl FnOnce() -> Instant,
    ) -> Result<(), AuthErrorCode> {
        let record = self.find(scope, flow_id)?;
        let mut cleanup =
            record.cleanup.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let expires_at = {
            let (mut state, _) = record.state_at(clock)?;
            match state.snapshot {
                AuthFlow::Pending { expires_at, .. } => {
                    state.cancelling = true;
                    expires_at
                }
                AuthFlow::Cancelled { .. } => return Ok(()),
                _ => return Err(AuthErrorCode::Conflict),
            }
        };

        // A worker being stopped may itself be trying to complete this flow.
        if let Some(stop) = cleanup.as_mut() {
            stop()?;
        }
        *cleanup = None;

        let mut state = record.state.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        state.snapshot = AuthFlow::Cancelled { flow_id: flow_id.clone(), expires_at };
        state.cancelling = false;
        Ok(())
    }

    /// Complete a host exchange at most once. The adapter must verify the
    /// canonical upstream identity and any account generation against the bound
    /// scope and request, then commit credentials and metadata before returning
    /// success. Keep network exchanges outside this per-flow commit lock.
    pub fn complete(
        &self,
        scope: AuthAccountScope<'_>,
        flow_id: &AuthFlowId,
        clock: impl FnOnce() -> Instant,
        result: Result<AuthAccount, AuthErrorCode>,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthStartRequest,
            &AuthAccount,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let record = self.find(scope, flow_id)?;
        let (mut state, _) = record.state_at(clock)?;
        if state.polling {
            return Err(AuthErrorCode::Conflict);
        }
        record.commit(&mut state, flow_id, result, persist)
    }
}
