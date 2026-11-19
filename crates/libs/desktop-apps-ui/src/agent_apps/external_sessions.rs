use crate::{AgentAppsContext, Connection, Notice, error as notify_error};
use gateway_external_session::AgentAppsReadError;
use iced::Task;
use desktop_ui_controls::{
    selectable_text::markdown_selection::SelectionMessage,
    text_editor_context_menu::TextContextEffect,
};
use vw_api_external_session::*;
pub mod compression;
pub mod delete;
pub mod details;
pub mod export;
pub mod find;
pub mod images;
pub mod list;
pub mod maintenance;
pub mod paths;
pub mod reader;
pub mod resume;
#[derive(Clone, Default)]
pub struct State {
    pub open: bool,
    pub sources_open: bool,
    pub apps_open: bool,
    pub deletion: delete::State,
    pub maintenance: maintenance::State,
    pub compression: compression::State,
    pub resume: resume::State,
    pub reader: Option<reader::State>,
    pub list_offset: iced::widget::scrollable::AbsoluteOffset,
    pub error: Option<String>,
    pub by_time: bool,
    pub host: String,
    pub busy: bool,
    pub page: Option<SessionCatalogPage>,
    pub generation: u64,
    pub provider: Option<SessionProvider>,
    provider_selected: bool,
    pub search_draft: String,
    pub search: String,
    pub collapsed: std::collections::HashSet<Option<String>>,
    pub app_counts: Option<(Vec<SessionCatalogGroup>, bool)>,
}
impl State {
    pub fn cancel_reader(&mut self) {
        self.resume.reset();
        if let Some(reader) = self.reader.take() {
            reader.cancel();
        }
    }
    pub fn invalidate(&mut self) {
        self.cancel_reader();
        *self = Self {
            open: self.open,
            by_time: self.by_time,
            generation: self.generation.wrapping_add(1),
            ..Self::default()
        };
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
    TextContext(Option<TextContextEffect>),
    Open,
    Applications(bool),
    SearchDraft(String),
    Search,
    ClearSearch,
    Provider(Option<SessionProvider>),
    ToggleProject(Option<String>),
    CountsLoaded(Scope, Result<SessionCatalogPage, AgentAppsReadError>),
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
    ReaderOutline,
    ReaderOptions,
    ReaderSearch,
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
fn request_error(action: &str, error: &AgentAppsReadError) -> String {
    match error {
        AgentAppsReadError::Transport(_) => format!("{action}失败，无法连接网关"),
        AgentAppsReadError::Http { status: 401, .. } => format!("{action}失败，网关身份验证失败"),
        AgentAppsReadError::Http { status: 403, .. } => format!("{action}失败，网关拒绝了请求"),
        AgentAppsReadError::Http { status: 503, .. } => format!("{action}失败，会话服务暂不可用"),
        AgentAppsReadError::Http { status, .. } => format!("{action}失败（HTTP {status}）"),
        AgentAppsReadError::Decode(_) => format!("{action}失败，网关响应格式无效"),
    }
}

impl SelectionMessage for Event {
    fn none() -> Self {
        Self::TextContext(None)
    }

    fn copy(text: String) -> Self {
        Self::context(TextContextEffect::Copy(text))
    }

    fn context(effect: TextContextEffect) -> Self {
        Self::TextContext(Some(effect))
    }

    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::none()
    }
}

fn message(event: Event) -> Event {
    event
}
pub fn update(
    state: &mut State,
    connection: Connection,
    event: Event,
    notice: &mut Option<Notice>,
) -> Task<Event> {
    let Connection { client, context, catalog_context, project, host, generation, token } =
        connection;
    if let Event::CountsLoaded(scope, result) = event {
        if scope.context == context && catalog_context.as_ref() == Some(&context)
            && scope.host == host && scope.project == project && scope.catalog == generation
            && scope.generation == state.generation && state.open {
            state.app_counts = result.ok().map(|page| (page.groups, page.truncated));
        }
        return Task::none();
    }
    if let Event::Loaded(scope, result) = event {
        if scope.context != context
            || catalog_context.as_ref() != Some(&context)
            || scope.host != host
            || project != scope.project
            || scope.catalog != generation
            || scope.generation != state.generation
            || !state.open
            || !state.busy
        {
            return Task::none();
        }
        state.busy = false;
        match result {
            Ok(mut page) => {
                state.error = None;
                if !scope.append && state.provider.is_none() && state.search.is_empty() {
                    state.app_counts = Some((page.groups.clone(), page.truncated));
                }
                if !scope.append && !state.provider_selected && state.search.is_empty() {
                    let provider = page.groups.iter()
                        .find(|group| group.count > 0 && group.provider == SessionProvider::Codex)
                        .map(|group| group.provider)
                        .or_else(|| page.sessions.first().map(|session| session.identity.provider));
                    if let Some(provider) = provider {
                        return update(state, Connection {
                            client, context, catalog_context, project, host, generation, token,
                        }, Event::Provider(Some(provider)), notice);
                    }
                }
                if !state.by_time && state.search.is_empty() {
                    let first_project = page.sessions.first().map(|session| &session.project_dir);
                    for session in &page.sessions {
                        let already_listed = state.page.as_ref().is_some_and(|previous|
                            previous.sessions.iter().any(|old| old.project_dir == session.project_dir));
                        if !already_listed && (scope.append || Some(&session.project_dir) != first_project) {
                            state.collapsed.insert(session.project_dir.clone());
                        }
                    }
                }
                if scope.append {
                    if let Some(previous) = &mut state.page {
                        for session in page.sessions.drain(..) {
                            if !previous.sessions.iter().any(|old| old.identity == session.identity)
                            {
                                previous.sessions.push(session);
                            }
                        }
                        previous.total = page.total;
                        previous.next_cursor = page.next_cursor;
                        previous.groups = page.groups;
                        previous.truncated |= page.truncated;
                    }
                } else {
                    state.page = Some(page);
                }
            }
            Err(error) => {
                let message = match error {
                    AgentAppsReadError::Http { status: 410, .. } => {
                        "会话列表已过期，请刷新重试".to_owned()
                    }
                    _ => request_error("外部会话读取", &error),
                };
                state.error = Some(message.clone());
                return notify_error(notice, message);
            }
        }
        return Task::none();
    }
    if state.deletion.deleting || !state.deletion.pending.is_empty() {
        return Task::none();
    }
    match &event {
        Event::SearchDraft(value) => { state.search_draft = value.clone(); return Task::none(); }
        Event::Search => state.search = state.search_draft.trim().to_owned(),
        Event::ClearSearch => { state.search_draft.clear(); state.search.clear(); }
        Event::Provider(provider) => {
            state.provider_selected = true;
            if state.provider == *provider && state.page.is_some() && state.error.is_none() {
                return Task::none();
            }
            state.provider = *provider;
        }
        _ => {}
    }
    if matches!(event, Event::Refresh) {
        state.resume.reset();
    }
    if matches!(event, Event::Refresh) {
        state.deletion = delete::State::default();
    }
    if (state.busy || state.deletion.loading) && matches!(event, Event::Next) {
        return Task::none();
    }
    if project.trim().is_empty() {
        return notify_error(notice, "请先选择项目");
    }
    if catalog_context.as_ref() != Some(&context) || host.is_empty() {
        return notify_error(notice, "请先连接应用服务");
    }
    let cursor = if matches!(event, Event::Next) {
        let Some(cursor) = state.page.as_ref().and_then(|p| p.next_cursor.clone()) else {
            return Task::none();
        };
        Some(cursor)
    } else {
        None
    };

    state.busy = true;
    state.error = None;
    state.host = host.clone();
    let append = cursor.is_some();
    if !append {
        state.cancel_reader();
        state.page = None;
        if matches!(event, Event::Refresh) {
            state.app_counts = None;
        }
        state.collapsed.clear();
        state.deletion = delete::State::default();
        state.list_offset = Default::default();
    }
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope {
        context,
        host: host.clone(),
        project: project.clone(),
        catalog: generation,
        generation: state.generation,
        append,
    };
    let query = SessionCatalogQuery {
        project_id: project.into(),
        target_host_id: scope.host.clone(),
        provider: state.provider,
        instance_id: None,
        project_dir: None,
        search: state.search.clone(),
        page_size: 20,
        cursor,
    };

    let counts = if state.app_counts.is_none() && (query.provider.is_some() || !query.search.is_empty()) {
        let mut counts_query = query.clone();
        counts_query.cursor = None;
        counts_query.provider = None;
        counts_query.search.clear();
        counts_query.page_size = 1;
        let counts_scope = scope.clone();
        let counts_client = client.clone();
        let counts_token = token.clone();
        Task::perform(async move {
            Event::CountsLoaded(counts_scope,
                gateway_external_session::GatewayExternalSessionClient::new(counts_client.transport().clone())
                    .external_sessions_catalog(&counts_query, &counts_token).await)
        }, |event| event)
    } else { Task::none() };
    let load = Task::perform(
        async move {
            message(Event::Loaded(
                scope,
                gateway_external_session::GatewayExternalSessionClient::new(
                    client.transport().clone(),
                )
                .external_sessions_catalog(&query, &token)
                .await,
            ))
        },
        |message| message,
    );
    Task::batch([counts, load, if append { Task::none() } else {
        iced::widget::operation::scroll_to("external-session-list", state.list_offset)
    }])
}

pub fn close(state: &mut State) {
    state.cancel_reader();
    state.deletion = delete::State::default();
    state.open = false;
    state.sources_open = false;
    state.apps_open = false;
    state.busy = false;
    state.generation = state.generation.wrapping_add(1);
}

/// Compose the catalog and its scoped overlays using local messages.
pub fn view(state: &State, loopback: bool, i18n: desktop_i18n::I18n) -> iced::Element<'_, Event> {
    let base = if let Some(reader) = &state.reader {
        reader::view(state, reader, loopback, i18n)
    } else {
        list::view(state, loopback)
    };
    compression::view(
        state,
        maintenance::view(
            state,
            paths::view(state, resume::view(state, delete::view(state, base))),
        ),
    )
}

