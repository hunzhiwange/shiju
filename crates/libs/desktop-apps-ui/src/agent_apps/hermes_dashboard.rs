//! Hermes owns deep configuration. Only approved local probe URLs reach the browser.
use crate::{AgentAppsContext, Connection, Notice, error as notify_error};
type Message = Event;
use desktop_ui_controls::system_settings_common::*;
use gateway_hermes_memory::GatewayHermesMemoryClient;
use iced::{
    Element, Task,
    widget::{button, column, row, text},
};
use vw_api_hermes_dashboard::*;
use vw_api_task::TaskStatus;

#[derive(Clone, Default)]
pub struct State {
    busy: bool,
    offline: bool,
    operation: Option<String>,
    progress: String,
    generation: u64,
    attempts: u32,
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
    Probe,
    Start,
    Cancel,
    Poll,
    OpenUrl(String),
    Reply(Scope, Result<(Option<DashboardOperation>, Option<DashboardProbe>), String>),
}
pub(crate) fn message(event: Event) -> Message {
    event
}
pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![
        text("Hermes Web 设置").size(15),
        text("模型、工具和定时任务等深层设置由 Hermes Web 面板提供。")
    ]
    .spacing(8);
    if !state.progress.is_empty() {
        content = content.push(text(&state.progress));
    }
    let mut actions = row![
        button(text(if state.operation.is_some() { "重试查询" } else { "打开 / 重试" }))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then(|| message(Event::Probe)))
    ]
    .spacing(8);
    if state.offline && state.operation.is_none() {
        content = content.push(text("面板离线。是否在执行主机启动 Hermes dashboard？"));
        actions = actions.push(
            button(text("确认启动"))
                .padding([8, 12])
                .style(primary_action_btn_style)
                .on_press_maybe((!state.busy).then(|| message(Event::Start))),
        );
    }
    if state.operation.is_some() {
        actions = actions.push(
            button(text("取消启动"))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(message(Event::Cancel)),
        );
    }
    content.push(actions).into()
}
fn approved_url(value: &str, local: bool) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    local
        && matches!(url.scheme(), "http" | "https")
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}
pub fn update(
    state: &mut State,
    connection: Connection,
    event: Event,
    notice: &mut Option<Notice>,
) -> Task<Message> {
    let Connection { client, context, catalog_context, project, host, generation, token } =
        connection;
    let local = client.endpoint().is_loopback() && host == local_app_auth::HOST_ID;
    if let Event::Reply(scope, result) = event {
        if scope.context != context
            || scope.host != host
            || scope.project != project
            || scope.catalog != generation
            || scope.generation != state.generation
        {
            return Task::none();
        }

        state.busy = false;
        match result {
            Err(error) => {
                state.progress = error.clone();
                return notify_error(notice, error);
            }
            Ok((operation, probe)) => {
                if let Some(op) = operation {
                    state.progress = format!("启动进度：{:?}", op.phase);
                    match op.status {
                        TaskStatus::Queued | TaskStatus::Running => {
                            state.operation = Some(op.operation_id.0.clone())
                        }
                        TaskStatus::Completed => state.operation = None,
                        TaskStatus::Cancelled => {
                            state.operation = None;
                            state.offline = false;
                            state.progress = "启动已取消".into();
                            return Task::none();
                        }
                        TaskStatus::Failed => {
                            state.operation = None;
                            state.offline = true;
                            state.progress = "启动失败，可重试".into();
                            return notify_error(notice, "Hermes 启动失败");
                        }
                    }
                }
                match probe {
                    Some(DashboardProbe::Online { url }) => {
                        if !approved_url(&url, local) {
                            return notify_error(notice, "面板地址不适合本机访问");
                        }
                        state.operation = None;
                        state.offline = false;
                        state.progress = "面板在线".into();
                        return Task::done(Event::OpenUrl(url));
                    }
                    Some(DashboardProbe::Offline) => {
                        state.offline = true;
                        state.progress = "面板尚未在线".into();
                    }
                    None => {}
                }
                if state.operation.is_some() && state.attempts < 60 {
                    state.attempts += 1;
                    return update(
                        state,
                        Connection {
                            client,
                            context,
                            catalog_context,
                            project,
                            host,
                            generation,
                            token,
                        },
                        Event::Poll,
                        notice,
                    );
                }
            }
        }
        return Task::none();
    }
    if catalog_context.as_ref() != Some(&context) || token.is_empty() {
        return notify_error(notice, "请先连接应用服务");
    }
    if !local {
        return notify_error(notice, "远程 Hermes 面板无受支持的本机访问地址");
    }

    if state.busy && !matches!(event, Event::Cancel) {
        return Task::none();
    }
    if matches!(event, Event::Start) && (!state.offline || state.operation.is_some()) {
        return Task::none();
    }
    if matches!(event, Event::Cancel) && state.operation.is_none() {
        return Task::none();
    }
    state.generation = state.generation.wrapping_add(1);
    state.busy = true;
    if matches!(event, Event::Start | Event::Probe) {
        state.attempts = 0;
    }
    let scope = Scope {
        context,
        host: host.clone(),
        project: project.to_string(),
        catalog: generation,
        generation: state.generation,
    };
    let client = GatewayHermesMemoryClient::new(client.transport().clone());
    let query = DashboardScope { project_id: project.into(), target_host_id: scope.host.clone() };
    let id = state.operation.clone();
    let token = token.clone();
    Task::perform(
        async move {
            let result = async {
                let operation = match event {
                    Event::Start => Some(client.hermes_dashboard_start(&query, &token).await),
                    Event::Cancel => Some(
                        client
                            .hermes_dashboard_cancel(
                                &query,
                                id.as_deref().unwrap_or_default(),
                                &token,
                            )
                            .await,
                    ),
                    _ => {
                        if matches!(event, Event::Poll) {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                        if let Some(id) = id {
                            Some(client.hermes_dashboard_operation(&query, &id, &token).await)
                        } else {
                            None
                        }
                    }
                }
                .transpose()
                .map_err(|_| "启动状态不可用，请重试查询或取消".to_string())?;
                // Preserve an accepted operation even if the subsequent probe fails.
                let probe = if matches!(event, Event::Cancel) {
                    None
                } else {
                    match client.hermes_dashboard_probe(&query, &token).await {
                        Ok(probe) => Some(probe),
                        Err(_) if operation.is_some() => None,
                        Err(_) => return Err("面板探测失败，请检查权限与连接".to_string()),
                    }
                };
                Ok((operation, probe))
            }
            .await;
            (scope, result)
        },
        |(scope, result)| message(Event::Reply(scope, result)),
    )
}
