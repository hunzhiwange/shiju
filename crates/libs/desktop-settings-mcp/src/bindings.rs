//! Native binding matrix over visible catalog state and trusted installation observations.
use crate::toolbar_button;
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length, widget::{checkbox, column, row, text}};
use vw_api_app_identity::{AppId, AppInstallation, AppInstallationEnvironment};
use vw_api_mcp_management::*;
#[derive(Debug, Clone)]
pub enum BindingsMessage { Select(McpCatalogRecord, bool), Apply(McpAppTarget, bool), RetryFailed }
type Message = BindingsMessage;
fn message(value: BindingsMessage) -> Message { value }
#[derive(Clone, Copy)]
pub struct BindingsView<'a> {
    pub selected: &'a [McpCatalogRecord], pub running: bool,
    pub result: &'a Option<Result<McpDistributionResult, String>>,
}
pub fn panel<'a>(catalog: Option<&'a Result<McpCatalogResponse, String>>, state: Option<BindingsView<'a>>, installations: Vec<&AppInstallation>, host: &str, loading: bool) -> Element<'a, Message> {
    let Some(Ok(catalog)) = catalog else {
        return text("加载目录后可管理原生应用绑定").into();
    };
    let empty: &[McpCatalogRecord] = &[];
    let selected = state.map(|s| s.selected).unwrap_or(empty);
    let running = state.is_some_and(|s| s.running);
    let mut body = column![text("原生应用绑定矩阵").size(16), text("选择一个目录项进行单项启停，或选择同一作用域的多项批量启停。运行时与 ACP 仍在各自页面独立选择。刷新目录不会执行原生写入。").size(12).style(settings_muted_text_style)].spacing(10).width(Length::Fill);
    for record in &catalog.records {
        let record_for_toggle = record.clone();
        body = body.push(
            checkbox(selected.contains(record))
                .label(format!("{} · 修订 {}", record.name, record.record.revision))
                .style(settings_checkbox_style)
                .on_toggle_maybe((!running).then_some(move |value| {
                    message(BindingsMessage::Select(record_for_toggle.clone(), value))
                })),
        );
    }
    if running {
        body = body.push(text("正在分发，等待逐项目标结果...").size(12));
    }
    for app_id in crate::distribution::APPS {
        body = body.push(text(format!("{app_id:?}")).size(14));
        if let Some(reason) = crate::distribution::unsupported_reason(app_id) {
            body = body.push(text(reason).size(12));
            continue;
        }
        let mut found = false;
        {
            for installation in &installations {
                        if installation.app_id != app_id
                            || installation.target_host_id != host
                            || !matches!(
                                installation.environment,
                                AppInstallationEnvironment::Macos
                            )
                        {
                            continue;
                        }
                        found = true;
                        let target = McpAppTarget {
                            app_id,
                            installation_id: installation.id.clone(),
                            target_host_id: installation.target_host_id.clone(),
                        };
                        let enabled = !running
                            && !loading
                            && selected.iter().all(|record| catalog.records.contains(record))
                            && crate::distribution::supported(catalog, selected, app_id)
                            && crate::distribution::project(selected).is_ok();
                        let label = if selected.len() > 1 { "批量启用" } else { "启用" };
                        let disable = if selected.len() > 1 { "批量禁用" } else { "禁用" };
                        body = body.push(
                            row![
                                text(installation.id.0.clone()).size(12),
                                toolbar_button(
                                    label,
                                    enabled.then(|| message(BindingsMessage::Apply(
                                        target.clone(),
                                        true
                                    )))
                                ),
                                toolbar_button(
                                    disable,
                                    enabled.then(|| message(BindingsMessage::Apply(
                                        target.clone(),
                                        false
                                    )))
                                )
                            ]
                            .spacing(8)
                            .wrap(),
                        );
                        for record in selected {
                            let observed = catalog.bindings.iter().find(|binding| {
                                binding.target == target && binding.record.id == record.record.id
                            });
                            let label = match observed {
                                Some(binding) => format!(
                                    "{} · 期望{} · 修订 {} · 原生 {:?}（上次观察）",
                                    record.name,
                                    if binding.enabled { "启用" } else { "禁用" },
                                    binding.record.revision,
                                    binding.native_status
                                ),
                                None => format!("{} · 未绑定 · 原生未验证", record.name),
                            };
                            body = body.push(text(label).size(12).style(settings_muted_text_style));
                        }
            }
        }
        if !found {
            body = body.push(
                text("尚无当前主机的可信 macOS 安装实例，请在应用管理中刷新")
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        if !selected.is_empty() && !crate::distribution::supported(catalog, selected, app_id) {
            body = body.push(
                text(
                    if app_id == AppId::Agy
                        && selected.iter().any(|r| r.transport == McpTransport::Sse)
                    {
                        "AGY 不支持 SSE 原生分发"
                    } else {
                        "所选传输不支持或能力尚未验证"
                    },
                )
                .size(12),
            );
        }
    }
    if let Some(result) = state.and_then(|s| s.result.as_ref()) {
        match result {
            Ok(result) => {
                body = body.push(text(crate::distribution::summary(result)).size(14));
                if result.items.iter().any(|item| {
                    matches!(item.native_outcome, McpDistributionOutcome::Failed { .. })
                }) {
                    body = body.push(toolbar_button(
                        "仅重试失败项",
                        (!running && !loading)
                            .then(|| message(BindingsMessage::RetryFailed)),
                    ));
                }
                for item in &result.items {
                    body = body.push(text(crate::distribution::item_label(item)).size(12));
                }
            }
            Err(error) => body = body.push(settings_error_banner(error.clone())),
        }
    }
    settings_panel(body).into()
}
