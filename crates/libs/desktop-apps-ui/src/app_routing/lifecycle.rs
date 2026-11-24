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
    listener: super::config::State,
    attached: Option<(Vec<vw_api_app_identity::AppId>, bool)>,
    unconfigured: bool,
    pub(super) outbound: super::outbound::State,
    context: Option<Context>,
    serial: u64,
    busy: bool,
    confirming_restore: bool,
    status: Option<RoutingStatus>,
    retry_start: bool,
    health: Option<RoutingHealth>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Refresh,
    Start,
    Stop,
    StopWithRestore,
}

#[derive(Clone, Debug)]
pub enum Event {
    Requests(super::requests::Event),
    Config(super::config::Event),
    Listener(super::config::Event),
    InspectPage,
    Attached(Context, u64, vw_api_transport::id::ProjectId, Vec<vw_api_app_identity::AppId>, bool),
    Outbound(super::outbound::Event),
    Run(Operation),
    ConfirmRestore,
    CancelRestore,
    Loaded(
        Context,
        u64,
        Operation,
        Result<(RoutingStatus, RoutingHealth), StatusFailure>,
    ),
}

#[derive(Clone, Debug)]
pub enum StatusFailure {
    Unconfigured,
    Request(String),
    Restore {
        status: RoutingStatus,
        health: RoutingHealth,
        message: String,
    },
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
            listener: super::config::State::with_serial(self.listener.next_serial()),
            ..Default::default()
        };
    }

    fn finish(
        &mut self,
        context: &Context,
        serial: u64,
        result: Result<(RoutingStatus, RoutingHealth), StatusFailure>,
    ) -> bool {
        if self.context.as_ref() != Some(context) || self.serial != serial || !self.busy {
            return false;
        }
        self.busy = false;
        self.unconfigured = matches!(&result, Err(StatusFailure::Unconfigured));
        match result {
            Ok((status, health)) => {
                self.status = Some(status);
                self.health = Some(health);
                self.error = None;
            }
            Err(StatusFailure::Restore { status, health, message }) => {
                self.status = Some(status);
                self.health = Some(health);
                self.error = Some(message);
            }
            Err(error) => {
                // A failed command may have reached the host. Never infer its outcome.
                self.status = None;
                self.health = None;
                self.error = match error {
                    StatusFailure::Unconfigured => None,
                    StatusFailure::Request(error) => Some(error),
                    StatusFailure::Restore { .. } => unreachable!(),
                };
            }
        }
        true
    }
}

type Message = Event;
fn message(event: Event) -> Message { event }

pub fn context(gateway: Option<&AgentAppsContext>, host: &str, generation: u64, access: u64, loading: bool, _token: &str) -> Option<Context> {
    if host.is_empty() || loading { return None; }
    Some(Context { gateway: gateway?.clone(), host: host.into(), generation, access })
}

