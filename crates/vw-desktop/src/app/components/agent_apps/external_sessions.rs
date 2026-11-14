//! Read-only external history catalog; credentials stay in the application connection.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_common::*;
use iced::{Element, Task, widget::{button, column, text}};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::external_session::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    busy: bool,
    page: Option<SessionCatalogPage>,
    generation: u64,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone, Debug)]
pub struct Scope {
    context: AgentAppsContext,
    host: String,
    project: String,
    catalog: u64,
    generation: u64,
}
#[derive(Clone, Debug)]
pub enum Event {
    Refresh,
    Next,
    Loaded(Scope, Result<SessionCatalogPage, AgentAppsReadError>),
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::ExternalSessions(event)))
}
pub(crate) fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![text("外部会话").size(15),
        button(text(if state.busy { "正在读取会话…" } else { "读取 / 刷新外部会话" }))
            .padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then(|| message(Event::Refresh)))].spacing(8);
    if let Some(page) = &state.page {
        content = content.push(text(format!("共 {} 项；当前页 {} 项", page.total, page.sessions.len())));
        if page.sessions.is_empty() { content = content.push(text("当前授权范围内没有会话")); }
        for session in &page.sessions {
            content = content.push(text(format!("{:?} · {}", session.identity.provider,
                session.title.as_deref().unwrap_or(&session.identity.session_id.0))));
            if let Some(summary) = &session.summary { content = content.push(text(summary)); }
        }
        if page.truncated { content = content.push(text("来源扫描已截断，目录可能不完整")); }
        if page.next_cursor.is_some() {
            content = content.push(button(text("下一页")).padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe((!state.busy).then(|| message(Event::Next))));
        }
    }
    content.into()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
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
            || !app.apps_catalog.external_sessions.busy { return Task::none(); }
        app.apps_catalog.external_sessions.busy = false;
        match result {
            Ok(page) => app.apps_catalog.external_sessions.page = Some(page),
            Err(_) => return app.show_error_toast("外部会话读取失败，请检查主机授权并重试"),
        }
        return Task::none();
    }
    if app.apps_catalog.external_sessions.busy { return Task::none(); }
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
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope { context, host: app.apps_catalog.host.clone(), project: project.clone(),
        catalog: app.apps_catalog.generation, generation: state.generation };
    let query = SessionCatalogQuery { project_id: project.into(), target_host_id: scope.host.clone(),
        provider: None, instance_id: None, project_dir: None, search: String::new(), page_size: 20, cursor };
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        message(Event::Loaded(scope, client.external_sessions_catalog(&query, &token).await))
    }, |message| message)
}
