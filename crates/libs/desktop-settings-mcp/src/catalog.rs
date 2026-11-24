//! Managed catalog presentation and read-only labels.
use crate::toolbar_button;
use desktop_ui_controls::system_settings_common::*;
use iced::{Alignment, Element, Length, widget::{column, row, text}};
use vw_api_mcp_management::*;
#[derive(Debug, Clone)]
pub enum Message { Refresh, Open(Option<McpCatalogRecord>), Import }
/// Managed catalog and connection editing; the surrounding page owns scrolling.
pub fn panel(
    i18n: desktop_i18n::I18n,
    result: Option<&Result<McpCatalogResponse, String>>,
    loading: bool,
) -> Element<'_, Message> {
    let mut body = column![
        content(i18n, result, loading),
        toolbar_button(i18n.__("新增受管连接"), Some(Message::Open(None))),
        toolbar_button(i18n.__("从原生应用导入"), Some(Message::Import)),
    ]
    .spacing(8);
    if let Some(Ok(response)) = result {
        for record in &response.records {
            body = body.push(
                row![
                    text(record.name.clone()),
                    toolbar_button(
                        i18n.__("替换连接定义"),
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
    i18n: desktop_i18n::I18n,
    result: Option<&Result<McpCatalogResponse, String>>,
    loading: bool,
) -> Element<'_, Message> {
    let mut body =
        column![
            text(i18n.__("受管 MCP 目录")).size(16),
            toolbar_button(
                if loading { i18n.__("刷新中...") } else { i18n.__("刷新目录") },
                Some(Message::Refresh)
            ),
            text(i18n.__(
                "原生应用：独立配置分发；识矩运行时：项目选择；ACP：会话挂载。三种使用范围独立。"
            ))
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(10)
    .width(Length::Fill);
    match result {
        Some(Ok(response)) => {
            if response.records.is_empty() {
                body = body.push(text(i18n.__("当前目录为空")).size(12));
            }
            for label in rows(i18n, response) {
                body = body.push(text(label).size(13));
            }
            body = body.push(
                text(i18n.__("原生状态为上次观察；刷新目录不执行连接或重新核验原生文件。"))
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        Some(Err(error)) => {
            body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
                i18n,
                &error.clone(),
            )));
        }
        None => {
            body = body.push(
                text(if loading {
                    i18n.__("正在加载目录")
                } else {
                    i18n.__("尚未加载，请刷新目录")
                })
                .size(12)
                .style(settings_muted_text_style),
            );
        }
    }
    settings_panel(body).into()
}

pub fn rows(i18n: desktop_i18n::I18n, catalog: &McpCatalogResponse) -> Vec<String> {
    let mut rows = Vec::new();
    for record in &catalog.records {
        rows.push(
            desktop_i18n::__!(
                i18n,
                "%s · %s · 修订 %s",
                (record.name).to_string(),
                (format!("{:?}", record.transport)).to_string(),
                (record.record.revision).to_string()
            )
            .expect("valid extension display template"),
        );
        let mut bound = false;
        for binding in
            catalog.bindings.iter().filter(|binding| binding.record.id == record.record.id)
        {
            bound = true;
            let status = match binding.native_status {
                McpNativeStatus::Present => i18n.__("存在"),
                McpNativeStatus::Missing => i18n.__("缺失"),
                McpNativeStatus::Unreadable => i18n.__("无法读取"),
                McpNativeStatus::NotVerified => i18n.__("未验证"),
                McpNativeStatus::Unsupported => i18n.__("不支持"),
            };
            let desired =
                if binding.enabled { i18n.__("期望启用") } else { i18n.__("期望禁用") };
            let revision = if binding.record.revision == record.record.revision {
                i18n.__("当前修订")
            } else {
                i18n.__("旧修订")
            };
            rows.push(
                desktop_i18n::__!(
                    i18n,
                    "原生应用 %s · %s · %s · %s · %s（上次观察）",
                    (format!("{:?}", binding.target.app_id)).to_string(),
                    (binding.target.installation_id.0).to_string(),
                    (desired).to_string(),
                    (revision).to_string(),
                    (status).to_string()
                )
                .expect("valid extension display template"),
            );
        }
        if !bound {
            rows.push(i18n.__("原生应用：未绑定").into());
        }
        rows.push(i18n.__("识矩运行时：项目选择状态未提供；ACP：会话挂载状态未提供").into());
    }
    for capability in &catalog.capabilities {
        if let Some(reason) = crate::distribution::unsupported_reason(capability.app_id) {
            rows.push(format!("{:?}: {}", capability.app_id, i18n.__(reason)));
        }
        let label = |support| match support {
            McpSupport::Supported => i18n.__("支持"),
            McpSupport::Unsupported => i18n.__("不支持"),
            McpSupport::NotVerified => i18n.__("未验证"),
        };
        rows.push(
            desktop_i18n::__!(
                i18n,
                "原生应用 %s 能力 · stdio %s · HTTP %s · SSE %s",
                (format!("{:?}", capability.app_id)).to_string(),
                (label(capability.stdio)).to_string(),
                (label(capability.streamable_http)).to_string(),
                (label(capability.sse)).to_string()
            )
            .expect("valid extension display template"),
        );
    }
    rows
}
