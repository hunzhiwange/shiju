//! Confirmation and retry for authorization account operations.

use crate::app::{
    Message,
    components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_close_button,
        settings_error_banner, settings_field_label, settings_modal_header, settings_modal_layout,
        settings_modal_overlay_with_escape, settings_muted_text_style,
    },
    message::agent_apps_auth::{Event, actions::ActionEvent, message},
    state::agent_apps_auth::{AccountAction, AccountOperation, ActionPhase},
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};

pub(super) fn overlay<'a>(
    action: &'a AccountAction,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    use AccountOperation::*;
    let generation = action.generation;
    let close = message(Event::AccountAction(ActionEvent::Close(generation)));
    let busy = matches!(action.phase, ActionPhase::Saving);
    let description = match action.operation {
        SetDefault => "在当前执行主机上将此账号设为此服务的默认账号。",
        Delete => "删除此账号及其托管凭据。执行主机会拒绝删除仍被供应商引用的账号。",
        Logout => "清除当前执行主机上的登录凭据，保留账号记录与供应商绑定。",
        Revoke => "先在执行主机登出，再请求上游撤销授权。",
    };
    let mut body = column![
        settings_field_label("授权账号"),
        text(&action.account.label).wrapping(Wrapping::WordOrGlyph),
        text(super::service_name(action.account.service)).size(13).style(settings_muted_text_style),
        text(description).size(13).wrapping(Wrapping::WordOrGlyph),
    ]
    .width(Length::Fill)
    .spacing(8);

    body = match &action.phase {
        ActionPhase::Checking => body.push(
            text(if action.operation == Delete {
                "正在检查绑定使用者…"
            } else {
                "正在检查执行主机能力…"
            })
            .style(settings_muted_text_style),
        ),
        ActionPhase::Ready(users) if action.operation == Delete => {
            let mut body = body.push(settings_field_label("绑定使用者"));
            if users.is_empty() {
                body = body.push(text("未查到可见的供应商绑定。").style(settings_muted_text_style));
            } else {
                body = body.push(settings_error_banner("请解除以下供应商的绑定，再重新检查。"));
                for user in users {
                    body = body.push(text(user).wrapping(Wrapping::WordOrGlyph));
                }
            }
            body
        }
        ActionPhase::Blocked(reason)
        | ActionPhase::CheckFailed(reason)
        | ActionPhase::SaveFailed(reason) => body.push(settings_error_banner(*reason)),
        _ => body,
    };

    let mut footer = row![
        button(text("取消"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!busy).then(|| close.clone())),
    ]
    .spacing(8);
    if action.phase.can_review() {
        footer = footer.push(
            button(text("重新检查"))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(message(Event::AccountAction(ActionEvent::Review(generation)))),
        );
    }
    if action.phase.can_confirm() || busy {
        let label = match action.phase {
            ActionPhase::Saving => "正在提交…",
            ActionPhase::SaveFailed(_) => "重试",
            _ => action.operation.label(),
        };
        footer = footer.push(
            button(text(label)).padding([8, 14]).style(primary_action_btn_style).on_press_maybe(
                (!busy).then(|| message(Event::AccountAction(ActionEvent::Confirm(generation)))),
            ),
        );
    }
    let card = settings_modal_layout(
        settings_modal_header(
            text(action.operation.label()).size(18),
            settings_close_button(close.clone()),
        ),
        body,
        Some(container(footer).align_right(Length::Fill).into()),
    )
    .max_width(520)
    .max_height(425)
    .width(Length::Fill)
    .height(Length::Fill);
    settings_modal_overlay_with_escape(base, close, card)
}
