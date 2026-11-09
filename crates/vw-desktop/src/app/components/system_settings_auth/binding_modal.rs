//! Scoped account picker opened from a saved supplier's editor.

use crate::app::{
    Message,
    components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_close_button,
        settings_error_banner, settings_field_label, settings_modal_header, settings_modal_layout,
        settings_modal_overlay_with_escape, settings_muted_text_style,
    },
    message::agent_apps_auth::{Event, message},
    state::agent_apps_auth::Binding,
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};
use vw_gateway_client::vw_api_types::agent_apps_auth::AuthRegion;

pub(in crate::app) fn overlay<'a>(
    binding: &'a Binding,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let generation = binding.generation;
    let busy = binding.loading || binding.saving;
    let close = message(Event::CloseBinding(generation));
    let mut body = column![
        settings_field_label("供应商"),
        text(&binding.provider_name).wrapping(Wrapping::WordOrGlyph),
        text(format!("执行主机：{}", binding.host))
            .size(13)
            .style(settings_muted_text_style)
            .wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(8)
    .width(Length::Fill);
    if let Some(region) = binding.region {
        let name = match region {
            AuthRegion::MainlandCn => "中国大陆",
            AuthRegion::Global => "国际",
        };
        body =
            body.push(text(format!("授权地区：{name}")).size(13).style(settings_muted_text_style));
    }
    if let Some(error) = binding.error {
        body = body.push(settings_error_banner(error));
    }
    if !binding.connected {
        body = body.push(text("请使用当前供应商所属主机和主体建立管理连接。")).push(
            button(text("管理连接"))
                .style(rounded_action_btn_style)
                .padding([8, 14])
                .on_press(message(Event::Open)),
        );
    } else if binding.loading {
        body = body.push(text("正在读取授权账号…").style(settings_muted_text_style));
    } else {
        body = body.push(settings_field_label("可用账号")).push(
            text("仅显示与此供应商兼容且已授权的账号。").size(13).style(settings_muted_text_style),
        );
        if binding.accounts.is_empty() && binding.error.is_none() {
            body = body.push(
                text("暂无可用账号。可在授权中心登录后重新选择。").style(settings_muted_text_style),
            );
        }
        for account in &binding.accounts {
            let selected = binding.selected.as_ref() == Some(&account.account_id);
            let label =
                if selected { format!("✓ {}", account.label) } else { account.label.clone() };
            body = body.push(
                button(text(label).wrapping(Wrapping::WordOrGlyph))
                    .width(Length::Fill)
                    .style(if selected {
                        primary_action_btn_style
                    } else {
                        rounded_action_btn_style
                    })
                    .padding([8, 14])
                    .on_press_maybe((!busy).then(|| {
                        message(Event::SelectBinding(generation, account.account_id.clone()))
                    })),
            );
        }
        body = body.push(
            button(text(if binding.error.is_some() { "重新读取" } else { "刷新账号" }))
                .style(rounded_action_btn_style)
                .padding([8, 14])
                .on_press_maybe((!busy).then(|| message(Event::LoadBinding(generation)))),
        );
    }
    let footer = row![
        button(text("取消"))
            .style(rounded_action_btn_style)
            .padding([8, 14])
            .on_press_maybe((!binding.saving).then(|| close.clone())),
        button(text(if binding.saving { "正在保存…" } else { "保存" }))
            .style(primary_action_btn_style)
            .padding([8, 14])
            .on_press_maybe(
                (!busy && binding.selected.is_some())
                    .then(|| message(Event::SaveBinding(generation)))
            ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(
        settings_modal_header(
            text(format!("{} · 选择授权账号", super::service_name(binding.service))).size(18),
            settings_close_button(close.clone()),
        ),
        body,
        Some(container(footer).align_right(Length::Fill).into()),
    )
    .max_width(500)
    .max_height(440)
    .width(Length::Fill)
    .height(Length::Fill);
    settings_modal_overlay_with_escape(base, close, card)
}
