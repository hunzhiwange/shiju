//! Host-owned routing: the desktop only sends explicit SDK commands.
use gateway_app_routing::AppRoutingError;
use desktop_ui_controls::system_settings_common::*;
use crate::AgentAppsContext;
use iced::{
    Element, Task,
    widget::{button, column, row, text},
};
use vw_api_app_routing::{RoutingHealth, RoutingState, RoutingStatus};
use {vw_gateway_client::GatewayClient};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    gateway: AgentAppsContext,
    pub(super) host: String,
    generation: u64,
    access: u64,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub(super) requests: super::requests::State,
    pub(super) config: super::config::State,
    pub(super) outbound: super::outbound::State,
    context: Option<Context>,
    serial: u64,
    busy: bool,
    status: Option<RoutingStatus>,
    health: Option<RoutingHealth>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Refresh,
    Start,
    Stop,
}

#[derive(Clone, Debug)]
pub enum Event {
    Requests(super::requests::Event),
    Config(super::config::Event),
    Outbound(super::outbound::Event),
    Run(Operation),
    Loaded(Context, u64, Result<(RoutingStatus, RoutingHealth), String>),
}

impl State {
    pub fn invalidate(&mut self) {
        self.requests.invalidate();
        let requests = self.requests.clone();
        self.outbound.invalidate();
        let outbound = self.outbound.clone();
        let serial = self.serial.wrapping_add(1);
        let config_serial = self.config.next_serial();
        *self = Self {
            serial,
            requests,
            outbound,
            config: super::config::State::with_serial(config_serial),
            ..Default::default()
        };
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

type Message = Event;
fn message(event: Event) -> Message { event }

pub fn context(gateway: Option<&AgentAppsContext>, host: &str, generation: u64, access: u64, loading: bool, token: &str) -> Option<Context> {
    if token.is_empty() || host.is_empty() || loading { return None; }
    Some(Context { gateway: gateway?.clone(), host: host.into(), generation, access })
}

pub(super) fn error_message(error: AppRoutingError) -> String {
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
    let client = gateway_app_routing::GatewayAppRoutingClient::new(client.transport().clone());
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

pub fn update(state: &mut State, context: Option<Context>, project: Option<String>, token: &str, client: Option<vw_gateway_client::GatewayClient>, notice: &mut Option<crate::Notice>, event: Event) -> Task<Message> {
    if let Event::Requests(event) = event {
        return super::requests::update(&mut state.requests, context, project, token, client, notice, event);
    }
    if let Event::Outbound(event) = event {
        return super::outbound::update(&mut state.outbound, context, project, token, client, notice, event);
    }
    if let Event::Config(event) = event {
        return super::config::update(&mut state.config, context, project, token, client, notice, event);
    }
    let Some(context) = context else {
        return Task::none();
    };
    match event {
        Event::Requests(_) | Event::Config(_) | Event::Outbound(_) => unreachable!(),
        Event::Run(operation) => {
            if state.context.as_ref() != Some(&context) {
                state.invalidate();
            }
            if state.busy {
                return Task::none();
            }
            if !matches!(operation, Operation::Refresh) && !can_control(state, operation) {
                return Task::none();
            }
            let client = match client {
                Some(client) => client,
                None => return crate::error(notice, "无法连接网关，请重新连接执行主机"),
            };
            state.context = Some(context.clone());
            state.serial = state.serial.wrapping_add(1);
            state.busy = true;
            state.status = None;
            state.health = None;
            state.error = None;
            let serial = state.serial;
            let token = token.to_owned();
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
            if state.finish(&request, serial, result) {
                if let Some(error) = state.error.clone() {
                    return crate::error(notice, error);
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

pub fn view(state: &State, context: Option<Context>) -> Element<'_, Message> {
    let connected = context.is_some();
    let fresh = connected && state.context == context;
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
    settings_panel(
        body.push(
            row![
                refresh,
                start,
                stop,
                button(text("监听设置"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(
                        connected
                            .then_some(message(Event::Config(super::config::Event::Open(None))))
                    )
            ]
            .spacing(8)
            .wrap(),
        ),
    )
    .into()
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
