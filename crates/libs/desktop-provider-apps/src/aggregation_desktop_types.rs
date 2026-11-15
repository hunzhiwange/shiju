use crate::Scope;
use vw_api_transport::ProjectId;
use vw_api_app_aggregation::*;
use vw_api_app_aggregation::operations::ProviderOperationProgress;
use gateway_aggregation_desktop::AgentAppsSettingsError;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub scope: Scope,
    pub project: ProjectId,
    pub access: u64,
}
#[derive(Clone, Debug)]
pub struct Session {
    pub binding: Binding,
    pub status: Option<DesktopProfileStatus>,
    pub revision: String,
    pub dirty: bool,
    pub preview: Option<AggregationPreview>,
    pub operation: Option<ProviderOperationProgress>,
    pub confirmation: Option<bool>,
}
impl Session {
    pub fn new(binding: Binding) -> Self {
        Self {
            binding,
            status: None,
            revision: String::new(),
            dirty: false,
            preview: None,
            operation: None,
            confirmation: None,
        }
    }
    pub fn invalidate(&mut self) {
        self.preview = None;
        self.confirmation = None;
    }
    pub fn running(&self) -> bool {
        self.operation.as_ref().is_some_and(|o| {
            matches!(
                o.status,
                vw_api_task::TaskStatus::Queued
                    | vw_api_task::TaskStatus::Running
            )
        })
    }
    pub fn can_publish(&self) -> bool {
        !self.dirty
            && self.preview.as_ref().is_some_and(|p| {
                !p.revision.is_empty()
                    && p.preview_token.as_ref().is_some_and(|t| !t.is_empty())
                    && !p.issues.iter().any(|i| i.blocking)
            })
    }
}
#[derive(Clone, Debug)]
pub enum Action {
    Read,
    Save,
    Preview,
    Publish,
    Restore,
    Poll,
    Cancel,
}
#[derive(Clone, Debug)]
pub enum Reply {
    Read(DesktopProfileStatus, DesktopMappingView),
    Saved(DesktopMappingView),
    Preview(AggregationPreview),
    Operation(ProviderOperationProgress),
}
#[derive(Clone)]
pub enum Event {
    Open(String),
    Refresh,
    Save,
    Preview,
    Review(bool),
    Dismiss,
    Confirm,
    Poll,
    Cancel,
    Finished { binding: Binding, generation: u64, result: Result<Reply, AgentAppsSettingsError> },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DesktopScopedEvent([redacted])")
    }
}
