//! Confirmation and retry for authorization account operations.

use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use crate::{types::*, Event, ActionEvent};

fn message<U>(event: Event<U>) -> Event<U> { event }
use iced::{
    Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};

pub(super) fn overlay<'a, U: Clone + 'static>(
    i18n: I18n,
    action: &'a AccountAction,
    base: Element<'a, Event<U>>,
) -> Element<'a, Event<U>> {
    use AccountOperation::*;
    let generation = action.generation;
    let close = message(Event::AccountAction(ActionEvent::Close(generation)));
    let busy = matches!(action.phase, ActionPhase::Saving);
    let description = match action.operation {
        SetDefault => i18n.__("将此账号设为该服务的默认账号。"),
        Delete => i18n.__("删除此账号及其托管凭据。请先解除供应商对该账号的引用。"),
        Logout => i18n.__("清除登录凭据，保留账号记录与供应商绑定。"),
        Revoke => i18n.__("先登出账号，再请求上游撤销授权。"),
    };
    let mut body = column![
        settings_field_label(i18n.__("授权账号")),
        text(&action.account.label).wrapping(Wrapping::WordOrGlyph),
        text(super::service_name(i18n, action.account.service)).size(13).style(settings_muted_text_style),
        text(i18n.__(description)).size(13).wrapping(Wrapping::WordOrGlyph),
    ]
    .width(Length::Fill)
    .spacing(8);

    body = match &action.phase {
        ActionPhase::Checking => body.push(
            text(if action.operation == Delete {
                i18n.__("正在检查绑定使用者…")
            } else {
                i18n.__("正在检查可用操作…")
            })
            .style(settings_muted_text_style),
        ),
        ActionPhase::Ready(users) if action.operation == Delete => {
            let mut body = body.push(settings_field_label(i18n.__("绑定使用者")));
            if users.is_empty() {
                body = body.push(text(i18n.__("未查到可见的供应商绑定。")).style(settings_muted_text_style));
            } else {
                body = body.push(settings_error_banner(i18n.__("请解除以下供应商的绑定，再重新检查。")));
                for user in users {
                    body = body.push(text(user).wrapping(Wrapping::WordOrGlyph));
                }
            }
            body
        }
        ActionPhase::Blocked(reason)
        | ActionPhase::CheckFailed(reason)
        | ActionPhase::SaveFailed(reason) => body.push(settings_error_banner(i18n.__(reason))),
        _ => body,
    };

    let mut footer = row![
        button(text(i18n.__("取消")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!busy).then(|| close.clone())),
    ]
    .spacing(8);
    if action.phase.can_review() {
        footer = footer.push(
            button(text(i18n.__("重新检查")))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(message(Event::AccountAction(ActionEvent::Review(generation)))),
        );
    }
    if action.phase.can_confirm() || busy {
        let label = match action.phase {
            ActionPhase::Saving => i18n.__("正在提交…"),
            ActionPhase::SaveFailed(_) => i18n.__("重试"),
            _ => i18n.__(action.operation.label()),
        };
        footer = footer.push(
            button(text(label)).padding([8, 14]).style(primary_action_btn_style).on_press_maybe(
                (!busy).then(|| message(Event::AccountAction(ActionEvent::Confirm(generation)))),
            ),
        );
    }
    let card = settings_modal_layout(
        settings_modal_header(
            text(i18n.__(action.operation.label())).size(18),
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
