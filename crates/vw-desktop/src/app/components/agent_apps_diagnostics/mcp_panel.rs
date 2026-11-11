//! MCP handshake evidence is separate from production connection state.
use super::super::system_settings_common::*;
use crate::app::{
    App, Message,
    message::{
        agent_apps_diagnostics::{self as diagnostics, AppChoice, McpDiagnosticMessage as Event},
        settings::mcp_management_list as catalog,
    },
};
use iced::{
    Element,
    widget::{button, column, row, text},
};

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.mcp_settings.diagnostics;
    let current = diagnostics::context(app);
    let visible = current.is_some() && current.as_ref() == state.context.as_ref();
    let active = visible && state.active();
    let control = |label: &'static str, event: Option<Event>| {
        button(text(label))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(event.map(diagnostics::message))
    };
    let choices: Vec<_> =
        app.apps_catalog.rows.iter().map(|r| AppChoice(r.entry.id, r.entry.name.clone())).collect();
    let mut body = column![
        text("MCP 连通检测").size(16),
        text("仅隔离握手，不执行工具；检测结果不代表生产连接状态。")
            .size(12)
            .style(settings_muted_text_style),
        settings_select(choices, state.app.clone(), |choice| diagnostics::message(Event::Select(
            choice
        )))
        .disabled(active),
    ]
    .spacing(8);
    if let Some(Ok(catalog)) = catalog::visible(app) {
        body = body.push(control(
            "检测全部",
            (!active && !catalog.records.is_empty()).then(|| {
                Event::Start(catalog.records.iter().map(|r| r.record.id.clone()).collect())
            }),
        ));
        for record in &catalog.records {
            body = body.push(
                row![
                    text(format!("{} · {}", record.name, record.record.id.0)),
                    control(
                        "检测",
                        (!active).then(|| Event::Start(vec![record.record.id.clone()]))
                    )
                ]
                .spacing(8),
            );
        }
    }
    if visible {
        body = body.push(diagnostics::mcp_results::view(state));
        if active {
            body = body.push(
                row![
                    control(
                        "取消检测",
                        (!state.cancelling && state.job.is_some()).then_some(Event::Cancel)
                    ),
                    control("刷新检测", state.error.as_ref().map(|_| Event::Refresh))
                ]
                .spacing(8),
            );
        }
        if let Some(error) = &state.error {
            body = body.push(settings_error_banner(error.clone()));
        }
    }
    body.into()
}

#[cfg(test)]
#[path = "mcp_panel_tests.rs"]
mod tests;
