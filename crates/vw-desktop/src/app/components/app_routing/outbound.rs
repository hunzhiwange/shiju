//! Host-owned egress configuration and bounded diagnostic jobs.
use super::lifecycle::{self, Context};
use crate::app::{App, Message, components::system_settings_common::*, state::AppsCatalogState};
use iced::{
    Element, Task,
    widget::{button, column, container, row, text},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticJobSnapshot, DiagnosticJobState, DiagnosticOutcome, DiagnosticResult,
    },
    app_routing::{RoutingEgressMode, RoutingEgressStatus, RoutingEgressUpdate},
    id::ProjectId,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    serial: u64,
    context: Option<Context>,
    project: Option<String>,
    app: Option<AppId>,
    mode: Option<RoutingEgressMode>,
    secret: Secret,
    status: Option<RoutingEgressStatus>,
    cancel: Option<Arc<AtomicBool>>,
    progress: String,
    error: Option<String>,
}
// Message and state debugging must never print a write-only proxy URL.
#[derive(Clone, Default)]
pub(crate) struct Secret(String);
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open(AppId),
    Close,
    Mode(RoutingEgressMode),
    Edit(Secret),
    Save,
    Reload,
    Start(bool),
    Cancel,
    Loaded(Context, u64, Result<RoutingEgressStatus, String>),
    Progress(Context, u64, bool, Result<DiagnosticJobSnapshot, String>),
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::Routing(lifecycle::Event::Outbound(
            event,
        )),
    ))
}
impl State {
    pub(super) fn invalidate(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        *self = Self { serial: self.serial.wrapping_add(1), ..Default::default() };
    }
    fn draft(&self) -> Result<RoutingEgressUpdate, String> {
        let mode = self.mode.ok_or("请先读取出口设置")?;
        if mode == RoutingEgressMode::Custom && self.secret.0.trim().is_empty() {
            return Err("自定义出口请重新输入完整代理地址".into());
        }
        Ok(RoutingEgressUpdate {
            mode,
            proxy_url: (mode == RoutingEgressMode::Custom).then(|| self.secret.0.trim().to_owned()),
        })
    }
}
fn terminal(state: DiagnosticJobState) -> bool {
    !matches!(
        state,
        DiagnosticJobState::Queued {}
            | DiagnosticJobState::Running {}
            | DiagnosticJobState::Cancelling {}
    )
}
fn progress(snapshot: &DiagnosticJobSnapshot) -> String {
    let label = match snapshot.state {
        DiagnosticJobState::Queued {} => "排队中",
        DiagnosticJobState::Running {} => "检测中",
        DiagnosticJobState::Cancelling {} => "取消中",
        DiagnosticJobState::Cancelled {} => "已取消",
        DiagnosticJobState::Succeeded {} => "检测完成",
        DiagnosticJobState::Failed { .. } => "检测失败",
        DiagnosticJobState::Interrupted {} => "检测中断",
    };
    let mut label = format!("{label}：已处理 {} 项", snapshot.progress.completed);
    for result in snapshot.results.iter().flatten() {
        match result {
            DiagnosticResult::OutboundScan { report } => {
                if let DiagnosticOutcome::Succeeded { evidence } = &report.outcome {
                    if evidence.tcp_open {
                        label.push_str(&format!("；候选端口 {}（仅 TCP 可达）", evidence.port));
                    }
                }
            }
            DiagnosticResult::OutboundProxy { report } => {
                if let DiagnosticOutcome::Succeeded { evidence } = &report.outcome {
                    label.push_str(&format!("；目标 HTTP {}", evidence.http_status));
                }
            }
            _ => {}
        }
    }
    label
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(context) = lifecycle::current(app) else {
        return Task::none();
    };
    let project = app.project_id.clone();
    let state = &mut app.apps_catalog.routing.outbound;
    match event {
        Event::Close => {
            if !state.busy {
                state.invalidate();
            }
            return Task::none();
        }
        Event::Cancel => {
            if let Some(cancel) = &state.cancel {
                cancel.store(true, Ordering::Relaxed);
                state.progress = "正在请求取消并等待主机释放资源".into();
            }
            return Task::none();
        }
        Event::Open(id) => {
            if state.busy {
                return Task::none();
            }
            state.invalidate();
            state.open = true;
            state.context = Some(context.clone());
            state.project = project.clone();
            state.app = Some(id);
        }
        Event::Loaded(request, serial, result) => {
            if request != context
                || state.context.as_ref() != Some(&request)
                || state.serial != serial
                || !state.busy
            {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(status) => {
                    state.mode = Some(status.configured_mode);
                    state.status = Some(status);
                    state.secret.0.clear();
                }
                Err(error) => {
                    state.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
            return Task::none();
        }
        Event::Progress(request, serial, done, result) => {
            if request != context
                || state.context.as_ref() != Some(&request)
                || state.serial != serial
                || !state.busy
            {
                return Task::none();
            }
            if done {
                state.busy = false;
                state.cancel = None;
            }
            match result {
                Ok(snapshot) => state.progress = progress(&snapshot),
                Err(error) => {
                    state.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
            return Task::none();
        }
        _ => {}
    }
    if !state.open
        || state.busy
        || state.context.as_ref() != Some(&context)
        || state.project != project
    {
        return Task::none();
    }
    match event {
        Event::Mode(mode) => {
            state.mode = Some(mode);
            state.secret.0.clear();
            return Task::none();
        }
        Event::Edit(value) => {
            state.secret = value;
            return Task::none();
        }
        _ => {}
    }
    let draft = if matches!(event, Event::Save) {
        match state.draft() {
            Ok(draft) => Some(draft),
            Err(error) => {
                state.error = Some(error);
                return Task::none();
            }
        }
    } else {
        None
    };
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("无法连接网关");
    };
    let token = app.apps_catalog.token.clone();
    state.busy = true;
    state.error = None;
    state.serial = state.serial.wrapping_add(1);
    let serial = state.serial;
    #[cfg(target_arch = "wasm32")]
    if matches!(event, Event::Start(_)) {
        state.busy = false;
        return app.show_error_toast("此平台不支持出口检测");
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Event::Start(scan) = event {
        let Some(project) = project.filter(|p| !p.is_empty()) else {
            state.busy = false;
            return app.show_error_toast("请先选择项目");
        };
        let Some(id) = state.app else {
            state.busy = false;
            return Task::none();
        };
        let cancel = Arc::new(AtomicBool::new(false));
        state.cancel = Some(cancel.clone());
        state.progress = "正在启动检测（使用已保存出口）".into();
        return Task::run(
            iced::stream::channel(8, async move |mut output| {
                use iced::futures::SinkExt;
                let project: ProjectId = project.into();
                let run = async {
                    let job = bounded(async {
                        if scan {
                            client
                                .app_routing_start_outbound_scan(
                                    &context.host,
                                    &project,
                                    id,
                                    &token,
                                )
                                .await
                        } else {
                            client
                                .app_routing_start_outbound_probe(
                                    &context.host,
                                    &project,
                                    id,
                                    &token,
                                )
                                .await
                        }
                    })
                    .await?;
                    let outcome = async {
                        let mut cancelling = false;
                        // Server probes have their own deadlines; never leave the desktop loop unbounded.
                        for _ in 0..120 {
                            let request = async {
                                if cancel.load(Ordering::Relaxed) || cancelling {
                                    cancelling = true;
                                    if scan {
                                        client
                                            .app_routing_cancel_outbound_scan(
                                                &context.host,
                                                &project,
                                                id,
                                                &job.task_id,
                                                &token,
                                            )
                                            .await
                                    } else {
                                        client
                                            .app_routing_cancel_outbound_probe(
                                                &context.host,
                                                &project,
                                                id,
                                                &job.task_id,
                                                &token,
                                            )
                                            .await
                                    }
                                } else if scan {
                                    client
                                        .app_routing_outbound_scan_status(
                                            &context.host,
                                            &project,
                                            id,
                                            &job.task_id,
                                            &token,
                                        )
                                        .await
                                } else {
                                    client
                                        .app_routing_outbound_probe_status(
                                            &context.host,
                                            &project,
                                            id,
                                            &job.task_id,
                                            &token,
                                        )
                                        .await
                                }
                            };
                            let snapshot = bounded(request).await?;
                            if snapshot.job != job {
                                return Err("检测身份不匹配".into());
                            }
                            let done = terminal(snapshot.state);
                            if output
                                .send(Event::Progress(context.clone(), serial, done, Ok(snapshot)))
                                .await
                                .is_err()
                            {
                                cancel.store(true, Ordering::Relaxed);
                            }
                            if done {
                                return Ok(());
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        Err("检测状态超时，请重新读取确认".to_string())
                    }
                    .await;
                    if outcome.is_err() {
                        if scan {
                            let _ = bounded(client.app_routing_cancel_outbound_scan(
                                &context.host,
                                &project,
                                id,
                                &job.task_id,
                                &token,
                            ))
                            .await;
                        } else {
                            let _ = bounded(client.app_routing_cancel_outbound_probe(
                                &context.host,
                                &project,
                                id,
                                &job.task_id,
                                &token,
                            ))
                            .await;
                        }
                    }
                    outcome
                };
                let result = run.await;
                if let Err(error) = result {
                    let _ = output.send(Event::Progress(context, serial, true, Err(error))).await;
                }
            }),
            message,
        );
    }
    Task::perform(
        async move {
            let result = bounded(async {
                match draft {
                    Some(draft) => {
                        client.app_routing_set_egress(&context.host, &token, &draft).await
                    }
                    None => client.app_routing_egress(&context.host, &token).await,
                }
            })
            .await;
            Event::Loaded(context, serial, result)
        },
        message,
    )
}
pub(crate) fn entry(id: AppId) -> Element<'static, Message> {
    button(text("出口设置与检测"))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(message(Event::Open(id)))
        .into()
}
fn mode_label(mode: RoutingEgressMode) -> &'static str {
    match mode {
        RoutingEgressMode::Direct => "直连",
        RoutingEgressMode::System => "系统代理",
        RoutingEgressMode::Custom => "自定义代理",
    }
}
pub(crate) fn overlay<'a>(
    catalog: &'a AppsCatalogState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &catalog.routing.outbound;
    if !state.open {
        return base;
    }
    let header = settings_modal_header(
        text("执行主机出口设置").size(20),
        settings_close_button(message(Event::Close)),
    );
    let mut body = column![text("出口属于执行主机，影响该主机的常驻路由。检测使用已保存配置；扫描仅检查固定回环端口，不自动保存候选。")].spacing(12);
    if let Some(status) = &state.status {
        body = body.push(text(format!(
            "已保存：{}；实际：{}；包含凭据：{}",
            mode_label(status.configured_mode),
            mode_label(status.effective_mode),
            if status.has_credentials { "是" } else { "否" }
        )));
        let mut modes = row![].spacing(8);
        for mode in
            [RoutingEgressMode::Direct, RoutingEgressMode::System, RoutingEgressMode::Custom]
        {
            modes = modes.push(
                button(text(mode_label(mode)))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then_some(message(Event::Mode(mode)))),
            );
        }
        body = body.push(modes).push(text(format!(
            "草稿模式：{}",
            mode_label(state.mode.unwrap_or(status.configured_mode))
        )));
        if state.mode == Some(RoutingEgressMode::Custom) {
            let mut input =
                settings_text_input("完整代理 URL（仅写入）", &state.secret.0).secure(true);
            if !state.busy {
                input = input.on_input(|value| message(Event::Edit(Secret(value))));
            }
            body = body.push(input);
        }
    }
    body = body.push(text(&state.progress));
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    let mut actions = row![].spacing(8);
    for (label, event, enabled) in [
        ("关闭", Event::Close, !state.busy),
        ("重新读取", Event::Reload, !state.busy),
        ("保存", Event::Save, !state.busy && state.status.is_some()),
        ("测试已保存出口", Event::Start(false), !state.busy && state.status.is_some()),
        ("有限扫描", Event::Start(true), !state.busy),
        ("取消检测", Event::Cancel, state.cancel.is_some()),
    ] {
        let style = if matches!(event, Event::Save) {
            primary_action_btn_style
        } else {
            rounded_action_btn_style
        };
        actions = actions.push(
            button(text(label))
                .padding([8, 12])
                .style(style)
                .on_press_maybe(enabled.then_some(message(event))),
        );
    }
    let card =
        settings_modal_layout(header, body, Some(actions.wrap().into())).width(760).height(520);
    settings_modal_overlay(Some(base), message(Event::Close), container(card).padding(12))
}

async fn bounded<T>(
    request: impl std::future::Future<Output = Result<T, vw_gateway_client::AppRoutingError>>,
) -> Result<T, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::timeout(std::time::Duration::from_secs(15), request)
            .await
            .map_err(|_| "出口请求超时，结果未知，请重新读取确认".to_string())?
            .map_err(lifecycle::error_message)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = request;
        Err("此平台不支持出口设置".into())
    }
}
