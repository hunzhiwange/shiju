//! Link preview dialogs; host supplies only launch context and handoff messages.
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

pub fn provider<'a, M: Clone + 'static>(
    base: Element<'a, M>,
    url: &str,
    i18n: I18n,
    close: M,
) -> Element<'a, M> {
    let (title, lines) = provider_content(url, i18n);
    let body = column(lines.into_iter().map(|line| text(line).into())).spacing(12);
    let card = settings_modal_layout(
        settings_modal_header(text(title), settings_close_button(close.clone())),
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
    i18n: I18n,
    close: M,
    host: &str,
    accept: M,
) -> Element<'a, M> {
    let parsed = desktop_platform::mcp_link_preview::parse(url, host);
    let valid = parsed.is_ok();
    let body = desktop_platform::mcp_link_preview::body(parsed, i18n).map(|_| unreachable!());
    let footer = row![
        button(preview_actions(i18n)[0])
            .padding([8, 10])
            .style(primary_action_btn_style)
            .on_press_maybe(valid.then_some(accept)),
        button(preview_actions(i18n)[1])
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press(close.clone())
    ]
    .spacing(8);
    settings_modal_overlay(
        Some(base),
        close.clone(),
        settings_modal_layout(
            settings_modal_header(text(i18n.__("MCP 链接预览")), settings_close_button(close)),
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
    i18n: I18n,
    close: M,
    accept: M,
) -> Element<'a, M> {
    let parsed = desktop_platform::prompt_link_preview::parse(url);
    let valid = parsed.is_ok();
    let error = parsed.as_ref().err().map(|error| i18n.__(&error).to_owned());
    let content = desktop_platform::prompt_link_preview::body(parsed, i18n).map(|_| unreachable!());
    let body = column![content].spacing(12);
    let body = if let Some(error) = error { body.push(settings_error_banner(error)) } else { body };
    let footer = row![
        button(i18n.__("继续确认导入"))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(valid.then_some(accept)),
        button(i18n.__("取消"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(10);
    settings_modal_overlay(
        Some(base),
        close.clone(),
        settings_modal_layout(
            settings_modal_header(text(i18n.__("提示词链接预览")), settings_close_button(close)),
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
    i18n: I18n,
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
    let error = parsed.as_ref().err().map(|error| i18n.__(&error).to_owned());
    let content = desktop_platform::skill_link_preview::body(parsed, i18n).map(|_| unreachable!());
    let body = column![content].spacing(12);
    let body = if let Some(error) = error { body.push(settings_error_banner(error)) } else { body };
    let footer = row![
        button(i18n.__("继续确认仓库"))
            .padding([8, 10])
            .style(primary_action_btn_style)
            .on_press_maybe((valid && directory.is_some() && !repositories_open).then_some(accept)),
        button(preview_actions(i18n)[1])
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8);
    let card = settings_modal_layout(
        settings_modal_header(text(i18n.__("技能链接预览")), settings_close_button(close.clone())),
        body,
        Some(footer.into()),
    )
    .style(settings_modal_card_style)
    .width(640)
    .height(420);
    settings_modal_overlay(Some(base), close, card)
}

/// State generation used by the real provider overlay; no payload values are translated.
pub(crate) fn provider_content(url: &str, i18n: I18n) -> (&'static str, Vec<String>) {
    (
        i18n.__("供应商链接预览"),
        desktop_platform::provider_link_preview::lines(
            desktop_platform::provider_link_preview::parse(url),
            i18n,
        ),
    )
}

pub(crate) fn preview_actions(i18n: I18n) -> [&'static str; 2] {
    [i18n.__("继续预览"), i18n.__("拒绝链接")]
}
