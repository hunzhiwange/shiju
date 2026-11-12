//! Managed specifications and three independent usage scopes.
use super::*;
use crate::app::message::settings::mcp_management_tools::{self as tools, ToolsMessage};
use iced::widget::column;
use vw_gateway_client::vw_api_types::mcp_management::*;

pub(super) fn panel(app: &App) -> Element<'_, Message> {
    let mut body = column![
        text("工具详情与使用范围").size(16),
        text("请选择检测应用后，显式查看目录工具；发现和连接成功均不代表获得调用授权。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(10);
    if let Some(Ok(catalog)) = crate::app::message::settings::mcp_management_list::visible(app) {
        for record in &catalog.records {
            body = body.push(
                row![
                    text(record.name.clone()).width(Length::Fill),
                    toolbar_button(
                        "工具详情",
                        Some(tools::message(ToolsMessage::Open(record.clone())))
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    settings_panel(body).into()
}

pub(crate) fn card(app: &App) -> Element<'_, Message> {
    let Some(state) = tools::visible(app) else {
        return column![].into();
    };
    let context = state.context.as_ref().unwrap();
    let close = tools::message(ToolsMessage::Close);
    let header = row![
        column![text("工具详情与使用范围").size(18), text(context.record.name.clone()).size(13)]
            .spacing(5)
            .width(Length::Fill),
        settings_close_button(close.clone()),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let mut body = column![
        text(format!(
            "目录修订 {} · 连接修订 {}",
            context.record.record.revision, context.record.connection_revision
        ))
        .size(12),
        text("调用授权：未在此检查；实际调用仍由执行会话权限与撤销规则判定。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(12)
    .width(Length::Fill);
    if let Some(Ok(catalog)) = crate::app::message::settings::mcp_management_list::visible(app) {
        let mut bound = false;
        for binding in
            catalog.bindings.iter().filter(|binding| binding.record.id == context.record.record.id)
        {
            bound = true;
            body = body.push(
                text(format!(
                    "原生应用 {:?} · {} · {:?} · 修订 {}（上次观察）",
                    binding.target.app_id,
                    if binding.enabled { "期望启用" } else { "期望禁用" },
                    binding.native_status,
                    binding.record.revision
                ))
                .size(12),
            );
        }
        if !bound {
            body = body.push(text("原生应用：未绑定").size(12));
        }
    }
    body = body
        .push(settings_divider())
        .push(text("ACP 入站会话使用范围").size(14))
        .push(
            text("输入需要核验的入站 ACP 会话ID；读取需要该项目及会话的独立授权，不会恢复连接。")
                .size(12)
                .style(settings_muted_text_style),
        )
        .push(
            settings_text_input("入站 ACP 会话ID", &state.acp_session)
                .on_input(|value| tools::message(ToolsMessage::AcpSessionChanged(value))),
        )
        .push(toolbar_button(
            if state.acp_loading { "读取中..." } else { "读取 ACP 挂载" },
            (!state.acp_loading && !state.acp_session.trim().is_empty())
                .then(|| tools::message(ToolsMessage::ReadAcp)),
        ));
    match &state.acp {
        Some(Ok(usage)) => {
            body = body.push(
                text(format!(
                    "ACP 会话 {} · 挂载修订 {}",
                    usage.session_id.as_ref(),
                    usage.mount_revision
                ))
                .size(12),
            );
            let mounts: Vec<_> = usage
                .mounts
                .iter()
                .filter(|mount| {
                    mount.scope == context.record.scope
                        && mount.record.id == context.record.record.id
                })
                .collect();
            if mounts.is_empty() {
                body = body.push(text("ACP：该会话未挂载此目录记录").size(13));
            }
            for mount in mounts {
                let status = if mount.failure.is_some() {
                    "已失效"
                } else if mount.record == context.record.record {
                    "有效引用"
                } else {
                    "非当前目录修订"
                };
                body = body.push(
                    text(format!("ACP：修订 {} · {}", mount.record.revision, status)).size(13),
                );
                if let Some(code) = mount.failure {
                    body = body.push(settings_error_banner(format!("ACP 挂载：{code:?}")));
                }
            }
        }
        Some(Err(error)) => body = body.push(settings_error_banner(error.clone())),
        None => {
            body =
                body.push(text("ACP：尚未读取已验证会话").size(12).style(settings_muted_text_style))
        }
    }
    body = body.push(settings_divider());
    if let Some(result) = &state.result {
        let runtime = match &result.runtime {
            Some(McpRuntimeUsage::NotSelected) => "识矩运行时：项目未选择".to_owned(),
            Some(McpRuntimeUsage::Selected { record, disabled, current, generation }) => format!(
                "识矩运行时：{} · 修订 {} · {} · 代次 {}",
                if *disabled { "已禁用" } else { "已选择" },
                record.revision,
                if *current { "当前有效修订" } else { "修订已失效" },
                generation
            ),
            None => "识矩运行时：未验证".to_owned(),
        };
        body = body.push(text(runtime).size(13));
        if let Some(code) = result.runtime_error {
            body = body.push(settings_error_banner(format!("运行时选择读取失败：{code:?}")));
        }
        if let Some(code) = result.connection_error {
            body = body.push(settings_error_banner(format!("连接检查失败：{code:?}")));
        }
        if let Some(code) = result.tools_error {
            body = body.push(settings_error_banner(format!("工具发现失败：{code:?}")));
        }
        body = body.push(
            text(if result.connection_error.is_some() || result.tools_error.is_some() {
                "工具规格未验证".to_owned()
            } else {
                format!("实际发现 {} 个工具", result.tools.len())
            })
            .size(14),
        );
        for tool in &result.tools {
            body = body.push(settings_panel(
                column![
                    super::tool_summary(tool),
                    text(format!("工具 ID：{}", tool.id.as_ref())).size(12),
                    text(tool.description.clone()).size(12),
                    text(format!(
                        "规格标注：只读 {} · 破坏性 {} · 并发安全 {} · 暴露 {:?}",
                        tool.read_only, tool.destructive, tool.concurrency_safe, tool.exposure
                    ))
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
                "正在取消，等待执行端清理"
            } else if state.terminal {
                "工具详情未验证"
            } else {
                "正在发现工具并读取使用范围"
            })
            .size(13),
        );
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    let mut actions = row![toolbar_button("关闭", Some(close))].spacing(8);
    if !state.terminal {
        actions = actions
            .push(toolbar_button("取消发现", Some(tools::message(ToolsMessage::Cancel))))
            .push(toolbar_button("刷新状态", Some(tools::message(ToolsMessage::Refresh))));
    } else {
        actions = actions.push(toolbar_button(
            "重新发现",
            Some(tools::message(ToolsMessage::Open(context.record.clone()))),
        ));
    }
    settings_modal_layout(header, body, Some(actions.into())).width(860).height(650).into()
}
