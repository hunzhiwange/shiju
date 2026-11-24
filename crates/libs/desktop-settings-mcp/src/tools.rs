//! Tool details and independent usage scopes, using already validated read models.
use crate::toolbar_button;
use desktop_ui_controls::system_settings_common::*;
use iced::{Alignment, Element, Length, widget::{column, row, text}};
use vw_api_mcp_management::*;
#[derive(Debug, Clone)]
pub enum ToolsMessage { Open(McpCatalogRecord), Close, AcpSessionChanged(String), ReadAcp, Cancel, Refresh }
type Message = ToolsMessage;
fn message(value: ToolsMessage) -> Message { value }
pub struct ToolsView<'a> {
    pub record: &'a McpCatalogRecord,
    pub acp_session: &'a str, pub acp_loading: bool,
    pub acp: &'a Option<Result<McpAcpUsage, String>>,
    pub result: &'a Option<McpToolDetails>, pub error: &'a Option<String>,
    pub cancelling: bool, pub terminal: bool,
}
pub fn panel(
    i18n: desktop_i18n::I18n,
    catalog: Option<&Result<McpCatalogResponse, String>>,
) -> Element<'_, Message> {
    let mut body = column![
        text(i18n.__("工具详情与使用范围")).size(16),
        text(i18n.__("请选择检测应用后，显式查看目录工具；发现和连接成功均不代表获得调用授权。"))
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(10);
    if let Some(Ok(catalog)) = catalog {
        for record in &catalog.records {
            body = body.push(
                row![
                    text(record.name.clone()).width(Length::Fill),
                    toolbar_button(
                        i18n.__("工具详情"),
                        Some(message(ToolsMessage::Open(record.clone())))
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    settings_panel(body).into()
}

pub fn card<'a>(
    i18n: desktop_i18n::I18n,
    state: ToolsView<'a>,
    catalog: Option<&'a Result<McpCatalogResponse, String>>,
) -> Element<'a, Message> {
    let close = message(ToolsMessage::Close);
    let header = row![
        column![
            text(i18n.__("工具详情与使用范围")).size(18),
            text(state.record.name.clone()).size(13)
        ]
        .spacing(5)
        .width(Length::Fill),
        settings_close_button(close.clone()),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let mut body = column![
        text(
            desktop_i18n::__!(
                i18n,
                "目录修订 %s · 连接修订 %s",
                (state.record.record.revision).to_string(),
                (state.record.connection_revision).to_string()
            )
            .expect("valid extension display template")
        )
        .size(12),
        text(i18n.__("调用授权：未在此检查；实际调用仍由执行会话权限与撤销规则判定。"))
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(12)
    .width(Length::Fill);
    if let Some(Ok(catalog)) = catalog {
        let mut bound = false;
        for binding in
            catalog.bindings.iter().filter(|binding| binding.record.id == state.record.record.id)
        {
            bound = true;
            body = body.push(
                text(
                    desktop_i18n::__!(
                        i18n,
                        "原生应用 %s · %s · %s · 修订 %s（上次观察）",
                        (format!("{:?}", binding.target.app_id)).to_string(),
                        (if binding.enabled {
                            i18n.__("期望启用")
                        } else {
                            i18n.__("期望禁用")
                        })
                        .to_string(),
                        (format!("{:?}", binding.native_status)).to_string(),
                        (binding.record.revision).to_string()
                    )
                    .expect("valid extension display template"),
                )
                .size(12),
            );
        }
        if !bound {
            body = body.push(text(i18n.__("原生应用：未绑定")).size(12));
        }
    }
    body = body
        .push(settings_divider())
        .push(text(i18n.__("ACP 入站会话使用范围")).size(14))
        .push(
            text(i18n.__(
                "输入需要核验的入站 ACP 会话ID；读取需要该项目及会话的独立授权，不会恢复连接。",
            ))
            .size(12)
            .style(settings_muted_text_style),
        )
        .push(
            settings_text_input(i18n.__("入站 ACP 会话ID"), state.acp_session)
                .on_input(move |value| message(ToolsMessage::AcpSessionChanged(value))),
        )
        .push(toolbar_button(
            if state.acp_loading { i18n.__("读取中...") } else { i18n.__("读取 ACP 挂载") },
            (!state.acp_loading && !state.acp_session.trim().is_empty())
                .then(|| message(ToolsMessage::ReadAcp)),
        ));
    match &state.acp {
        Some(Ok(usage)) => {
            body = body.push(
                text(
                    desktop_i18n::__!(
                        i18n,
                        "ACP 会话 %s · 挂载修订 %s",
                        (usage.session_id.as_ref()).to_string(),
                        (usage.mount_revision).to_string()
                    )
                    .expect("valid extension display template"),
                )
                .size(12),
            );
            let mounts: Vec<_> = usage
                .mounts
                .iter()
                .filter(|mount| {
                    mount.scope == state.record.scope
                        && mount.record.id == state.record.record.id
                })
                .collect();
            if mounts.is_empty() {
                body = body.push(text(i18n.__("ACP：该会话未挂载此目录记录")).size(13));
            }
            for mount in mounts {
                let status = if mount.failure.is_some() {
                    i18n.__("已失效")
                } else if mount.record == state.record.record {
                    i18n.__("有效引用")
                } else {
                    i18n.__("非当前目录修订")
                };
                body = body.push(
                    text(
                        desktop_i18n::__!(
                            i18n,
                            "ACP：修订 %s · %s",
                            (mount.record.revision).to_string(),
                            (status).to_string()
                        )
                        .expect("valid extension display template"),
                    )
                    .size(13),
                );
                if let Some(code) = mount.failure {
                    body = body.push(settings_error_banner(
                        desktop_i18n::__!(
                            i18n,
                            "ACP 挂载：%s",
                            (format!("{:?}", code)).to_string()
                        )
                        .expect("valid extension display template"),
                    ));
                }
            }
        }
        Some(Err(error)) => {
            body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
                i18n,
                &error.clone(),
            )))
        }
        None => {
            body = body.push(
                text(i18n.__("ACP：尚未读取已验证会话")).size(12).style(settings_muted_text_style),
            )
        }
    }
    body = body.push(settings_divider());
    if let Some(result) = &state.result {
        let runtime = match &result.runtime {
            Some(McpRuntimeUsage::NotSelected) => i18n.__("识矩运行时：项目未选择").to_owned(),
            Some(McpRuntimeUsage::Selected { record, disabled, current, generation }) => {
                desktop_i18n::__!(
                    i18n,
                    "识矩运行时：%s · 修订 %s · %s · 代次 %s",
                    (if *disabled { i18n.__("已禁用") } else { i18n.__("已选择") }).to_string(),
                    (record.revision).to_string(),
                    (if *current {
                        i18n.__("当前有效修订")
                    } else {
                        i18n.__("修订已失效")
                    })
                    .to_string(),
                    (generation).to_string()
                )
                .expect("valid extension display template")
            }
            None => i18n.__("识矩运行时：未验证").to_owned(),
        };
        body = body.push(text(runtime).size(13));
        if let Some(code) = result.runtime_error {
            body = body.push(settings_error_banner(
                desktop_i18n::__!(
                    i18n,
                    "运行时选择读取失败：%s",
                    (format!("{:?}", code)).to_string()
                )
                .expect("valid extension display template"),
            ));
        }
        if let Some(code) = result.connection_error {
            body = body.push(settings_error_banner(
                desktop_i18n::__!(i18n, "连接检查失败：%s", (format!("{:?}", code)).to_string())
                    .expect("valid extension display template"),
            ));
        }
        if let Some(code) = result.tools_error {
            body = body.push(settings_error_banner(
                desktop_i18n::__!(i18n, "工具发现失败：%s", (format!("{:?}", code)).to_string())
                    .expect("valid extension display template"),
            ));
        }
        body = body.push(
            text(if result.connection_error.is_some() || result.tools_error.is_some() {
                i18n.__("工具规格未验证").to_owned()
            } else {
                desktop_i18n::__!(i18n, "实际发现 %s 个工具", (result.tools.len()).to_string())
                    .expect("valid extension display template")
            })
            .size(14),
        );
        for tool in &result.tools {
            body = body.push(settings_panel(
                column![
                    crate::tool_summary(tool),
                    text(
                        desktop_i18n::__!(i18n, "工具 ID：%s", (tool.id.as_ref()).to_string())
                            .expect("valid extension display template")
                    )
                    .size(12),
                    text(tool.description.clone()).size(12),
                    text(
                        desktop_i18n::__!(
                            i18n,
                            "规格标注：只读 %s · 破坏性 %s · 并发安全 %s · 暴露 %s",
                            (tool.read_only).to_string(),
                            (tool.destructive).to_string(),
                            (tool.concurrency_safe).to_string(),
                            (format!("{:?}", tool.exposure)).to_string()
                        )
                        .expect("valid extension display template")
                    )
                    .size(12),
                    text(serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default())
                        .size(12),
                ]
                .spacing(8),
            ));
        }
    } else {
        body = body.push(
            text(if state.cancelling {
                i18n.__("正在取消，等待执行端清理")
            } else if state.terminal {
                i18n.__("工具详情未验证")
            } else {
                i18n.__("正在发现工具并读取使用范围")
            })
            .size(13),
        );
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
            i18n,
            &error.clone(),
        )));
    }
    let mut actions = row![toolbar_button(i18n.__("关闭"), Some(close))].spacing(8);
    if !state.terminal {
        actions = actions
            .push(toolbar_button(i18n.__("取消发现"), Some(message(ToolsMessage::Cancel))))
            .push(toolbar_button(i18n.__("刷新状态"), Some(message(ToolsMessage::Refresh))));
    } else {
        actions = actions.push(toolbar_button(
            i18n.__("重新发现"),
            Some(message(ToolsMessage::Open(state.record.clone()))),
        ));
    }
    settings_modal_layout(header, body, Some(actions.into())).width(860).height(650).into()
}
