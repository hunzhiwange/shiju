//! Device and browser authorization challenge presentation.

use crate::{Event, LinkAction, types::*};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;

fn message<U>(event: Event<U>) -> Event<U> {
    event
}
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};
use {vw_api_app_auth::AuthRegion, vw_api_transport::TimestampMs};

impl LoginChallenge {
    pub fn detail(&self) -> (&'static str, &str) {
        match self.region {
            Some(AuthRegion::MainlandCn) => ("授权地区", "中国大陆"),
            Some(AuthRegion::Global) => ("授权地区", "国际"),
            None => ("授权域", self.verification_url.authority()),
        }
    }

    pub fn remaining_seconds(&self, now: TimestampMs) -> i64 {
        let remaining = self.expires_at.0.saturating_sub(now.0).max(0);
        remaining / 1_000 + i64::from(remaining % 1_000 != 0)
    }
}

pub(super) fn overlay<'a, U: Clone + 'static>(
    i18n: I18n,
    login: &'a Login,
    base: Element<'a, Event<U>>,
) -> Element<'a, Event<U>> {
    let close = message(Event::CloseLogin);
    let generation = login.generation;
    let is_device = login.target.is_device();
    let submit = || message(Event::PasteSubmit(generation));
    let body: Element<'_, Event<U>> = match &login.phase {
        LoginPhase::Starting => {
            let detail = if is_device {
                i18n.__("正在请求设备码。")
            } else {
                i18n.__("正在请求授权链接。")
            };
            status_panel(
                column![
                    status_heading(i18n.__("正在启动登录")),
                    text(i18n.__(detail)).size(13).style(settings_muted_text_style),
                ]
                .spacing(8),
            )
            .into()
        }
        LoginPhase::Pending(challenge) => {
            let (label, detail) = challenge.detail();
            let remaining = challenge.remaining_seconds(now());
            let mut status = column![
                row![
                    status_heading(if remaining > 0 { i18n.__("等待授权") } else { i18n.__("授权已过期") }),
                    settings_value_badge(desktop_i18n::__!(i18n, "剩余 %d 秒", remaining).expect("valid settings message")),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap(),
            ]
            .spacing(8);
            if remaining > 0 {
                status = status.push(
                    text(if is_device {
                        i18n.__("在验证页输入设备码，确认授权后此窗口会自动更新。")
                    } else {
                        i18n.__("在浏览器中完成登录和授权，此窗口会自动更新。")
                    })
                    .size(13)
                    .style(settings_muted_text_style),
                );
            }
            if let Some(code) = &challenge.user_code {
                status = status.push(
                    row![
                        settings_field_label(i18n.__("设备码")),
                        super::title(code.as_str(), 24).wrapping(Wrapping::WordOrGlyph),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .wrap(),
                );
            }
            let mut fields = column![
                status_panel(status),
                row![
                    button(
                        row![super::action_icon(Icon::Globe, 14.0), text(i18n.__("打开验证页")).size(13)]
                            .spacing(6)
                            .align_y(Alignment::Center)
                    )
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press_maybe(
                        (remaining > 0).then_some(Event::LoginLink(generation, LinkAction::Open,))
                    ),
                    button(
                        row![super::action_icon(Icon::Copy, 14.0), text(i18n.__("复制登录链接")).size(13)]
                            .spacing(6)
                            .align_y(Alignment::Center)
                    )
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press_maybe(
                        (remaining > 0).then_some(Event::LoginLink(generation, LinkAction::Copy,))
                    ),
                ]
                .spacing(8)
                .wrap(),
                column![
                    row![
                        settings_field_label(i18n.__(label)),
                        text(i18n.__(detail)).size(13).wrapping(Wrapping::WordOrGlyph),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .wrap(),
                    text(challenge.verification_url.as_str())
                        .size(12)
                        .wrapping(Wrapping::WordOrGlyph)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
            ]
            .spacing(12);
            if challenge.can_paste {
                let mut input =
                    settings_text_input(i18n.__("粘贴授权码或回调地址"), &login.paste_input).secure(true);
                if accepts_paste(login) {
                    input =
                        input.on_input(move |value| message(Event::PasteInput(generation, value)));
                    if !login.paste_input.trim().is_empty() {
                        input = input.on_submit(submit());
                    }
                }
                fields = fields
                    .push(settings_field_label(i18n.__("Claude 手工回填")))
                    .push(
                        text(i18n.__("若浏览器没有自动完成授权，可在此提交授权码或回调地址。"))
                            .size(13)
                            .style(settings_muted_text_style),
                    )
                    .push(input);
            }
            fields.into()
        }
        LoginPhase::Failed(reason) => settings_error_banner(i18n.__(reason)),
    };
    let mut footer = row![
        button(text(i18n.__("取消")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if matches!(login.phase, LoginPhase::Failed(_)) {
        footer = footer.push(
            button(text(i18n.__("重试")))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press(message(Event::RetryLogin)),
        );
    } else if let LoginPhase::Pending(challenge) = &login.phase
        && challenge.can_paste
    {
        footer = footer.push(
            button(text(if login.paste_submitting { i18n.__("正在提交…") } else { i18n.__("提交回填") }))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    (accepts_paste(login) && !login.paste_input.trim().is_empty()).then(submit),
                ),
        );
    }
    let title = format!(
        "{} · {}",
        super::service_name(i18n, login.target.service()),
        if is_device { i18n.__("设备码登录") } else { i18n.__("浏览器登录") },
    );
    let card = settings_modal_layout(
        settings_modal_header(super::title(title, 18), settings_close_button(close.clone())),
        body,
        Some(container(footer).align_right(Length::Fill).into()),
    )
    .max_width(500)
    .max_height(440)
    .width(Length::Fill)
    .height(Length::Fill);
    settings_modal_overlay_with_escape(base, close, card)
}

fn status_heading<U: Clone + 'static>(label: &'static str) -> iced::widget::Row<'static, Event<U>> {
    row![super::action_icon(Icon::Clock, 16.0), super::title(label, 15)]
        .spacing(8)
        .align_y(Alignment::Center)
}

fn status_panel<'a, U: Clone + 'static>(
    content: iced::widget::Column<'a, Event<U>>,
) -> iced::widget::Container<'a, Event<U>> {
    container(content).padding([12, 14]).width(Length::Fill).style(|theme: &iced::Theme| {
        let mut style = super::auth_panel_style(theme);
        style.background = Some(theme.extended_palette().background.weak.color.into());
        style
    })
}

fn accepts_paste(login: &Login) -> bool {
    matches!(login.target, LoginTarget::Claude)
        && !login.paste_submitting
        && matches!(&login.phase, LoginPhase::Pending(challenge)
            if challenge.can_paste && challenge.expires_at.0 > now().0)
}