pub(super) fn error_message(error: AppRoutingError) -> String {
    match error {
        AppRoutingError::Http { status: 401 | 403, .. } => {
            "网关拒绝了路由服务请求".into()
        }
        AppRoutingError::Http { status, body } => {
            // Only interpret known codes; never render arbitrary response payloads.
            let reason = match body.as_ref().map(|b| b.error.code.as_str()) {
                Some("config_not_applied") => "监听设置无法应用，请检查端口占用及应用接入状态",
                Some("address_in_use") => "监听地址已占用",
                Some("apps_attached") => "仍有应用接入，请先退出路由模式",
                Some("transition_in_progress") => "服务正在切换，请稍后刷新",
                Some("unsupported_host") => "当前环境不支持此操作",
                Some("revision_conflict") => "应用配置已变化或需要恢复",
                Some("target_failed") => "应用配置恢复失败",
                _ => "请检查网关连接与服务配置后刷新",
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
) -> Result<(RoutingStatus, RoutingHealth), StatusFailure> {
    let client = gateway_app_routing::GatewayAppRoutingClient::new(client.transport().clone());
    // A bound encompasses both status and health, including the command response.
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            let (status, restore_error) = match operation {
                Operation::Refresh => (client.app_routing_status(&host, &token).await, None),
                Operation::Start => (client.app_routing_start(&host, &token).await, None),
                Operation::Stop => (client.app_routing_stop(&host, &token).await, None),
                Operation::StopWithRestore => {
                    match client.app_routing_stop_with_restore(&host, &token).await {
                        Ok(result) => (Ok(result.status), restore_failure_message(&result.failures)),
                        Err(error) => (
                            client.app_routing_status(&host, &token).await,
                            Some(error_message(error)),
                        ),
                    }
                }
            };
            let status = status.map_err(|error| match &error {
                AppRoutingError::Http { status: 503, body: Some(body) }
                    if matches!(operation, Operation::Refresh)
                        && body.error.code == "routing_control_not_configured" =>
                {
                    StatusFailure::Unconfigured
                }
                AppRoutingError::Http { status: 503, .. }
                    if matches!(operation, Operation::Refresh) =>
                {
                    StatusFailure::Request(
                        "网关暂时无法读取路由状态，请检查路由授权与网关连接后刷新。".into(),
                    )
                }
                _ => StatusFailure::Request(
                    restore_error.clone().unwrap_or_else(|| error_message(error)),
                ),
            })?;
            let health = if status.state == RoutingState::Running {
                match client.app_routing_health(&host, &token).await {
                    Ok(health) => health,
                    Err(AppRoutingError::Http { status: 503, body })
                        if body.as_ref().is_some_and(|b| b.error.code == "not_ready") =>
                    {
                        RoutingHealth { ready: false }
                    }
                    Err(_) if restore_error.is_some() => RoutingHealth { ready: false },
                    Err(error) => return Err(StatusFailure::Request(error_message(error))),
                }
            } else {
                RoutingHealth { ready: false }
            };
            match restore_error {
                Some(message) => Err(StatusFailure::Restore { status, health, message }),
                None => Ok((status, health)),
            }
        })
        .await
        .map_err(|_| StatusFailure::Request("服务请求超时，请刷新确认".into()))?
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (client, host, token, operation);
        Err(StatusFailure::Request(error_message(AppRoutingError::Transport)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn restore_failure_message(failures: &[gateway_app_routing::RoutingRestoreFailure]) -> Option<String> {
    use vw_api_app_identity::AppId;
    if failures.is_empty() {
        return None;
    }
    let names = failures
        .iter()
        .map(|failure| {
            let name = match failure.app_id {
                AppId::Claude => "Claude Code".into(),
                AppId::Codex => "Codex".into(),
                AppId::ClaudeDesktop => "Claude Desktop".into(),
                app => format!("{app:?}"),
            };
            format!("{name}（HTTP {}）", failure.status)
        })
        .collect::<Vec<_>>()
        .join("、");
    Some(format!("以下应用恢复直连失败：{names}。请处理后重试。"))
}

pub fn update(
    state: &mut State,
    context: Option<Context>,
    project: Option<String>,
    token: &str,
    client: Option<vw_gateway_client::GatewayClient>,
    notice: &mut Option<crate::Notice>,
    event: Event,
    inline_page: bool,
) -> Task<Message> {
    if let Event::Requests(event) = event {
        return super::requests::update(&mut state.requests, context, project, token, client, notice, event);
    }
    if let Event::Outbound(event) = event {
        return super::outbound::update(&mut state.outbound, context, project, token, client, notice, event);
    }
    if let Event::Config(event) = event {
        return super::config::update(&mut state.config, context, project, token, client, notice, event);
    }
    if let Event::Listener(event) = event {
        return super::config::update_inline(
            &mut state.listener,
            context,
            project,
            token,
            client,
            notice,
            event,
        );
    }
    let Some(context) = context else {
        return Task::none();
    };
    match event {
        Event::Requests(_) | Event::Config(_) | Event::Outbound(_) | Event::Listener(_) => {
            unreachable!()
        }
        Event::InspectPage => {
            if !inline_page || state.context.as_ref() != Some(&context) || state.busy {
                return Task::none();
            }
            let mut tasks = Vec::new();
            if state.status.is_some() && !state.listener.is_open() {
                tasks.push(Task::done(Event::Listener(super::config::Event::Open(None))));
            }
            if let Some(client) = client {
                state.attached = None;
                let scope = vw_api_app_aggregation::AggregationQuery {
                    target_host_id: context.host.clone(),
                    project_id: vw_api_transport::local_settings_project(project.as_deref()).into(),
                };
                let serial = state.serial;
                let project = scope.project_id.clone();
                tasks.push(Task::perform(attached_apps(client, scope), move |(apps, complete)| {
                    Event::Attached(context.clone(), serial, project.clone(), apps, complete)
                }));
            }
            Task::batch(tasks)
        }
        Event::Attached(request, serial, request_project, apps, complete) => {
            if request == context
                && state.context.as_ref() == Some(&request)
                && serial == state.serial
                && vw_api_transport::local_settings_project(project.as_deref())
                    == request_project.as_ref()
            {
                state.attached = Some((apps, complete));
            }
            Task::none()
        }
        Event::ConfirmRestore => {
            state.confirming_restore = state.context.as_ref() == Some(&context)
                && can_control(state, Operation::StopWithRestore);
            Task::none()
        }
        Event::CancelRestore => {
            state.confirming_restore = false;
            Task::none()
        }
        Event::Run(operation) => {
            if state.context.as_ref() != Some(&context) {
                state.invalidate();
            }
            let confirmed = std::mem::take(&mut state.confirming_restore);
            if matches!(operation, Operation::StopWithRestore) && !confirmed {
                return Task::none();
            }
            if state.busy || (inline_page && state.listener.is_busy()) {
                return Task::none();
            }
            if !matches!(operation, Operation::Refresh) && !can_control(state, operation) {
                return Task::none();
            }
            let client = match client {
                Some(client) => client,
                None => return crate::error(notice, "无法连接网关，请重试"),
            };
            state.context = Some(context.clone());
            state.serial = state.serial.wrapping_add(1);
            state.busy = true;
            state.status = None;
            state.health = None;
            state.error = None;
            state.unconfigured = false;
            state.attached = None;
            let serial = state.serial;
            let token = token.to_owned();
            Task::perform(
                async move {
                    let result = request(client, context.host.clone(), token, operation).await;
                    Event::Loaded(context, serial, operation, result)
                },
                message,
            )
        }
        Event::Loaded(request, serial, operation, result) => {
            if request != context {
                return Task::none();
            }
            let failed = result.is_err();
            if state.finish(&request, serial, result) {
                match operation {
                    Operation::Start => state.retry_start = failed,
                    Operation::Stop | Operation::StopWithRestore if !failed => {
                        state.retry_start = false;
                    }
                    _ => {}
                }
                if matches!(operation, Operation::StopWithRestore) && !failed {
                    *notice = Some(crate::Notice::Success(
                        desktop_i18n::format::Message::new("应用已恢复直连，路由服务已停止", vec![]),
                    ));
                }
                if inline_page {
                    return Task::done(Event::InspectPage);
                }
                if let Some(error) = state.error.clone() {
                    return crate::error(notice, error);
                }
            }
            Task::none()
        }
    }
}

fn can_control(state: &State, operation: Operation) -> bool {
    if state.busy || state.listener.is_busy() {
        return false;
    }
    match (operation, state.status.as_ref().map(|s| s.state)) {
        (Operation::Start, Some(RoutingState::Stopped | RoutingState::Failed)) => true,
        (Operation::Start, None | Some(RoutingState::Running)) => state.retry_start,
        (Operation::Stop | Operation::StopWithRestore, Some(RoutingState::Running)) => true,
        _ => false,
    }
}

pub fn view(i18n: desktop_i18n::I18n, state: &State, context: Option<Context>) -> Element<'_, Message> {
    service_view(i18n, state, context, false)
}

pub fn page_service(i18n: desktop_i18n::I18n, state: &State, context: Option<Context>) -> Element<'_, Message> {
    service_view(i18n, state, context, true)
}

fn service_view(i18n: desktop_i18n::I18n, state: &State, context: Option<Context>, inline: bool) -> Element<'_, Message> {
    let connected = context.is_some();
    let fresh = connected && state.context == context;
    let status = state.status.as_ref().filter(|_| fresh);
    let label = service_label(i18n, state, context);
    service_card(i18n, state, connected, fresh, status, label, inline)
}

pub(crate) fn service_title(i18n: desktop_i18n::I18n) -> &'static str {
    i18n.__("路由服务")
}

