//! Scoped account picker opened from a saved supplier's editor.

use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use crate::{types::*, Event};

fn message<U>(event: Event<U>) -> Event<U> { event }
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};
use vw_api_app_auth::AuthRegion;

pub(crate) fn overlay<'a, U: Clone + 'static>(
    binding: &'a Binding,
    i18n: I18n,
    base: Element<'a, Event<U>>,
) -> Element<'a, Event<U>> {
    let generation = binding.generation;
    let busy = binding.loading || binding.saving;
    let close = message(Event::CloseBinding(generation));
    let mut body = column![
        settings_field_label(i18n.__("供应商")),
        text(&binding.provider_name).wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(8)
    .width(Length::Fill);
    if let Some(region) = binding.region {
        let name = match region {
            AuthRegion::MainlandCn => i18n.__("中国大陆"),
            AuthRegion::Global => i18n.__("国际"),
        };
        body =
            body.push(text(desktop_i18n::__!(i18n, "授权地区：%s", (name).to_string()).expect("valid settings message")).size(13).style(settings_muted_text_style));
    }
    if let Some(error) = binding.error {
        body = body.push(settings_error_banner(i18n.__(error)));
    }
    if !binding.connected {
        body = body.push(text(i18n.__("账号尚未加载，请重试。"))).push(
            button(text(i18n.__("读取账号")))
                .style(rounded_action_btn_style)
                .padding([8, 14])
                .on_press(message(Event::LoadBinding(generation))),
        );
    } else if binding.loading {
        body = body.push(text(i18n.__("正在读取授权账号…")).style(settings_muted_text_style));
    } else {
        body = body.push(settings_field_label(i18n.__("可用账号"))).push(
            text(i18n.__("仅显示与此供应商兼容且已授权的账号。")).size(13).style(settings_muted_text_style),
        );
        if binding.accounts.is_empty() && binding.error.is_none() {
            body = body.push(
                text(i18n.__("暂无可用账号。可在授权中心登录后重新选择。")).style(settings_muted_text_style),
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
            button(text(if binding.error.is_some() { i18n.__("重新读取") } else { i18n.__("刷新账号") }))
                .style(rounded_action_btn_style)
                .padding([8, 14])
                .on_press_maybe((!busy).then(|| message(Event::LoadBinding(generation)))),
        );
    }
    let footer = row![
        button(text(i18n.__("取消")))
            .style(rounded_action_btn_style)
            .padding([8, 14])
            .on_press_maybe((!binding.saving).then(|| close.clone())),
        button(text(if binding.saving { i18n.__("正在保存…") } else { i18n.__("保存") }))
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
            text(desktop_i18n::__!(i18n, "%s · 选择授权账号", (super::service_name(i18n, binding.service)).to_string()).expect("valid settings message")).size(18),
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
