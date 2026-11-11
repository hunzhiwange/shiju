//! Managed catalog reads stay on the selected gateway; native observations are not runtime mounts.
use super::{McpMessage, SettingsMessage};
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::{AgentAppsReadError, GatewayClient, vw_api_types::mcp_management::*};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub(crate) endpoint: AgentAppsContext,
    pub(crate) host: String,
    pub(crate) credentials_generation: u64,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct CatalogState {
    context: Option<Context>,
    generation: u64,
    pub(crate) loading: bool,
    pub(crate) result: Option<Result<McpCatalogResponse, String>>,
}
#[derive(Debug, Clone)]
pub enum CatalogMessage {
    Refresh,
    Loaded { context: Context, generation: u64, result: Result<McpCatalogResponse, String> },
}
pub(crate) fn context(app: &App) -> Option<Context> {
    let client = crate::app::config::gateway_client().ok()?;
    Some(Context {
        endpoint: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: app.project_path.clone(),
        },
        host: app.apps_catalog.host.clone(),
        credentials_generation: app.apps_catalog.generation,
    })
}
pub(crate) async fn load(
    client: GatewayClient,
    host: &str,
    token: &str,
) -> Result<McpCatalogResponse, AgentAppsReadError> {
    client.mcp_management_catalog(host, token).await
}
impl CatalogState {
    fn finish(
        &mut self,
        current: &Context,
        request: &Context,
        generation: u64,
        result: Result<McpCatalogResponse, String>,
    ) -> bool {
        if current != request {
            return false;
        }
        if self.context.as_ref() == Some(request) && self.generation == generation {
            self.loading = false;
            self.result = Some(result);
            return true;
        }
        false
    }
}
pub(crate) fn loading(app: &App) -> bool {
    app.mcp_settings.catalog.loading
        && context(app).as_ref() == app.mcp_settings.catalog.context.as_ref()
}
pub(crate) fn visible(app: &App) -> Option<&Result<McpCatalogResponse, String>> {
    (context(app).as_ref() == app.mcp_settings.catalog.context.as_ref())
        .then_some(app.mcp_settings.catalog.result.as_ref())
        .flatten()
}
pub(crate) fn update(app: &mut App, message: CatalogMessage) -> Task<Message> {
    match message {
        CatalogMessage::Refresh => {
            let Some(context) = context(app) else {
                return app.show_error_toast("网关不可用");
            };
            let state = &mut app.mcp_settings.catalog;
            state.generation = state.generation.wrapping_add(1);
            state.context = Some(context.clone());
            state.result = None;
            state.loading = false;
            if context.host.is_empty()
                || app.apps_catalog.token.is_empty()
                || app.apps_catalog.context.as_ref() != Some(&context.endpoint)
            {
                state.result = Some(Err("请先在应用管理中连接目标主机".into()));
                return Task::none();
            }
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(error) => return app.show_error_toast(error),
            };
            let generation = state.generation;
            state.loading = true;
            let token = app.apps_catalog.token.clone();
            Task::perform(
                async move {
                    let result = load(client, &context.host, &token).await.map_err(|error| {
                        crate::app::components::system_settings_apps::catalog::error_label(&error)
                            .to_owned()
                    });
                    CatalogMessage::Loaded { context, generation, result }
                },
                |message| Message::Settings(SettingsMessage::Mcp(McpMessage::Catalog(message))),
            )
        }
        CatalogMessage::Loaded { context: request, generation, result } => {
            if let Some(current) = context(app) {
                let accepted =
                    app.mcp_settings.catalog.finish(&current, &request, generation, result);
                if accepted && let Some(Err(error)) = visible(app) {
                    return app.show_error_toast(error.clone());
                }
            }
            Task::none()
        }
    }
}
pub(crate) fn rows(catalog: &McpCatalogResponse) -> Vec<String> {
    let mut rows = Vec::new();
    for record in &catalog.records {
        rows.push(format!(
            "{} · {:?} · 修订 {}",
            record.name, record.transport, record.record.revision
        ));
        let mut bound = false;
        for binding in
            catalog.bindings.iter().filter(|binding| binding.record.id == record.record.id)
        {
            bound = true;
            let status = match binding.native_status {
                McpNativeStatus::Present => "存在",
                McpNativeStatus::Missing => "缺失",
                McpNativeStatus::Unreadable => "无法读取",
                McpNativeStatus::NotVerified => "未验证",
                McpNativeStatus::Unsupported => "不支持",
            };
            let desired = if binding.enabled { "期望启用" } else { "期望禁用" };
            let revision = if binding.record.revision == record.record.revision {
                "当前修订"
            } else {
                "旧修订"
            };
            rows.push(format!(
                "原生应用 {:?} · {} · {} · {} · {}（上次观察）",
                binding.target.app_id, binding.target.installation_id.0, desired, revision, status
            ));
        }
        if !bound {
            rows.push("原生应用：未绑定".into());
        }
        rows.push("识矩运行时：项目选择状态未提供；ACP：会话挂载状态未提供".into());
    }
    for capability in &catalog.capabilities {
        let label = |support| match support {
            McpSupport::Supported => "支持",
            McpSupport::Unsupported => "不支持",
            McpSupport::NotVerified => "未验证",
        };
        rows.push(format!(
            "原生应用 {:?} 能力 · stdio {} · HTTP {} · SSE {}",
            capability.app_id,
            label(capability.stdio),
            label(capability.streamable_http),
            label(capability.sse)
        ));
    }
    rows
}

#[cfg(test)]
#[path = "mcp_management_list_tests.rs"]
mod tests;
