use vw_api_external_session::*;
use super::reader;
use std::collections::BTreeSet;
use gateway_external_session::AgentAppsReadError;
#[derive(Clone, Default)]
pub struct State {
    pub expanded: BTreeSet<(usize, usize)>,
    pub process_open: bool,
    pub pages: Vec<Page>,
}
#[derive(Clone)]
pub struct Page {
    pub reference: ContentRef,
    pub text: String,
    pub next: Option<u32>,
    pub started: bool,
    pub pending: bool,
    pub serial: u64,
    pub abort: Option<iced::task::Handle>,
}
impl State {
    pub fn cancel(&self) {
        for page in &self.pages {
            if let Some(abort) = &page.abort {
                abort.abort();
            }
        }
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Process,
    Toggle(usize, usize),
    More(ContentRef),
    Loaded(reader::Scope, u64, ContentRef, u32, Result<SessionContentResponse, AgentAppsReadError>),
}
