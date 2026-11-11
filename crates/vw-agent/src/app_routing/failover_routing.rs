//! Ordinary routing consumes the saved queue without changing its configuration.
use super::{
    aggregation_routing::AggregationTarget, dispatch::JsonTarget, route_auth::VerifiedRouteBinding,
};
use crate::gateway::agent_apps_high_availability_store::FailoverQueueState;
use axum::http::StatusCode;
use vw_agent_apps::{
    aggregation::failover_guard::{FailoverError, admit},
    high_availability::{
        queue::Queue,
        selection::{SelectionError, SelectionSnapshot, select},
    },
    providers::{failover::ProviderSnapshot, live::commit::AppState},
};

pub(crate) struct FailoverPublication {
    pub(crate) binding: VerifiedRouteBinding,
    pub(crate) revision: String,
    pub(crate) state: AppState,
    pub(crate) providers: Vec<ProviderSnapshot>,
    pub(crate) default: AggregationTarget,
    pub(crate) members: Vec<AggregationTarget>,
}

impl FailoverPublication {
    /// The listener calls this inside its control permit. A configuration commit
    /// must recheck the state and queue preimages before publishing the token.
    pub(super) fn select(
        self,
        queue: &FailoverQueueState,
    ) -> Result<(String, JsonTarget), StatusCode> {
        if self.binding.app() != vw_api_types::agent_apps::AppId::Codex {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        admit(self.binding.app(), self.binding.project().tenant_scope(), &self.state).map_err(
            |error| match error {
                FailoverError::Forbidden => StatusCode::FORBIDDEN,
                FailoverError::UnsupportedApp => StatusCode::UNPROCESSABLE_ENTITY,
                FailoverError::Inactive | FailoverError::AggregationConflict => {
                    StatusCode::CONFLICT
                }
            },
        )?;
        if self.revision.trim().is_empty()
            || self.state.mode.as_ref().and_then(|mode| mode.proxy_route.as_deref())
                != Some(self.default.provider_id.as_str())
        {
            return Err(StatusCode::CONFLICT);
        }
        let eligible = Queue::new(self.binding.app(), Vec::new())
            .candidates(&self.providers)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let targets = std::iter::once(self.default).chain(self.members).collect::<Vec<_>>();
        let mut ids = std::collections::HashSet::new();
        for target in &targets {
            if target.binding != self.binding {
                return Err(StatusCode::FORBIDDEN);
            }
            if target.revision != self.revision || !ids.insert(&target.provider_id) {
                return Err(StatusCode::CONFLICT);
            }
            if !eligible.iter().any(|provider| provider.id == target.provider_id) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            crate::model_relay::upstream::validate_target(
                &target.target.url,
                &reqwest::Request::new(reqwest::Method::POST, target.target.url.clone()),
            )?;
            vw_agent_apps::providers::validation::validate(
                self.binding.app(),
                &target.target.request_options,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        }
        // Stale/ineligible entries remain saved, but are never request targets.
        let snapshot = SelectionSnapshot {
            app: self.binding.app(),
            tenant: self.binding.project().tenant_scope(),
            state: &self.state,
            enabled: queue.enabled,
            members: &queue.members,
            providers: &self.providers,
        };
        // Initial publication has no admitted attempts. Production request
        // selection uses the same rule with the execution owner's registry.
        let candidates = select(&snapshot, |_| true).map_err(|error| match error {
            SelectionError::Forbidden => StatusCode::FORBIDDEN,
            SelectionError::UnsupportedMode => StatusCode::CONFLICT,
            SelectionError::NoProvidersConfigured | SelectionError::AllProvidersCircuitOpen => {
                StatusCode::SERVICE_UNAVAILABLE
            }
        })?;
        let chosen = candidates
            .iter()
            .find(|entry| targets.iter().any(|target| target.provider_id == entry.provider.id))
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let index = targets
            .iter()
            .position(|target| target.provider_id == chosen.provider.id)
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let selected = targets.into_iter().nth(index).ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        Ok((selected.provider_id, selected.target))
    }
}