pub(crate) fn service_label(
    i18n: desktop_i18n::I18n,
    state: &State,
    context: Option<Context>,
) -> &'static str {
    let connected = context.is_some();
    let fresh = connected && state.context == context;
    let status = state.status.as_ref().filter(|_| fresh);

    (if fresh && state.unconfigured {
        i18n.__("网关未启用路由管理")
    } else if fresh && state.busy {
        i18n.__("正在处理…")
    } else {
        match status.map(|status| status.state) {
            Some(RoutingState::Stopped) => i18n.__("已停止"),
            Some(RoutingState::Starting) => i18n.__("正在启动"),
            Some(RoutingState::Running) if state.health.is_some_and(|health| health.ready) => {
                i18n.__("运行中")
            }
            Some(RoutingState::Running) => i18n.__("运行中 · 尚未就绪"),
            Some(RoutingState::Stopping) => i18n.__("正在停止"),
            Some(RoutingState::Failed) => i18n.__("启动失败"),
            None if !connected => i18n.__("等待连接执行主机"),
            None => i18n.__("状态未知"),
        }
    }) as _
}

pub fn page_can_configure(state: &State, context: Option<Context>) -> bool {
    context.is_some() && state.context == context && state.status.is_some()
}

pub(crate) fn service_address(status: Option<&RoutingStatus>) -> Option<String> {
    let status = status?;
    Some(std::net::SocketAddr::new(status.bound_address?, status.bound_port?).to_string())
}

