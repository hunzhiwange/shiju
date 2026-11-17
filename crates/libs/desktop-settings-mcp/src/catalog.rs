//! Managed catalog presentation and read-only labels.
use crate::toolbar_button;
use desktop_ui_controls::system_settings_common::*;
use iced::{Alignment, Element, Length, widget::{column, row, text}};
use vw_api_mcp_management::*;
#[derive(Debug, Clone)]
pub enum Message { Refresh, Open(Option<McpCatalogRecord>), Import }
/// Managed catalog and connection editing; the surrounding page owns scrolling.
pub fn panel(result: Option<&Result<McpCatalogResponse, String>>, loading: bool) -> Element<'_, Message> {
    let mut body = column![
        content(result, loading),
        toolbar_button("新增受管连接", Some(Message::Open(None))),
        toolbar_button(
            "从原生应用导入",
            Some(Message::Import)
        ),
    ]
    .spacing(8);
    if let Some(Ok(response)) = result {
        for record in &response.records {
            body = body.push(
                row![
                    text(record.name.clone()),
                    toolbar_button(
                        "替换连接定义",
                        Some(Message::Open(Some(record.clone())))
                    )
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    body.into()
}

pub fn content(
    result: Option<
        &Result<McpCatalogResponse, String>,
    >,
    loading: bool,
) -> Element<'_, Message> {
    let mut body = column![
        text("受管 MCP 目录").size(16),
        toolbar_button(
            if loading { "刷新中..." } else { "刷新目录" },
            Some(Message::Refresh)
        ),
        text("原生应用：独立配置分发；识矩运行时：项目选择；ACP：会话挂载。三种使用范围独立。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(10)
    .width(Length::Fill);
    match result {
        Some(Ok(response)) => {
            if response.records.is_empty() {
                body = body.push(text("当前目录为空").size(12));
            }
            for label in rows(response) {
                body = body.push(text(label).size(13));
            }
            body = body.push(
                text("原生状态为上次观察；刷新目录不执行连接或重新核验原生文件。")
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        Some(Err(error)) => {
            body = body.push(settings_error_banner(error.clone()));
        }
        None => {
            body = body.push(
                text(if loading {
                    "正在加载目录"
                } else {
                    "尚未加载，请刷新目录"
                })
                .size(12)
                .style(settings_muted_text_style),
            );
        }
    }
    settings_panel(body).into()
}

pub fn rows(catalog: &McpCatalogResponse) -> Vec<String> {
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
        if let Some(reason) = crate::distribution::unsupported_reason(capability.app_id)
        {
            rows.push(format!("{:?}：{reason}", capability.app_id));
        }
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
