//! Short-lived editor references share the diagnostic host lifetime, never provider storage.
use super::{
    DiagnosticJobError, DiagnosticJobOwner, DiagnosticJobs, MAX_JOBS, MAX_PROBES_PER_JOB, RETENTION,
};
use tokio::time::Instant;
use vw_agent_apps::diagnostics::endpoints::EndpointCandidate;
use vw_api_types::{
    agent_apps_diagnostics::{DiagnosticEndpointCandidate, DiagnosticEndpointId},
    id::ProviderId,
};
#[derive(Default)]
pub(super) struct Candidates(Vec<Batch>);
struct Batch {
    owner: DiagnosticJobOwner,
    provider: ProviderId,
    expires: Instant,
    rows: Vec<DiagnosticEndpointCandidate>,
}
impl DiagnosticJobs {
    pub(crate) fn remember_endpoint_candidates(
        &self,
        owner: DiagnosticJobOwner,
        provider: ProviderId,
        rows: Vec<DiagnosticEndpointCandidate>,
    ) -> Result<(), DiagnosticJobError> {
        if rows.len() > MAX_PROBES_PER_JOB {
            return Err(DiagnosticJobError::InvalidInput);
        }
        let mut cache = self.0.endpoints.lock();
        cache.0.retain(|b| {
            b.expires > Instant::now() && !(b.owner == owner && b.provider == provider)
        });
        if cache.0.len() >= MAX_JOBS {
            return Err(DiagnosticJobError::CapacityExceeded);
        }
        cache.0.push(Batch { owner, provider, rows, expires: Instant::now() + RETENTION });
        Ok(())
    }
    pub(crate) fn resolve_endpoint_candidates(
        &self,
        owner: &DiagnosticJobOwner,
        provider: &ProviderId,
        ids: &[DiagnosticEndpointId],
    ) -> Option<Vec<(DiagnosticEndpointId, EndpointCandidate)>> {
        let mut cache = self.0.endpoints.lock();
        cache.0.retain(|b| b.expires > Instant::now());
        let batch = cache.0.iter().find(|b| &b.owner == owner && &b.provider == provider)?;
        let mut seen = std::collections::HashSet::new();
        ids.iter()
            .map(|id| {
                if !seen.insert(id) {
                    return None;
                }
                let row = batch.rows.iter().find(|r| &r.id == id)?;
                Some((
                    id.clone(),
                    EndpointCandidate {
                        url: row.display_url.clone(),
                        sources: row.sources.clone(),
                    },
                ))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "endpoints_tests.rs"]
mod tests;