fn service_card<'a>(i18n: desktop_i18n::I18n,
    state: &'a State,
    connected: bool,
    fresh: bool,
    status: Option<&RoutingStatus>,
    label: &'static str,
    inline: bool,
) -> Element<'a, Message> {
    use iced::{Alignment, Length, widget::container};

    let mut details = column![
        text(service_title(i18n)).size(15),
        text(format!("●  {label}")).size(13).style(settings_muted_text_style),
    ]
    .width(Length::Fill)
    .spacing(5);
    if let Some(address) = service_address(status) {
        details = details.push(text(address).size(13).style(settings_muted_text_style));
    }
    let stopping = matches!(status.map(|s| s.state), Some(RoutingState::Running));
    let operation = if stopping { Operation::Stop } else { Operation::Start };
    let action_label = if stopping {
        i18n.__("停止")
    } else if state.retry_start {
        i18n.__("重试启动")
    } else {
        i18n.__("启动")
    };
    let action =
        button(text(action_label).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (fresh && can_control(state, operation)).then_some(Event::Run(operation)),
            );
    let refresh = button(text(i18n.__("刷新")).size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(
            (connected && (!fresh || !state.busy && !state.listener.is_busy()))
                .then_some(Event::Run(Operation::Refresh)),
        );
    let mut controls = row![refresh, action].spacing(8).align_y(Alignment::Center);
    if stopping {
        controls = controls.push(
            button(text(i18n.__("恢复直连并停止")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (fresh && can_control(state, Operation::StopWithRestore))
                        .then_some(Event::ConfirmRestore),
                ),
        );
    }
    if stopping && fresh && state.retry_start {
        controls = controls.push(
            button(text(i18n.__("重试启动")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    can_control(state, Operation::Start).then_some(Event::Run(Operation::Start)),
                ),
        );
    }
    let mut card = column![
        container(row![details, controls.wrap()].spacing(8).align_y(Alignment::Center))
            .padding([16, 0])
    ];
    if fresh && state.unconfigured {
        card = card.push(
            container(
                text(i18n.__("请检查网关的执行主机策略与路由管理授权，重启网关后刷新。"))
                    .size(13)
                    .style(settings_muted_text_style),
            )
            .padding(iced::padding::bottom(14)),
        );
    }
    if let Some(error) = state.error.as_ref().filter(|_| fresh) {
        card =
            card.push(container(settings_error_banner(i18n.__(error).to_owned())).padding(iced::padding::bottom(14)));
    }
    if inline
        && fresh
        && !state.unconfigured
        && (state.status.is_some() || state.busy && state.listener.is_open())
    {
        card = card
            .push(settings_divider())
            .push(super::config::listener_fields(i18n, &state.listener, status.map(|s| s.state)))
            .push(settings_divider())
            .push(super::config::client_host_field(i18n, &state.listener))
            .push(settings_divider())
            .push(super::config::logging_field(i18n, &state.listener, status.is_some()));
    } else if !inline && !state.unconfigured {
        card = card.push(
            container(
                row![
                    button(text(i18n.__("监听设置")))
                        .padding([8, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            connected.then_some(Event::Config(super::config::Event::Open(None)))
                        ),
                    button(text(i18n.__("连接地址")))
                        .padding([8, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            connected
                                .then_some(Event::Config(super::config::Event::OpenClientHost))
                        ),
                ]
                .spacing(8)
                .wrap(),
            )
            .padding(iced::padding::bottom(14)),
        );
    }
    settings_group_card(card).into()
}

pub fn restore_confirmation(i18n: desktop_i18n::I18n, state: &State, context: Option<Context>) -> Option<Element<'_, Event>> {
    use iced::{Alignment, Length};
    if context.is_none() || state.context != context || !state.confirming_restore {
        return None;
    }
    let header = row![
        text(i18n.__("恢复直连并停止")).size(17).width(Length::Fill),
        settings_close_button(Event::CancelRestore),
    ]
    .align_y(Alignment::Center);
    let actions = row![
        button(text(i18n.__("取消")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(Event::CancelRestore),
        button(text(i18n.__("恢复直连并停止")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press(Event::Run(Operation::StopWithRestore)),
    ]
    .spacing(8);
    Some(
        settings_modal_card(
            column![
                header,
                text(i18n.__("接入的应用将恢复到各自的直连供应商。全部恢复后，路由服务将停止。"))
                    .size(SETTINGS_CONTROL_TEXT_SIZE),
                actions,
            ]
            .spacing(16),
        )
        .max_width(460)
        .into(),
    )
}

pub fn attached(
    state: &State,
    context: Option<Context>,
) -> Option<(&[vw_api_app_identity::AppId], bool)> {
    if context.is_none() || state.context != context {
        return None;
    }
    state.attached.as_ref().map(|(apps, complete)| (apps.as_slice(), *complete))
}

pub const ROUTING_APPS: [vw_api_app_identity::AppId; 4] = {
    use vw_api_app_identity::AppId::*;
    [Claude, Codex, Gemini, GrokBuild]
};

async fn attached_apps(
    client: GatewayClient,
    scope: vw_api_app_aggregation::AggregationQuery,
) -> (Vec<vw_api_app_identity::AppId>, bool) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let [claude, codex, gemini, grok] = ROUTING_APPS;
        let read = |id| {
            let client = &client;
            let scope = &scope;
            async move {
                use gateway_aggregation::AggregationApiError;
                use vw_api_app_provider_environment::mode_state::{ModeStateQuery, ProviderMode};

                match client.aggregation_state(id, scope).await {
                    Ok(state) => return Some(state.attached),
                    Err(AggregationApiError::Http { status: 409, body })
                        if body["error"]["code"] == "mode_uninitialized" => {}
                    Err(AggregationApiError::Http { status: 422, body })
                        if body["error"]["code"] == "unsupported_app" => {}
                    Err(_) => return None,
                }
                if scope.target_host_id != vw_api_transport::LOCAL_HOST_ID {
                    return None;
                }

                let query = ModeStateQuery {
                    target_host_id: scope.target_host_id.clone(),
                    provider_id: None,
                    mode: ProviderMode::Direct,
                };
                client.provider_mode_state(id, &query, "").await.ok().map(|state| state.attached)
            }
        };
        let Ok(results) = tokio::time::timeout(std::time::Duration::from_secs(8), async {
            let (a, b, c, d) = tokio::join!(read(claude), read(codex), read(gemini), read(grok));
            [a, b, c, d]
        })
        .await
        else {
            return (Vec::new(), false);
        };
        let mut apps = Vec::new();
        let mut complete = true;
        for (id, result) in ROUTING_APPS.into_iter().zip(results) {
            match result {
                Some(true) => apps.push(id),
                Some(false) => {}
                None => complete = false,
            }
        }
        (apps, complete)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (client, scope);
        (Vec::new(), false)
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
