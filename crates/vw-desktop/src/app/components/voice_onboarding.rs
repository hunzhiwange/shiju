//! Explicit setup only: opening this modal never captures focus or requests permission.
use super::system_settings_common::*;
use crate::app::voice::permissions::{Permission, PermissionStatus};
use crate::app::{
    App, Message,
    message::voice::{Event, OnboardingEvent},
};
use iced::{
    Element, Length,
    widget::{button, column, row, text},
};
fn msg(event: OnboardingEvent) -> Message {
    Message::Voice(Event::Onboarding(event))
}
pub(crate) fn with_onboarding<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !app.voice.insertion.onboarding {
        return base;
    }
    modal(base, app.voice.insertion.permissions, app.voice.insertion.pending.as_deref())
}
pub(super) fn modal<'a>(
    base: Element<'a, Message>,
    statuses: [PermissionStatus; 3],
    pending: Option<&'a str>,
) -> Element<'a, Message> {
    let close = msg(OnboardingEvent::Close);
    let mut body = column![
        text("试用跨应用听写").size(19),
        text("先在语音设置中配置并保存全局听写快捷键；关闭此弹窗，切换到其它应用的普通文本框，再按住快捷键说话。"),
        text("松开完成；双击免提，再按完成。Escape 取消。录音不会激活 ShiJu；焦点、文本或选择区变化后拒绝插入。"),
        text("仅 macOS；密码、安全输入、只读或无法核验的目标不写入。无法插入时结果保留在这里，由你点击复制，不自动修改剪贴板。"),
    ].spacing(12);
    for ((permission, label), status) in [
        (Permission::Microphone, "麦克风"),
        (Permission::InputMonitoring, "输入监控"),
        (Permission::Accessibility, "辅助功能"),
    ]
    .into_iter()
    .zip(statuses)
    {
        let state = match status {
            PermissionStatus::Granted => "已允许",
            PermissionStatus::Denied => "已拒绝",
            PermissionStatus::Restricted => "受限制",
            PermissionStatus::Unsupported => "当前平台不可用",
            PermissionStatus::Unknown => "尚未确认",
            PermissionStatus::NotDetermined => "尚未申请",
        };
        body = body.push(
            row![
                text(format!("{label}：{state}")).width(Length::Fill),
                button(text("检查 / 授权"))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(msg(OnboardingEvent::Permission(permission)))
            ]
            .spacing(8),
        );
    }
    if let Some(text_value) = pending {
        body = body
            .push(text("保留的听写结果：写入未确认时先检查目标，避免重复插入").size(16))
            .push(text(text_value));
    }
    let mut footer = row![
        button(text("刷新权限"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(msg(OnboardingEvent::Refresh))
    ]
    .spacing(8);
    if pending.is_some() {
        footer = footer.push(
            button(text("复制听写结果"))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press(msg(OnboardingEvent::Copy)),
        );
        footer = footer.push(
            button(text("丢弃结果"))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(msg(OnboardingEvent::Discard)),
        );
    }
    footer = footer.push(
        button(text("关闭，前往目标应用试用"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    );
    settings_modal_overlay(
        Some(base),
        close.clone(),
        settings_modal_layout(
            settings_modal_header(text("跨应用听写").size(20), settings_close_button(close)),
            body,
            Some(footer.wrap().into()),
        )
        .width(680)
        .height(Length::Fill),
    )
}
#[cfg(test)]
#[path = "voice_onboarding_tests.rs"]
mod tests;
