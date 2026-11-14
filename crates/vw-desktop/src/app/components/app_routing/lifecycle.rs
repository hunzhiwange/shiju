//! Host-owned routing: the desktop only sends explicit SDK commands.
use crate::app::{
    App, Message,
    components::system_settings_common::*,
    message::{SettingsMessage, settings::AppsCatalogMessage},
    state::{AgentAppsContext, AppsCatalogState},
};
use iced::{
    Element, Task,
    widget::{button, column, row, text},
};
use vw_gateway_client::vw_api_types::app_routing::{RoutingHealth, RoutingState, RoutingStatus};
use vw_gateway_client::{AppRoutingError, GatewayClient};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Context {
    gateway: AgentAppsContext,
    host: String,
    generation: u64,
    access: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    context: Option<Context>,
    serial: u64,
    busy: bool,
    status: Option<RoutingStatus>,
    health: Option<RoutingHealth>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Refresh,
    Start,
    Stop,
}

#[derive(Clone, Debug)]
pub(crate) enum Event {
    Run(Operation),
    Loaded(Context, u64, Result<(RoutingStatus, RoutingHealth), String>),
}

impl State {
    pub(crate) fn invalidate(&mut self) {
        let serial = self.serial.wrapping_add(1);
        *self = Self { serial, ..Default::default() };
    }

    fn finish(
        &mut self,
        context: &Context,
        serial: u64,
        result: Result<(RoutingStatus, RoutingHealth), String>,
    ) -> bool {
        if self.context.as_ref() != Some(context) || self.serial != serial || !self.busy {
            return false;
        }
        self.busy = false;
        match result {
            Ok((status, health)) => {
                self.status = Some(status);
                self.health = Some(health);
                self.error = None;
            }
            Err(error) => {
                // A failed command may have reached the host. Never infer its outcome.
                self.status = None;
                self.health = None;
                self.error = Some(error);
            }
        }
        true
    }
}

fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Routing(event)))
}

fn context(catalog: &AppsCatalogState) -> Option<Context> {
    if catalog.token.is_empty() || catalog.host.is_empty() || catalog.loading {
        return None;
    }
    Some(Context {
        gateway: catalog.context.clone()?,
        host: catalog.host.clone(),
        generation: catalog.generation,
        access: catalog.model_diagnostics.access_generation,
    })
}

fn current(app: &App) -> Option<Context> {
    let context = context(&app.apps_catalog)?;
    let client = crate::app::config::gateway_client().ok()?;
    if context.gateway
        != (AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: app.project_path.clone(),
        })
    {
        return None;
    }
    Some(context)
}

fn error_message(error: AppRoutingError) -> String {
    match error {
        AppRoutingError::Http { status: 401 | 403, .. } => {
            "无服务操作授权，请重新连接执行主机".into()
        }
        AppRoutingError::Http { status, body } => {
            // Only interpret known codes; never render arbitrary response payloads.
            let reason = match body.as_ref().map(|b| b.error.code.as_str()) {
                Some("address_in_use") => "监听地址已占用",
                Some("apps_attached") => "仍有应用接入，请先退出路由模式",
                Some("transition_in_progress") => "服务正在切换，请稍后刷新",
                Some("unsupported_host") => "执行主机不支持此操作",
                Some("revision_conflict") => "应用配置已变化或需要恢复",
                Some("target_failed") => "应用配置恢复失败",
                _ => "请检查主机连接与服务配置后刷新",
            };
            format!("服务操作失败（HTTP {status}）：{reason}")
        }
        _ => "服务请求失败，当前状态未知，请刷新重试".into(),
    }
}

