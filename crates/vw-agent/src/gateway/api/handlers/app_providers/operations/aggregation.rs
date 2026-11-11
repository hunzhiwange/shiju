//! Project/subject/action ownership on the existing provider registry.
use super::*;
use vw_api_types::{agent_apps::aggregation::*, id::ProjectScope};
#[derive(Clone, PartialEq, Eq)]
pub(in crate::gateway::api::handlers) struct Owner {
    pub project: ProjectScope,
    pub host: String,
    pub app: AppId,
    pub principal: String,
}
pub(super) struct Owned {
    owner: Owner,
    outcome: Arc<Mutex<Option<AggregationOutcome>>>,
}
impl Running {
    fn aggregation_progress(&self) -> serde_json::Value {
        let owned = self.aggregation.as_ref().expect("scoped aggregation operation");
        let mut progress = self.operation.snapshot();
        let outcome = if matches!(progress.status, TaskStatus::Completed | TaskStatus::Failed) {
            owned.outcome.lock().unwrap_or_else(|e| e.into_inner()).clone().or_else(|| {
                (progress.status == TaskStatus::Failed).then(|| AggregationOutcome::Failed {
                    status: 502,
                    code: AggregationErrorCode::TargetFailed,
                    recovery: RecoveryState::Failed,
                })
            })
        } else {
            None
        };
        progress.aggregation_outcome = outcome;
        serde_json::to_value(progress).unwrap_or_default()
    }
}
impl State {
    pub(in crate::gateway::api::handlers) fn aggregation_policy(&self) -> Option<&std::path::Path> {
        self.policy.as_deref()
    }
    pub(in crate::gateway::api::handlers) fn aggregation_replay(
        &self,
        owner: &Owner,
        request: &str,
        fingerprint: &[u8; 32],
    ) -> Result<Option<serde_json::Value>, StatusCode> {
        let registry = self.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        replay(&registry, owner, request, fingerprint)
    }
    pub(in crate::gateway::api::handlers) fn aggregation_start(
        &self,
        owner: Owner,
        request: String,
        fingerprint: [u8; 32],
        work: impl FnOnce() -> AggregationOutcome + Send + 'static,
    ) -> Result<serde_json::Value, StatusCode> {
        let mut registry = self.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if let Some(progress) = replay(&registry, &owner, &request, &fingerprint)? {
            return Ok(progress);
        }
        // Do not evict terminal requests: retry must never reapply a publication.
        if registry.len() >= 1024 {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let operation = ProviderOperation::new(TaskId::from(id.clone()), request.clone());
        let outcome = Arc::new(Mutex::new(None));
        let worker = outcome.clone();
        operation
            .start(move || {
                let result = work();
                let succeeded = matches!(result, AggregationOutcome::Published { .. });
                *worker.lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
                if succeeded { Ok(()) } else { Err("aggregation publication failed".into()) }
            })
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let running = Running {
            tenant: owner.project.tenant_scope().clone(),
            host: owner.host.clone(),
            request_id: request,
            fingerprint,
            operation,
            editor_outcome: Default::default(),
            aggregation: Some(Owned { owner, outcome }),
        };
        let progress = running.aggregation_progress();
        registry.insert(id, running);
        Ok(progress)
    }
    pub(super) fn aggregation_owned(
        &self,
        headers: &HeaderMap,
        host: &str,
        id: &str,
        cancel: bool,
        project: Option<ProjectScope>,
        authenticated: &str,
    ) -> Result<Option<serde_json::Value>, StatusCode> {
        let owner = {
            let registry = self.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            registry.get(id).and_then(|r| r.aggregation.as_ref()).map(|r| r.owner.clone())
        };
        let Some(owner) = owner else { return Ok(None) };
        let requested = project.ok_or(StatusCode::NOT_FOUND)?;
        if owner.project != requested || owner.host != host || owner.principal != authenticated {
            return Err(StatusCode::NOT_FOUND);
        }
        // Authorize each lookup/cancel, with no existence disclosure to another scope.
        let (_, _, principal) = super::super::super::agent_apps::authorize_aggregation_write(
            self.policy.as_deref(),
            headers,
            &requested,
            host,
            owner.app,
            true,
        )?;
        if owner.project != requested || owner.host != host || owner.principal != principal {
            return Err(StatusCode::NOT_FOUND);
        }
        let registry = self.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let running = registry.get(id).ok_or(StatusCode::NOT_FOUND)?;
        if cancel {
            running.operation.cancel();
        }
        Ok(Some(running.aggregation_progress()))
    }
}
fn replay(
    registry: &HashMap<String, Running>,
    owner: &Owner,
    request: &str,
    fingerprint: &[u8; 32],
) -> Result<Option<serde_json::Value>, StatusCode> {
    if let Some(running) = registry.values().find(|r| {
        r.aggregation.as_ref().is_some_and(|r| &r.owner == owner) && r.request_id == request
    }) {
        if &running.fingerprint != fingerprint {
            return Err(StatusCode::CONFLICT);
        }
        return Ok(Some(running.aggregation_progress()));
    }
    Ok(None)
}
