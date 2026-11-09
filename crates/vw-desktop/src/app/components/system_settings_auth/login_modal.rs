//! Device and browser authorization challenge presentation.

use crate::app::{
    Message,
    components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_close_button,
        settings_error_banner, settings_field_label, settings_modal_header, settings_modal_layout,
        settings_modal_overlay_with_escape, settings_muted_text_style, settings_text_input,
    },
    message::agent_apps_auth::{
        Event,
        browser::{LinkAction, accepts_paste},
        message,
    },
    state::agent_apps_auth::{Login, LoginChallenge, LoginPhase, now},
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};
use vw_gateway_client::vw_api_types::{TimestampMs, agent_apps_auth::AuthRegion};

impl LoginChallenge {
    pub(super) fn detail(&self) -> (&'static str, &str) {
        match self.region {
            Some(AuthRegion::MainlandCn) => ("授权地区", "中国大陆"),
            Some(AuthRegion::Global) => ("授权地区", "国际"),
            None => ("授权域", self.verification_url.authority()),
        }
    }

    pub(super) fn remaining_seconds(&self, now: TimestampMs) -> i64 {
        let remaining = self.expires_at.0.saturating_sub(now.0).max(0);
        remaining / 1_000 + i64::from(remaining % 1_000 != 0)
    }
}

pub(super) fn overlay<'a>(login: &'a Login, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = message(Event::CloseLogin);
    let generation = login.generation;
    let is_device = login.target.is_device();
    let submit = || message(Event::PasteSubmit(generation));
    let body: Element<'_, Message> = match &login.phase {
        LoginPhase::Starting => text(if is_device {
            "正在向执行主机请求设备码…"
        } else {
            "正在向执行主机请求授权链接…"
        })
        .style(settings_muted_text_style)
        .into(),
        LoginPhase::Pending(challenge) => {
            let (label, detail) = challenge.detail();
            let remaining = challenge.remaining_seconds(now());
            let mut fields = column![
                text(if is_device {
                    "请在浏览器中访问以下地址并输入设备码。"
                } else {
                    "请在浏览器中完成授权。"
                })
                .style(settings_muted_text_style)
            ]
            .spacing(8);
            if let Some(code) = &challenge.user_code {
                fields = fields
                    .push(settings_field_label("设备码"))
                    .push(text(code).size(24).wrapping(Wrapping::WordOrGlyph));
            }
            fields =
                fields
                    .push(
                        row![
                            settings_field_label(label),
                            text(detail).wrapping(Wrapping::WordOrGlyph)
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .push(text(format!("剩余 {remaining} 秒")).style(settings_muted_text_style))
                    .push(
                        row![
                            button(text("打开浏览器"))
                                .style(rounded_action_btn_style)
                                .padding([6, 12])
                                .on_press_maybe((remaining > 0).then(|| message(
                                    Event::LoginLink(generation, LinkAction::Open)
                                ))),
                            button(text("复制链接"))
                                .style(rounded_action_btn_style)
                                .padding([6, 12])
                                .on_press_maybe((remaining > 0).then(|| message(
                                    Event::LoginLink(generation, LinkAction::Copy)
                                ))),
                        ]
                        .spacing(8),
                    );
            if challenge.can_paste {
                let mut input =
                    settings_text_input("粘贴授权码或回调地址", &login.paste_input).secure(true);
                if accepts_paste(login) {
                    input =
                        input.on_input(move |value| message(Event::PasteInput(generation, value)));
                    if !login.paste_input.trim().is_empty() {
                        input = input.on_submit(submit());
                    }
                }
                fields = fields
                    .push(settings_field_label("Claude 手工回填"))
                    .push(
                        text("若浏览器没有自动完成授权，可在此提交授权码或回调地址。")
                            .size(13)
                            .style(settings_muted_text_style),
                    )
                    .push(input);
            }
            fields
                .push(settings_field_label("验证地址"))
                .push(text(challenge.verification_url.as_str()).wrapping(Wrapping::WordOrGlyph))
                .into()
        }
        LoginPhase::Failed(reason) => settings_error_banner(*reason),
    };
    let mut footer = row![
        button(text("取消"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if matches!(login.phase, LoginPhase::Failed(_)) {
        footer = footer.push(
            button(text("重试"))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press(message(Event::RetryLogin)),
        );
    } else if let LoginPhase::Pending(challenge) = &login.phase
        && challenge.can_paste
    {
        footer = footer.push(
            button(text(if login.paste_submitting { "正在提交…" } else { "提交回填" }))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    (accepts_paste(login) && !login.paste_input.trim().is_empty()).then(submit),
                ),
        );
    }
    let title = format!(
        "{} · {}",
        super::service_name(login.target.service()),
        if is_device { "设备码登录" } else { "浏览器登录" },
    );
    let card = settings_modal_layout(
        settings_modal_header(text(title).size(18), settings_close_button(close.clone())),
        body,
        Some(container(footer).align_right(Length::Fill).into()),
    )
    .max_width(500)
    .max_height(440)
    .width(Length::Fill)
    .height(Length::Fill);
    settings_modal_overlay_with_escape(base, close, card)
}
