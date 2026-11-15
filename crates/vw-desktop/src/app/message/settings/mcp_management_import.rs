//! Confirmation only: files and credentials stay on the selected execution host.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
use iced::Task;
use {
    gateway_mcp_distribution::*, vw_api_mcp_management::*, vw_api_types::agent_apps::AppId,
    vw_api_types::agent_apps::AppInstallationEnvironment,
};

pub use desktop_settings_mcp::management_import::{
    ImportMessage, ImportState, import, preview_labels, result_labels,
};
pub(crate) fn message(event: ImportMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Import(event)))
}
pub(crate) fn targets(app: &App) -> Vec<McpAppTarget> {
    let Some(context) = catalog::context(app) else {
        return vec![];
    };
    if !authorized(app, &context) || app.apps_catalog.loading {
        return vec![];
    }
    app.apps_catalog
        .rows
        .iter()
        .flat_map(|row| {
            row.observation
                .as_ref()
                .ok()
                .into_iter()
                .flat_map(|o| &o.installations)
                .filter(|i| {
                    i.app_id == row.entry.id
                        && i.target_host_id == context.host
                        && matches!(i.environment, AppInstallationEnvironment::Macos)
                        && super::mcp_management_bindings::unsupported_reason(i.app_id).is_none()
                })
                .map(|i| McpAppTarget {
                    app_id: i.app_id,
                    installation_id: i.id.clone(),
                    target_host_id: i.target_host_id.clone(),
                })
        })
        .collect()
}
fn authorized(app: &App, context: &catalog::Context) -> bool {
    catalog::context(app).as_ref() == Some(context)
        && !context.host.is_empty()
        && !app.apps_catalog.token.is_empty()
        && app.apps_catalog.context.as_ref() == Some(&context.endpoint)
}
pub(crate) fn visible(app: &App) -> Option<&ImportState> {
    app.mcp_settings.import.as_ref().filter(|s| catalog::context(app).as_ref() == Some(&s.context))
}
/// Decoded clipboard drafts share confirmation; system URI routing has a separate owner.
fn open_draft(app: &mut App, request: McpCreateRequest) -> Task<Message> {
    let Some(context) = catalog::context(app).filter(|c| authorized(app, c)) else {
        return app.show_error_toast("请先连接目标主机");
    };
    if request.target_host_id != context.host {
        return app.show_error_toast("草稿目标主机与当前主机不匹配");
    }
    let mut state = ImportState::new(context);
    state.draft = Some(request);
    app.mcp_settings.import = Some(state);
    Task::none()
}
/// Capture the same authorized scope as native imports without issuing a request.
pub(crate) fn open_link(app: &mut App, drafts: Vec<Result<McpCreateRequest, String>>) -> bool {
    if app.mcp_settings.import.is_some() || drafts.is_empty() {
        return false;
    }
    let Some(context) = catalog::context(app).filter(|c| authorized(app, c)) else { return false };
    if drafts.iter().flatten().any(|d| d.target_host_id != context.host) {
        return false;
    }
    let mut state = ImportState::new(context);
    state.link_drafts = drafts;
    app.mcp_settings.import = Some(state);
    true
}
pub(crate) fn update(app: &mut App, event: ImportMessage) -> Task<Message> {
    if matches!(event, ImportMessage::Open) {
        if app
            .mcp_settings
            .import
            .as_ref()
            .is_some_and(|s| s.importing && authorized(app, &s.context))
        {
            return Task::none();
        }
        let Some(context) = catalog::context(app).filter(|c| authorized(app, c)) else {
            return app.show_error_toast("请先在应用管理中连接目标主机");
        };
        app.mcp_settings.import = Some(ImportState::new(context));
        return Task::none();
    }
    if matches!(event, ImportMessage::Close) {
        if !app.mcp_settings.import.as_ref().is_some_and(|s| s.importing) {
            app.mcp_settings.import = None;
        }
        return Task::none();
    }
    let Some(state) = app.mcp_settings.import.as_ref() else {
        return Task::none();
    };
    if !authorized(app, &state.context) {
        return app.show_error_toast("连接上下文已变化，请关闭后重新导入");
    }
    let context = state.context.clone();
    let generation = state.generation;
    if state.importing && !matches!(event, ImportMessage::Finished(..)) {
        return Task::none();
    }
    match event {
        ImportMessage::Select(target) => {
            if !targets(app).contains(&target) {
                return app.show_error_toast("请选择当前主机的可信安装实例");
            }
            let state = app.mcp_settings.import.as_mut().unwrap();
            state.invalidate();
            state.source = Some(target);
            state.account.clear();
            state.draft = None;
        }
        ImportMessage::Account(account) => {
            let state = app.mcp_settings.import.as_mut().unwrap();
            state.invalidate();
            state.account = account;
        }
        ImportMessage::Preview => {
            let state = app.mcp_settings.import.as_ref().unwrap();
            if state.loading {
                return Task::none();
            }
            let Some(target) = state.source.clone().filter(|t| targets(app).contains(t)) else {
                return app.show_error_toast("请选择可信安装实例");
            };
            let account = if target.app_id == AppId::Claude {
                if state.account.trim().is_empty() {
                    return app.show_error_toast("Claude 需要受管官方账号 ID");
                }
                Some(state.account.clone())
            } else {
                None
            };
            let Ok(client) = crate::app::config::gateway_client() else {
                return app.show_error_toast("网关不可用");
            };
            let state = app.mcp_settings.import.as_mut().unwrap();
            state.invalidate();
            state.loading = true;
            let generation = state.generation;
            let token = app.apps_catalog.token.clone();
            return Task::perform(
                async move {
                    let result =
                        gateway_mcp_read::GatewayMcpReadClient::new(client.transport().clone())
                            .mcp_management_native(&target, account.as_deref(), &token)
                            .await
                            .map_err(|_| "原生预览失败，请检查主机、账号与权限".into())
                            .and_then(|p| {
                                if p.target == target {
                                    Ok(p)
                                } else {
                                    Err("预览目标不匹配".into())
                                }
                            });
                    ImportMessage::Previewed(context, generation, result)
                },
                message,
            );
        }
        ImportMessage::Previewed(request, serial, result) => {
            let state = app.mcp_settings.import.as_mut().unwrap();
            if request != context || serial != generation || !state.loading {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(preview) => state.preview = Some(preview),
                Err(error) => state.result = Some(Err(error)),
            }
        }
        ImportMessage::Paste => {
            let state = app.mcp_settings.import.as_mut().unwrap();
            state.invalidate();
            state.loading = true;
            let generation = state.generation;
            return iced::clipboard::read().map(move |value| {
                message(ImportMessage::Pasted(context.clone(), generation, value))
            });
        }
        ImportMessage::Pasted(request, serial, value) => {
            let state = app.mcp_settings.import.as_mut().unwrap();
            if request != context || serial != generation || !state.loading {
                return Task::none();
            }
            state.loading = false;
            let result = value
                .filter(|v| v.len() <= 262_144)
                .and_then(|v| serde_json::from_str::<McpCreateRequest>(&v).ok());
            if let Some(draft) = result {
                return open_draft(app, draft);
            }
            state.result = Some(Err(
                "剪贴板须包含已解码的 MCP 创建草稿 JSON（最多256 KiB）；系统深链由统一入口解析。"
                    .into(),
            ));
        }
        ImportMessage::Confirm => {
            let state = app.mcp_settings.import.as_ref().unwrap();
            if !state.can_confirm() {
                return Task::none();
            }
            let source = state.source.clone();
            if state.link_drafts.is_empty()
                && state.draft.is_none()
                && !source.as_ref().is_some_and(|t| targets(app).contains(t))
            {
                return app.show_error_toast("安装实例已变化，请重新预览");
            }
            let Ok(client) = crate::app::config::gateway_client() else {
                return app.show_error_toast("网关不可用");
            };
            let state = app.mcp_settings.import.as_mut().unwrap();
            state.importing = true;
            let draft = state.draft.clone();
            let link_drafts = state.link_drafts.clone();
            let account = source
                .as_ref()
                .filter(|t| t.app_id == AppId::Claude)
                .map(|_| state.account.clone());
            let token = app.apps_catalog.token.clone();
            return Task::perform(
                async move {
                    let write_client =
                        gateway_mcp_write::GatewayMcpWriteClient::new(client.transport().clone());
                    let result = if !link_drafts.is_empty() {
                        let mut labels = Vec::new();
                        for (index, entry) in link_drafts.into_iter().enumerate() {
                            let outcome = match entry {
                                Err(error) => error,
                                Ok(draft) => {
                                    match write_client.mcp_management_create(&draft, &token).await {
                                        Ok(_) => "成功：已保存定义，未启用或分发".into(),
                                        Err(gateway_mcp_write::AgentAppsSettingsError::Http {
                                            status: 409,
                                            ..
                                        }) => "失败：定义冲突，未覆盖".into(),
                                        Err(_) => "失败：未确认保存成功，请刷新目录核实".into(),
                                    }
                                }
                            };
                            labels.push(format!("第 {} 项 · {outcome}", index + 1));
                        }
                        Ok(labels)
                    } else if let Some(draft) = draft {
                        write_client
                            .mcp_management_create(&draft, &token)
                            .await
                            .map(|r| {
                                vec![format!("{} · 已导入定义，尚未分发或启用", r.record.id.0)]
                            })
                            .map_err(|error| match error {
                                gateway_mcp_write::AgentAppsSettingsError::Http {
                                    status: 409,
                                    ..
                                } => "冲突，未覆盖已有定义".into(),
                                _ => "草稿导入失败，请刷新目录核实".into(),
                            })
                    } else {
                        import(
                            &client,
                            McpImportSource {
                                target: source.expect("confirmed source"),
                                account_id: account,
                            },
                            &token,
                        )
                        .await
                    };
                    ImportMessage::Finished(context, generation, result)
                },
                message,
            );
        }
        ImportMessage::Finished(request, serial, result) => {
            let state = app.mcp_settings.import.as_mut().unwrap();
            if request != context || serial != generation || !state.importing {
                return Task::none();
            }
            state.importing = false;
            let feedback = if result.is_ok() {
                app.show_info_toast("导入请求已返回，请查看冲突与逐项结果")
            } else {
                app.show_error_toast("导入未确认成功，请查看结果并刷新目录")
            };
            app.mcp_settings.import.as_mut().unwrap().result = Some(result);
            return Task::batch([feedback, catalog::update(app, catalog::CatalogMessage::Refresh)]);
        }
        ImportMessage::Open | ImportMessage::Close => {}
    }
    Task::none()
}
#[cfg(test)]
#[path = "mcp_management_import_tests.rs"]
mod tests;
