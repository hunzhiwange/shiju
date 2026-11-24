//! Managed connection form presentation; persistence stays in the update layer.
use crate::{form_field, management_bindings::matrix::app_label, toolbar_button};
use desktop_assets::assets::Icon;
use desktop_settings_apps::system_settings_apps::list::brand_icon;
use desktop_ui_controls::system_settings_common::{icons::icon_btn_svg_style, *};
use iced::{
    Alignment, Element, Length,
    widget::{
        Space, button, checkbox, column, container, row, rule, scrollable, text, text_editor,
    },
};
use vw_api_mcp_management::{McpConfigValue, McpPresetDraftView};
#[derive(Clone)]
pub enum FormMessage {
    Replace(bool),
    ArgAdd,
    ArgRemove(usize),
    ArgSet(usize, String),
    PasteEditor(text_editor::Action),
    ParsePaste,
    ChoosePaste(usize),
    SkipPaste(usize),
    Details(bool),
    Target(usize, bool),
    Retry,
    Close,
    Field(&'static str, String),
    Json(bool),
    Editor(text_editor::Action),
    Description(text_editor::Action),
    Preset(usize),
    ValueAdd,
    ValueRemove(String),
    ValueName(String, String),
    ValueKind(String, bool),
    ValueText(String, String),
    ValueService(String, String),
    Save,
}
type Message = FormMessage;
fn message(value: FormMessage) -> Message {
    value
}
pub struct ConnectionDraft<'a> {
    pub draft: &'a crate::management_form::Draft,
    pub editing: bool,
    pub id: &'a str,
    pub name: &'a str,
    pub transport: &'a str,
    pub command: &'a str,
    pub args: &'a str,
    pub values: &'a str,
    pub cwd: &'a str,
    pub url: &'a str,
    pub editor: &'a text_editor::Content,
    pub json: bool,
    pub saving: bool,
    pub error: &'a Option<String>,
    pub presets: &'a [McpPresetDraftView],
}
/// Shared settings modal owns fixed header/footer and the 4px scrolling content area.
pub fn connection_form_panel(
    i18n: desktop_i18n::I18n,
    draft: ConnectionDraft<'_>,
) -> Element<'_, Message> {
    let input = |label: &'static str, field: &'static str, value: &str| {
        form_field(
            label,
            settings_text_input("", value).on_input_maybe(
                (!draft.saving).then_some(move |value| message(FormMessage::Field(field, value))),
            ),
        )
    };
    let mut body = column![].spacing(15).width(Length::Fill);
    if !draft.editing && !draft.presets.is_empty() {
        let mut presets = row![].spacing(7);
        for (index, preset) in draft.presets.iter().enumerate() {
            presets = presets.push(
                button(text(preset.name.as_str()).size(13))
                    .padding([6, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!draft.saving).then_some(FormMessage::Preset(index))),
            );
        }
        body = body.push(form_field(i18n.__("从模板开始"), presets.wrap()));
    }
    let paste = text_editor(&draft.draft.paste)
        .placeholder(i18n.__("粘贴 JSON、Codex TOML 或 OpenCode 配置"))
        .height(78)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style);
    body = body.push(form_field(
        i18n.__("粘贴配置"),
        column![
            if draft.saving { paste } else { paste.on_action(FormMessage::PasteEditor) },
            row![
                Space::new().width(Length::Fill),
                toolbar_button(
                    i18n.__("解析配置"),
                    (!draft.saving && !draft.draft.paste.text().trim().is_empty())
                        .then_some(FormMessage::ParsePaste)
                ),
            ],
        ]
        .spacing(6),
    ));
    for (index, candidate) in draft.draft.candidates.iter().enumerate() {
        let conflict = draft.draft.catalog.iter().any(|r| r.record.id.0 == candidate.id);
        body = body.push(
            row![
                text(format!(
                    "{} · {}",
                    candidate.id,
                    if candidate.connection.is_err() {
                        i18n.__("无效项")
                    } else if conflict {
                        i18n.__("同名项")
                    } else {
                        i18n.__("新项")
                    }
                )),
                toolbar_button(
                    if conflict {
                        i18n.__("按当前修订替换")
                    } else {
                        i18n.__("填入草稿")
                    },
                    (!draft.saving && candidate.connection.is_ok())
                        .then_some(FormMessage::ChoosePaste(index))
                ),
                toolbar_button(
                    i18n.__("跳过"),
                    (!draft.saving).then_some(FormMessage::SkipPaste(index))
                ),
            ]
            .spacing(8),
        );
    }
    body = body.push(rule::horizontal(1)).push(form_field(
        if draft.editing { i18n.__("目录 ID") } else { i18n.__("名称（目录 ID）") },
        settings_text_input(i18n.__("例如 context7"), draft.id).on_input_maybe(
            (!draft.saving && !draft.editing).then_some(|id| FormMessage::Field("id", id)),
        ),
    ));
    if draft.editing {
        body = body.push(
            checkbox(draft.draft.replace_connection)
                .label(i18n.__("替换私有连接"))
                .style(settings_checkbox_style)
                .on_toggle_maybe((!draft.saving).then_some(FormMessage::Replace)),
        );
        if draft.draft.replace_connection {
            body = body.push(
                text(i18n.__("请填写新的连接定义。省略的现有凭据引用会保留。"))
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
    }
    if draft.draft.replace_connection {
        body = body.push(
            row![
                settings_field_label(i18n.__("连接配置")),
                Space::new().width(Length::Fill),
                container(
                    row![
                        segment(
                            i18n.__("表单"),
                            !draft.json,
                            (!draft.saving && draft.json).then_some(FormMessage::Json(false))
                        ),
                        segment(
                            "JSON",
                            draft.json,
                            (!draft.saving && !draft.json).then_some(FormMessage::Json(true))
                        ),
                    ]
                    .spacing(3)
                )
                .padding(3)
                .style(segment_track),
            ]
            .align_y(Alignment::Center),
        );
        if draft.json {
            let editor = text_editor(&draft.editor)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .height(340)
                .style(settings_text_editor_style);
            body = body.push(form_field(
                i18n.__("服务器配置"),
                if draft.saving { editor } else { editor.on_action(FormMessage::Editor) },
            ));
        } else {
            let transport = |label: &'static str, kind: &'static str| {
                segment(
                    label,
                    draft.transport == kind,
                    (!draft.saving && draft.transport != kind)
                        .then(|| FormMessage::Field("transport", kind.into())),
                )
            };
            body = body.push(form_field(
                i18n.__("连接方式"),
                container(
                    row![
                        transport(i18n.__("本地命令（stdio）"), "stdio"),
                        transport("HTTP", "streamable_http"),
                        transport("SSE", "sse"),
                    ]
                    .spacing(4)
                    .wrap(),
                )
                .padding(3)
                .style(segment_track),
            ));
            if draft.transport == "stdio" {
                body = body.push(form_field(
                    i18n.__("命令"),
                    settings_text_input(i18n.__("例如 npx 或 uvx"), draft.command).on_input_maybe(
                        (!draft.saving).then_some(|value| FormMessage::Field("command", value)),
                    ),
                ));
                body = body.push(settings_field_label(i18n.__("参数")));
                let args: Vec<String> = serde_json::from_str(draft.args).unwrap_or_default();
                for (index, arg) in args.iter().enumerate() {
                    body = body.push(
                        row![
                            settings_text_input(i18n.__("独立参数（保留空格）"), arg)
                                .on_input_maybe(
                                    (!draft.saving)
                                        .then_some(move |value| FormMessage::ArgSet(index, value))
                                ),
                            toolbar_button(
                                "×",
                                (!draft.saving).then_some(FormMessage::ArgRemove(index))
                            )
                        ]
                        .spacing(8),
                    );
                }
                body = body.push(toolbar_button(
                    i18n.__("+ 添加参数"),
                    (!draft.saving).then_some(FormMessage::ArgAdd),
                ));
            } else {
                body = body.push(input("URL", "url", &draft.url));
            }
            body = body.push(
                text(if draft.transport == "stdio" {
                    i18n.__("环境变量")
                } else {
                    i18n.__("请求头")
                })
                .size(14),
            );
            let values: std::collections::BTreeMap<String, McpConfigValue> =
                serde_json::from_str(&draft.values).unwrap_or_default();
            for (name, value) in values {
                let key = name.clone();
                let remove = name.clone();
                let kind_key = name.clone();
                let value_key = name.clone();
                let credential = matches!(value, McpConfigValue::Credential { .. });
                let mut fields = row![
                    settings_text_input(i18n.__("字段名"), &name)
                        .width(Length::FillPortion(2))
                        .on_input_maybe((!draft.saving).then_some(move |value| message(
                            FormMessage::ValueName(key.clone(), value)
                        ))),
                    settings_select(
                        ["literal", "credential"],
                        Some(if credential { "credential" } else { "literal" }),
                        move |kind| message(FormMessage::ValueKind(
                            kind_key.clone(),
                            kind == "credential"
                        ))
                    )
                    .disabled(draft.saving),
                ]
                .spacing(6);
                let text_value = match value {
                    McpConfigValue::Literal { value } => value,
                    McpConfigValue::Credential { reference } => {
                        let service = serde_json::to_value(reference.service)
                            .expect("service")
                            .as_str()
                            .unwrap()
                            .to_owned();
                        fields = fields.push(
                            settings_select(
                                [
                                    "github_copilot",
                                    "codex_oauth",
                                    "xai_oauth",
                                    "kimi_oauth",
                                    "claude_official",
                                    "agy_oauth",
                                    "dsh_account",
                                ]
                                .map(str::to_owned),
                                Some(service),
                                move |service| {
                                    message(FormMessage::ValueService(name.clone(), service))
                                },
                            )
                            .disabled(draft.saving),
                        );
                        reference.account_id
                    }
                };
                fields = fields.push(
                    settings_text_input(
                        if credential {
                            i18n.__("凭据账号 ID")
                        } else {
                            i18n.__("非秘密值")
                        },
                        &text_value,
                    )
                    .on_input_maybe((!draft.saving).then_some(move |value| {
                        message(FormMessage::ValueText(value_key.clone(), value))
                    })),
                );
                body = body.push(
                    fields
                        .push(toolbar_button(
                            "×",
                            (!draft.saving).then_some(FormMessage::ValueRemove(remove)),
                        ))
                        .align_y(Alignment::Center)
                        .wrap(),
                );
            }
            body = body.push(toolbar_button(
                if draft.transport == "stdio" {
                    i18n.__("+ 添加环境变量")
                } else {
                    i18n.__("+ 添加请求头")
                },
                (!draft.saving).then_some(FormMessage::ValueAdd),
            ));
            if draft.transport == "stdio" {
                body = body.push(input(i18n.__("工作目录（可选）"), "cwd", draft.cwd));
            }
        }
    }
    body = body.push(rule::horizontal(1)).push(settings_field_label(i18n.__("写入到")));
    if draft.draft.targets.is_empty() {
        body = body.push(
            text(i18n.__("未发现可信的安装实例。仍可单独保存定义。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    if !draft.draft.targets.is_empty() {
        body = body.push(
            text(if draft.editing {
                i18n.__("保存时应用勾选变更。替换连接会重发到仍勾选的应用。")
            } else {
                i18n.__("添加时写入已勾选的应用，也可取消勾选，仅保存定义。")
            })
            .size(12)
            .style(settings_muted_text_style),
        );
    }
    for (row_index, pair) in draft.draft.targets.chunks(2).enumerate() {
        let mut targets = row![].spacing(12);
        for (column_index, (target, selected, _, _)) in pair.iter().enumerate() {
            let index = row_index * 2 + column_index;
            let supported = draft.draft.target_supported(target);
            let repeated = draft
                .draft
                .targets
                .iter()
                .filter(|(other, _, _, _)| other.app_id == target.app_id)
                .count()
                > 1;
            let mut label = column![text(app_label(target.app_id)).size(13)];
            if repeated {
                label = label.push(
                    text(target.installation_id.0.clone())
                        .size(11)
                        .style(settings_muted_text_style),
                );
            }
            if !supported {
                label = label.push(
                    text(i18n.__("当前传输不可用")).size(11).style(settings_muted_text_style),
                );
            }
            targets = targets.push(
                container(
                    row![
                        checkbox(*selected)
                            .size(17)
                            .style(settings_checkbox_style)
                            .on_toggle_maybe(
                                (!draft.saving && (supported || *selected))
                                    .then_some(move |value| FormMessage::Target(index, value))
                            ),
                        brand_icon(target.app_id, 17.0),
                        label,
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .width(Length::FillPortion(1)),
            );
        }
        if pair.len() == 1 {
            targets = targets.push(Space::new().width(Length::FillPortion(1)));
        }
        body = body.push(targets);
    }
    body = body.push(rule::horizontal(1)).push(
        button(
            row![
                icon_svg(
                    if draft.draft.details { Icon::ChevronDown } else { Icon::ChevronRight },
                    13.0
                )
                .style(icon_btn_svg_style),
                text(i18n.__("说明与链接")).size(14),
            ]
            .spacing(7)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press(FormMessage::Details(!draft.draft.details)),
    );
    if draft.draft.details {
        let description = text_editor(&draft.draft.description)
            .height(64)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .padding(SETTINGS_CONTROL_PADDING)
            .style(settings_text_editor_style);
        body = body.push(input(i18n.__("显示名"), "name", &draft.draft.name)).push(form_field(
            i18n.__("描述"),
            if draft.saving {
                description
            } else {
                description.on_action(FormMessage::Description)
            },
        ));
        for (label, field, value) in [
            (i18n.__("标签（逗号分隔）"), "tags", &draft.draft.tags),
            (i18n.__("主页"), "homepage", &draft.draft.homepage),
            (i18n.__("文档"), "documentation", &draft.draft.documentation),
        ] {
            body = body.push(input(label, field, value));
        }
        if draft.editing {
            body = body.push(input(
                i18n.__("公开连接摘要（请勿填写凭据）"),
                "summary",
                &draft.draft.summary,
            ));
        }
    }
    for (_, _, result) in &draft.draft.deliveries {
        match result {
            Ok(result) => {
                for item in &result.items {
                    body = body.push(text(crate::distribution::item_label(i18n, item)).size(12));
                }
            }
            Err(error) => {
                body = body.push(settings_error_banner(
                    desktop_i18n::settings_extensions::feedback(i18n, &error.clone()),
                ))
            }
        }
    }
    if !draft.draft.deliveries.is_empty() {
        body = body.push(toolbar_button(
            i18n.__("重试失败目标"),
            (!draft.saving).then_some(FormMessage::Retry),
        ));
    }
    if let Some(error) = &draft.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
            i18n,
            &error.clone(),
        )));
    }
    let close = message(FormMessage::Close);
    let header = row![
        icon_btn(
            Icon::ChevronLeft,
            i18n.__("返回 MCP 列表"),
            (!draft.saving).then_some(close.clone())
        ),
        text(if draft.editing { i18n.__("编辑 MCP") } else { i18n.__("添加 MCP") }).size(22),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let footer = row![
        Space::new().width(Length::Fill),
        toolbar_button(i18n.__("取消"), (!draft.saving).then_some(close)),
        button(text(if draft.saving {
            i18n.__("保存中…")
        } else if draft.editing {
            i18n.__("保存")
        } else {
            i18n.__("添加 MCP")
        }))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe((!draft.saving).then_some(message(FormMessage::Save))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    column![
        container(header).padding([0, 7]).width(Length::Fill),
        rule::horizontal(1),
        scrollable(container(body).padding([0, 7]).width(Length::Fill))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4)
            ))
            .height(Length::Fill),
        rule::horizontal(1),
        container(footer).padding([0, 7]).width(Length::Fill),
    ]
    .spacing(12)
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

fn segment_track(theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style::default()
        .background(theme.extended_palette().background.weak.color)
        .border(iced::border::rounded(10))
}

fn segment(
    label: &'static str,
    active: bool,
    message: Option<FormMessage>,
) -> iced::widget::Button<'static, FormMessage> {
    button(text(label).size(13))
        .padding([7, 12])
        .style(move |theme, status| settings_segment_button_style(theme, status, active))
        .on_press_maybe(message)
}

impl std::fmt::Debug for FormMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FormMessage { .. }")
    }
}
