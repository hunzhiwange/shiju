use crate::management_import::*;
use crate::management_update::{self as scope, Update};
use gateway_mcp_distribution::*;
use iced::Task;
use vw_api_app_identity::AppId;
use vw_api_mcp_management::*;
fn authorized(app: &Update<'_>, context: &crate::management_context::Context) -> bool {
    app.context.as_ref() == Some(context)
        && app.authorized
        && !context.host.is_empty()
        && !app.token.is_empty()
}
/// Decoded clipboard drafts share confirmation; system URI routing has a separate owner.
fn open_draft(app: &mut Update<'_>, request: McpCreateRequest) -> Task<ImportMessage> {
    let Some(context) = scope::context(app).filter(|c| authorized(app, c)) else {
        return app.error("请先连接目标主机");
    };
    if request.target_host_id != context.host {
        return app.error("草稿目标主机与当前主机不匹配");
    }
    let mut state = ImportState::new(context);
    state.draft = Some(request);
    app.state.import = Some(state);
    Task::none()
}
/// Capture the same authorized scope as native imports without issuing a request.
pub fn open_link(app: &mut Update<'_>, drafts: Vec<Result<McpCreateRequest, String>>) -> bool {
    if app.state.import.is_some() || drafts.is_empty() {
        return false;
    }
    let Some(context) = scope::context(app).filter(|c| authorized(app, c)) else { return false };
    if drafts.iter().flatten().any(|d| d.target_host_id != context.host) {
        return false;
    }
    let mut state = ImportState::new(context);
    state.link_drafts = drafts;
    app.state.import = Some(state);
    true
}
pub fn update(app: &mut Update<'_>, event: ImportMessage) -> Task<ImportMessage> {
    if matches!(event, ImportMessage::Open) {
        if app.state.import.as_ref().is_some_and(|s| s.importing && authorized(app, &s.context)) {
            return Task::none();
        }
        let Some(context) = scope::context(app).filter(|c| authorized(app, c)) else {
            return app.error("请先在应用管理中连接目标主机");
        };
        app.state.import = Some(ImportState::new(context));
        return Task::none();
    }
    if matches!(event, ImportMessage::Close) {
        if !app.state.import.as_ref().is_some_and(|s| s.importing) {
            app.state.import = None;
        }
        return Task::none();
    }
    let Some(state) = app.state.import.as_ref() else {
        return Task::none();
    };
    if !authorized(app, &state.context) {
        return app.error("连接上下文已变化，请关闭后重新导入");
    }
    let context = state.context.clone();
    let generation = state.generation;
    if state.importing && !matches!(event, ImportMessage::Finished(..)) {
        return Task::none();
    }
    match event {
        ImportMessage::Select(target) => {
            if !app.targets.clone().contains(&target) {
                return app.error("请选择当前主机的可信安装实例");
            }
            let state = app.state.import.as_mut().unwrap();
            state.invalidate();
            state.source = Some(target);
            state.account.clear();
            state.draft = None;
        }
        ImportMessage::Account(account) => {
            let state = app.state.import.as_mut().unwrap();
            state.invalidate();
            state.account = account;
        }
        ImportMessage::Preview => {
            let state = app.state.import.as_ref().unwrap();
            if state.loading {
                return Task::none();
            }
            let Some(target) = state.source.clone().filter(|t| app.targets.clone().contains(t))
            else {
                return app.error("请选择可信安装实例");
            };
            let account = if target.app_id == AppId::Claude {
                if state.account.trim().is_empty() {
                    return app.error("Claude 需要受管官方账号 ID");
                }
                Some(state.account.clone())
            } else {
                None
            };
            let Ok(client) = app.client.clone() else {
                return app.error("网关不可用");
            };
            let state = app.state.import.as_mut().unwrap();
            state.invalidate();
            state.loading = true;
            let generation = state.generation;
            let token = app.token.clone();
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
                std::convert::identity,
            );
        }
        ImportMessage::Previewed(request, serial, result) => {
            let state = app.state.import.as_mut().unwrap();
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
            let state = app.state.import.as_mut().unwrap();
            state.invalidate();
            state.loading = true;
            let generation = state.generation;
            return iced::clipboard::read()
                .map(move |value| ImportMessage::Pasted(context.clone(), generation, value));
        }
        ImportMessage::Pasted(request, serial, value) => {
            let state = app.state.import.as_mut().unwrap();
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
            let state = app.state.import.as_ref().unwrap();
            if !state.can_confirm() {
                return Task::none();
            }
            let source = state.source.clone();
            if state.link_drafts.is_empty()
                && state.draft.is_none()
                && !source.as_ref().is_some_and(|t| app.targets.clone().contains(t))
            {
                return app.error("安装实例已变化，请重新预览");
            }
            let Ok(client) = app.client.clone() else {
                return app.error("网关不可用");
            };
            let state = app.state.import.as_mut().unwrap();
            state.importing = true;
            let draft = state.draft.clone();
            let link_drafts = state.link_drafts.clone();
            let account = source
                .as_ref()
                .filter(|t| t.app_id == AppId::Claude)
                .map(|_| state.account.clone());
            let token = app.token.clone();
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
                std::convert::identity,
            );
        }
        ImportMessage::Finished(request, serial, result) => {
            let state = app.state.import.as_mut().unwrap();
            if request != context || serial != generation || !state.importing {
                return Task::none();
            }
            state.importing = false;
            let feedback = if result.is_ok() {
                app.info("导入请求已返回，请查看冲突与逐项结果")
            } else {
                app.error("导入未确认成功，请查看结果并刷新目录")
            };
            app.state.import.as_mut().unwrap().result = Some(result);
            return Task::batch([feedback, app.refresh()]);
        }
        ImportMessage::Open | ImportMessage::Close => {}
    }
    Task::none()
}

pub fn trusted_targets<'a>(
    context: Option<&crate::management_context::Context>,
    authorized: bool,
    loading: bool,
    installations: impl IntoIterator<Item = (AppId, &'a vw_api_app_identity::AppInstallation)>,
) -> Vec<McpAppTarget> {
    let Some(context) = context.filter(|c| authorized && !loading && !c.host.is_empty()) else {
        return vec![];
    };
    installations
        .into_iter()
        .filter(|(app_id, i)| {
            i.app_id == *app_id
                && i.target_host_id == context.host
                && matches!(i.environment, vw_api_app_identity::AppInstallationEnvironment::Macos)
                && crate::distribution::unsupported_reason(i.app_id).is_none()
        })
        .map(|(_, i)| McpAppTarget {
            app_id: i.app_id,
            installation_id: i.id.clone(),
            target_host_id: i.target_host_id.clone(),
        })
        .collect()
}
