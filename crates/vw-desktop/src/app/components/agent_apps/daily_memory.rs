//! Page-scoped daily memory drafts; content is never included in diagnostics.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_common::*;
use iced::{Element, Length, Task, widget::{button, column, container, row, text, text_editor}};
use vw_gateway_client::{WorkspaceMemoryScope, vw_api_types::workspace_memory::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    search: super::daily_memory_search::State,
    generation: u64,
    request: u64,
    files: Vec<DailyMemoryFileInfo>,
    pub(super) filename: Option<DailyMemoryFilename>,
    pub(super) editor: text_editor::Content,
    pub(super) loaded: bool,
    listing: bool,
    busy: bool,
    delete: Option<DailyMemoryFilename>,
    error: Option<String>,
}
impl State {
    pub fn invalidate(&mut self) { self.search.close(); *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() }; }
    pub(super) fn create_draft(&mut self, filename: DailyMemoryFilename) {
        self.request = self.request.wrapping_add(1);
        self.filename = Some(filename);
        self.editor = text_editor::Content::new();
        self.loaded = true;
    }
}
#[derive(Clone)]
pub struct Scope { context: AgentAppsContext, host: String, project: String, catalog: u64, generation: u64 }
#[derive(Clone)]
pub enum Event {
    Search(super::daily_memory_search::Event),
    Open, Close, Refresh, Today, Select(DailyMemoryFilename), Edit(text_editor::Action), Save,
    AskDelete(DailyMemoryFilename), CancelDelete, ConfirmDelete,
    Listed(Scope, Result<Vec<DailyMemoryFileInfo>, String>),
    Read(Scope, u64, Result<WorkspaceFileContent, String>),
    Written(Scope, Result<(), String>), Deleted(Scope, DailyMemoryFilename, Result<(), String>),
}
pub(super) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::DailyMemory(event)))
}
pub(crate) fn entry() -> Element<'static, Message> {
    button(text("每日记忆")).padding([8,12]).style(rounded_action_btn_style).on_press(message(Event::Open)).into()
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open { return base; }
    let mut body = column![row![
        button(text("创建今日草稿")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.busy && !state.listing && state.error.is_none()).then(|| message(Event::Today))),
        button(text("刷新列表")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.listing && !state.busy).then(|| message(Event::Refresh)))].spacing(8)].spacing(8);
    if let Some(error) = &state.error { body = body.push(text(error)); }
    body = body.push(super::daily_memory_search::controls(&state.search));
    if state.search.active() { body = body.push(super::daily_memory_search::results(&state.search, state.busy)); }
    for file in state.files.iter().filter(|_| !state.search.active()) {
        let modified = i64::try_from(file.modified_at).ok().and_then(|s| chrono::DateTime::from_timestamp(s, 0))
            .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "时间不可用".into());
        body = body.push(column![row![
            button(text(format!("{} · {} B · {}", file.date, file.size_bytes, modified))).padding([8,12]).style(rounded_action_btn_style)
                .on_press_maybe((!state.busy).then(|| message(Event::Select(file.filename.clone())))),
            button(text("删除")).padding([8,12]).style(rounded_action_btn_style)
                .on_press_maybe((!state.busy).then(|| message(Event::AskDelete(file.filename.clone()))))].spacing(8), text(&file.preview)]);
    }
    if let Some(filename) = &state.filename {
        body = body.push(text(filename.as_str()));
        let mut editor = settings_text_editor(&state.editor).height(240);
        if state.loaded && !state.busy { editor = editor.on_action(|a| message(Event::Edit(a))); }
        body = body.push(editor);
    }
    if let Some(filename) = &state.delete {
        body = body.push(text(format!("确认删除 {}？", filename.as_str()))).push(row![
            button(text("确认删除")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.busy).then(|| message(Event::ConfirmDelete))),
            button(text("取消删除")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.busy).then(|| message(Event::CancelDelete)))].spacing(8));
    }
    let close = message(Event::Close);
    let footer = row![button(text("保存")).padding([8,12]).style(primary_action_btn_style)
        .on_press_maybe((state.loaded && !state.busy && state.delete.is_none()).then(|| message(Event::Save)))];
    let card = settings_modal_layout(settings_modal_header(text("OpenClaw 每日记忆").size(20), settings_close_button(close.clone())), body, Some(footer.into())).width(760).height(640);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) { if !app.apps_catalog.daily_memory.busy { app.apps_catalog.daily_memory.invalidate(); } return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("网关不可用，草稿已保留"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|id| !id.is_empty()) else { return Task::none(); };
    let state = &mut app.apps_catalog.daily_memory;
    let reply = match &event { Event::Search(super::daily_memory_search::Event::Finished(s, ..)) | Event::Listed(s, _) | Event::Read(s, ..) | Event::Written(s, _) | Event::Deleted(s, ..) => Some(s), _ => None };
    if let Some(s) = reply {
        if !state.open || s.context != context || s.host != app.apps_catalog.host || s.project != project || s.catalog != app.apps_catalog.generation || s.generation != state.generation { return Task::none(); }
    } else if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用服务"); }
    match event.clone() {
        Event::Listed(_, result) => { state.listing = false; match result { Ok(files) => {state.files = files; state.error = None;}, Err(e) => state.error = Some(e) } return Task::none(); }
        Event::Read(_, request, result) => {
            if request != state.request { return Task::none(); }
            match result { Ok(WorkspaceFileContent(Some(content))) => {state.editor = text_editor::Content::with_text(&content); state.loaded = true;}, _ => state.error = Some("读取失败，请重新打开；草稿未替换".into()) }
            return Task::none();
        }
        Event::Written(_, result) => { state.busy = false; if let Err(e) = result { state.error = Some(e); return Task::none(); } state.error = None; }
        Event::Deleted(_, filename, result) => { state.busy = false; if let Err(e) = result {state.error = Some(e); return Task::none();} state.delete = None; if state.filename.as_ref() == Some(&filename) {state.filename = None; state.loaded = false;} state.error = None; }
        Event::Edit(action) => { if state.loaded && !state.busy { state.editor.perform(action); } return Task::none(); }
        Event::AskDelete(filename) => { if !state.busy {state.request = state.request.wrapping_add(1); state.delete = Some(filename);} return Task::none(); }
        Event::CancelDelete => { if !state.busy {state.delete = None;} return Task::none(); }
        Event::Open => { if state.open {return Task::none();} state.open = true; }
        _ => {}
    }
    let scope = Scope { context, host: app.apps_catalog.host.clone(), project: project.to_string(), catalog: app.apps_catalog.generation, generation: state.generation };
    let query = WorkspaceMemoryScope { project_id: project.into(), target_host_id: scope.host.clone() };
    let token = app.apps_catalog.token.clone();
    if let Event::Search(event) = event.clone() {
        if !state.open { return Task::none(); }
        return super::daily_memory_search::update(&mut state.search, event, client, scope, query, token);
    }
    if state.busy {return Task::none();}
    let selected = match event.clone() {
        Event::Today => {
            if state.listing || state.error.is_some() {return Task::none();}
            let Ok(filename) = chrono::Local::now().format("%Y-%m-%d.md").to_string().parse::<DailyMemoryFilename>() else {return Task::none();};
            if !state.files.iter().any(|f| f.filename == filename) {state.create_draft(filename); return Task::none();} Some(filename)
        }
        Event::Select(filename) => Some(filename), _ => None,
    };
    if let Some(filename) = selected {
        state.request = state.request.wrapping_add(1); let request = state.request;
        state.filename = Some(filename.clone()); state.loaded = false; state.error = None;
        return Task::perform(async move { message(Event::Read(scope, request, client.read_daily_memory_file(&query, &filename, &token).await.map_err(|e| e.to_string()))) }, |m| m);
    }
    if matches!(event, Event::Save) {
        if !state.loaded {return Task::none();} let Some(filename) = state.filename.clone() else {return Task::none();};
        let content = state.editor.text(); state.busy = true;
        return Task::perform(async move {message(Event::Written(scope, client.write_daily_memory_file(&query, &filename, content, &token).await.map_err(|e| e.to_string())))}, |m| m);
    }
    if matches!(event, Event::ConfirmDelete) {
        let Some(filename) = state.delete.clone() else {return Task::none();}; state.busy = true; state.request = state.request.wrapping_add(1);
        return Task::perform(async move {let result = client.delete_daily_memory_file(&query, &filename, &token).await.map_err(|e| e.to_string()); message(Event::Deleted(scope, filename, result))}, |m| m);
    }
    if state.listing {return Task::none();} state.listing = true;
    Task::perform(async move {message(Event::Listed(scope, client.list_daily_memory_files(&query, &token).await.map_err(|e| e.to_string())))}, |m| m)
}

/// Search shortcuts are scoped to the daily-memory list, outside its editor.
pub(crate) fn subscription(state: &State) -> iced::Subscription<Message> {
    if !state.open || state.filename.is_some() { return iced::Subscription::none(); }
    iced::keyboard::on_key_press(|key, modifiers| {
        use iced::keyboard::{Key, key::Named};
        let event = match key {
            Key::Character(c) if modifiers.command() && c.eq_ignore_ascii_case("f") => super::daily_memory_search::Event::Open,
            Key::Named(Named::Escape) => super::daily_memory_search::Event::Close,
            _ => return None,
        };
        Some(message(Event::Search(event)))
    })
}
