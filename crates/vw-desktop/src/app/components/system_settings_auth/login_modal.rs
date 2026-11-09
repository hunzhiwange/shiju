//! Device authorization challenge presentation.

use crate::app::{
    Message,
    components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_close_button,
        settings_error_banner, settings_field_label, settings_modal_header, settings_modal_layout,
        settings_modal_overlay_with_escape, settings_muted_text_style,
    },
    message::agent_apps_auth::{Event, message},
    state::agent_apps_auth::{DeviceChallenge, DeviceLogin, DevicePhase},
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};
use std::time::{SystemTime, UNIX_EPOCH};
use vw_gateway_client::vw_api_types::{TimestampMs, agent_apps_auth::AuthRegion};

impl DeviceChallenge {
    pub(super) fn detail(&self) -> (&'static str, &str) {
        match self.region {
            Some(AuthRegion::MainlandCn) => ("授权地区", "中国大陆"),
            Some(AuthRegion::Global) => ("授权地区", "国际"),
            None => {
                let authority = self.verification_url.authority();
                ("授权域", authority)
            }
        }
    }

    pub(super) fn remaining_seconds(&self, now: TimestampMs) -> i64 {
        let remaining = self.expires_at.0.saturating_sub(now.0).max(0);
        remaining / 1_000 + i64::from(remaining % 1_000 != 0)
    }
}

fn now() -> TimestampMs {
    TimestampMs(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
            .unwrap_or(0),
    )
}

pub(super) fn overlay<'a>(
    login: &'a DeviceLogin,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let close = message(Event::CloseDevice);
    let body: Element<'_, Message> = match &login.phase {
        DevicePhase::Starting => {
            text("正在向执行主机请求设备码…").style(settings_muted_text_style).into()
        }
        DevicePhase::Pending(challenge) => {
            let (label, detail) = challenge.detail();
            let remaining = challenge.remaining_seconds(now());
            column![
                text("请在浏览器中访问以下地址并输入设备码。").style(settings_muted_text_style),
                settings_field_label("设备码"),
                text(&challenge.user_code).size(24).wrapping(Wrapping::WordOrGlyph),
                settings_field_label(label),
                text(detail).wrapping(Wrapping::WordOrGlyph),
                settings_field_label("验证地址"),
                text(challenge.verification_url.as_str()).wrapping(Wrapping::WordOrGlyph),
                text(format!("剩余 {remaining} 秒")).style(settings_muted_text_style),
            ]
            .spacing(8)
            .into()
        }
        DevicePhase::Failed(reason) => settings_error_banner(*reason),
    };
    let mut footer = row![
        button(text("取消"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if matches!(login.phase, DevicePhase::Failed(_)) {
        footer = footer.push(
            button(text("重试"))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press(message(Event::RetryDevice)),
        );
    }
    let title = format!("{} · 设备码登录", super::service_name(login.target.service()));
    let card = settings_modal_layout(
        settings_modal_header(text(title).size(18), settings_close_button(close.clone())),
        body,
        Some(container(footer).align_right(Length::Fill).into()),
    )
    .max_width(500)
    .max_height(390)
    .width(Length::Fill)
    .height(Length::Fill);
    settings_modal_overlay_with_escape(base, close, card)
}
