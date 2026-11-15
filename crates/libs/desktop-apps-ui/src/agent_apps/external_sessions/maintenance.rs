use vw_api_external_session::*;
use super::Scope;
use gateway_external_session::AgentAppsReadError;
use vw_api_task::TaskStatus;
#[derive(Clone, Default)]
pub struct State {
    pub target: Option<HistoryMaintenanceTarget>,
    pub options: Option<HistoryOptions>,
    pub saved_options: Option<HistoryOptions>,
    pub backups: Vec<HistoryBackupSummary>,
    pub progress: Option<HistoryMaintenanceProgress>,
    pub confirmation: Option<String>,
    pub pending: bool,
    pub uncertain: bool,
}
impl State {
    pub fn is_open(&self) -> bool { self.target.is_some() }
    pub fn busy(&self) -> bool {
        self.pending || self.uncertain || self.progress.as_ref().is_some_and(|p|
            matches!(p.status, TaskStatus::Queued | TaskStatus::Running))
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(HistoryMaintenanceTarget), Close, Reload,
    Unify(bool), Migrate(bool), Save, Start(HistoryMaintenanceAction),
    AskRestore(String), ConfirmRestore, DismissRestore, Query, Cancel,
    Loaded(Scope, HistoryMaintenanceTarget, Result<(HistoryOptions, HistoryBackups), AgentAppsReadError>),
    Saved(Scope, HistoryMaintenanceTarget, Result<HistoryOptions, AgentAppsReadError>),
    Progress(Scope, HistoryMaintenanceTarget, Result<HistoryMaintenanceProgress, AgentAppsReadError>),
}
