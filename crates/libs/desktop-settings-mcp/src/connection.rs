//! Global MCP editor and direct synchronization modal.
use crate::{
    form_field,
    management_form::{Draft, FormMessage as M},
    toolbar_button,
};
use desktop_settings_apps::system_settings_apps::list::brand_icon;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, checkbox, column, container, row, text, text_editor},
};
use vw_api_app_identity::AppId;
use vw_api_mcp_management::{McpConfigValue, McpFailureCode};

pub fn connection_form_panel(i18n: desktop_i18n::I18n, draft: &Draft) -> Element<'_, M> {
    let editable = !draft.saving && draft.retry.is_empty() && draft.deliveries.is_empty();
    let input = |label, field, value: &str| {
        form_field(
            label,
            settings_text_input("", value)
                .on_input_maybe(editable.then_some(move |value| M::Field(field, value))),
        )
    };
    let mut body = column![].spacing(14).width(Length::Fill);
    if draft.sync_only {
        body = body.push(text(draft.id.as_str()).size(16));
    } else {
        if !draft.presets.is_empty() {
            let presets = row(draft.presets.iter().enumerate().map(|(index, preset)| {
                button(text(preset.name.as_str()).size(13))
                    .padding([6, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(editable.then_some(M::Preset(index)))
                    .into()
            }))
            .spacing(7)
            .wrap();
            body = body.push(form_field(i18n.__("从模板开始"), presets));
        }
        let paste = text_editor(&draft.paste)
            .placeholder(i18n.__("粘贴 JSON、Codex TOML 或 OpenCode 配置"))
            .height(78)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_editor_style);
        body = body.push(form_field(
            i18n.__("粘贴配置"),
            column![
                if editable { paste.on_action(M::PasteEditor) } else { paste },
                toolbar_button(
                    i18n.__("解析配置"),
                    (editable && !draft.paste.text().trim().is_empty()).then_some(M::ParsePaste)
                )
            ]
            .spacing(6),
        ));
        for (index, candidate) in draft.candidates.iter().enumerate() {
            body = body.push(
                row![
                    text(candidate.id.as_str())
                        .width(Length::Fill)
                        .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                    toolbar_button(
                        i18n.__("填入草稿"),
                        (editable && candidate.connection.is_ok()).then_some(M::ChoosePaste(index))
                    ),
                    toolbar_button(i18n.__("跳过"), editable.then_some(M::SkipPaste(index))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        body = body.push(form_field(
            i18n.__("服务器名称"),
            settings_text_input(i18n.__("例如 context7"), &draft.id)
                .on_input_maybe((editable && !draft.editing()).then_some(|id| M::Field("id", id))),
        ));
        body = body.push(
            row![
                settings_field_label(i18n.__("连接配置")),
                Space::new().width(Length::Fill),
                segment(
                    i18n.__("表单"),
                    !draft.json,
                    (editable && draft.json).then_some(M::Json(false))
                ),
                segment("JSON", draft.json, (editable && !draft.json).then_some(M::Json(true))),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
        if draft.json {
            let editor = text_editor(&draft.editor)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .height(300)
                .style(settings_text_editor_style);
            body = body.push(if editable { editor.on_action(M::Editor) } else { editor });
        } else {
            let transport = |label, kind: &'static str| {
                segment(
                    label,
                    draft.transport == kind,
                    (editable && draft.transport != kind)
                        .then(|| M::Field("transport", kind.into())),
                )
            };
            body = body.push(form_field(
                i18n.__("连接方式"),
                row![
                    transport(i18n.__("本地命令（stdio）"), "stdio"),
                    transport("HTTP", "streamable_http"),
                    transport("SSE", "sse"),
                ]
                .spacing(4)
                .wrap(),
            ));
            if draft.transport == "stdio" {
                body = body.push(input(i18n.__("命令"), "command", &draft.command));
                body = body.push(settings_field_label(i18n.__("参数")));
                let args: Vec<String> = serde_json::from_str(&draft.args).unwrap_or_default();
                for (index, arg) in args.iter().enumerate() {
                    body = body.push(
                        row![
                            settings_text_input(i18n.__("独立参数（保留空格）"), arg)
                                .on_input_maybe(
                                    editable.then_some(move |value| M::ArgSet(index, value))
                                ),
                            toolbar_button("×", editable.then_some(M::ArgRemove(index))),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    );
                }
                body =
                    body.push(toolbar_button(i18n.__("+ 添加参数"), editable.then_some(M::ArgAdd)));
            } else {
                body = body.push(input("URL", "url", &draft.url));
            }
            let stdio = draft.transport == "stdio";
            body = body.push(settings_field_label(i18n.__(if stdio {
                "环境变量"
            } else {
                "请求头"
            })));
            let values: std::collections::BTreeMap<String, McpConfigValue> =
                serde_json::from_str(&draft.values).unwrap_or_default();
            for (name, field) in values {
                let key = name.clone();
                let value_key = name.clone();
                let value = match field {
                    McpConfigValue::Literal { value } => value,
                    McpConfigValue::Credential { .. } => String::new(),
                };
                body = body.push(
                    row![
                        settings_text_input(i18n.__("字段名"), &name)
                            .width(Length::FillPortion(1))
                            .on_input_maybe(
                                editable.then_some(move |value| M::ValueName(key.clone(), value))
                            ),
                        settings_text_input(i18n.__("值"), &value)
                            .width(Length::FillPortion(2))
                            .on_input_maybe(
                                editable
                                    .then_some(move |value| M::ValueText(value_key.clone(), value))
                            ),
                        toolbar_button("×", editable.then_some(M::ValueRemove(name))),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .wrap(),
                );
            }
            body = body.push(toolbar_button(
                i18n.__(if stdio { "+ 添加环境变量" } else { "+ 添加请求头" }),
                editable.then_some(M::ValueAdd),
            ));
        }
        body = body.push(
            row![
                checkbox(draft.global.enabled)
                    .label(i18n.__("启用"))
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(editable.then_some(M::GlobalEnabled)),
                checkbox(draft.global.required)
                    .label(i18n.__("必需"))
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(editable.then_some(M::GlobalRequired)),
            ]
            .spacing(12)
            .wrap(),
        );
        body = body.push(input(i18n.__("超时（秒）"), "timeout", &draft.global.timeout));
    }

    body = body.push(sync_targets(i18n, draft, editable));
    if !draft.retry.is_empty() {
        body =
            body.push(toolbar_button(i18n.__("重试失败目标"), (!draft.saving).then_some(M::Retry)));
    }
    if let Some(error) = &draft.error {
        body = body
            .push(settings_error_banner(desktop_i18n::settings_extensions::feedback(i18n, error)));
    }

    let title = if draft.sync_only {
        "同步到第三方"
    } else if draft.editing() {
        "编辑 MCP"
    } else {
        "添加 MCP"
    };
    let close: Element<'_, M> = if draft.saving {
        Space::new().width(32).into()
    } else {
        settings_close_button(M::Close).into()
    };
    let header = settings_modal_header(text(i18n.__(title)).size(SETTINGS_PAGE_TITLE_SIZE), close);
    let action = if draft.saving {
        if draft.sync_only { "同步中…" } else { "保存中…" }
    } else if draft.sync_only {
        "同步到第三方"
    } else if draft.editing() {
        "保存"
    } else {
        "添加 MCP"
    };
    let footer = row![
        Space::new().width(Length::Fill),
        toolbar_button(
            i18n.__(if draft.sync_only { "关闭" } else { "取消" }),
            (!draft.saving).then_some(M::Close)
        ),
        button(text(i18n.__(action)).size(13))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (editable
                    && (!draft.sync_only || draft.targets.iter().any(|(_, selected)| *selected)))
                .then_some(M::Save)
            ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    settings_modal_layout(header, body, Some(footer.into()))
        .width(if draft.sync_only { 590 } else { 720 })
        .height(if draft.sync_only { 540 } else { 620 })
        .into()
}

fn sync_targets<'a>(i18n: desktop_i18n::I18n, draft: &'a Draft, editable: bool) -> Element<'a, M> {
    let mut body = column![settings_group_header(settings_field_label(i18n.__("同步到")))]
        .spacing(10)
        .width(Length::Fill);
    if draft.targets.is_empty() {
        let hint = if draft.loading {
            "正在读取可同步的应用…"
        } else {
            "未发现可同步的应用"
        };
        return body.push(text(i18n.__(hint)).size(12).style(settings_muted_text_style)).into();
    }
    let mut choices = column![].spacing(10);
    for pair in draft.targets.chunks(2) {
        let mut line = row![].spacing(12);
        for (info, selected) in pair {
            let app = info.app_id;
            let supported = draft.target_supported(info);
            let choice = checkbox(*selected)
                .label(app_label(app))
                .text_size(13)
                .style(settings_checkbox_style)
                .on_toggle_maybe(
                    (editable && (supported || *selected))
                        .then_some(move |value| M::Target(app, value)),
                );
            let mut item =
                column![row![brand_icon(app, 17.0), choice].spacing(8).align_y(Alignment::Center)]
                    .spacing(4)
                    .width(Length::FillPortion(1));
            let status = match draft.deliveries.iter().find(|delivery| delivery.app_id == app) {
                Some(delivery) => Some(match delivery.failure {
                    None => "已应用",
                    Some(McpFailureCode::Conflict) => "应用中已有同名 MCP 配置",
                    Some(McpFailureCode::TargetUnavailable) => "目标不可用",
                    Some(_) => "同步失败",
                }),
                None if !supported => Some("当前传输不可用"),
                None => None,
            };
            if let Some(status) = status {
                item = item.push(text(i18n.__(status)).size(12).style(settings_muted_text_style));
            }
            line = line.push(item);
        }
        if pair.len() == 1 {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        choices = choices.push(line);
    }
    body = body.push(settings_group_card(container(choices).padding([12, 0])));
    body.into()
}

fn segment(
    label: &'static str,
    active: bool,
    message: Option<M>,
) -> iced::widget::Button<'static, M> {
    button(text(label).size(13))
        .padding([7, 12])
        .style(move |theme, status| settings_segment_button_style(theme, status, active))
        .on_press_maybe(message)
}

fn app_label(app: AppId) -> &'static str {
    match app {
        AppId::Claude => "Claude Code",
        AppId::Codex => "Codex",
        AppId::Gemini => "Gemini CLI",
        AppId::GrokBuild => "Grok Build",
        AppId::OpenCode => "OpenCode",
        AppId::Hermes => "Hermes",
        AppId::Pi => "Pi",
        AppId::Mcode => "MiniMax Code",
        AppId::Kimi => "Kimi Code",
        AppId::Dsh => "DeepSeek Harness",
        AppId::Agy => "Antigravity",
        AppId::Omp => "Oh My Pi",
        AppId::ClaudeDesktop => "Claude Desktop",
        AppId::OpenClaw => "OpenClaw",
    }
}
