//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use iced::{Alignment, Element, Length};
use serde_json::Value;

use super::{App, Message, message};
use crate::app::components::chat_panel::tools::{tool_inline_summary, tool_verb};
use crate::app::components::chat_panel::utils::truncate_chars;

pub(super) struct PermissionModalQueue {
    requests: Vec<vw_gateway_client::PendingPermissionRequestDto>,
    current_index: usize,
    previous_request_id: Option<String>,
    next_request_id: Option<String>,
}

pub(super) fn permission_modal_queue(
    current: &vw_gateway_client::PendingPermissionRequestDto,
    queued: &[vw_gateway_client::PendingPermissionRequestDto],
) -> PermissionModalQueue {
    let mut requests = queued.to_vec();
    if requests.is_empty() {
        requests.push(current.clone());
    }

    let current_index = requests.iter().position(|request| request.id == current.id).unwrap_or(0);
    let previous_request_id = current_index
        .checked_sub(1)
        .and_then(|idx| requests.get(idx))
        .map(|request| request.id.clone());
    let next_request_id = requests.get(current_index + 1).map(|request| request.id.clone());

    PermissionModalQueue { requests, current_index, previous_request_id, next_request_id }
}

/// 模块内可见函数，执行 with_permission_modal 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn with_permission_modal<'a>(
    app: &App,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(req): Option<vw_gateway_client::PendingPermissionRequestDto> =
        app.permission_modal_request.clone()
    else {
        // Keep the base at the same widget-tree position as in settings_modal_overlay.
        return iced::widget::stack![root_content].into();
    };

    use crate::app::components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_close_button,
        settings_modal_header, settings_modal_layout, settings_modal_overlay,
        settings_muted_text_style, settings_panel,
    };
    use iced::widget::{Space, button, column, container, row, text};

    let PermissionModalQueue {
        requests: pending_requests,
        current_index,
        previous_request_id,
        next_request_id,
    } = permission_modal_queue(&req, &app.permission_modal_requests);

    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let reject_message = Message::Chat(message::ChatMessage::PermissionReject);
    let header = settings_modal_header(
        text(permission_modal_title_localized(i18n, &req)).size(18),
        settings_close_button(reject_message.clone()),
    );
    let mut content = column![].width(Length::Fill).spacing(14);

    if pending_requests.len() > 1 {
        let mut nav_row = row![
            text(crate::app::i18n::modals::render(
                i18n,
                "待审批 %u/%u",
                &[(current_index + 1).into(), pending_requests.len().into()]
            ))
            .size(13),
            Space::new().width(Length::Fill)
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if let Some(previous_request_id) = previous_request_id {
            nav_row = nav_row.push(
                button(text(i18n.__("上一项")).size(12))
                    .padding([5, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::Chat(message::ChatMessage::PermissionSelectRequest(
                        previous_request_id,
                    ))),
            );
        }
        if let Some(next_request_id) = next_request_id {
            nav_row = nav_row.push(
                button(text(i18n.__("下一项")).size(12))
                    .padding([5, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::Chat(message::ChatMessage::PermissionSelectRequest(
                        next_request_id,
                    ))),
            );
        }

        content = content.push(nav_row);

        let mut pending_list = column![].spacing(6);
        for item in pending_requests.iter().take(6) {
            let label = permission_request_selector_label_localized(i18n, item);
            let item_button = if item.id == req.id {
                button(text(label).size(12))
                    .width(Length::Fill)
                    .padding([8, 12])
                    .style(primary_action_btn_style)
            } else {
                button(text(label).size(12))
                    .width(Length::Fill)
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(Message::Chat(message::ChatMessage::PermissionSelectRequest(
                        item.id.clone(),
                    )))
            };
            pending_list = pending_list.push(item_button);
        }
        if pending_requests.len() > 6 {
            pending_list = pending_list.push(
                text(crate::app::i18n::modals::render(
                    i18n,
                    "还有 %u 项待审批…",
                    &[(pending_requests.len() - 6).into()],
                ))
                .size(12),
            );
        }
        content = content.push(pending_list);
    }

    if let Some(reason) = req
        .metadata
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .map(ToOwned::to_owned)
    {
        content = content.push(text(reason).size(14));
    }

    let mut details = column![permission_meta_line(i18n.__("权限"), &req.permission)].spacing(8);

    if let Some(target) = permission_argument_summary(&req) {
        details = details.push(permission_meta_line(i18n.__("目标"), &target));
    }

    if let Some(requested_by) = permission_metadata_text(&req, "requested_by") {
        details = details.push(permission_meta_line(i18n.__("发起者"), &requested_by));
    }

    if let Some(requested_channel) = permission_metadata_text(&req, "requested_channel") {
        details = details.push(permission_meta_line(i18n.__("通道"), &requested_channel));
    }

    if let Some(expires_at) = permission_metadata_text(&req, "expires_at") {
        details = details.push(permission_meta_line(i18n.__("过期"), &expires_at));
    }

    if let Some(tool) = req.tool.as_ref() {
        details = details
            .push(permission_meta_line(i18n.__("消息"), &tool.message_id))
            .push(permission_meta_line(i18n.__("调用"), &tool.call_id));
    }

    content = content.push(settings_panel(details).padding([12, 14]));

    if !req.patterns.is_empty() {
        let mut patterns = column![text(i18n.__("目标")).size(13)].spacing(6);
        for pattern in req.patterns.iter().map(String::as_str).take(8) {
            patterns =
                patterns.push(settings_panel(text(pattern.to_owned()).size(13)).padding([8, 10]));
        }
        if req.patterns.len() > 8 {
            patterns = patterns.push(
                text(crate::app::i18n::modals::render(
                    i18n,
                    "还有 %u 项…",
                    &[(req.patterns.len() - 8).into()],
                ))
                .size(12),
            );
        }
        content = content.push(patterns);
    }

    if !req.always.is_empty() {
        content = content.push(
            text(crate::app::i18n::modals::render(
                i18n,
                "始终允许将记住 %u 个模式",
                &[req.always.len().into()],
            ))
            .size(12)
            .style(settings_muted_text_style),
        );
    }

    if let Some(arguments_preview) = permission_arguments_preview(&req) {
        content = content.push(permission_detail_block(i18n.__("参数"), arguments_preview));
    }

    let reject = button(text(i18n.__("拒绝")).size(13))
        .on_press(reject_message.clone())
        .padding([8, 12])
        .style(rounded_action_btn_style);
    let always = button(text(i18n.__("始终允许")).size(13))
        .on_press(Message::Chat(message::ChatMessage::PermissionApproveAlways))
        .padding([8, 12])
        .style(rounded_action_btn_style);
    let approve_once = button(text(i18n.__("仅此一次")).size(13))
        .on_press(Message::Chat(message::ChatMessage::PermissionApproveOnce))
        .padding([8, 12])
        .style(primary_action_btn_style);

    let mut action_row = row![reject].spacing(8).align_y(Alignment::Center);
    if pending_requests.len() > 1 && pending_requests.iter().all(|request| !request.always.is_empty()) {
        action_row = action_row.push(
            button(text(i18n.__("全部始终允许")).size(13))
                .on_press(Message::Chat(message::ChatMessage::PermissionApproveAllAlways))
                .padding([8, 12])
                .style(rounded_action_btn_style),
        );
    }
    if !req.always.is_empty() {
        action_row = action_row.push(always);
    }
    let actions = container(
        action_row
            .push(approve_once)
            .wrap()
            .align_x(iced::alignment::Horizontal::Right),
    )
    .align_right(Length::Fill);
    let card = settings_modal_layout(header, content, Some(actions.into())).width(Length::Fill);
    let dialog =
        crate::app::components::toast::modal_with_feedback(app, card).width(584).max_height(620);

    settings_modal_overlay(Some(root_content), reject_message, dialog)
}

fn permission_modal_title_localized(
    i18n: crate::app::i18n::I18n,
    req: &vw_gateway_client::PendingPermissionRequestDto,
) -> String {
    req.metadata
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            permission_argument_summary(req)
                .map(|summary| {
                    crate::app::i18n::modals::render(
                        i18n,
                        "批准%s：%s",
                        &[i18n.__(tool_verb(&req.permission)).into(), summary.into()],
                    )
                })
                .unwrap_or_else(|| {
                    crate::app::i18n::modals::render(
                        i18n,
                        "需要批准 %s 操作",
                        &[req.permission.clone().into()],
                    )
                })
        })
}

