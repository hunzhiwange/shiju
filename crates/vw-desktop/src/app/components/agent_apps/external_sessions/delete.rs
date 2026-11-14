//! Selection and confirmation only: all destructive work goes through the authorized SDK.
use super::{message, Scope};
use crate::app::{App, Message, components::system_settings_common::*, state::AgentAppsContext};
use iced::{Element, Length, Task, widget::{button, column, container, row, scrollable, text}};
use iced::widget::scrollable::{Direction, Scrollbar};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::external_session::*};

#[derive(Clone, Default)]
pub(super) struct State {
    pub(super) selecting: bool,
    pub(super) selected: Vec<SessionMeta>,
    pub(super) failures: Vec<(ExternalSessionIdentity, String)>,
    pending: Vec<SessionMeta>,
    loading: bool,
    deleting: bool,
}
impl State {
    pub(super) fn busy(&self) -> bool { self.loading || self.deleting }
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
fn msg(event: Event) -> Message { message(super::Event::Delete(event)) }

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用，请重新连接");
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone() else { return Task::none(); };
    let state = &mut app.apps_catalog.external_sessions;
    if !state.open || app.apps_catalog.context.as_ref() != Some(&context)
        || state.host != app.apps_catalog.host || app.apps_catalog.token.is_empty() {
        return Task::none();
    }
    let scope = Scope { context, host: state.host.clone(), project: project.clone(),
        catalog: app.apps_catalog.generation, generation: state.generation, append: false };
    let current = |other: &Scope| other.context == scope.context && other.host == scope.host
        && other.project == scope.project && other.catalog == scope.catalog
        && other.generation == scope.generation;
    match event {
        Event::Selected(other, result) => {
            if !current(&other) || !state.deletion.loading { return Task::none(); }
            state.deletion.loading = false;
            match result {
                Ok(sessions) => { state.deletion.selected = sessions; state.deletion.failures.clear(); }
                Err(_) => return app.show_error_toast("全选匹配结果失败，原有选择保留，请重试"),
            }
        }
        Event::Finished(other, requested, result) => {
            if !current(&other) || !state.deletion.deleting { return Task::none(); }
            state.deletion.deleting = false;
            state.deletion.pending.clear();
            let mut removed = Vec::new();
            state.deletion.failures.clear();
            for identity in requested {
                let matches = result.as_ref().ok().map(|outcomes| outcomes.iter()
                    .filter(|outcome| outcome.identity == identity).collect::<Vec<_>>()).unwrap_or_default();
                let failure = if matches.len() == 1 {
                    match &matches[0].result {
                        DeleteSessionResult::Deleted | DeleteSessionResult::AlreadyAbsent => {
                            removed.push(identity.clone()); None
                        }
                        DeleteSessionResult::Failed { error } => Some(format!("删除失败：{:?}", error.code)),
                    }
                } else { Some("删除结果未确认，请刷新后核实再重试".into()) };
                if let Some(reason) = failure { state.deletion.failures.push((identity, reason)); }
            }
            if let Some(page) = &mut state.page {
                page.sessions.retain(|session| !removed.contains(&session.identity));
                page.total = page.total.saturating_sub(removed.len());
                // Counts belong to the old catalog snapshot; refresh will rebuild them.
                page.groups.clear();
                if !removed.is_empty() { page.next_cursor = None; }
            }
            state.deletion.selected.retain(|session| !removed.contains(&session.identity));
            if state.reader.as_ref().is_some_and(|reader| removed.contains(&reader.identity)) {
                state.cancel_reader();
            }
            let failed = state.deletion.failures.len();
            let notice = format!("已删除或已不存在 {} 项，失败 {} 项；失败项保留。刷新以更新目录", removed.len(), failed);
            return if failed == 0 { app.show_success_toast(notice) } else { app.show_error_toast(notice) };
        }
        _ if state.deletion.busy() => return Task::none(),
        Event::Enter => state.deletion.selecting = true,
        Event::Exit => state.deletion = State::default(),
        Event::Clear => { state.deletion.selected.clear(); state.deletion.failures.clear(); }
        Event::Cancel => state.deletion.pending.clear(),
        Event::Toggle(identity) => {
            if !state.deletion.selecting || !state.deletion.pending.is_empty() { return Task::none(); }
            if let Some(index) = state.deletion.selected.iter().position(|session| session.identity == identity) {
                state.deletion.selected.remove(index);
            } else if let Some(session) = state.page.as_ref().and_then(|page| page.sessions.iter()
                .find(|session| session.identity == identity && session.capabilities.delete)) {
                state.deletion.selected.push(session.clone());
            }
        }
        Event::Ask(identity) => {
            if state.busy { return Task::none(); }
            state.deletion.pending = match identity {
                Some(identity) => state.page.as_ref().and_then(|page| page.sessions.iter()
                    .find(|session| session.identity == identity && session.capabilities.delete))
                    .cloned().into_iter().collect(),
                None => state.deletion.selected.clone(),
            };
        }
        Event::SelectAll => {
            if state.busy || !state.deletion.selecting { return Task::none(); }
            state.deletion.loading = true;
            let token = app.apps_catalog.token.clone();
            return Task::perform(async move {
                let result = async {
                    let mut query = SessionCatalogQuery { project_id: project.into(), target_host_id: scope.host.clone(),
                        provider: None, instance_id: None, project_dir: None, search: String::new(), page_size: 200, cursor: None };
                    let mut sessions: Vec<SessionMeta> = Vec::new();
                    let mut cursors = Vec::new();
                    loop {
                        let page = client.external_sessions_catalog(&query, &token).await?;
                        if page.truncated { return Err(AgentAppsReadError::Decode("catalog truncated".into())); }
                        for session in page.sessions {
                            if session.capabilities.delete && !sessions.iter().any(|old| old.identity == session.identity) {
                                sessions.push(session);
                            }
                        }
                        let Some(cursor) = page.next_cursor else { break; };
                        if cursors.contains(&cursor) { return Err(AgentAppsReadError::Decode("repeated cursor".into())); }
                        cursors.push(cursor.clone());
                        query.cursor = Some(cursor);
                    }
                    Ok(sessions)
                }.await;
                msg(Event::Selected(scope, result))
            }, |message| message);
        }
        Event::Confirm => {
            if state.busy || state.deletion.pending.is_empty() { return Task::none(); }
            let requested: Vec<_> = state.deletion.pending.iter().map(|session| session.identity.clone()).collect();
            // Retain single-item failures in selection as well as batch failures.
            state.deletion.selecting = true;
            for session in state.deletion.pending.clone() {
                if !state.deletion.selected.iter().any(|old| old.identity == session.identity) {
                    state.deletion.selected.push(session);
                }
            }
            state.deletion.deleting = true;
            let token = app.apps_catalog.token.clone();
            return Task::perform(async move {
                let mut outcomes = Vec::new();
                // The batch endpoint accepts at most 200 identities per request.
                for chunk in requested.chunks(200) {
                    let requests: Vec<_> = chunk.iter().cloned().map(|identity| DeleteSessionRequest { identity }).collect();
                    match client.external_sessions_delete(&requests, &token).await {
                        Ok(mut batch) => outcomes.append(&mut batch),
                        Err(_) => break, // Missing outcomes remain unconfirmed and selected.
                    }
                }
                msg(Event::Finished(scope, requested, Ok(outcomes)))
            }, |message| message);
        }
    }
    Task::none()
}

pub(super) fn view<'a>(state: &'a super::State, base: Element<'a, Message>) -> Element<'a, Message> {
    if state.deletion.pending.is_empty() { return base; }
    let enabled = !state.deletion.deleting;
    let close = msg(Event::Cancel);
    let mut content = column![text("将从来源永久删除，不经过废纸篓，无法撤销。")].spacing(8).padding([0, 7]);
    for session in &state.deletion.pending {
        content = content.push(text(format!("{:?} · {} · {}", session.identity.provider,
            session.identity.instance_id, session.title.as_deref().unwrap_or(&session.identity.session_id.0))));
    }
    let card = settings_modal_card(column![
        row![text(format!("删除 {} 个会话？", state.deletion.pending.len())).size(18), settings_close_button(close.clone())].spacing(12),
        scrollable(content).height(Length::Fixed(220.0))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        row![button(text("取消")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(enabled.then(|| close.clone())),
            button(text(if enabled { "确认删除" } else { "正在删除…" })).padding([8, 12]).style(primary_action_btn_style)
                .on_press_maybe(enabled.then(|| msg(Event::Confirm)))].spacing(8),
    ].spacing(16)).width(Length::Fixed(560.0));
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
