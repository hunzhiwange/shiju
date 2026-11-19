//! Managed connection form presentation; persistence stays in the update layer.
use crate::{form_field, toolbar_button};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, checkbox, column, row, text, text_editor},
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
pub fn connection_form_panel(draft: ConnectionDraft<'_>) -> Element<'_, Message> {
    let input = |label: &'static str, field: &'static str, value: &str| {
        form_field(
            label,
            settings_text_input("", value).on_input_maybe(
                (!draft.saving).then_some(move |value| message(FormMessage::Field(field, value))),
            ),
        )
    };
    let mut body = column![
        text("先保存定义，再发布到勾选的应用；粘贴和编辑不会运行命令。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(12);
    if draft.editing {
        body = body.push(text("目录不提供原始连接；请输入替换定义。省略的现有凭据引用由服务端保留；未知遮罩会被拒绝。").size(12));
    } else {
        body =
            body.push(input("目录 ID", "id", &draft.id)).push(input("名称", "name", &draft.name));
        for (index, preset) in draft.presets.iter().enumerate() {
            body = body.push(
                button(text(format!("预设：{}（未启用草稿）", preset.name)))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!draft.saving).then_some(message(FormMessage::Preset(index)))),
            );
        }
    }
    if draft.editing {
        body = body.push(
            checkbox(draft.draft.replace_connection)
                .label("替换私有连接定义")
                .style(settings_checkbox_style)
                .on_toggle_maybe((!draft.saving).then_some(FormMessage::Replace)),
        );
    }
    body = body.push(text("粘贴 JSON / Codex TOML / OpenCode 配置").size(14));
    let paste = text_editor(&draft.draft.paste)
        .height(110)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style);
    body = body.push(if draft.saving { paste } else { paste.on_action(FormMessage::PasteEditor) });
    body =
        body.push(toolbar_button("解析粘贴", (!draft.saving).then_some(FormMessage::ParsePaste)));
    for (index, candidate) in draft.draft.candidates.iter().enumerate() {
        let conflict = draft.draft.catalog.iter().any(|r| r.record.id.0 == candidate.id);
        body = body.push(
            row![
                text(format!(
                    "{} · {}",
                    candidate.id,
                    if candidate.connection.is_err() {
                        "无效项"
                    } else if conflict {
                        "同名项"
                    } else {
                        "新项"
                    }
                )),
                toolbar_button(
                    if conflict { "按当前修订替换" } else { "填入草稿" },
                    (!draft.saving && candidate.connection.is_ok())
                        .then_some(FormMessage::ChoosePaste(index))
                ),
                toolbar_button("跳过", (!draft.saving).then_some(FormMessage::SkipPaste(index))),
            ]
            .spacing(8),
        );
    }
    if draft.editing {
        body = body.push(text(format!("目录 ID（锁定）：{}", draft.id)));
    }
    body = body.push(input("显示名称", "name", draft.name));
    if draft.draft.replace_connection {
        body = body.push(
            checkbox(draft.json).label("JSON 编辑").style(settings_checkbox_style).on_toggle_maybe(
                (!draft.saving).then_some(|value| message(FormMessage::Json(value))),
            ),
        );
        if draft.json {
            let editor = text_editor(&draft.editor)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .height(340)
                .style(settings_text_editor_style);
            body = body.push(if draft.saving {
                editor
            } else {
                editor.on_action(move |action| message(FormMessage::Editor(action)))
            });
        } else {
            body = body.push(form_field(
                "传输",
                settings_select(
                    ["stdio", "streamable_http", "sse"].map(str::to_owned),
                    Some(draft.transport.to_owned()),
                    |value| message(FormMessage::Field("transport", value)),
                )
                .disabled(draft.saving),
            ));
            if draft.transport == "stdio" {
                body = body.push(input("命令", "command", &draft.command)).push(input(
                    "工作目录（可选）",
                    "cwd",
                    &draft.cwd,
                ));
                let args: Vec<String> = serde_json::from_str(draft.args).unwrap_or_default();
                for (index, arg) in args.iter().enumerate() {
                    body = body.push(
                        row![
                            settings_text_input("独立参数（保留空格）", arg).on_input_maybe(
                                (!draft.saving)
                                    .then_some(move |value| FormMessage::ArgSet(index, value))
                            ),
                            toolbar_button(
                                "移除",
                                (!draft.saving).then_some(FormMessage::ArgRemove(index))
                            )
                        ]
                        .spacing(8),
                    );
                }
                body = body.push(toolbar_button(
                    "添加参数",
                    (!draft.saving).then_some(FormMessage::ArgAdd),
                ));
            } else {
                body = body.push(input("URL", "url", &draft.url));
            }
            body = body.push(
                text(if draft.transport == "stdio" { "环境变量" } else { "请求头" }).size(14),
            );
            let values: std::collections::BTreeMap<String, McpConfigValue> =
                serde_json::from_str(&draft.values).unwrap_or_default();
            for (name, value) in values {
                let key = name.clone();
                let remove = name.clone();
                let kind_key = name.clone();
                let value_key = name.clone();
                let credential = matches!(value, McpConfigValue::Credential { .. });
                let mut fields = column![
                    settings_text_input("字段名", &name).on_input_maybe((!draft.saving).then_some(
                        move |value| message(FormMessage::ValueName(key.clone(), value))
                    )),
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
                        if credential { "凭据账号 ID" } else { "非秘密值" },
                        &text_value,
                    )
                    .on_input_maybe((!draft.saving).then_some(move |value| {
                        message(FormMessage::ValueText(value_key.clone(), value))
                    })),
                );
                body = body.push(settings_panel(
                    column![
                        fields,
                        button(text("移除字段"))
                            .padding([8, 14])
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!draft.saving)
                                    .then_some(message(FormMessage::ValueRemove(remove)))
                            )
                    ]
                    .spacing(8),
                ));
            }
            body = body.push(
                button(text("添加字段"))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!draft.saving).then_some(message(FormMessage::ValueAdd))),
            );
        }
    }
    body = body.push(text("目标应用").size(14)).push(
        text("连接替换将重发所有仍勾选的已启用目标；取消勾选会发布禁用。没有目标时仍可保存。")
            .size(12),
    );
    if draft.draft.targets.is_empty() {
        body = body.push(text("没有可信且可写的安装实例，请在应用管理刷新安装状态。").size(12));
    }
    let mut targets = row![].spacing(14);
    for (index, (target, selected, _, _)) in draft.draft.targets.iter().enumerate() {
        let supported = draft.draft.target_supported(target);
        targets = targets.push(
            checkbox(*selected)
                .label(format!(
                    "{:?} · {}{}",
                    target.app_id,
                    target.installation_id.0,
                    if supported { "" } else { "（传输不支持）" }
                ))
                .style(settings_checkbox_style)
                .on_toggle_maybe(
                    (!draft.saving && (supported || *selected))
                        .then_some(move |value| FormMessage::Target(index, value)),
                ),
        );
        if index % 2 == 1 {
            body = body.push(targets);
            targets = row![].spacing(14);
        }
    }
    body = body.push(targets);
    body = body.push(
        checkbox(draft.draft.details)
            .label("说明与公开摘要")
            .style(settings_checkbox_style)
            .on_toggle(FormMessage::Details),
    );
    if draft.draft.details {
        for (label, field, value) in [
            ("描述", "description", &draft.draft.description),
            ("标签（逗号分隔）", "tags", &draft.draft.tags),
            ("主页", "homepage", &draft.draft.homepage),
            ("文档", "documentation", &draft.draft.documentation),
        ] {
            body = body.push(input(label, field, value));
        }
        if draft.editing {
            body =
                body.push(input("公开连接摘要（请勿填写凭据）", "summary", &draft.draft.summary));
        } else {
            body = body.push(text("公开连接摘要由网关生成，可在保存后编辑。").size(12));
        }
    }
    for (_, _, result) in &draft.draft.deliveries {
        match result {
            Ok(result) => {
                for item in &result.items {
                    body = body.push(text(crate::distribution::item_label(item)).size(12));
                }
            }
            Err(error) => body = body.push(settings_error_banner(error.clone())),
        }
    }
    if !draft.draft.deliveries.is_empty() {
        body = body
            .push(toolbar_button("重试失败目标", (!draft.saving).then_some(FormMessage::Retry)));
    }
    if let Some(error) = &draft.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    let close = message(FormMessage::Close);
    let header =
        settings_modal_header(text("受管 MCP 连接").size(20), settings_close_button(close.clone()));
    let footer = row![
        Space::new().width(Length::Fill),
        toolbar_button("取消", (!draft.saving).then_some(close)),
        button(text(if draft.saving { "保存中..." } else { "保存定义" }))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe((!draft.saving).then_some(message(FormMessage::Save))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    settings_modal_layout(header, body, Some(footer.into())).width(680).height(620).into()
}

impl std::fmt::Debug for FormMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FormMessage { .. }")
    }
}
