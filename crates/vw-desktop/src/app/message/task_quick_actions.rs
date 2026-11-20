use crate::app::components::system_settings::task_settings::quick_actions as settings;
use crate::app::views::task_board::modals::quick_actions as board;
use crate::app::{App, Message, state::ToastKind};
use iced::Task;
use std::io;
use std::path::Path;
use task_file_store::task_quick_actions::TaskQuickAction;
use vw_api_project::ProjectDto;
use vw_api_transport::TenantScope;
use vw_gateway_client::GatewayClient;

#[derive(Default)]
pub(crate) struct State {
    pub settings: settings::State,
    pub board: board::State,
}

#[derive(Clone, Debug)]
pub enum Event {
    Settings(settings::Event),
    Board(board::Event),
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Settings(event) => settings::update(app, event),
        Event::Board(event) => board::update(app, event),
    }
}

pub(crate) fn warning(app: &mut App, message: &'static str) -> Task<Message> {
    app.show_localized_toast(ToastKind::Warning, message, vec![])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Origin {
    pub path: String,
    generation: u64,
    gateway: String,
    tenant: TenantScope,
}

impl Origin {
    pub fn new(app: &App, path: &str) -> Self {
        let endpoint = crate::app::config::gateway_client_endpoint();
        Self {
            path: path.to_owned(),
            generation: app.session_state.session_view_cache.navigation_generation,
            gateway: endpoint.base_url(),
            tenant: endpoint.tenant_scope().clone(),
        }
    }

    pub fn visible_in(&self, app: &App, path: Option<&str>) -> bool {
        path == Some(self.path.as_str())
            && self.generation == app.session_state.session_view_cache.navigation_generation
    }

    pub fn matches(&self, app: &App, path: Option<&str>) -> bool {
        self.visible_in(app, path) && *self == Self::new(app, &self.path)
    }

    pub async fn resolve(&self) -> Result<(GatewayClient, ProjectDto), &'static str> {
        const UNAVAILABLE: &str = "无法访问当前项目";
        let client = crate::app::config::gateway_client().map_err(|_| UNAVAILABLE)?;
        let endpoint = client.endpoint();
        if endpoint.base_url() != self.gateway || endpoint.tenant_scope() != &self.tenant {
            return Err(UNAVAILABLE);
        }
        let response = gateway_project::GatewayProjectClient::new(client.transport().clone())
            .project_resolve(
                &self.tenant,
                &vw_api_project::ResolveProjectRequest {
                    directory: self.path.clone(),
                    create_if_missing: false,
                },
            )
            .await
            .map_err(|_| UNAVAILABLE)?;
        Ok((client, response.project))
    }

    pub async fn catalog(
        &self,
        project: &ProjectDto,
        operation: impl FnOnce(&Path) -> io::Result<Vec<TaskQuickAction>> + Send + 'static,
    ) -> Result<Vec<TaskQuickAction>, &'static str> {
        let path = self.path.clone();
        let resolved = project.directory.clone();
        tokio::task::spawn_blocking(move || {
            let path = Path::new(&path);
            let resolved = Path::new(&resolved);
            if !path.is_absolute()
                || !resolved.is_absolute()
                || path.canonicalize().ok().zip(resolved.canonicalize().ok())
                    .is_none_or(|(path, resolved)| path != resolved)
            {
                return Err("项目的本地目录不可用");
            }
            operation(resolved).map_err(|_| "无法读写项目快捷操作，请重新加载后重试")
        })
        .await
        .map_err(|_| "无法读写项目快捷操作，请重新加载后重试")?
    }
}
