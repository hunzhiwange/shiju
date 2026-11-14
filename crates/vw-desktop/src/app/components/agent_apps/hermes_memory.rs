//! Independent, page-scoped Hermes drafts. Payloads never enter debug output.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_common::*;
use iced::{Element, Task, widget::{button, column, row, text, text_editor}};
use vw_gateway_client::{HermesMemoryScope, vw_api_types::hermes_memory::*};

#[derive(Clone, Default)]
pub(crate) struct Draft {
    pub editor: text_editor::Content,
    pub saved: Option<String>,
    pub edited: bool,
    pub loading: bool,
    pub saving: Option<String>,
    pub path: Option<String>,
}
impl Draft {
    pub fn dirty(&self) -> bool { self.saved.as_ref().is_some_and(|saved| *saved != self.editor.text()) }
    pub fn hydrate(&mut self, response: HermesMemoryResponse) {
        if self.saved.is_none() {
            if !self.edited { self.editor = text_editor::Content::with_text(&response.content); }
            self.saved = Some(response.content);
        }
        self.path = response.path;
        self.loading = false;
    }
    pub fn saved(&mut self) {
        if let Some(snapshot) = self.saving.take() { self.saved = Some(snapshot); }
    }
}
#[derive(Clone, Default)]
pub(crate) struct State {
    pub drafts: [Draft; 2],
    pub active: usize,
    pub generation: u64,
    pub open: bool,
}
impl State {
    pub fn invalidate(&mut self) { *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() }; }
}
#[derive(Clone)]
pub struct Scope {
    context: AgentAppsContext, host: String, project: String, catalog: u64, generation: u64,
}
#[derive(Clone)]
pub enum Event {
    Open, Select(usize), Edit(text_editor::Action), Save, Retry,
    Loaded(Scope, usize, Result<HermesMemoryResponse, String>),
    Saved(Scope, usize, Result<HermesWriteOutcome, String>),
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::HermesMemory(event)))
}
fn kind(index: usize) -> HermesMemoryKind { if index == 0 { HermesMemoryKind::Memory } else { HermesMemoryKind::User } }
pub(crate) fn view(state: &State) -> Element<'_, Message> {
    if !state.open { return button(text("打开 Hermes 记忆")).padding([8,12]).style(rounded_action_btn_style).on_press(message(Event::Open)).into(); }
    let draft = &state.drafts[state.active];
    let mut tabs = row![].spacing(8);
    for (index, label) in ["代理记忆 MEMORY.md", "用户记忆 USER.md"].into_iter().enumerate() {
        let label = format!("{}{}", label, if state.drafts[index].dirty() { " ·" } else { "" });
        tabs = tabs.push(button(text(label)).padding([8,12]).style(rounded_action_btn_style).on_press(message(Event::Select(index))));
    }
    let mut editor = settings_text_editor(&draft.editor).height(220);
    if draft.saved.is_some() { editor = editor.on_action(|action| message(Event::Edit(action))); }
    column![tabs, text(draft.path.as_deref().unwrap_or("执行主机路径尚未返回")), editor,
        row![button(text(if draft.saving.is_some() { "保存中" } else { "保存" })).padding([8,12]).style(primary_action_btn_style)
            .on_press_maybe((draft.dirty() && draft.saving.is_none()).then(|| message(Event::Save))),
            button(text("重新读取")).padding([8,12]).style(rounded_action_btn_style)
            .on_press_maybe((!draft.loading && draft.saving.is_none()).then(|| message(Event::Retry)))].spacing(8)]
        .spacing(8).into()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("网关不可用"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|id| !id.is_empty()) else { return Task::none(); };
    let state = &mut app.apps_catalog.hermes_memory;
    let reply_scope = match &event { Event::Loaded(scope, ..) | Event::Saved(scope, ..) => Some(scope), _ => None };
    if let Some(scope) = reply_scope {
        if !state.open || scope.context != context || scope.host != app.apps_catalog.host || scope.project != project
            || scope.catalog != app.apps_catalog.generation || scope.generation != state.generation { return Task::none(); }
    } else if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        return app.show_error_toast("请先连接应用服务");
    }
    match event {
        Event::Loaded(_, index, result) => {
            let draft = &mut state.drafts[index]; draft.loading = false;
            match result { Ok(response) if response.kind == kind(index) => draft.hydrate(response),
                _ => return app.show_error_toast("Hermes 记忆读取失败，请重试") }
            return Task::none();
        }
        Event::Saved(_, index, result) => {
            match result {
                Ok(outcome) => { state.drafts[index].saved(); return app.show_success_toast(match outcome.backup_path {
                    Some(path) => format!("记忆已保存，下次启动或新建会话生效；备份：{path}"),
                    None => "记忆已保存，下次启动或新建会话生效".into(),
                }); }
                Err(_) => { state.drafts[index].saving = None; return app.show_error_toast("Hermes 记忆保存失败，草稿已保留"); }
            }
        }
        Event::Select(index) => { if index < 2 { state.active = index; } return Task::none(); }
        Event::Edit(action) => {
            let draft = &mut state.drafts[state.active];
            if draft.saved.is_some() { draft.edited |= action.is_edit(); draft.editor.perform(action); }
            return Task::none();
        }
        Event::Open => { if state.open { return Task::none(); } state.open = true; }
        Event::Save | Event::Retry => { if !state.open { return Task::none(); } }
    }
    let scope = Scope { context, host: app.apps_catalog.host.clone(), project: project.to_string(), catalog: app.apps_catalog.generation, generation: state.generation };
    let query = HermesMemoryScope { project_id: project.into(), target_host_id: scope.host.clone() };
    let token = app.apps_catalog.token.clone();
    if matches!(event, Event::Save) {
        let index = state.active;
        let draft = &mut state.drafts[index];
        if !draft.dirty() || draft.saving.is_some() { return Task::none(); }
        let snapshot = draft.editor.text(); draft.saving = Some(snapshot.clone());
        return Task::perform(async move {
            let result = client.set_hermes_memory(&query, kind(index), snapshot, &token).await.map_err(|_| "保存失败".into());
            message(Event::Saved(scope, index, result))
        }, |message| message);
    }
    let indices: Vec<usize> = if matches!(event, Event::Open) { vec![0,1] } else { vec![state.active] };
    Task::batch(indices.into_iter().filter_map(|index| {
        if state.drafts[index].loading || state.drafts[index].saving.is_some() { return None; }
        state.drafts[index].loading = true;
        let (client, query, token, scope) = (client.clone(), query.clone(), token.clone(), scope.clone());
        Some(Task::perform(async move {
            let result = client.hermes_memory(&query, kind(index), &token).await.map_err(|_| "读取失败".into());
            message(Event::Loaded(scope, index, result))
        }, |message| message))
    }).collect::<Vec<_>>())
}
