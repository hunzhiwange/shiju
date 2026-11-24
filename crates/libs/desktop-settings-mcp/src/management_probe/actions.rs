//! Explicit, revision-bound matrix actions. Page entry never starts network probes.
use crate::{management_effect::Effect, third_party::Scope};
pub use desktop_apps_ui::agent_apps_diagnostics::mcp::McpDiagnosticMessage as ProbeMessage;
use iced::Task;
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub scope: Option<Scope>,
    pub generation: u64,
    pub busy: bool,
    pub records: Vec<McpCatalogRecord>,
    pub more: bool,
    pub row: Option<McpCatalogRecord>,
    pub delete: Option<McpCatalogRecord>,
    pub export: Option<McpSafeExportResponse>,
    pub deleted: Option<McpDistributionResult>,
    pub error: Option<String>,
}
#[derive(Debug, Clone)]
pub enum Message {
    More,
    Row(McpCatalogRecord),
    Close,
    CopyAll,
    CopyRow(McpCatalogRecord),
    Exported(Scope, u64, Result<McpSafeExportResponse, String>),
    ConfirmCopy,
    Clipboard(Scope, u64, String, Option<String>),
    Delete(McpCatalogRecord),
    ConfirmDelete,
    Deleted(Scope, u64, Result<McpDistributionResult, String>),
}
pub struct Update<'a> {
    pub state: &'a mut State,
    pub scope: &'a Scope,
    pub catalog: &'a [McpCatalogRecord],
    pub authorized: bool,
    pub client: Result<GatewayClient, String>,
    pub token: String,
    pub effects: Vec<Effect>,
}
/// Call after page, credentials or catalog changes; asynchronous results keep their origin.
pub fn invalidate(state: &mut State, scope: Option<&Scope>, records: &[McpCatalogRecord]) {
    if state.scope.as_ref() != scope
        || state.records != records
        || state.row.as_ref().is_some_and(|r| !records.contains(r))
        || state.delete.as_ref().is_some_and(|r| !records.contains(r))
    {
        let generation = state.generation.wrapping_add(1);
        *state = State {
            scope: scope.cloned(),
            generation,
            records: records.to_vec(),
            ..Default::default()
        };
    }
}
fn update_inner(app: &mut Update<'_>, event: Message) -> Task<Message> {
    invalidate(app.state, Some(app.scope), app.catalog);
    let state = &mut *app.state;
    match event {
        Message::Exported(scope, generation, result) => {
            if &scope != app.scope || generation != state.generation {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(response) => state.export = Some(response),
                Err(e) => state.error = Some(e),
            }
        }
        Message::Deleted(scope, generation, result) => {
            if &scope != app.scope || generation != state.generation {
                return Task::none();
            }
            state.busy = false;
            state.delete = None;
            match result {
                Ok(result) => {
                    app.effects.push(Effect::Info(format!(
                        "目录删除已保存；{}",
                        crate::distribution::summary(&result)
                    )));
                    state.deleted = Some(result);
                    app.effects.push(Effect::Refresh);
                }
                Err(e) => state.error = Some(e),
            }
        }
        Message::Clipboard(scope, generation, expected, actual) => {
            if &scope == app.scope && generation == state.generation {
                state.busy = false;
                if actual.as_deref() == Some(expected.as_str()) {
                    state.export = None;
                    app.effects.push(Effect::Success("已复制 mcpServers JSON".into()));
                } else {
                    state.error = Some("剪贴板写入未确认，请重试".into());
                }
            }
        }
        _ if state.busy => return Task::none(),
        Message::Close => {
            state.more = false;
            state.row = None;
            state.delete = None;
            state.export = None;
            state.error = None;
        }
        Message::More => {
            state.more = !state.more;
            state.row = None;
        }
        Message::Row(record) if app.catalog.contains(&record) => {
            state.more = false;
            state.row = Some(record);
        }
        Message::Delete(record) if app.authorized && app.catalog.contains(&record) => {
            state.delete = Some(record);
            state.row = None;
        }
        Message::ConfirmDelete if app.authorized => {
            let Some(record) = state.delete.clone().filter(|r| app.catalog.contains(r)) else {
                return Task::none();
            };
            let Ok(client) = app.client.clone() else {
                state.error = Some("网关不可用".into());
                return Task::none();
            };
            let scope = app.scope.clone();
            let generation = state.generation;
            let token = app.token.clone();
            let project_id = match &record.scope {
                McpCatalogScope::Tenant { .. } => None,
                McpCatalogScope::Project { scope } => Some(scope.project_id().clone()),
            };
            let request = McpDeleteRequest {
                target_host_id: scope.context.host.clone(),
                project_id,
                expected_revision: record.record.revision,
            };
            state.busy = true;
            return Task::perform(
                async move {
                    let result =
                        gateway_mcp_write::GatewayMcpWriteClient::new(client.transport().clone())
                            .mcp_management_delete(&record.record.id, &request, &token)
                            .await
                            .map_err(|_| "删除未确认，请刷新目录及逐目标清理状态".to_owned());
                    Message::Deleted(scope, generation, result)
                },
                std::convert::identity,
            );
        }
        Message::CopyAll | Message::CopyRow(_) => {
            let id = match event {
                Message::CopyRow(record) if app.catalog.contains(&record) => {
                    Some(record.record.id.0)
                }
                Message::CopyAll => None,
                _ => return Task::none(),
            };
            let Ok(client) = app.client.clone() else {
                state.error = Some("网关不可用".into());
                return Task::none();
            };
            let scope = app.scope.clone();
            let generation = state.generation;
            let token = app.token.clone();
            state.busy = true;
            state.export = None;
            return Task::perform(
                async move {
                    let result =
                        gateway_mcp_read::GatewayMcpReadClient::new(client.transport().clone())
                            .mcp_management_export(&scope.context.host, id.as_deref(), &token)
                            .await
                            .map_err(|_| "无法读取可复制的安全定义".to_owned());
                    Message::Exported(scope, generation, result)
                },
                std::convert::identity,
            );
        }
        Message::ConfirmCopy => {
            let Some(json) = state.export.as_ref().and_then(|r| r.json.clone()) else {
                return Task::none();
            };
            let scope = app.scope.clone();
            let generation = state.generation;
            state.busy = true;
            return iced::clipboard::write::<Message>(json.clone()).chain(
                iced::clipboard::read().map(move |actual| {
                    Message::Clipboard(scope.clone(), generation, json.clone(), actual)
                }),
            );
        }
        _ => {}
    }
    Task::none()
}
/// Existing diagnostic reducer owns authorization, cancellation and late-job cleanup.
pub fn probe(
    state: &mut desktop_apps_ui::agent_apps_diagnostics::mcp_state::McpState,
    current: Option<crate::diagnostic_context::Context>,
    catalog: Option<&Result<McpCatalogResponse, String>>,
    visible: &[McpCatalogRecord],
    token: &str,
    client: Result<GatewayClient, String>,
    notice: &mut Option<desktop_provider_apps::Notice>,
    event: ProbeMessage,
) -> Task<ProbeMessage> {
    if let ProbeMessage::Start(ids) = &event {
        if current.as_ref().is_none_or(|c| {
            ids.is_empty()
                || ids.iter().any(|id| {
                    !visible.iter().any(|r| &r.record.id == id && c.connections.contains(r))
                })
        }) {
            return Task::none();
        }
    }
    desktop_apps_ui::agent_apps_diagnostics::mcp::update(
        state,
        current,
        catalog,
        token,
        Some(client),
        notice,
        event,
    )
}

