//! Read-only, content-free snapshots; observing requests never owns host execution.
use super::lifecycle::{self, Context};
use crate::app::{App, Message, components::system_settings_common::*, state::AppsCatalogState};
use iced::{Element, Task, widget::{button, column, container, text}};
use vw_gateway_client::vw_api_types::{agent_apps::AppId, app_routing::{RoutingRequestState, RoutingRequestsStatus}, id::ProjectId};

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    serial: u64,
    context: Option<Context>,
    project: Option<String>,
    app: Option<AppId>,
    snapshot: Option<RoutingRequestsStatus>,
    error: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open(AppId),
    Close,
    Refresh,
    Loaded(Context, u64, Result<RoutingRequestsStatus, String>),
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::Routing(lifecycle::Event::Requests(event)),
    ))
}
impl State {
    pub(super) fn invalidate(&mut self) {
        *self = Self { serial: self.serial.wrapping_add(1), ..Default::default() };
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        // Closing only discards this observation; it never cancels host requests.
        app.apps_catalog.routing.requests.invalidate();
        return Task::none();
    }
    let Some(context) = lifecycle::current(app) else { return Task::none() };
    let project = app.project_id.clone();
    let state = &mut app.apps_catalog.routing.requests;
    match event {
        Event::Open(id) => {
            state.invalidate();
            state.open = true;
            state.context = Some(context.clone());
            state.project = project.clone();
            state.app = Some(id);
        }
        Event::Loaded(request, serial, result) => {
            if request != context || state.context.as_ref() != Some(&request)
                || state.project != project || state.serial != serial || !state.busy {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(snapshot) => { state.snapshot = Some(snapshot); state.error = None; }
                Err(error) => {
                    state.snapshot = None;
                    state.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
            return Task::none();
        }
        Event::Refresh => {}
        Event::Close => unreachable!(),
    }
    if !state.open || state.busy || state.context.as_ref() != Some(&context) || state.project != project {
        return Task::none();
    }
    let Some(project) = project.filter(|p| !p.is_empty()) else {
        return app.show_error_toast("请先选择项目");
    };
    let Some(id) = state.app else { return Task::none() };
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("无法连接网关");
    };
    state.serial = state.serial.wrapping_add(1);
    let serial = state.serial;
    state.busy = true;
    state.snapshot = None;
    state.error = None;
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        #[cfg(not(target_arch = "wasm32"))]
        let result = tokio::time::timeout(std::time::Duration::from_secs(15),
            client.app_routing_requests(&context.host, &ProjectId::from(project), id, &token))
            .await.map_err(|_| "读取请求状态超时，请刷新重试".to_string())
            .and_then(|result| result.map_err(lifecycle::error_message));
        #[cfg(target_arch = "wasm32")]
        let result = {
            let _ = (client, project, id, token);
            Err("此平台不支持读取请求状态".to_string())
        };
        Event::Loaded(context, serial, result)
    }, message)
}
pub(crate) fn entry(id: AppId) -> Element<'static, Message> {
    button(text("请求状态" )).padding([8, 12]).style(rounded_action_btn_style)
        .on_press(message(Event::Open(id))).into()
}
pub(crate) fn overlay<'a>(catalog: &'a AppsCatalogState, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &catalog.routing.requests;
    if !state.open { return base; }
    let header = settings_modal_header(text("路由请求状态").size(20), settings_close_button(message(Event::Close)));
    let mut body = column![text("只读取脱敏快照；关闭窗口不会停止或取消请求。历史记录有限，刷新间可能被淘汰。")].spacing(12);
    if state.busy { body = body.push(text("正在读取")); }
    if let Some(snapshot) = &state.snapshot {
        body = body.push(text(format!("活跃请求：{}", snapshot.active_count)));
        for (index, request) in snapshot.requests.iter().enumerate() {
            let label = match request.state {
                RoutingRequestState::Active => "进行中",
                RoutingRequestState::Completed => "已完成",
                RoutingRequestState::Failed => "失败",
                RoutingRequestState::Cancelled => "已取消",
            };
            // Do not render even opaque server identifiers or arbitrary error payloads.
            body = body.push(text(format!("请求 {}：{} · {} ms", index + 1, label, request.elapsed_ms)));
        }
    }
    if let Some(error) = &state.error { body = body.push(settings_error_banner(error.clone())); }
    let refresh = button(text("刷新请求状态")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press_maybe((!state.busy).then_some(message(Event::Refresh)));
    let card = settings_modal_layout(header, body, Some(refresh.into())).width(700).height(500);
    settings_modal_overlay(Some(base), message(Event::Close), container(card).padding(12))
}
