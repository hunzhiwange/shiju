//! Link preview dialogs; host supplies only launch context and handoff messages.
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

pub fn provider<'a, M: Clone + 'static>(
    base: Element<'a, M>,
    url: &str,
    close: M,
) -> Element<'a, M> {
    let parsed = desktop_platform::provider_link_preview::parse(url);
    let body = desktop_platform::provider_link_preview::body(parsed).map(|_| unreachable!());
    let card = settings_modal_layout(
        settings_modal_header(text("供应商链接预览"), settings_close_button(close.clone())),
        body,
        None,
    )
    .width(Length::Fixed(560.0))
    .height(Length::Fixed(340.0));
    settings_modal_overlay(Some(base), close, card)
}

pub fn mcp<'a, M: Clone + 'static>(
    base: Element<'a, M>,
    url: &str,
    close: M,
    host: &str,
    accept: M,
) -> Element<'a, M> {
    let parsed = desktop_platform::mcp_link_preview::parse(url, host);
    let valid = parsed.is_ok();
    let body = desktop_platform::mcp_link_preview::body(parsed).map(|_| unreachable!());
    let footer = row![
        button("继续预览")
            .padding([8, 10])
            .style(primary_action_btn_style)
            .on_press_maybe(valid.then_some(accept)),
        button("拒绝链接").padding([8, 10]).style(rounded_action_btn_style).on_press(close.clone())
    ]
    .spacing(8);
    settings_modal_overlay(
        Some(base),
        close.clone(),
        settings_modal_layout(
            settings_modal_header(text("MCP 链接预览"), settings_close_button(close)),
            body,
            Some(footer.into()),
        )
        .style(settings_modal_card_style)
        .width(640)
        .height(360),
    )
}

pub fn prompt<'a, M: Clone + 'static>(
    base: Element<'a, M>,
    url: &str,
    close: M,
    accept: M,
) -> Element<'a, M> {
    let parsed = desktop_platform::prompt_link_preview::parse(url);
    let valid = parsed.is_ok();
    let error = parsed.as_ref().err().map(|error| error.to_string());
    let content = desktop_platform::prompt_link_preview::body(parsed).map(|_| unreachable!());
    let body = column![content].spacing(12);
    let body = if let Some(error) = error { body.push(settings_error_banner(error)) } else { body };
    let footer = row![
        button("继续确认导入")
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(valid.then_some(accept)),
        button("取消").padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
    ]
    .spacing(10);
    settings_modal_overlay(
        Some(base),
        close.clone(),
        settings_modal_layout(
            settings_modal_header(text("提示词链接预览"), settings_close_button(close)),
            body,
            Some(footer.into()),
        )
        .style(settings_modal_card_style)
        .width(700)
        .height(620),
    )
}

pub fn skill<'a, M: Clone + 'static>(
    base: Element<'a, M>,
    url: &str,
    close: M,
    host: &str,
    directory: Option<&str>,
    repositories_open: bool,
    accept: M,
) -> Element<'a, M> {
    let parsed = desktop_platform::skill_link_preview::parse(
        url,
        directory.unwrap_or_default().to_string(),
        host.to_string(),
    );
    let valid = parsed.is_ok();
    let error = parsed.as_ref().err().map(|error| error.to_string());
    let content = desktop_platform::skill_link_preview::body(parsed).map(|_| unreachable!());
    let body = column![content].spacing(12);
    let body = if let Some(error) = error { body.push(settings_error_banner(error)) } else { body };
    let footer = row![
        button("继续确认仓库")
            .padding([8, 10])
            .style(primary_action_btn_style)
            .on_press_maybe((valid && directory.is_some() && !repositories_open).then_some(accept)),
        button("拒绝链接").padding([8, 10]).style(rounded_action_btn_style).on_press(close.clone()),
    ]
    .spacing(8);
    let card = settings_modal_layout(
        settings_modal_header(text("技能链接预览"), settings_close_button(close.clone())),
        body,
        Some(footer.into()),
    )
    .style(settings_modal_card_style)
    .width(640)
    .height(420);
    settings_modal_overlay(Some(base), close, card)
}