async fn request(
    client: GatewayClient,
    host: String,
    token: String,
    operation: Operation,
) -> Result<(RoutingStatus, RoutingHealth), String> {
    // A bound encompasses both status and health, including the command response.
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            let status = match operation {
                Operation::Refresh => client.app_routing_status(&host, &token).await,
                Operation::Start => client.app_routing_start(&host, &token).await,
                Operation::Stop => client.app_routing_stop(&host, &token).await,
            }
            .map_err(error_message)?;
            let health = if status.state == RoutingState::Running {
                match client.app_routing_health(&host, &token).await {
                    Ok(health) => health,
                    Err(AppRoutingError::Http { status: 503, body })
                        if body.as_ref().is_some_and(|b| b.error.code == "not_ready") =>
                    {
                        RoutingHealth { ready: false }
                    }
                    Err(error) => return Err(error_message(error)),
                }
            } else {
                RoutingHealth { ready: false }
            };
            Ok((status, health))
        })
        .await
        .map_err(|_| "服务请求超时，操作结果未知，请刷新确认".to_string())?
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (client, host, token, operation);
        Err(error_message(AppRoutingError::Transport))
    }
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(context) = current(app) else {
        return Task::none();
    };
    match event {
        Event::Run(operation) => {
            let state = &mut app.apps_catalog.routing;
            if state.context.as_ref() != Some(&context) {
                state.invalidate();
            }
            if state.busy {
                return Task::none();
            }
            if !matches!(operation, Operation::Refresh) && !can_control(state, operation) {
                return Task::none();
            }
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(_) => return app.show_error_toast("无法连接网关，请重新连接执行主机"),
            };
            state.context = Some(context.clone());
            state.serial = state.serial.wrapping_add(1);
            state.busy = true;
            state.status = None;
            state.health = None;
            state.error = None;
            let serial = state.serial;
            let token = app.apps_catalog.token.clone();
            Task::perform(
                async move {
                    let result = request(client, context.host.clone(), token, operation).await;
                    Event::Loaded(context, serial, result)
                },
                message,
            )
        }
        Event::Loaded(request, serial, result) => {
            if request != context {
                return Task::none();
            }
            let state = &mut app.apps_catalog.routing;
            if state.finish(&request, serial, result) {
                if let Some(error) = state.error.clone() {
                    return app.show_error_toast(error);
                }
            }
            Task::none()
        }
    }
}

fn can_control(state: &State, operation: Operation) -> bool {
    if state.busy {
        return false;
    }
    match (operation, state.status.as_ref().map(|s| s.state)) {
        (Operation::Start, Some(RoutingState::Stopped | RoutingState::Failed)) => true,
        (Operation::Stop, Some(RoutingState::Running)) => true,
        _ => false,
    }
}

pub(crate) fn view(catalog: &AppsCatalogState) -> Element<'_, Message> {
    let state = &catalog.routing;
    let connected = context(catalog).is_some();
    let fresh = connected && state.context == context(catalog);
    let status = state.status.as_ref().filter(|_| fresh);
    let label = if fresh && state.busy {
        "正在请求"
    } else {
        match status.map(|s| s.state) {
            Some(RoutingState::Stopped) => "已停止",
            Some(RoutingState::Starting) => "正在启动",
            Some(RoutingState::Running) => "运行中",
            Some(RoutingState::Stopping) => "正在停止",
            Some(RoutingState::Failed) => "失败",
            None => "未知，请刷新",
        }
    };
    let mut body = column![
        text("常驻路由服务").size(17),
        text("关闭桌面窗口后服务继续运行；停止前请先退出应用路由模式。").size(13),
        text(format!("服务：{label}")),
        text(match state.health.filter(|_| fresh) {
            Some(RoutingHealth { ready: true }) => "健康：已就绪",
            Some(_) => "健康：未就绪",
            None => "健康：未知",
        }),
        text("实际监听地址"),
    ]
    .spacing(8);
    if let Some(address) =
        status.and_then(|s| Some(std::net::SocketAddr::new(s.bound_address?, s.bound_port?)))
    {
        body = body.push(text(address.to_string()).font(iced::Font::MONOSPACE));
    } else {
        body = body.push(text("尚未取得实际地址"));
    }
    if fresh {
        if let Some(error) = &state.error {
            body = body.push(settings_error_banner(error.clone()));
        }
    }
    let refresh =
        button(text("刷新服务")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(
            (connected && (!fresh || !state.busy))
                .then_some(message(Event::Run(Operation::Refresh))),
        );
    let start =
        button(text("启动服务")).padding([8, 12]).style(primary_action_btn_style).on_press_maybe(
            (fresh && can_control(state, Operation::Start))
                .then_some(message(Event::Run(Operation::Start))),
        );
    let stop =
        button(text("停止服务")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(
            (fresh && can_control(state, Operation::Stop))
                .then_some(message(Event::Run(Operation::Stop))),
        );
    settings_panel(body.push(row![refresh, start, stop].spacing(8).wrap())).into()
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
