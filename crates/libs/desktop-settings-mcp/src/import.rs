//! Confirmation presentation receives redacted labels and explicit trusted targets.
use crate::{
    management_bindings::matrix::{APPS, app_label},
    management_import::SourceReport,
    toolbar_button,
};
use desktop_assets::assets::Icon;
use desktop_settings_apps::system_settings_apps::list::brand_icon;
use desktop_ui_controls::system_settings_common::{icons::icon_btn_svg_style, *};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text},
};
use vw_api_app_identity::AppId;
use vw_api_mcp_management::*;
#[derive(Clone)]
pub enum ImportMessage {
    Close,
    Select(McpAppTarget),
    Account(String),
    Preview,
    Paste,
    Confirm,
    SelectAll,
    ToggleSources,
    Retry,
}
type Message = ImportMessage;
fn message(value: ImportMessage) -> Message {
    value
}
pub struct ImportView<'a> {
    pub importing: bool,
    pub loading: bool,
    pub link_drafts: &'a [Result<McpCreateRequest, String>],
    pub draft: &'a Option<McpCreateRequest>,
    pub source: &'a Option<McpAppTarget>,
    pub all: bool,
    pub show_sources: bool,
    pub can_retry: bool,
    pub account: &'a str,
    pub has_preview: bool,
    pub can_confirm: bool,
    pub result: &'a Option<Result<Vec<String>, String>>,
}
pub fn panel<'a>(i18n: desktop_i18n::I18n,
    state: ImportView<'a>,
    targets: Vec<McpAppTarget>,
    preview_labels: Vec<String>,
    reports: &'a [SourceReport],
) -> Element<'a, Message> {
    let idle = !state.importing && !state.loading;
    if let Some(Ok(labels)) = state.result {
        return completion(i18n, &state, labels, reports, idle);
    }
    let mut body = column![].spacing(11).width(Length::Fill);
    if !state.link_drafts.is_empty() {
        body = body.push(text(desktop_i18n::__!(i18n, "MCP 深链：%s 项", (state.link_drafts.len()).to_string()).expect("valid extension display template")))
            .push(text(i18n.__("仅保存当前租户目录定义；链接指定的应用与 enabled 不授予执行或分发权限。连接参数已隐藏。")));
        for (index, entry) in state.link_drafts.iter().enumerate() {
            body = body.push(text(desktop_i18n::__!(i18n, "第 %s 项 · %s", (index + 1).to_string(), (if entry.is_ok() {
                    i18n.__("等待确认")
                } else {
                    i18n.__("格式不支持，确认后列为失败")
                }).to_string()).expect("valid extension display template")));
        }
    } else if let Some(draft) = &state.draft {
        let transport = match draft.connection {
            McpConnectionDefinition::Stdio { .. } => "stdio",
            McpConnectionDefinition::StreamableHttp { .. } => "HTTP",
            McpConnectionDefinition::Sse { .. } => "SSE",
        };
        body = body.push(text(i18n.__("深链草稿确认")).size(18))
            .push(text(format!("{} · {} · {transport}", draft.id, draft.name.as_deref().unwrap_or(i18n.__("未命名")))))
            .push(text(desktop_i18n::__!(i18n, "作用域：%s", draft.project_id.as_ref().map(|id| desktop_i18n::__!(i18n, "项目 %s", id.0.as_str()).expect("valid project scope template")).unwrap_or_else(|| i18n.__("当前租户").to_owned())).expect("valid scope template")))
            .push(text(i18n.__("连接参数及凭据值已隐藏。定义可能包含命令、内网地址或环境变量；请仅确认可信来源。已有同名目录项将明确报告冲突，不会覆盖。")));
    } else {
        body = body
            .push(text(i18n.__("选择导入来源")).size(16))
            .push(text(i18n.__("确认将导入所选应用当前全部 MCP 定义；确认时会重新读取应用配置。")).size(12));
        body = body.push(toolbar_button(
            if state.all {
                i18n.__("已选择全部可导入来源")
            } else {
                i18n.__("选择全部可导入来源")
            },
            idle.then_some(ImportMessage::SelectAll),
        ));
        let mut choices = row![].spacing(7);
        for target in targets {
            let selected = state.all || state.source.as_ref() == Some(&target);
            choices = choices.push(
                button(
                    row![
                        brand_icon(target.app_id, 16.0),
                        column![
                            text(app_label(target.app_id)).size(13),
                            text(target.installation_id.0.clone())
                                .size(11)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(2),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                )
                .padding([7, 11])
                .style(move |theme, status| settings_segment_button_style(theme, status, selected))
                .on_press_maybe(idle.then(|| message(ImportMessage::Select(target)))),
            );
        }
        body = body.push(choices.wrap());
        body = body.push(
            text(i18n.__("Claude Desktop 与 OpenClaw 不支持原生导入。没有安装实例时请先在应用管理中刷新。"))
                .size(12),
        );
        if state.all || state.source.as_ref().is_some_and(|t| t.app_id == AppId::Claude) {
            body = body
                .push(text(i18n.__("受管 Claude 官方账号 ID（仅引用）")).size(12))
                .push(settings_text_input(i18n.__("账号 ID"), state.account).on_input_maybe(
                    idle.then_some(|value| message(ImportMessage::Account(value))),
                ));
        }
        body = body.push(
            row![
                toolbar_button(
                    if state.loading { i18n.__("预览中...") } else { i18n.__("读取脱敏预览") },
                    idle.then(|| message(ImportMessage::Preview))
                ),
                toolbar_button(i18n.__("粘贴深链草稿"), idle.then(|| message(ImportMessage::Paste)))
            ]
            .spacing(8)
            .wrap(),
        );
        body = body.push(text(i18n.__("粘贴入口接收已解码的 MCP 创建草稿 JSON；系统深链由统一入口解析。不会打开链接、下载配置或读取本机配置文件。")).size(12).style(settings_muted_text_style));
        if state.has_preview {
            for label in preview_labels {
                body = body.push(text(desktop_i18n::settings_extensions::feedback(i18n, &label)).size(13));
            }
        }
    }
    if let Some(Err(error)) = state.result {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(i18n, &error.clone())));
    }
    let header = settings_modal_header(
        text(i18n.__("从应用导入")).size(20),
        settings_close_button(ImportMessage::Close),
    );
    let footer = row![
        Space::new().width(Length::Fill),
        toolbar_button(i18n.__("取消"), (!state.importing).then_some(ImportMessage::Close)),
        button(text(if state.importing { i18n.__("导入中…") } else { i18n.__("确认导入") }).size(13))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe(state.can_confirm.then_some(ImportMessage::Confirm)),
    ]
    .spacing(8);
    settings_modal_layout(header, body, Some(footer.into())).width(550).height(520).into()
}

