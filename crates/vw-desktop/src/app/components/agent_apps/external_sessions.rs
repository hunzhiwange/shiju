//! External history catalog; credentials stay in the application connection.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::{Element, Task};
#[path = "external_sessions/list.rs"]
mod list;
#[path = "external_sessions/reader.rs"]
mod reader;
#[path = "external_sessions/details.rs"]
mod details;
#[path = "external_sessions/images.rs"]
mod images;
#[path = "external_sessions/find.rs"]
mod find;
#[path = "external_sessions/delete.rs"]
mod delete;
#[path = "external_sessions/resume.rs"]
mod resume;
#[path = "external_sessions/paths.rs"]
mod paths;
#[path = "external_sessions/export.rs"]
mod export;
use vw_gateway_client::{AgentAppsReadError, vw_api_types::external_session::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    sources_open: bool,
    deletion: delete::State,
    resume: resume::State,
    reader: Option<reader::State>,
    list_offset: iced::widget::scrollable::AbsoluteOffset,
    error: bool,
    by_time: bool,
    host: String,
    busy: bool,
    page: Option<SessionCatalogPage>,
    generation: u64,
}
impl State {
    pub(crate) fn cancel_reader(&mut self) {
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
    context: AgentAppsContext,
    host: String,
    project: String,
    catalog: u64,
    generation: u64,
    append: bool,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
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
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::ExternalSessions(event)))
}
pub(crate) fn view(state: &State) -> Element<'_, Message> {
    let base = if let Some(reader) = &state.reader { reader::view(state, reader) } else { list::view(state) };
    paths::view(state, resume::view(state, delete::view(state, base)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Export(event) = event { return export::update(app, event); }
    if let Event::Path(event) = event { return paths::update(app, event); }
    if let Event::Resume(event) = event { return resume::update(app, event); }
    if let Event::Delete(event) = event { return delete::update(app, event); }
    if let Event::Details(event) = event { return details::update(app, event); }
    if let Event::Image(event) = event { return images::update(app, event); }
    if reader::handles(&event) { return reader::update(app, event); }
    match event {
        Event::Open => {
            app.apps_catalog.external_sessions.open = true;
            let navigation = crate::app::message::view::update(app,
                crate::app::message::view::ViewMessage::OpenSystemSettingsTab(
                    crate::app::components::system_settings::SystemTab::Apps));
            return navigation;
        }
        Event::Close => {
            let state = &mut app.apps_catalog.external_sessions;
            state.cancel_reader();
            state.deletion = delete::State::default();
            state.open = false;
            state.sources_open = false;
            state.busy = false;
            state.generation = state.generation.wrapping_add(1);
            return Task::none();
        }
        Event::GroupByTime(value) => { app.apps_catalog.external_sessions.by_time = value; return Task::none(); }
        _ => {}
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用，请重新连接");
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if let Event::Loaded(scope, result) = event {
        if scope.context != context || app.apps_catalog.context.as_ref() != Some(&context)
            || scope.host != app.apps_catalog.host || app.project_id.as_ref() != Some(&scope.project)
            || scope.catalog != app.apps_catalog.generation
            || scope.generation != app.apps_catalog.external_sessions.generation
            || !app.apps_catalog.external_sessions.open
            || !app.apps_catalog.external_sessions.busy { return Task::none(); }
        app.apps_catalog.external_sessions.busy = false;
        match result {
            Ok(mut page) => {
                let state = &mut app.apps_catalog.external_sessions;
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
                app.apps_catalog.external_sessions.error = true;
                return app.show_error_toast("外部会话读取失败，请检查主机授权并重试");
            }
        }
        return Task::none();
    }
    if app.apps_catalog.external_sessions.deletion.busy() { return Task::none(); }
    if matches!(event, Event::Refresh) { app.apps_catalog.external_sessions.resume.reset(); }
    if matches!(event, Event::Refresh) { app.apps_catalog.external_sessions.deletion = delete::State::default(); }
    if app.apps_catalog.external_sessions.busy && matches!(event, Event::Next) { return Task::none(); }
    let Some(project) = app.project_id.clone().filter(|id| !id.trim().is_empty()) else {
        return app.show_error_toast("请先选择项目");
    };
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.host.is_empty()
        || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用服务"); }
    let cursor = if matches!(event, Event::Next) {
        let Some(cursor) = app.apps_catalog.external_sessions.page.as_ref().and_then(|p| p.next_cursor.clone()) else { return Task::none(); };
        Some(cursor)
    } else { None };
    let state = &mut app.apps_catalog.external_sessions;
    state.busy = true;
    state.error = false;
    state.host = app.apps_catalog.host.clone();
    let append = cursor.is_some();
    if !append { state.page = None; }
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope { context, host: app.apps_catalog.host.clone(), project: project.clone(),
        catalog: app.apps_catalog.generation, generation: state.generation, append };
    let query = SessionCatalogQuery { project_id: project.into(), target_host_id: scope.host.clone(),
        provider: None, instance_id: None, project_dir: None, search: String::new(), page_size: 20, cursor };
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        message(Event::Loaded(scope, client.external_sessions_catalog(&query, &token).await))
    }, |message| message)
}
