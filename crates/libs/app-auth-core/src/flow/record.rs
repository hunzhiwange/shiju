use super::*;

impl FlowRecord {
    pub(super) fn scope(&self) -> AuthAccountScope<'_> {
        AuthAccountScope {
            tenant: &self.tenant,
            principal: &self.principal,
            host: &self.host,
            service: self.service,
        }
    }

    pub(super) fn state_at(
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

    pub(super) fn reap_expired(&self, now: Instant) -> Result<(), AuthErrorCode> {
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

    pub(super) fn validate_account(&self, account: &AuthAccount) -> Result<(), AuthErrorCode> {
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

    pub(super) fn commit(
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
