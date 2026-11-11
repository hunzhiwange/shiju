//! Managed connection modal; uses the common settings controls and scrolling layout.
use super::{form_field, toolbar_button};
use crate::app::Message;
use crate::app::components::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, checkbox, column, row, text, text_editor},
};
use vw_gateway_client::vw_api_types::mcp_management::McpConfigValue;

/// Shared settings modal owns fixed header/footer and the 4px scrolling content area.
pub(crate) fn connection_form_panel(
    draft: &crate::app::message::settings::mcp_management_form::Draft,
) -> Element<'_, Message> {
    use crate::app::message::settings::mcp_management_form::{FormMessage, message};
    let input = |label: &'static str, field: &'static str, value: &str| {
        form_field(
            label,
            settings_text_input("", value).on_input_maybe(
                (!draft.saving).then_some(move |value| message(FormMessage::Field(field, value))),
            ),
        )
    };
    let mut body = column![
        text("保存仅更新目录定义，不启用、分发或连接服务器。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(12);
    if draft.record.is_some() {
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
    body = body.push(
        checkbox(draft.json)
            .label("JSON 编辑")
            .style(settings_checkbox_style)
            .on_toggle_maybe((!draft.saving).then_some(|value| message(FormMessage::Json(value)))),
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
            editor.on_action(|action| message(FormMessage::Editor(action)))
        });
    } else {
        body = body.push(form_field(
            "传输",
            settings_select(
                ["stdio", "streamable_http", "sse"].map(str::to_owned),
                Some(draft.transport.clone()),
                |value| message(FormMessage::Field("transport", value)),
            )
            .disabled(draft.saving),
        ));
        if draft.transport == "stdio" {
            body = body
                .push(input("命令", "command", &draft.command))
                .push(input("参数（JSON 字符串数组）", "args", &draft.args))
                .push(input("工作目录（可选）", "cwd", &draft.cwd));
        } else {
            body = body.push(input("URL", "url", &draft.url));
        }
        body = body
            .push(text(if draft.transport == "stdio" { "环境变量" } else { "请求头" }).size(14));
        let values: std::collections::BTreeMap<String, McpConfigValue> =
            serde_json::from_str(&draft.values).expect("typed graphical values");
        for (name, value) in values {
            let key = name.clone();
            let remove = name.clone();
            let kind_key = name.clone();
            let value_key = name.clone();
            let credential = matches!(value, McpConfigValue::Credential { .. });
            let mut fields =
                column![
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
                            (!draft.saving).then_some(message(FormMessage::ValueRemove(remove)))
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
