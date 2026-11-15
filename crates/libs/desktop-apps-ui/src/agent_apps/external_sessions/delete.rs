use vw_api_external_session::*;
use super::Scope;
use gateway_external_session::AgentAppsReadError;
#[derive(Clone, Default)]
pub struct State {
    pub selecting: bool,
    pub selected: Vec<SessionMeta>,
    pub failures: Vec<(ExternalSessionIdentity, String)>,
    pub pending: Vec<SessionMeta>,
    pub loading: bool,
    pub deleting: bool,
}
impl State {
    pub fn busy(&self) -> bool { self.loading || self.deleting }
}
#[derive(Clone, Debug)]
pub enum Event {
    Enter,
    Exit,
    Clear,
    Toggle(ExternalSessionIdentity),
    SelectAll,
    Selected(Scope, Result<Vec<SessionMeta>, AgentAppsReadError>),
    Ask(Option<ExternalSessionIdentity>),
    Cancel,
    Confirm,
    Finished(Scope, Vec<ExternalSessionIdentity>, Result<Vec<DeleteSessionOutcome>, AgentAppsReadError>),
}
