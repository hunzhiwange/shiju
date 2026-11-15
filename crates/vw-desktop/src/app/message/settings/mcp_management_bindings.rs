//! Native bindings use trusted installation identities; runtime and ACP stay independent.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
#[cfg(test)]
use gateway_mcp_distribution::{McpBindingRequest, McpBulkRequest};
use iced::Task;
use vw_api_mcp_management::*;
#[cfg(test)]
use vw_gateway_client::GatewayClient;

#[cfg(test)]
use desktop_settings_mcp::distribution::{APPS, item_label};
pub(crate) use desktop_settings_mcp::distribution::{
    project, summary, supported, unsupported_reason,
};
pub use desktop_settings_mcp::management_bindings::{BindingsMessage, BindingsState, apply};
pub(crate) fn message(value: BindingsMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Bindings(value)))
}
pub(crate) fn visible(app: &App) -> Option<&BindingsState> {
    let state = &app.mcp_settings.bindings;
    (catalog::context(app).as_ref() == state.context.as_ref()).then_some(state)
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
        BindingsMessage::RetryFailed => {
            if app.mcp_settings.bindings.running || catalog::loading(app) {
                return Task::none();
            }
            let Some(Ok(result)) = app.mcp_settings.bindings.result.as_ref() else {
                return Task::none();
            };
            let failed: Vec<_> = result
                .items
                .iter()
                .filter(|item| matches!(item.native_outcome, McpDistributionOutcome::Failed { .. }))
                .collect();
            let Some(first) = failed.first() else {
                return Task::none();
            };
            let target = first.target.clone();
            let enabled = first.enabled;
            let Some(Ok(current)) = catalog::visible(app) else {
                return app.show_error_toast("请先刷新目录再重试失败项");
            };
            let mut records = Vec::new();
            for item in failed {
                if item.target != target || item.enabled != enabled {
                    return app.show_error_toast("失败项目标不一致，请重新选择");
                }
                let Some(record) =
                    current.records.iter().find(|record| record.record == item.record)
                else {
                    return app.show_error_toast("失败项目录修订已变更，请重新选择");
                };
                records.push(record.clone());
            }
            // Apply performs the same capability, scope and installation checks as a new request.
            app.mcp_settings.bindings.selected = records;
            update(app, BindingsMessage::Apply(target, enabled))
        }
        BindingsMessage::Apply(target, enabled) => {
            if app.mcp_settings.bindings.running {
                return Task::none();
            }
            let records = app.mcp_settings.bindings.selected.clone();
            let valid_catalog = matches!(catalog::visible(app), Some(Ok(c)) if records.iter().all(|r| c.records.contains(r)) && supported(c, &records, target.app_id));
            let valid_installation = app.apps_catalog.context.as_ref() == Some(&context.endpoint)
                && !app.apps_catalog.loading
                && !app.apps_catalog.token.is_empty()
                && target.target_host_id == context.host
                && app.apps_catalog.rows.iter().any(|row| {
                    row.entry.id == target.app_id
                        && row.observation.as_ref().is_ok_and(|observation| {
                            observation.installations.iter().any(|i| {
                                i.app_id == target.app_id
                                    && i.id == target.installation_id
                                    && i.target_host_id == context.host
                                    && matches!(
                                        i.environment,
                                        vw_api_app_identity::AppInstallationEnvironment::Macos
                                    )
                            })
                        })
                });
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
