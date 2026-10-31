//! Source-labelled MCP forms using the same controls and modal surface as settings.
use super::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style, settings_pick_list_menu_style, settings_pick_list_style,
    settings_text_input_style,
};
use crate::app::{
    App, Message,
    message::mcp_form::{self, McpFormMessage},
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length};
use vw_gateway_client::vw_api_types::mcp_form::McpFormAction;

#[derive(Clone, PartialEq, Eq)]
struct Choice {
    value: String,
    label: String,
}
impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
fn submit(id: &str, action: McpFormAction) -> Message {
    Message::McpForm(McpFormMessage::Submit { id: id.to_owned(), action })
}
pub(crate) fn with_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some((request, draft)) = mcp_form::current(app) else {
        return base;
    };
    let cancel = submit(&request.id, McpFormAction::Cancel);
    let mut fields = column![text(&request.message).size(14)].spacing(14);
    for field in &draft.fields {
        let label = format!("{}{}", field.title, if field.required { " *" } else { "（可选）" });
        let mut content = column![text(label).size(13)].spacing(6);
        if !field.description.is_empty() {
            content =
                content.push(text(&field.description).size(12).style(settings_muted_text_style));
        }
        let id = request.id.clone();
        let name = field.name.clone();
        let control: Element<'a, Message> = if field.options.is_empty() {
            let input = text_input(
                "请填写",
                draft.values.get(&field.name).map(String::as_str).unwrap_or(""),
            )
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill);
            if draft.busy {
                input.into()
            } else {
                input
                    .on_input(move |value| {
                        Message::McpForm(McpFormMessage::Changed {
                            id: id.clone(),
                            field: name.clone(),
                            value,
                        })
                    })
                    .into()
            }
        } else {
            let choices: Vec<_> = field
                .options
                .iter()
                .map(|(value, label)| Choice { value: value.clone(), label: label.clone() })
                .collect();
            let selected =
                choices.iter().find(|c| draft.values.get(&field.name) == Some(&c.value)).cloned();
            let input = pick_list(choices, selected, move |choice: Choice| {
                Message::McpForm(McpFormMessage::Changed {
                    id: id.clone(),
                    field: name.clone(),
                    value: choice.value,
                })
            })
            .placeholder("请选择")
            .padding(SETTINGS_CONTROL_PADDING)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fill);
            input.into()
        };
        content = content.push(control);
        if !field.hint.is_empty() {
            content = content.push(text(&field.hint).size(11).style(settings_muted_text_style));
        }
        fields = fields.push(content);
    }
    if let Some(error) = &draft.error {
        fields = fields.push(text(error).size(12).color(app.app_theme.palette().danger));
    }
    let scroll = scrollable(container(fields).padding([0, 7]).width(Length::Fill))
        .id(iced::widget::Id::new("mcp-form-content"))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    let title = row![
        text("服务请求填写表单").size(18),
        Space::new().width(Length::Fill),
        settings_close_button(cancel.clone())
    ]
    .align_y(Alignment::Center);
    let mut accept = button(text(if draft.busy { "正在提交…" } else { "确认提交" }))
        .padding([9, 16])
        .style(primary_action_btn_style);
    let mut decline = button(text("拒绝")).padding([9, 16]).style(rounded_action_btn_style);
    let mut cancel_button = button(text("取消")).padding([9, 16]).style(rounded_action_btn_style);
    if !draft.busy {
        accept = accept.on_press(submit(&request.id, McpFormAction::Accept));
        decline = decline.on_press(submit(&request.id, McpFormAction::Decline));
        cancel_button = cancel_button.on_press(cancel.clone());
    }
    let card = settings_modal_card(
        column![
            title,
            text(format!("会话：{}\n服务：{}", request.session_id, request.server_name))
                .size(12)
                .style(settings_muted_text_style),
            text("内容由服务提供，仅在你确认后发送填写结果。")
                .size(12)
                .style(settings_muted_text_style),
            scroll,
            row![Space::new().width(Length::Fill), cancel_button, decline, accept]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(14),
    )
    .width(620)
    .height(Length::Fill)
    .max_height(680);
    settings_modal_overlay(Some(base), cancel, card)
}
