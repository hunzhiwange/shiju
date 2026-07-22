//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use iced::{Element, Length, Theme};
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
    mut root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(req): Option<vw_gateway_client::PendingPermissionRequestDto> =
        app.permission_modal_request.clone()
    else {
        return root_content;
    };

    use crate::app::components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_modal_card,
        settings_modal_overlay,
    };
    use iced::widget::{Space, button, column, container, row, text};

    let PermissionModalQueue {
        requests: pending_requests,
        current_index,
        previous_request_id,
        next_request_id,
    } = permission_modal_queue(&req, &app.permission_modal_requests);

    let mut content = column![text(permission_modal_title(&req)).size(16)].spacing(12);

    if pending_requests.len() > 1 {
        let mut nav_row = row![
            text(format!("待审批 {}/{}", current_index + 1, pending_requests.len())).size(13),
            Space::new().width(Length::Fill)
        ]
        .spacing(8);

        if let Some(previous_request_id) = previous_request_id {
            nav_row = nav_row.push(
                button(text("上一项").size(12))
                    .padding([5, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::Chat(message::ChatMessage::PermissionSelectRequest(
                        previous_request_id,
                    ))),
            );
        }
        if let Some(next_request_id) = next_request_id {
            nav_row = nav_row.push(
                button(text("下一项").size(12))
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
            let label = permission_request_selector_label(item);
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
            pending_list = pending_list
                .push(text(format!("还有 {} 项待审批…", pending_requests.len() - 6)).size(12));
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

    content = content.push(permission_meta_line("权限", &req.permission));

    if let Some(target) = permission_argument_summary(&req) {
        content = content.push(permission_meta_line("目标", &target));
    }

    if let Some(requested_by) = permission_metadata_text(&req, "requested_by") {
        content = content.push(permission_meta_line("发起者", &requested_by));
    }

    if let Some(requested_channel) = permission_metadata_text(&req, "requested_channel") {
        content = content.push(permission_meta_line("通道", &requested_channel));
    }

    if let Some(expires_at) = permission_metadata_text(&req, "expires_at") {
        content = content.push(permission_meta_line("过期", &expires_at));
    }

    if let Some(tool) = req.tool.as_ref() {
        content = content
            .push(permission_meta_line("消息", &tool.message_id))
            .push(permission_meta_line("调用", &tool.call_id));
    }

    if !req.patterns.is_empty() {
        let mut patterns = column![text("目标").size(13)].spacing(6);
        for pattern in req.patterns.iter().map(String::as_str).take(8) {
            let pattern_label = pattern.to_string();
            let pattern_text: Element<'_, Message> = container(text(pattern_label).size(13))
                .width(Length::Fill)
                .padding([8, 10])
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(palette.background.weak.color.scale_alpha(0.45).into()),
                        border: iced::Border {
                            width: 1.0,
                            color: palette.background.strong.color.scale_alpha(0.40),
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into();
            patterns = patterns.push(pattern_text);
        }
        if req.patterns.len() > 8 {
            patterns = patterns.push(text(format!("还有 {} 项…", req.patterns.len() - 8)).size(12));
        }
        content = content.push(patterns);
    }

    if !req.always.is_empty() {
        content = content.push(
            text(format!("始终允许将记住 {} 个模式", req.always.len())).size(12).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.82)),
                },
            ),
        );
    }

    if let Some(arguments_preview) = permission_arguments_preview(&req) {
        content = content.push(permission_detail_block("参数", arguments_preview));
    }

    let reject = button(text("拒绝").size(13))
        .on_press(Message::Chat(message::ChatMessage::PermissionReject))
        .padding([6, 12])
        .style(rounded_action_btn_style);
    let always = button(text("始终允许").size(13))
        .on_press(Message::Chat(message::ChatMessage::PermissionApproveAlways))
        .padding([6, 12])
        .style(rounded_action_btn_style);
    let approve_once = button(text("仅此一次").size(13))
        .on_press(Message::Chat(message::ChatMessage::PermissionApproveOnce))
        .padding([6, 12])
        .style(primary_action_btn_style);

    let mut action_row = row![Space::new().width(Length::Fill), reject].spacing(8);
    if pending_requests.len() > 1 {
        action_row = action_row.push(
            button(text("全部始终允许").size(13))
                .on_press(Message::Chat(message::ChatMessage::PermissionApproveAllAlways))
                .padding([6, 12])
                .style(rounded_action_btn_style),
        );
    }
    action_row = action_row.push(always).push(approve_once);

    content = content.push(action_row);

    let card = settings_modal_card(content).width(Length::Fixed(520.0));

    root_content = settings_modal_overlay(
        Some(root_content),
        Message::Chat(message::ChatMessage::PermissionReject),
        card,
    );

    root_content
}

fn permission_modal_title(req: &vw_gateway_client::PendingPermissionRequestDto) -> String {
    req.metadata
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            permission_argument_summary(req)
                .map(|summary| format!("批准{}：{}", tool_verb(&req.permission), summary))
                .unwrap_or_else(|| format!("需要批准 {} 操作", req.permission))
        })
}

fn permission_request_selector_label(
    req: &vw_gateway_client::PendingPermissionRequestDto,
) -> String {
    let prefix = tool_verb(&req.permission);
    let summary = permission_argument_summary(req).unwrap_or_else(|| req.permission.clone());
    truncate_chars(&format!("{} · {}", prefix, summary), 80).to_string()
}

fn permission_meta_line<'a>(label: &str, value: &str) -> iced::widget::Row<'a, Message> {
    use iced::widget::{row, text};

    row![text(format!("{}：", label)).size(12), text(value.to_string()).size(13),].spacing(6)
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
    use iced::widget::{column, container, text};

    column![
        text(label).size(13),
        container(text(value).size(12)).width(Length::Fill).padding([10, 12]).style(
            |theme: &Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(palette.background.weak.color.scale_alpha(0.45).into()),
                    border: iced::Border {
                        width: 1.0,
                        color: palette.background.strong.color.scale_alpha(0.40),
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                }
            }
        )
    ]
    .spacing(6)
}
#[cfg(test)]
#[path = "permission_modal_tests.rs"]
mod permission_modal_tests;
