//! Execution-host registry for scoped authorization attempts.

use crate::providers::validate_identity;
use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};
use uuid::Uuid;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthError, AuthErrorCode, AuthFlow, AuthFlowId,
        AuthService, AuthStartRequest,
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

type CancelCleanup = Box<dyn FnMut() -> Result<(), AuthErrorCode> + Send>;

struct FlowState {
    snapshot: AuthFlow,
    cancelling: bool,
}

struct FlowRecord {
    tenant: TenantScope,
    principal: String,
    host: String,
    service: AuthService,
    request: AuthStartRequest,
    deadline: Instant,
    state: Mutex<FlowState>,
    cancel_cleanup: Mutex<Option<CancelCleanup>>,
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
    ) -> Result<MutexGuard<'_, FlowState>, AuthErrorCode> {
        let mut state = self.state.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        if !state.cancelling
            && let AuthFlow::Pending { flow_id, expires_at, .. } = &state.snapshot
            && clock() >= self.deadline
        {
            state.snapshot =
                AuthFlow::Expired { flow_id: flow_id.clone(), expires_at: *expires_at };
        }
        Ok(state)
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
}

/// One registry per host runtime. Challenges and upstream secrets stay in
/// their host service adapter; only scoped, redacted state lives here.
#[derive(Default)]
pub struct AuthFlows {
    records: Mutex<HashMap<String, Arc<FlowRecord>>>,
}

impl AuthFlows {
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
    /// the same flow. The adapter owns teardown on paths other than cancellation.
    pub fn start_with_cleanup(
        &self,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        timing: AuthFlowTiming,
        cleanup: impl FnMut() -> Result<(), AuthErrorCode> + Send + 'static,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.register(scope, request, timing, Some(Box::new(cleanup)))
    }

    fn register(
        &self,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        timing: AuthFlowTiming,
        cancel_cleanup: Option<CancelCleanup>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        scope.validate()?;
        request.validate_for(scope.service)?;
        if let Some(id) = &request.target_account_id {
            validate_identity(id).map_err(|_| AuthErrorCode::InvalidInput)?;
        }
        if timing.deadline <= Instant::now() || timing.expires_at.0 <= 0 {
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
                    state: Mutex::new(FlowState { snapshot: state.clone(), cancelling: false }),
                    cancel_cleanup: Mutex::new(cancel_cleanup),
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
        Ok(self.find(scope, flow_id)?.state_at(clock)?.snapshot.clone())
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
            record.cancel_cleanup.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let expires_at = {
            let mut state = record.state_at(clock)?;
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
        let mut state = record.state_at(clock)?;
        if state.cancelling {
            return Err(AuthErrorCode::Conflict);
        }
        let AuthFlow::Pending { expires_at, .. } = state.snapshot else {
            return Err(AuthErrorCode::Conflict);
        };

        state.snapshot = match result.and_then(|account| {
            record.validate_account(&account)?;
            persist(record.scope(), &record.request, &account)?;
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
