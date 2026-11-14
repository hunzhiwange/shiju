//! Redacted import confirmation shares the settings modal's fixed header and footer.
use super::toolbar_button;
use crate::app::{
    App, Message,
    components::system_settings_common::*,
    message::settings::mcp_management_import::{
        self as import, ImportMessage, ImportState, message,
    },
};
use iced::{
    Element, Length,
    widget::{button, column, row, text},
};
use vw_gateway_client::vw_api_types::{agent_apps::AppId, mcp_management::McpConnectionDefinition};

pub(crate) fn panel<'a>(app: &App, state: &'a ImportState) -> Element<'a, Message> {
    let idle = !state.importing && !state.loading;
    let mut body = column![text(format!("目标主机：{}", state.context.host)), text("原生文件由授权目标主机读取；剪贴板只读取本机。取消确认不写入；确认只导入目录，不改写原生文件、不连接服务、不启用运行时或 ACP。").size(12).style(settings_muted_text_style)].spacing(10).width(Length::Fill);
    if !state.link_drafts.is_empty() {
        body = body.push(text(format!("MCP 深链：{} 项", state.link_drafts.len())))
            .push(text("仅保存当前租户目录定义；链接指定的应用与 enabled 不授予执行或分发权限。连接参数已隐藏。"));
        for (index, entry) in state.link_drafts.iter().enumerate() {
            body = body.push(text(format!("第 {} 项 · {}", index + 1,
                if entry.is_ok() { "等待确认" } else { "格式不支持，确认后列为失败" }))));
        }
    } else if let Some(draft) = &state.draft {
        let transport = match draft.connection {
            McpConnectionDefinition::Stdio { .. } => "stdio",
            McpConnectionDefinition::StreamableHttp { .. } => "HTTP",
            McpConnectionDefinition::Sse { .. } => "SSE",
        };
        body = body.push(text("深链草稿确认").size(18))
            .push(text(format!("{} · {} · {transport}", draft.id, draft.name.as_deref().unwrap_or("未命名"))))
            .push(text(format!("作用域：{}", draft.project_id.as_ref().map(|id| format!("项目 {}", id.0)).unwrap_or_else(|| "当前租户".into()))))
            .push(text("连接参数及凭据值已隐藏。定义可能包含命令、内网地址或环境变量；请仅确认可信来源。已有同名目录项将明确报告冲突，不会覆盖。"));
    } else {
        body = body.push(text("选择可信 macOS 安装实例").size(16)).push(
            text(
                "确认将导入所选应用当前全部 MCP 定义；预览不是文件快照，确认时由目标主机重新读取。",
            )
            .size(12),
        );
        for target in import::targets(app) {
            let selected = state.source.as_ref() == Some(&target);
            body = body.push(
                button(text(format!(
                    "{}{:?} · {}",
                    if selected { "已选择 · " } else { "" },
                    target.app_id,
                    target.installation_id.0
                )))
                .padding([7, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe(idle.then(|| message(ImportMessage::Select(target)))),
            );
        }
        body = body.push(
            text("Claude Desktop 与 OpenClaw 不支持原生导入。没有安装实例时请先在应用管理中刷新。")
                .size(12),
        );
        if state.source.as_ref().is_some_and(|t| t.app_id == AppId::Claude) {
            body = body
                .push(text("受管 Claude 官方账号 ID（仅引用）").size(12))
                .push(settings_text_input("账号 ID", &state.account).on_input_maybe(
                    idle.then_some(|value| message(ImportMessage::Account(value))),
                ));
        }
        body = body.push(
            row![
                toolbar_button(
                    if state.loading { "预览中..." } else { "读取脱敏预览" },
                    idle.then(|| message(ImportMessage::Preview))
                ),
                toolbar_button("粘贴深链草稿", idle.then(|| message(ImportMessage::Paste)))
            ]
            .spacing(8)
            .wrap(),
        );
        body = body.push(text("粘贴入口接收已解码的 MCP 创建草稿 JSON；系统深链由统一入口解析。不会打开链接、下载配置或读取本机配置文件。").size(12).style(settings_muted_text_style));
        if let Some(preview) = &state.preview {
            for label in import::preview_labels(preview) {
                body = body.push(text(label).size(13));
            }
        }
    }
    if let Some(result) = &state.result {
        match result {
            Ok(labels) => {
                for label in labels {
                    body = body.push(text(label.clone()).size(13));
                }
            }
            Err(error) => body = body.push(settings_error_banner(error.clone())),
        }
    }
    let close = message(ImportMessage::Close);
    let header =
        settings_modal_header(text("MCP 导入确认").size(20), settings_close_button(close.clone()));
    let footer = row![
        toolbar_button(
            if state.result.is_some() { "关闭" } else { "取消" },
            (!state.importing).then_some(close)
        ),
        button(text(if state.importing { "导入中..." } else { "确认导入目录" }))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe(state.can_confirm().then(|| message(ImportMessage::Confirm)))
    ]
    .spacing(8);
    settings_modal_layout(header, body, Some(footer.into())).width(680).height(620).into()
}