pub fn handles_reader(event: &Event) -> bool {
    reader::handles(event)
}

/// `verified_local` must come from the private local grant for this connection, host and token.
/// It permits revealing paths only after the event scope and catalog identity are checked.
pub fn update_local(
    state: &mut State,
    connection: &Connection,
    event: Event,
    notice: &mut Option<Notice>,
    verified_local: bool,
) -> Task<Event> {
    match event {
        Event::Maintenance(event) => maintenance::update(state, connection, notice, event),
        Event::Export(event) => export::update(state, connection, notice, event),
        Event::Path(event) => paths::update(state, connection, notice, event, verified_local),
        Event::Resume(event) => resume::update(state, connection, notice, event),
        Event::Delete(event) => delete::update(state, connection, notice, event),
        Event::Details(event) => details::update(state, connection, notice, event),
        Event::Image(event) => images::update(state, connection, notice, event),
        event if reader::handles(&event) => reader::update(state, connection, notice, event),
        _ => Task::none(),
    }
}

pub fn navigate(state: &mut State, event: &Event) -> Option<Task<Event>> {
    reader::navigate(state, event)
}

/// Host navigation is the only cross-page effect of preparing a history event.
pub enum HostDispatch {
    Blocked,
    OpenSettings,
    Handled,
    Continue { local: bool },
}
pub fn prepare_host_event(state: &mut State, event: &Event) -> HostDispatch {
    if state.compression.is_open() {
        return HostDispatch::Blocked;
    }
    if state.maintenance.is_open() && !matches!(event, Event::Maintenance(_)) {
        return HostDispatch::Blocked;
    }
    if (state.deletion.deleting || !state.deletion.pending.is_empty())
        && !matches!(event, Event::Delete(_) | Event::Loaded(..) | Event::CountsLoaded(..)) {
        return HostDispatch::Blocked;
    }
    match event {
        Event::Applications(open) => { state.apps_open = *open; return HostDispatch::Handled; }
        Event::Provider(_) => { state.apps_open = false; }
        Event::ToggleProject(dir) => {
            if !state.collapsed.remove(dir) { state.collapsed.insert(dir.clone()); }
            return HostDispatch::Handled;
        }
        Event::Open => {
            state.open = true;
            return HostDispatch::OpenSettings;
        }
        Event::Close => {
            close(state);
            return HostDispatch::Handled;
        }
        Event::GroupByTime(value) => {
            state.by_time = *value;
            return HostDispatch::Handled;
        }
        _ => {}
    }
    let local = matches!(
        event,
        Event::Maintenance(_)
            | Event::Export(_)
            | Event::Path(_)
            | Event::Resume(_)
            | Event::Delete(_)
            | Event::Details(_)
            | Event::Image(_)
    ) || handles_reader(event);
    HostDispatch::Continue { local }
}
