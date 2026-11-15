use vw_api_external_session::*;
use super::Scope;
use gateway_external_session::AgentAppsReadError;
#[derive(Clone, Default)]
pub struct State {
    pub identity: Option<ExternalSessionIdentity>,
    pub terminal: Option<SessionTerminal>,
    pub busy: bool,
    pub generation: u64,
}
impl State {
    pub fn reset(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(ExternalSessionIdentity),
    Close,
    Terminal(SessionTerminal),
    Launch,
    Copy,
    Command(Scope, ExternalSessionIdentity, u64, Result<SessionResumeCommandResponse, AgentAppsReadError>),
    Finished(Scope, ExternalSessionIdentity, u64, Result<SessionResumeResponse, AgentAppsReadError>),
}
