//! Desktop view state retains host-issued jobs while a panel is closed.
use crate::app::state::app_providers::list::Scope;
use vw_gateway_client::vw_api_types::agent_apps_diagnostics::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelContext {
    pub scope: Scope,
    pub project: vw_gateway_client::vw_api_types::ProjectId,
    pub credentials_generation: u64,
    pub provider: vw_gateway_client::vw_api_types::ProviderId,
}
impl ModelContext {
    pub fn query(&self) -> DiagnosticScopeQuery {
        DiagnosticScopeQuery {
            project_id: self.project.clone(),
            target_host_id: self.scope.host.clone(),
            app_id: self.scope.app,
        }
    }
    pub fn matches(&self, job: &DiagnosticJobIdentity, kind: DiagnosticProbeKind) -> bool {
        job.kind == kind
            && job.scope.target_host_id == self.scope.host
            && job.scope.app_id == self.scope.app
            && job.scope.project.project_id() == &self.project
            && job.scope.project.tenant_scope() == &self.scope.context.tenant
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ModelRun {
    pub context: ModelContext,
    pub model: String,
    pub kind: DiagnosticProbeKind,
    pub serial: u64,
    pub job: Option<DiagnosticJobIdentity>,
    pub snapshot: Option<DiagnosticJobSnapshot>,
    pub starting: bool,
    pub cancelling: bool,
    pub error: Option<String>,
    pub poll: Option<iced::task::Handle>,
}
impl ModelRun {
    pub fn terminal(&self) -> bool {
        self.snapshot.as_ref().is_some_and(|s| {
            matches!(
                s.state,
                DiagnosticJobState::Succeeded {}
                    | DiagnosticJobState::Failed { .. }
                    | DiagnosticJobState::Cancelled {}
                    | DiagnosticJobState::Interrupted {}
            )
        })
    }
    fn matches_result(&self, result: &DiagnosticResult) -> bool {
        match result {
            DiagnosticResult::Reachability { provider_id, .. } => {
                self.kind == DiagnosticProbeKind::Reachability
                    && provider_id == &self.context.provider
            }
            DiagnosticResult::Model { provider_id, model_id, .. } => {
                self.kind == DiagnosticProbeKind::Model
                    && provider_id == &self.context.provider
                    && model_id.as_ref() == self.model
            }
            DiagnosticResult::Streaming { provider_id, model_id, .. } => {
                self.kind == DiagnosticProbeKind::Streaming
                    && provider_id == &self.context.provider
                    && model_id.as_ref() == self.model
            }
            _ => false,
        }
    }
    pub fn accept(&mut self, serial: u64, snapshot: DiagnosticJobSnapshot) -> bool {
        if self.terminal()
            || self.serial != serial
            || self.job.as_ref() != Some(&snapshot.job)
            || !self.context.matches(&snapshot.job, self.kind)
            || !snapshot.results.iter().flatten().all(|result| self.matches_result(result))
        {
            return false;
        }
        self.poll = None;
        // A fresh host snapshot supersedes local cancellation intent, including
        // after an unconfirmed request failed while the panel was closed.
        self.cancelling = matches!(snapshot.state, DiagnosticJobState::Cancelling {});
        self.snapshot = Some(snapshot);
        self.error = None;
        true
    }
}
#[derive(Clone, Debug, Default)]
pub(crate) struct ModelState {
    pub access_generation: u64,
    pub open: Option<ModelContext>,
    pub model: String,
    pub runs: Vec<ModelRun>,
}

impl ModelState {
    pub fn close(&mut self) {
        self.open = None;
        for run in &mut self.runs {
            run.serial = run.serial.wrapping_add(1);
            if let Some(handle) = run.poll.take() {
                handle.abort();
            }
        }
    }
    pub fn invalidate_access(&mut self) {
        self.close();
        self.access_generation = self.access_generation.wrapping_add(1);
    }
}
