use crate::{AgentAppsContext, Connection, Notice, error as notify_error};
use iced::Task;
use gateway_external_session::AgentAppsReadError;
use vw_api_external_session::*;
pub mod reader;
pub mod delete;
pub mod resume;
pub mod maintenance;
pub mod compression;
pub mod details;
pub mod images;
pub mod find;
pub mod paths;
pub mod export;
#[derive(Clone, Default)]
pub struct State {
    pub open: bool,
    pub sources_open: bool,
    pub deletion: delete::State,
    pub maintenance: maintenance::State,
    pub compression: compression::State,
    pub resume: resume::State,
    pub reader: Option<reader::State>,
    pub list_offset: iced::widget::scrollable::AbsoluteOffset,
    pub error: bool,
    pub by_time: bool,
    pub host: String,
    pub busy: bool,
    pub page: Option<SessionCatalogPage>,
    pub generation: u64,
}
impl State {
    pub fn cancel_reader(&mut self) {
        self.resume.reset();
        if let Some(reader) = self.reader.take() { reader.cancel(); }
    }
    pub fn invalidate(&mut self) {
        self.cancel_reader();
        *self = Self { open: self.open, by_time: self.by_time, generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone, Debug)]
pub struct Scope {
    pub context: AgentAppsContext,
    pub host: String,
    pub project: String,
    pub catalog: u64,
    pub generation: u64,
    pub append: bool,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Maintenance(maintenance::Event),
    Compression(compression::Event),
    Delete(delete::Event),
    Resume(resume::Event),
    Path(paths::Event),
    Export(export::Event),
    Details(details::Event),
    Image(images::Event),
    Find(find::Event),
    Read(ExternalSessionIdentity),
    Back,
    Adjacent(bool),
    Turn(usize),
    ListScrolled(iced::widget::scrollable::AbsoluteOffset),
    ReaderScrolled(iced::widget::scrollable::AbsoluteOffset),
    Chunk(reader::Scope, Result<TranscriptChunk, AgentAppsReadError>),
    Close,
    GroupByTime(bool),
    Refresh,
    Next,
    Loaded(Scope, Result<SessionCatalogPage, AgentAppsReadError>),
}
fn message(event: Event) -> Event { event }
pub fn update(state: &mut State, connection: Connection, event: Event, notice: &mut Option<Notice>) -> Task<Event> {
    let Connection { client, context, catalog_context, project, host, generation, token } = connection;
    if let Event::Loaded(scope, result) = event {
        if scope.context != context || catalog_context.as_ref() != Some(&context)
            || scope.host != host || project != scope.project
            || scope.catalog != generation
            || scope.generation != state.generation
            || !state.open
            || !state.busy { return Task::none(); }
        state.busy = false;
        match result {
            Ok(mut page) => {

                state.error = false;
                if scope.append {
                    if let Some(previous) = &mut state.page {
                        for session in page.sessions.drain(..) {
                            if !previous.sessions.iter().any(|old| old.identity == session.identity) {
                                previous.sessions.push(session);
                            }
                        }
                        previous.total = page.total;
                        previous.next_cursor = page.next_cursor;
                        previous.groups = page.groups;
                        previous.truncated |= page.truncated;
                    }
                } else { state.page = Some(page); }
            }
            Err(_) => {
                state.error = true;
                return notify_error(notice, "外部会话读取失败，请检查主机授权并重试");
            }
        }
        return Task::none();
    }
    if state.deletion.busy() { return Task::none(); }
    if matches!(event, Event::Refresh) { state.resume.reset(); }
    if matches!(event, Event::Refresh) { state.deletion = delete::State::default(); }
    if state.busy && matches!(event, Event::Next) { return Task::none(); }
    if project.trim().is_empty() { return notify_error(notice, "请先选择项目"); }
    if catalog_context.as_ref() != Some(&context) || host.is_empty()
        || token.is_empty() { return notify_error(notice, "请先连接应用服务"); }
    let cursor = if matches!(event, Event::Next) {
        let Some(cursor) = state.page.as_ref().and_then(|p| p.next_cursor.clone()) else { return Task::none(); };
        Some(cursor)
    } else { None };

    state.busy = true;
    state.error = false;
    state.host = host.clone();
    let append = cursor.is_some();
    if !append { state.page = None; }
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope { context, host: host.clone(), project: project.clone(),
        catalog: generation, generation: state.generation, append };
    let query = SessionCatalogQuery { project_id: project.into(), target_host_id: scope.host.clone(),
        provider: None, instance_id: None, project_dir: None, search: String::new(), page_size: 20, cursor };

    Task::perform(async move {
        message(Event::Loaded(scope, gateway_external_session::GatewayExternalSessionClient::new(client.transport().clone()).external_sessions_catalog(&query, &token).await))
    }, |message| message)
}

pub fn close(state: &mut State) {
    state.cancel_reader();
    state.deletion = delete::State::default();
    state.open = false;
    state.sources_open = false;
    state.busy = false;
    state.generation = state.generation.wrapping_add(1);
}
