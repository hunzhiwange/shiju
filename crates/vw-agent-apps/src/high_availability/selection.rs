//! Candidate ordering from one authorized request's settled mode and queue.
use crate::{
    aggregation::failover_guard::{self, FailoverError},
    providers::{
        failover::{ProviderAuth, ProviderSnapshot},
        live::commit::AppState,
        mode_state::observe,
    },
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    UnsupportedMode,
    Forbidden,
    NoProvidersConfigured,
    AllProvidersCircuitOpen,
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedMode => "application mode does not support failover",
            Self::Forbidden => "application mode belongs to another tenant",
            Self::NoProvidersConfigured => "no eligible providers configured",
            Self::AllProvidersCircuitOpen => "all eligible provider circuits are open",
        })
    }
}

impl std::error::Error for SelectionError {}

/// The execution owner supplies these values from the same request snapshot.
pub struct SelectionSnapshot<'a> {
    pub app: AppId,
    pub tenant: &'a TenantScope,
    pub state: &'a AppState,
    pub enabled: bool,
    pub members: &'a [String],
    pub providers: &'a [ProviderSnapshot],
}

/// Availability is a read only check. Before sending, the executor must still
/// acquire a CircuitAttempt for the selected provider.
pub fn select<'a>(
    snapshot: &SelectionSnapshot<'a>,
    mut available: impl FnMut(&str) -> bool,
) -> Result<Vec<&'a ProviderSnapshot>, SelectionError> {
    let mode = observe(snapshot.state, snapshot.tenant).map_err(|_| SelectionError::Forbidden)?;
    let current = snapshot.providers.iter().find(|entry| {
        entry.provider.app_id == snapshot.app
            && entry.provider.configured
            && Some(entry.provider.id.as_str()) == mode.in_use_provider.as_deref()
    });
    if !snapshot.enabled
        || snapshot.app == AppId::Codex
            && current.is_some_and(|entry| entry.auth == ProviderAuth::OfficialAccount)
    {
        return current.map(|entry| vec![entry]).ok_or(SelectionError::NoProvidersConfigured);
    }
    failover_guard::admit(snapshot.app, snapshot.tenant, snapshot.state).map_err(|error| {
        match error {
            FailoverError::Forbidden => SelectionError::Forbidden,
            _ => SelectionError::UnsupportedMode,
        }
    })?;
    let candidates = snapshot
        .members
        .iter()
        .filter_map(|id| {
            snapshot.providers.iter().find(|entry| {
                entry.provider.id == *id
                    && entry.provider.app_id == snapshot.app
                    && entry.provider.configured
                    && entry.auth == ProviderAuth::ThirdParty
            })
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(SelectionError::NoProvidersConfigured);
    }
    let mut seen = std::collections::HashSet::new();
    let available = candidates
        .into_iter()
        .filter(|entry| seen.insert(entry.provider.id.as_str()) && available(&entry.provider.id))
        .collect::<Vec<_>>();
    if available.is_empty() {
        return Err(SelectionError::AllProvidersCircuitOpen);
    }
    Ok(available)
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod selection_tests;