/// Host maps Rewrite to the existing resync confirmation, and Tools to tool details.
#[derive(Debug, Clone)]
pub enum MenuMessage {
    Action(Message),
    Rewrite,
    Tools(McpCatalogRecord),
    Documentation(String),
}
pub fn menu(i18n: desktop_i18n::I18n, state: &State) -> iced::Element<'_, MenuMessage> {
    use iced::widget::{column, text};
    let mut body = column![].spacing(8);
    if state.more {
        body = body
            .push(crate::toolbar_button(
                i18n.__("按当前开关重写应用配置"),
                Some(MenuMessage::Rewrite),
            ))
            .push(crate::toolbar_button(
                i18n.__("复制全部为 mcpServers JSON"),
                Some(MenuMessage::Action(Message::CopyAll)),
            ));
    }
    if let Some(record) = &state.row {
        body = body
            .push(crate::toolbar_button(
                i18n.__("复制为 mcpServers JSON"),
                Some(MenuMessage::Action(Message::CopyRow(record.clone()))),
            ))
            .push(crate::toolbar_button(
                i18n.__("工具详情"),
                Some(MenuMessage::Tools(record.clone())),
            ));
        if let Some(url) = record
            .documentation
            .as_ref()
            .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
        {
            body = body.push(crate::toolbar_button(
                i18n.__("查看文档"),
                Some(MenuMessage::Documentation(url.clone())),
            ));
        }
        body = body.push(crate::toolbar_button(
            i18n.__("删除…"),
            Some(MenuMessage::Action(Message::Delete(record.clone()))),
        ));
    }
    if let Some(result) = &state.deleted {
        for item in &result.items {
            body = body.push(text(crate::distribution::item_label(i18n, item)));
        }
    }
    body.into()
}
/// Common modal overlay consumes background input; host mounts it above row menus.
pub fn overlay<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a State,
    background: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    use desktop_ui_controls::system_settings_common::*;
    use iced::widget::{column, row, text};
    let dialog: Option<iced::Element<'a, Message>> = if let Some(record) = &state.delete {
        Some(
            settings_modal_card(
                column![
                    text(
                        desktop_i18n::__!(i18n, "删除 %s？", (record.name).to_string())
                            .expect("valid extension display template")
                    ),
                    text(i18n.__("删除共享定义并清理受管目标；失败目标会单独报告。")),
                    text(desktop_i18n::settings_extensions::feedback(
                        i18n,
                        state.error.as_deref().unwrap_or("")
                    )),
                    row![
                        crate::toolbar_button(
                            i18n.__("取消"),
                            (!state.busy).then_some(Message::Close)
                        ),
                        crate::toolbar_button(
                            i18n.__("确认删除"),
                            (!state.busy).then_some(Message::ConfirmDelete)
                        )
                    ]
                    .spacing(8)
                ]
                .spacing(12),
            )
            .into(),
        )
    } else if let Some(export) = &state.export {
        let count = export
            .json
            .as_ref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.get("mcpServers").and_then(|v| v.as_object()).map(|m| m.len()))
            .unwrap_or(0);
        let mut body = column![text(
            desktop_i18n::__!(
                i18n,
                "可复制 %s 项，跳过 %s 项",
                (count).to_string(),
                (export.skipped.len()).to_string()
            )
            .expect("valid extension display template")
        )]
        .spacing(8);
        for skipped in &export.skipped {
            body = body.push(text(format!(
                "{}：{}",
                skipped.id.0,
                i18n.__(skip_label(skipped.reason))
            )));
        }
        if let Some(reason) = export.reason {
            body = body.push(text(i18n.__(skip_label(reason))));
        }
        if let Some(error) = &state.error {
            body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
                i18n,
                &error.clone(),
            )));
        }
        let footer = row![
            crate::toolbar_button(i18n.__("取消"), (!state.busy).then_some(Message::Close)),
            crate::toolbar_button(
                i18n.__("复制"),
                (!state.busy && count > 0).then_some(Message::ConfirmCopy)
            )
        ]
        .spacing(8);
        Some(
            settings_modal_layout(
                text(i18n.__("复制 mcpServers JSON")).size(18),
                body,
                Some(footer.into()),
            )
            .width(680)
            .height(540)
            .into(),
        )
    } else if let Some(error) = &state.error {
        Some(
            settings_modal_card(
                column![
                    settings_error_banner(desktop_i18n::settings_extensions::feedback(
                        i18n,
                        &error.clone()
                    )),
                    crate::toolbar_button(i18n.__("关闭"), Some(Message::Close))
                ]
                .spacing(10),
            )
            .into(),
        )
    } else {
        None
    };
    match dialog {
        Some(dialog) => settings_modal_overlay(Some(background), Message::Close, dialog),
        None => background,
    }
}

