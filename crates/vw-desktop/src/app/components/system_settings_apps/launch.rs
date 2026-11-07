//! Draft identities are projections only; all execution and credential lifetime stay on the host.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaunchChoice {
    ExistingConfiguration,
    Identity(LaunchIdentity),
}
impl std::fmt::Display for LaunchChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExistingConfiguration => f.write_str("主机现有 CLI 配置（可能使用已登录身份）"),
            Self::Identity(identity) => write!(
                f,
                "{}：{}",
                match identity.kind {
                    LaunchIdentityKind::Provider => "供应商",
                    LaunchIdentityKind::OfficialAccount => "官方账号",
                },
                identity.label
            ),
        }
    }
}
#[derive(Clone, Default)]
pub(crate) struct LaunchState {
    pub request: Option<LaunchRequest>,
    pub args: Vec<String>,
    pub identities: Vec<LaunchIdentity>,
    pub identities_loading: bool,
    pub identities_ready: bool,
    pub selected: Option<LaunchChoice>,
    pub session_id: Option<String>,
    pub phase: Option<LaunchSessionPhase>,
    pub busy: bool,
    pub generation: u64,
    pub error: Option<String>,
    pub accepted: bool,
}
impl LaunchState {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Default::default() };
    }
    pub fn blocked_reason(&self) -> Option<&'static str> {
        if self.busy {
            return Some("正在等待主机确认，请稍候。");
        }
        if self.accepted {
            return Some("已提交启动，请查看终端或上方会话状态。");
        }
        if self.error.is_some() {
            return Some("请先查看上方错误提示，关闭后重新打开。");
        }
        if self.identities_loading {
            return Some("正在读取启动身份，请稍候。");
        }
        if !self.identities_ready {
            return Some("启动身份尚未就绪，请关闭后重新打开。");
        }
        if self.selected.is_none() {
            return Some("请选择启动身份。");
        }
        let Some(request) = &self.request else {
            return Some("请先选择应用实例。");
        };
        if request.cwd.trim().is_empty() {
            return Some("请填写工作目录。");
        }
        None
    }
    pub fn begin(&mut self) -> Option<(u64, LaunchRequest)> {
        if self.blocked_reason().is_some() {
            return None;
        }
        let request = self.request.clone()?;
        self.busy = true;
        self.generation = self.generation.wrapping_add(1);
        Some((self.generation, request))
    }
    pub fn finish(&mut self, generation: u64, result: Result<LaunchResult, String>) -> bool {
        if !self.busy || self.generation != generation {
            return false;
        }
        self.busy = false;
        match result {
            Ok(value)
                if self
                    .request
                    .as_ref()
                    .is_some_and(|r| r.installation_id == value.installation_id) =>
            {
                self.accepted = true;
                self.error = None;
                self.session_id = value.session_id;
                self.phase = self.session_id.as_ref().map(|_| LaunchSessionPhase::Running);
            }
            Ok(_) => self.error = Some("执行主机返回了其他实例，启动未确认".into()),
            Err(error) => self.error = Some(error),
        }
        true
    }
}
impl std::fmt::Debug for LaunchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchState").field("busy", &self.busy).finish_non_exhaustive()
    }
}
#[derive(Clone)]
pub enum LaunchMessage {
    Open(AppId, AppInstallationId),
    Close,
    Cwd(String),
    Args(usize, String),
    AddArg,
    RemoveArg(usize),
    Select(LaunchChoice),
    Submit,
    Identities {
        scope: ReplyScope,
        result: Result<Vec<LaunchIdentity>, String>,
    },
    Session {
        scope: ReplyScope,
        id: String,
        result: Result<LaunchSessionStatus, String>,
    },
    Reply {
        context: AgentAppsContext,
        host: String,
        catalog_generation: u64,
        generation: u64,
        result: Result<LaunchResult, String>,
    },
}
#[derive(Clone)]
pub struct ReplyScope {
    pub context: AgentAppsContext,
    pub host: String,
    pub catalog_generation: u64,
    pub generation: u64,
}
impl ReplyScope {
    fn current(app: &App, context: AgentAppsContext) -> Self {
        Self {
            context,
            host: app.apps_catalog.host.clone(),
            catalog_generation: app.apps_catalog.generation,
            generation: app.apps_catalog.launch.generation,
        }
    }
    fn matches(&self, app: &App, current: &AgentAppsContext) -> bool {
        &self.context == current
            && app.apps_catalog.context.as_ref() == Some(current)
            && self.host == app.apps_catalog.host
            && self.catalog_generation == app.apps_catalog.generation
            && self.generation == app.apps_catalog.launch.generation
    }
}
fn event(value: LaunchMessage) -> Message {
    super::message(crate::app::message::settings::AppsCatalogMessage::Launch(value))
}
fn poll(
    client: vw_gateway_client::GatewayClient,
    scope: ReplyScope,
    id: String,
    token: String,
) -> Task<Message> {
    Task::perform(
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let result = client
                .agent_apps_launch_session(&scope.host, &id, &token)
                .await
                .map_err(|_| "会话状态暂不可用；请检查执行主机，不会重新启动 CLI".to_string());
            (scope, id, result)
        },
        |(scope, id, result)| event(LaunchMessage::Session { scope, id, result }),
    )
}
pub(crate) fn update(app: &mut App, message: LaunchMessage) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        app.apps_catalog.launch.invalidate();
        return app.show_error_toast("网关不可用，请重新加载应用");
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    if app.apps_catalog.context.as_ref() != Some(&context) {
        app.apps_catalog.launch.invalidate();
        return app.show_error_toast("连接上下文已更改，请重新加载应用");
    }
    match message {
        LaunchMessage::Identities { scope, result } => {
            if !scope.matches(app, &context) || !app.apps_catalog.launch.identities_loading {
                return Task::none();
            }
            let state = &mut app.apps_catalog.launch;
            state.identities_loading = false;
            match result {
                Ok(values) => {
                    state.identities = values;
                    state.identities_ready = true;
                }
                Err(error) => {
                    state.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
            return Task::none();
        }
        LaunchMessage::Session { scope, id, result } => {
            if !scope.matches(app, &context)
                || app.apps_catalog.launch.session_id.as_ref() != Some(&id)
            {
                return Task::none();
            }
            match result {
                Ok(status) => {
                    app.apps_catalog.launch.phase = Some(status.phase);
                    if matches!(
                        status.phase,
                        LaunchSessionPhase::Starting | LaunchSessionPhase::Running
                    ) {
                        return poll(client, scope, id, app.apps_catalog.token.clone());
                    }
                    return if status.phase == LaunchSessionPhase::Completed {
                        app.show_success_toast("CLI 已退出且执行端清理完成")
                    } else {
                        app.show_error_toast("CLI 会话失败或终端已关闭；请检查执行主机与账号状态")
                    };
                }
                Err(error) => {
                    app.apps_catalog.launch.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
        }
        LaunchMessage::Reply {
            context: requested,
            host,
            catalog_generation,
            generation,
            result,
        } => {
            let scope = ReplyScope { context: requested, host, catalog_generation, generation };
            if !scope.matches(app, &context) {
                return Task::none();
            }
            if app.apps_catalog.launch.finish(generation, result) {
                if let Some(error) = app.apps_catalog.launch.error.clone() {
                    return app.show_error_toast(error);
                }
                let toast = app.show_success_toast("执行主机已接受终端启动；尚未确认 CLI 完成");
                if let Some(id) = app.apps_catalog.launch.session_id.clone() {
                    return Task::batch([
                        toast,
                        poll(client, scope, id, app.apps_catalog.token.clone()),
                    ]);
                }
                return toast;
            }
            return Task::none();
        }
        _ => {}
    }
    if app.apps_catalog.launch.busy {
        return Task::none();
    }
    match message {
        LaunchMessage::Open(id, installation_id) => {
            let catalog = &mut app.apps_catalog;
            if catalog.loading || catalog.token.is_empty() {
                return Task::none();
            }
            let valid = catalog
                .rows
                .iter()
                .filter(|r| r.entry.id == id)
                .filter_map(|r| r.observation.as_ref().ok())
                .flat_map(|o| &o.installations)
                .any(|i| i.id == installation_id && i.target_host_id == catalog.host);
            if !valid {
                return app.show_error_toast("所选实例已失效，请刷新应用");
            }
            catalog.launch.invalidate();
            let cwd = super::local::launch_directory(
                &client,
                &catalog.host,
                &catalog.token,
                app.project_path.as_deref(),
            );
            catalog.launch.selected = cwd.as_ref().map(|_| LaunchChoice::ExistingConfiguration);
            catalog.launch.request = Some(LaunchRequest {
                app_id: id,
                installation_id,
                cwd: cwd.unwrap_or_default(),
                args: vec![],
                skip_permissions: false,
            });
            catalog.launch.identities_loading = true;
            let scope = ReplyScope::current(app, context);
            let host = scope.host.clone();
            let token = app.apps_catalog.token.clone();
            return Task::perform(
                async move {
                    client
                        .agent_apps_launch_identities(&host, id, &token)
                        .await
                        .map_err(|_| "身份列表读取失败，启动已禁用；请关闭后重试".to_string())
                },
                move |result| event(LaunchMessage::Identities { scope: scope.clone(), result }),
            );
        }
        LaunchMessage::Close => app.apps_catalog.launch.invalidate(),
        LaunchMessage::Select(choice) => {
            let state = &mut app.apps_catalog.launch;
            if !state.accepted
                && state.identities_ready
                && match &choice {
                    LaunchChoice::ExistingConfiguration => true,
                    LaunchChoice::Identity(value) => state.identities.contains(value),
                }
            {
                state.selected = Some(choice);
            }
        }
        LaunchMessage::Cwd(value) => {
            if !app.apps_catalog.launch.accepted
                && let Some(r) = &mut app.apps_catalog.launch.request
            {
                r.cwd = value;
            }
        }
        LaunchMessage::Args(index, value) => {
            if !app.apps_catalog.launch.accepted
                && let Some(arg) = app.apps_catalog.launch.args.get_mut(index)
            {
                *arg = value;
            }
        }
        LaunchMessage::AddArg => {
            if !app.apps_catalog.launch.accepted {
                app.apps_catalog.launch.args.push(String::new());
            }
        }
        LaunchMessage::RemoveArg(index) => {
            let state = &mut app.apps_catalog.launch;
            if !state.accepted && index < state.args.len() {
                state.args.remove(index);
            }
        }
        LaunchMessage::Submit => {
            let catalog = &mut app.apps_catalog;
            if catalog.launch.error.is_some() || catalog.host.is_empty() || catalog.token.is_empty()
            {
                return Task::none();
            }
            if let Some(r) = &mut catalog.launch.request {
                r.args = catalog.launch.args.clone();
            }
            let Some((generation, request)) = catalog.launch.begin() else {
                return Task::none();
            };
            let choice = catalog.launch.selected.clone().unwrap();
            let host = catalog.host.clone();
            let token = catalog.token.clone();
            let catalog_generation = catalog.generation;
            return Task::perform(
                async move {
                    let result = match choice {
                        LaunchChoice::ExistingConfiguration => {
                            client.agent_apps_launch_authorized(&host, &request, &token).await
                        }
                        LaunchChoice::Identity(identity) => {
                            client
                                .agent_apps_launch_identity_authorized(
                                    &host, &request, &identity, &token,
                                )
                                .await
                        }
                    }
                    .map_err(|e| {
                        match e {
                            vw_gateway_client::AgentAppsSettingsError::Http {
                                status: 401 | 403,
                                ..
                            } => "启动未授权；未回退其他主机或账号",
                            vw_gateway_client::AgentAppsSettingsError::Http {
                                status: 422, ..
                            } => "执行主机拒绝目录、身份、参数或实例；请关闭后刷新并重新选择",
                            vw_gateway_client::AgentAppsSettingsError::Http {
                                status: 501, ..
                            } => "执行主机尚不支持此身份的启动方式",
                            _ => "未确认启动结果；不会自动重试，请先检查执行主机上的终端",
                        }
                        .to_string()
                    });
                    (host, result)
                },
                move |(host, result)| {
                    event(LaunchMessage::Reply {
                        context: context.clone(),
                        host,
                        catalog_generation,
                        generation,
                        result,
                    })
                },
            );
        }
        LaunchMessage::Identities { .. }
        | LaunchMessage::Session { .. }
        | LaunchMessage::Reply { .. } => unreachable!(),
    }
    Task::none()
}
#[path = "launch_view.rs"]
mod view;
pub(crate) use view::overlay;

#[cfg(test)]
#[path = "launch_ui_refresh_07_tests.rs"]
mod launch_ui_refresh_07_tests;