fn completion<'a>(i18n: desktop_i18n::I18n,
    state: &ImportView<'a>,
    labels: &'a [String],
    reports: &'a [SourceReport],
    idle: bool,
) -> Element<'a, Message> {
    let mut body = column![].spacing(12);
    if reports.is_empty() {
        for label in labels {
            body = body.push(text(desktop_i18n::settings_extensions::feedback(i18n, label)).size(14));
        }
    } else {
        let added: usize = reports.iter().map(|r| r.added).sum();
        let bound: usize = reports.iter().map(|r| r.bindings).sum();
        let skipped: usize = reports.iter().map(|r| r.skipped).sum();
        let failed: usize = reports.iter().map(|r| r.failed).sum();
        let headline = match (added, failed) {
            (0, 0) => i18n.__("没有发现新的 MCP。").to_owned(),
            (0, _) => i18n.__("没有新增 MCP，有来源导入失败。").to_owned(),
            (_, _) => desktop_i18n::__!(i18n, "新增 %s 个 MCP。", (added).to_string()).expect("valid extension display template"),
        };
        body = body.push(text(headline).size(15)).push(
            text(desktop_i18n::__!(i18n, "绑定变更 %s 项 · 跳过 %s 项 · 失败 %s 项", (bound).to_string(), (skipped).to_string(), (failed).to_string()).expect("valid extension display template"))
                .size(13)
                .style(settings_muted_text_style),
        );
        if bound > 0 {
            body = body.push(
                text(i18n.__("同定义的 MCP 已关联到来源应用，目录中的连接定义保持原样。"))
                    .size(13)
                    .style(settings_muted_text_style),
            );
        }

        let disclosure = button(
            row![
                icon_svg(
                    if state.show_sources { Icon::ChevronDown } else { Icon::ChevronRight },
                    14.0
                )
                .style(icon_btn_svg_style),
                text(i18n.__("读取了哪些配置")).size(14),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([7, 8])
        .style(rounded_action_btn_style)
        .on_press_maybe(idle.then_some(ImportMessage::ToggleSources));
        body = body.push(Space::new().height(3)).push(disclosure);
        if state.show_sources {
            let mut details = column![].spacing(10);
            for app in APPS {
                for report in reports.iter().filter(|report| report.target.app_id == app) {
                    let repeated = reports.iter().filter(|r| r.target.app_id == app).count() > 1;
                    let mut name =
                        column![text(app_label(app)).size(13).style(settings_muted_text_style)];
                    if repeated {
                        name = name.push(
                            text(report.target.installation_id.0.as_str())
                                .size(11)
                                .style(settings_muted_text_style),
                        );
                    }
                    let (file, note) = config_location(app);
                    let mut config = column![
                        text(i18n.__(file))
                            .size(12)
                            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                    ]
                    .spacing(3)
                    .width(Length::Fill);
                    if let Some(note) = note {
                        config = config.push(text(i18n.__(note)).size(11).style(settings_muted_text_style));
                    }
                    for label in &report.labels {
                        config = config.push(text(desktop_i18n::settings_extensions::feedback(i18n, label)).size(12).style(settings_muted_text_style));
                    }
                    details = details.push(row![name.width(120), config].spacing(10));
                }
            }
            body = body.push(
                text(i18n.__("文件列显示适配器的默认位置，读取与导入结果见各行。"))
                    .size(12)
                    .style(settings_muted_text_style),
            );
            body = body.push(container(details).padding([12, 14]).width(Length::Fill).style(
                |theme: &iced::Theme| {
                    iced::widget::container::Style::default()
                        .background(theme.extended_palette().background.weak.color)
                        .border(iced::border::rounded(10))
                },
            ));
        }
    }
    // Reserve the footer before laying out long source or deep-link results.
    let long_result =
        (!reports.is_empty() && state.show_sources) || (reports.is_empty() && labels.len() > 6);
    let body = scrollable(container(body).padding([0, 7]).width(Length::Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(if long_result { Length::Fill } else { Length::Shrink });
    let mut footer = row![Space::new().width(Length::Fill)].spacing(8).align_y(Alignment::Center);
    if state.can_retry {
        footer = footer.push(toolbar_button(i18n.__("重试失败来源"), idle.then_some(ImportMessage::Retry)));
    }
    footer = footer.push(
        button(text(if state.importing { i18n.__("导入中…") } else { i18n.__("知道了") }).size(14))
            .padding([9, 17])
            .style(primary_action_btn_style)
            .on_press_maybe(idle.then_some(ImportMessage::Close)),
    );
    settings_modal_card(
        column![container(text(i18n.__("导入完成")).size(22)).padding([0, 7]), body, footer,].spacing(17),
    )
    .width(530)
    .height(if long_result { Length::Fixed(600.0) } else { Length::Shrink })
    .into()
}

/// Default path patterns from the native adapters. Resolved paths stay on the execution host.
fn config_location(app: AppId) -> (&'static str, Option<&'static str>) {
    match app {
        AppId::Claude => (".claude.json", Some("受管 Claude 账号配置 · 顶层 mcpServers")),
        AppId::Codex => ("~/.codex/config.toml", None),
        AppId::Gemini => ("~/.gemini/settings.json", None),
        AppId::GrokBuild => ("~/.grok/config.toml", None),
        AppId::OpenCode => ("~/.config/opencode/opencode.json", Some("也支持 opencode.jsonc")),
        AppId::Hermes => ("~/.hermes/config.yaml", None),
        AppId::Pi => ("~/.pi/agent/mcp.json", None),
        AppId::Mcode => ("~/.minimax/mcp.json", None),
        AppId::Kimi => ("~/.kimi-code/mcp.json", None),
        AppId::Dsh => ("~/.dsh/profiles/<profile>/cordis.patch.yml", Some("已初始化的配置档")),
        AppId::Agy => ("~/.gemini/config/mcp_config.json", None),
        AppId::Omp => ("~/.omp/agent/mcp.json", None),
        AppId::ClaudeDesktop | AppId::OpenClaw => ("", None),
    }
}

impl std::fmt::Debug for ImportMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ImportMessage { .. }")
    }
}