pub fn skip_label(reason: McpExportSkipReason) -> &'static str {
    match reason {
        McpExportSkipReason::ProtectedDefinition => "包含受保护定义，禁止复制",
        McpExportSkipReason::NotFound => "记录不存在或修订已变化",
        McpExportSkipReason::NoPublicDefinitions => "没有可公开复制的定义",
    }
}
pub fn outcome_label(outcome: McpConnectionOutcome) -> &'static str {
    match outcome {
        McpConnectionOutcome::Connected { .. } => "已连通",
        McpConnectionOutcome::Failed { .. } => "连接失败",
        McpConnectionOutcome::Cancelled => "已取消",
        McpConnectionOutcome::NotVerified => "未验证",
        McpConnectionOutcome::Unsupported => "不支持",
    }
}

/// Every user action carries the page identity captured when its control was rendered.
#[derive(Debug, Clone)]
pub struct ScopedMessage {
    pub scope: Scope,
    pub action: Message,
}
pub fn update(app: &mut Update<'_>, event: ScopedMessage) -> Task<ScopedMessage> {
    if &event.scope != app.scope {
        return Task::none();
    }
    let scope = event.scope;
    update_inner(app, event.action)
        .map(move |action| ScopedMessage { scope: scope.clone(), action })
}

pub fn menu_overlay<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a State,
    background: iced::Element<'a, MenuMessage>,
) -> iced::Element<'a, MenuMessage> {
    use desktop_ui_controls::system_settings_common::*;
    if state.more || state.row.is_some() {
        settings_modal_overlay(
            Some(background),
            MenuMessage::Action(Message::Close),
            settings_modal_card(menu(i18n, state)).width(380),
        )
    } else {
        background
    }
}
