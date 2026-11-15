use vw_api_external_session::*;
use crate::AgentAppsContext;
use super::{details, images, find};
use iced::widget::scrollable::AbsoluteOffset;
#[derive(Clone)]
pub struct State {
    pub identity: ExternalSessionIdentity,
    pub title: String,
    pub turns: Vec<TurnIndex>,
    pub messages: Vec<SessionMessage>,
    pub active: usize,
    pub loading: bool,
    pub failed: bool,
    pub total: usize,
    pub offset: AbsoluteOffset,
    pub abort: Option<iced::task::Handle>,
    pub generation: u64,
    pub details: details::State,
    pub images: images::State,
    pub find: find::State,
    pub exporting: bool,
}
impl State {
    pub fn cancel(&self) {
        self.details.cancel();
        self.images.cancel();
        if let Some(handle) = &self.abort {
            handle.abort();
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub context: AgentAppsContext,
    pub catalog: u64,
    pub generation: u64,
    pub identity: ExternalSessionIdentity,
}
