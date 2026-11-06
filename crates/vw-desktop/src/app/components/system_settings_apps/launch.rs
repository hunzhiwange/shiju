//! Launch drafts contain no credentials and never invoke a desktop terminal.
use vw_gateway_client::vw_api_types::agent_apps::*;

#[derive(Clone, Default)]
pub(crate) struct LaunchState {
    pub request: Option<LaunchRequest>,
    pub args: Vec<String>,
    pub busy: bool,
    pub generation: u64,
    pub error: Option<String>,
    pub accepted: bool,
}
impl LaunchState {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Default::default() };
    }
    pub fn begin(&mut self) -> Option<(u64, LaunchRequest)> {
        if self.busy || self.accepted {
            return None;
        }
        let request = self.request.clone()?;
        if request.cwd.trim().is_empty() {
            return None;
        }
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
    Submit,
    Reply {
        context: crate::app::state::AgentAppsContext,
        host: String,
        catalog_generation: u64,
        generation: u64,
        result: Result<LaunchResult, String>,
    },
}
fn event(value: LaunchMessage) -> crate::app::Message {
    super::message(crate::app::message::settings::AppsCatalogMessage::Launch(value))
}
pub(crate) fn update(
    app: &mut crate::app::App,
    message: LaunchMessage,
) -> iced::Task<crate::app::Message> {
    use iced::Task;
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用");
    };
    let context = crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    if app.apps_catalog.context.as_ref() != Some(&context) {
        app.apps_catalog.launch.invalidate();
        return app.show_error_toast("连接上下文已更改，请重新加载应用");
    }
    let catalog = &mut app.apps_catalog;
    if let LaunchMessage::Reply {
        context: requested,
        host,
        catalog_generation,
        generation,
        result,
    } = message
    {
        if requested != context || host != catalog.host || catalog_generation != catalog.generation
        {
            return Task::none();
        }
        if catalog.launch.finish(generation, result) {
            if let Some(error) = catalog.launch.error.clone() {
                return app.show_error_toast(error);
            }
            return app.show_success_toast("执行主机已接受终端启动");
        }
        return Task::none();
    }
    if catalog.launch.busy {
        return Task::none();
    }
    match message {
        LaunchMessage::Open(id, installation_id) => {
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
            catalog.launch.request = Some(LaunchRequest {
                app_id: id,
                installation_id,
                cwd: String::new(),
                args: vec![],
                skip_permissions: false,
            });
        }
        LaunchMessage::Close => catalog.launch.invalidate(),
        LaunchMessage::Cwd(value) => {
            if !catalog.launch.accepted
                && let Some(r) = &mut catalog.launch.request
            {
                r.cwd = value;
            }
        }
        LaunchMessage::Args(index, value) => {
            if !catalog.launch.accepted {
                if let Some(arg) = catalog.launch.args.get_mut(index) {
                    *arg = value;
                }
            }
        }
        LaunchMessage::AddArg => {
            if !catalog.launch.accepted {
                catalog.launch.args.push(String::new());
            }
        }
        LaunchMessage::RemoveArg(index) => {
            if !catalog.launch.accepted && index < catalog.launch.args.len() {
                catalog.launch.args.remove(index);
            }
        }
        LaunchMessage::Submit => {
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
            let host = catalog.host.clone();
            let token = catalog.token.clone();
            let catalog_generation = catalog.generation;
            return Task::perform(
                async move {
                    let result = client
                        .agent_apps_launch_authorized(&host, &request, &token)
                        .await
                        .map_err(|e| {
                            match e {
                                vw_gateway_client::AgentAppsSettingsError::Http {
                                    status: 401 | 403,
                                    ..
                                } => "启动未授权；未回退其他主机或账号",
                                vw_gateway_client::AgentAppsSettingsError::Http {
                                    status: 422,
                                    ..
                                } => "执行主机拒绝目录、参数或实例；请关闭后刷新并重新选择",
                                vw_gateway_client::AgentAppsSettingsError::Http {
                                    status: 501,
                                    ..
                                } => "执行主机尚不支持此启动方式",
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
        LaunchMessage::Reply { .. } => unreachable!(),
    }
    Task::none()
}
#[path = "launch_view.rs"]
mod view;
pub(crate) use view::overlay;
