//! 应用中心的新增和编辑书签弹窗。

use super::ui;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_field_label, settings_group_card,
    settings_group_heading_with_help, settings_hover_help_label, settings_modal_header,
    settings_modal_layout, settings_text_editor_style, settings_text_input_style,
};
use crate::app::i18n::I18n;
use crate::app::message::ViewMessage;
use crate::app::{App, Message};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, button, column, container, row, text, text::Wrapping, text_editor, text_input,
};
use iced::{Alignment, Element, Length};

const WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE: &str = r#"[
  {
    "name": "session_id",
    "domain": "example.com",
    "days": 365,
    "url_filter": "https://example.com/"
  }
]"#;

fn can_add_web_bookmark(title: &str, url: &str) -> bool {
    !title.trim().is_empty() && !url.trim().is_empty()
}

fn bookmark_field<'a>(
    label: &'a str,
    value: &'a str,
    on_input: fn(String) -> ViewMessage,
) -> Element<'a, Message> {
    column![
        settings_field_label(label),
        text_input("", value)
            .on_input(move |value| Message::View(on_input(value)))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

fn edit_web_bookmark_modal(app: &App) -> Option<(Element<'_, Message>, Message)> {
    let idx = app.editing_web_bookmark.unwrap_or(0);
    let i18n = I18n::new(app.desktop_locale);
    let close = Message::View(ViewMessage::WebBookmarkEditCancel);
    let header = settings_modal_header(
        text(i18n.__("编辑网址")).size(20),
        settings_close_button(close.clone()),
    );
    let fields = settings_group_card(
        column![
            bookmark_field(
                i18n.__("标题"),
                &app.edit_web_bookmark_title_input,
                ViewMessage::WebBookmarkEditTitleChanged,
            ),
            bookmark_field(
                i18n.__("网址"),
                &app.edit_web_bookmark_url_input,
                ViewMessage::WebBookmarkEditUrlChanged,
            ),
            row![
                bookmark_field(
                    i18n.__("宽度（可选）"),
                    &app.edit_web_bookmark_width_input,
                    ViewMessage::WebBookmarkEditWidthChanged,
                ),
                bookmark_field(
                    i18n.__("高度（可选）"),
                    &app.edit_web_bookmark_height_input,
                    ViewMessage::WebBookmarkEditHeightChanged,
                ),
            ]
            .spacing(10),
        ]
        .spacing(12)
        .padding([12, 0]),
    );

    let cookie_label = i18n.__("Cookie配置(JSON)");
    let cookie_help = format!(
        "{}\n\n{}\n{WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE}",
        i18n.__("支持按 JSON 为指定站点注入 Cookie。"),
        i18n.__("配置示例"),
    );
    let cookie = settings_group_card(
        column![
            row![
                settings_hover_help_label(
                    i18n,
                    &app.app_theme,
                    cookie_label,
                    settings_field_label(cookie_label),
                    cookie_help,
                )
                .wrap(),
                Tooltip::new(
                    button(text(i18n.__("插入例子")).size(13))
                        .on_press(Message::View(
                            ViewMessage::WebBookmarkEditCookieConfigsInsertExample
                        ))
                        .style(rounded_action_btn_style)
                        .padding([6, 10]),
                    ui::tooltip_bubble(i18n.__("插入预设 Cookie 配置示例")),
                    TooltipPosition::Top,
                )
                .gap(8.0),
            ]
            .align_y(Alignment::Center)
            .spacing(8)
            .wrap(),
            text_editor(&app.edit_web_bookmark_cookie_configs_editor)
                .on_action(|action| {
                    Message::View(ViewMessage::WebBookmarkEditCookieConfigsChanged(action))
                })
                .height(Length::Fixed(120.0))
                .wrapping(Wrapping::WordOrGlyph)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_editor_style),
        ]
        .spacing(8)
        .padding([12, 0]),
    );
    let body = column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("书签信息"),
            i18n.__("设置标题、网址以及独立窗口的默认尺寸。")
        ),
        fields,
        text(i18n.__("Cookie 配置")).size(17),
        cookie,
    ]
    .spacing(12);
    let actions = row![
        button(text(i18n.__("删除")).size(13))
            .on_press(Message::View(ViewMessage::WebBookmarkRemove(idx)))
            .padding([8, 14])
            .style(rounded_action_btn_style),
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .on_press(close.clone())
            .padding([8, 14])
            .style(rounded_action_btn_style),
        button(text(i18n.__("保存")).size(13))
            .on_press(Message::View(ViewMessage::WebBookmarkEditSave))
            .padding([8, 14])
            .style(primary_action_btn_style),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(560).height(610);
    Some((container(card).padding(12).into(), close))
}

fn add_web_bookmark_modal(app: &App) -> Option<(Element<'_, Message>, Message)> {
    let i18n = I18n::new(app.desktop_locale);
    let close = Message::View(ViewMessage::ToggleWebLinksMenu);
    let header = settings_modal_header(
        text(i18n.__("添加网址")).size(20),
        settings_close_button(close.clone()),
    );
    let fields = settings_group_card(
        column![
            bookmark_field(
                i18n.__("标题"),
                &app.web_bookmark_title_input,
                ViewMessage::WebBookmarkTitleChanged,
            ),
            bookmark_field(
                i18n.__("网址"),
                &app.web_bookmark_url_input,
                ViewMessage::WebBookmarkUrlChanged,
            ),
            row![
                bookmark_field(
                    i18n.__("宽度（可选）"),
                    &app.web_bookmark_width_input,
                    ViewMessage::WebBookmarkWidthChanged,
                ),
                bookmark_field(
                    i18n.__("高度（可选）"),
                    &app.web_bookmark_height_input,
                    ViewMessage::WebBookmarkHeightChanged,
                ),
            ]
            .spacing(10),
        ]
        .spacing(12)
        .padding([12, 0]),
    );
    let body = column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("新增网址书签"),
            i18n.__("保存一个可在应用中心快速打开的网页入口。"),
        ),
        fields,
    ]
    .spacing(12);
    let can_add = can_add_web_bookmark(&app.web_bookmark_title_input, &app.web_bookmark_url_input);
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .on_press(close.clone())
            .padding([8, 14])
            .style(rounded_action_btn_style),
        button(text(i18n.__("添加")).size(13))
            .on_press_maybe(can_add.then_some(Message::View(ViewMessage::WebBookmarkAddSave)))
            .padding([8, 14])
            .style(primary_action_btn_style),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(560).height(445);
    Some((container(card).padding(12).into(), close))
}

pub(super) fn active_modal(app: &App) -> Option<(Element<'_, Message>, Message)> {
    if app.editing_web_bookmark.is_some() {
        edit_web_bookmark_modal(app)
    } else if app.show_web_links_menu {
        add_web_bookmark_modal(app)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "modals_tests.rs"]
mod modals_tests;
