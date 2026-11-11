//! Native bindings use trusted installation identities; runtime and ACP stay independent.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{
    GatewayClient,
    mcp_management_distribution::{McpBindingRequest, McpBulkRequest},
    vw_api_types::{ProjectId, agent_apps::AppId, mcp_management::*},
};

pub(crate) const APPS: [AppId; 14] = [
    AppId::Claude,
    AppId::ClaudeDesktop,
    AppId::Codex,
    AppId::Gemini,
    AppId::GrokBuild,
    AppId::OpenCode,
    AppId::OpenClaw,
    AppId::Hermes,
    AppId::Pi,
    AppId::Mcode,
    AppId::Kimi,
    AppId::Dsh,
    AppId::Agy,
    AppId::Omp,
];
pub(crate) fn unsupported_reason(app: AppId) -> Option<&'static str> {
    match app {
        AppId::ClaudeDesktop | AppId::OpenClaw => Some("上游不支持原生分发"),
        _ => None,
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) struct BindingsState {
    pub(crate) context: Option<catalog::Context>,
    pub(crate) selected: Vec<McpCatalogRecord>,
    pub(crate) running: bool,
    generation: u64,
    pub(crate) result: Option<Result<McpDistributionResult, String>>,
}
#[derive(Debug, Clone)]
pub enum BindingsMessage {
    Select(McpCatalogRecord, bool),
    Apply(McpAppTarget, bool),
    Finished {
        context: catalog::Context,
        generation: u64,
        result: Result<McpDistributionResult, String>,
    },
}
pub(crate) fn message(value: BindingsMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Bindings(value)))
}
pub(crate) fn visible(app: &App) -> Option<&BindingsState> {
    let state = &app.mcp_settings.bindings;
    (catalog::context(app).as_ref() == state.context.as_ref()).then_some(state)
}
pub(crate) fn project(records: &[McpCatalogRecord]) -> Result<Option<ProjectId>, String> {
    let first = records.first().ok_or("请先选择目录项")?;
    if records.len() > 256 || records.iter().any(|record| record.scope != first.scope) {
        return Err("一次最多选择256项，且必须属于同一作用域".into());
    }
    Ok(match &first.scope {
        McpCatalogScope::Tenant { .. } => None,
        McpCatalogScope::Project { scope } => Some(scope.project_id().clone()),
    })
}
pub(crate) fn supported(
    catalog: &McpCatalogResponse,
    records: &[McpCatalogRecord],
    app: AppId,
) -> bool {
    unsupported_reason(app).is_none()
        && !records.is_empty()
        && records.iter().all(|record| {
            catalog.capabilities.iter().any(|cap| {
                cap.app_id == app
                    && match record.transport {
                        McpTransport::Stdio => cap.stdio,
                        McpTransport::StreamableHttp => cap.streamable_http,
                        McpTransport::Sse => cap.sse,
                    } == McpSupport::Supported
            })
        })
}
pub(crate) async fn apply(
    client: GatewayClient,
    records: Vec<McpCatalogRecord>,
    target: McpAppTarget,
    enabled: bool,
    token: String,
) -> Result<McpDistributionResult, String> {
    let project_id = project(&records)?;
    let response = if records.len() == 1 {
        client
            .mcp_management_bind(
                &records[0].record.id,
                &McpBindingRequest {
                    target: target.clone(),
                    project_id,
                    expected_revision: records[0].record.revision,
                    enabled,
                },
                &token,
            )
            .await
    } else {
        client
            .mcp_management_bulk(
                &McpBulkRequest {
                    target: target.clone(),
                    project_id,
                    records: records.iter().map(|r| r.record.clone()).collect(),
                    enabled,
                },
                &token,
            )
            .await
    }
    .map_err(|_| "绑定请求失败；请刷新目录核实期望与原生状态".to_owned())?;
    // Do not call an incomplete, duplicate or foreign response successful.
    if response.items.len() != records.len()
        || records.iter().any(|record| {
            response
                .items
                .iter()
                .filter(|item| {
                    item.record.id == record.record.id
                        && item.record.revision >= record.record.revision
                        && item.target == target
                        && item.enabled == enabled
                })
                .count()
                != 1
        })
    {
        return Err("响应不完整或目标不匹配；请刷新核实结果".into());
    }
    Ok(response)
}
pub(crate) fn summary(result: &McpDistributionResult) -> &'static str {
    if result.outcome == McpBatchOutcome::Succeeded
        && !result.items.is_empty()
        && result.items.iter().all(|item| {
            item.desired_committed
                && matches!(
                    item.native_outcome,
                    McpDistributionOutcome::Applied | McpDistributionOutcome::Unchanged
                )
        })
    {
        "全部目标已应用"
    } else {
        "存在失败、取消或未验证目标，请查看逐项结果"
    }
}
pub(crate) fn item_label(item: &McpDistributionItem) -> String {
    let native = match item.native_outcome {
        McpDistributionOutcome::Applied => "已应用".into(),
        McpDistributionOutcome::Unchanged => "无需变更".into(),
        McpDistributionOutcome::NotVerified => "未验证".into(),
        McpDistributionOutcome::Unsupported => "不支持".into(),
        McpDistributionOutcome::Cancelled => "已取消".into(),
        McpDistributionOutcome::Failed { code } => format!("失败：{code:?}"),
    };
    format!(
        "{} · {:?} / {} · 期望{} · {} · 原生{}",
        item.record.id.0,
        item.target.app_id,
        item.target.installation_id.0,
        if item.enabled { "启用" } else { "禁用" },
        if item.desired_committed { "期望已保存" } else { "期望未保存" },
        native
    )
}
impl BindingsState {
    fn finish(
        &mut self,
        current: &catalog::Context,
        request: &catalog::Context,
        generation: u64,
        result: Result<McpDistributionResult, String>,
    ) -> bool {
        if current != request
            || self.context.as_ref() != Some(request)
            || self.generation != generation
            || !self.running
        {
            return false;
        }
        self.running = false;
        self.selected.clear();
        self.result = Some(result);
        true
    }
}
pub(crate) fn update(app: &mut App, value: BindingsMessage) -> Task<Message> {
    let Some(context) = catalog::context(app) else {
        return app.show_error_toast("网关不可用");
    };
    if app.mcp_settings.bindings.context.as_ref() != Some(&context) {
        let generation = app.mcp_settings.bindings.generation.wrapping_add(1);
        app.mcp_settings.bindings =
            BindingsState { context: Some(context.clone()), generation, ..Default::default() };
    }
    match value {
        BindingsMessage::Select(record, selected) => {
            if app.mcp_settings.bindings.running {
                return Task::none();
            }
            let Some(Ok(current)) = catalog::visible(app) else {
                return Task::none();
            };
            if !current.records.contains(&record) {
                return Task::none();
            }
            let current_records = current.records.clone();
            let records = &mut app.mcp_settings.bindings.selected;
            records.retain(|r| current_records.contains(r) && r != &record);
            if selected {
                records.push(record);
            }
            Task::none()
        }
        BindingsMessage::Apply(target, enabled) => {
            if app.mcp_settings.bindings.running {
                return Task::none();
            }
            let records = app.mcp_settings.bindings.selected.clone();
            let valid_catalog = matches!(catalog::visible(app), Some(Ok(c)) if records.iter().all(|r| c.records.contains(r)) && supported(c, &records, target.app_id));
            let valid_installation = app.apps_catalog.context.as_ref() == Some(&context.endpoint) && !app.apps_catalog.loading && !app.apps_catalog.token.is_empty() && target.target_host_id == context.host && app.apps_catalog.rows.iter().any(|row| row.entry.id == target.app_id && row.observation.as_ref().is_ok_and(|observation| observation.installations.iter().any(|i| i.app_id == target.app_id && i.id == target.installation_id && i.target_host_id == context.host && matches!(i.environment, vw_gateway_client::vw_api_types::agent_apps::AppInstallationEnvironment::Macos))));
            if !valid_catalog || !valid_installation || project(&records).is_err() {
                return app.show_error_toast(
                    "选择无效、修订已变更或目标不支持；请刷新并选择同一作用域目录项",
                );
            }
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(_) => return app.show_error_toast("网关不可用"),
            };
            let state = &mut app.mcp_settings.bindings;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            state.running = true;
            state.result = None;
            let token = app.apps_catalog.token.clone();
            Task::perform(
                async move {
                    let result = apply(client, records, target, enabled, token).await;
                    BindingsMessage::Finished { context, generation, result }
                },
                message,
            )
        }
        BindingsMessage::Finished { context: request, generation, result } => {
            if !app.mcp_settings.bindings.finish(&context, &request, generation, result) {
                return Task::none();
            }
            let feedback = match app.mcp_settings.bindings.result.as_ref().expect("finished result")
            {
                Ok(result) if summary(result) == "全部目标已应用" => {
                    app.show_success_toast("全部目标已应用")
                }
                Ok(_) => app.show_error_toast("存在失败或未验证目标，请查看逐项结果"),
                Err(error) => app.show_error_toast(error.clone()),
            };
            Task::batch([feedback, catalog::update(app, catalog::CatalogMessage::Refresh)])
        }
    }
}
#[cfg(test)]
#[path = "mcp_management_bindings_tests.rs"]
mod tests;