fn permission_request_selector_label_localized(
    i18n: crate::app::i18n::I18n,
    req: &vw_gateway_client::PendingPermissionRequestDto,
) -> String {
    let prefix = i18n.__(tool_verb(&req.permission));
    let summary = permission_argument_summary(req).unwrap_or_else(|| req.permission.clone());
    truncate_chars(&format!("{} · {}", prefix, summary), 80).to_string()
}

fn permission_meta_line<'a>(label: &'a str, value: &str) -> iced::widget::Row<'a, Message> {
    use crate::app::components::system_settings_common::settings_muted_text_style;
    use iced::widget::{row, text};

    row![
        text(label).size(12).style(settings_muted_text_style),
        text(value.to_owned()).size(13).width(Length::Fill),
    ]
    .spacing(8)
}

fn permission_metadata_text(
    req: &vw_gateway_client::PendingPermissionRequestDto,
    key: &str,
) -> Option<String> {
    req.metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn permission_argument_summary(
    req: &vw_gateway_client::PendingPermissionRequestDto,
) -> Option<String> {
    let arguments = req.metadata.get("arguments")?;
    let raw_input = match arguments {
        Value::Null => return None,
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).ok()?,
    };

    tool_inline_summary(&req.permission, &raw_input).or_else(|| match arguments {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() { None } else { Some(truncate_chars(trimmed, 80).to_string()) }
        }
        _ => None,
    })
}

fn permission_arguments_preview(
    req: &vw_gateway_client::PendingPermissionRequestDto,
) -> Option<String> {
    let arguments = req.metadata.get("arguments")?;
    if matches!(arguments, Value::Null) {
        return None;
    }

    let preview = serde_json::to_string_pretty(arguments)
        .ok()
        .or_else(|| serde_json::to_string(arguments).ok())?;
    let trimmed = preview.trim();
    if trimmed.is_empty() || trimmed == "{}" || trimmed == "null" {
        return None;
    }

    Some(truncate_chars(trimmed, 320).to_string())
}

fn permission_detail_block<'a>(label: &'a str, value: String) -> iced::widget::Column<'a, Message> {
    use crate::app::components::system_settings_common::{settings_field_label, settings_panel};
    use iced::widget::{column, text};

    column![settings_field_label(label), settings_panel(text(value).size(12)).padding([10, 12]),]
        .spacing(6)
}

#[cfg(test)]
#[path = "permission_modal_tests.rs"]
mod permission_modal_tests;
