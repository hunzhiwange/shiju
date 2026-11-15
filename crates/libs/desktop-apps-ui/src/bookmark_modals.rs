//! 应用中心的新增和编辑书签弹窗。


use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_field_label, settings_group_card,
    settings_modal_header,
    settings_modal_layout, settings_text_editor_style, settings_text_input_style,
};
use desktop_i18n::I18n;


use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, button, column, container, row, text, text::Wrapping, text_editor, text_input,
};
use iced::{Alignment, Element, Length};

pub const WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE: &str = r#"[
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

fn bookmark_field<'a, M: Clone + 'a>(
    label: &'a str,
    value: &'a str,
    on_input: fn(String) -> Event,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    column![
        settings_field_label(label),
        text_input("", value)
            .on_input(move |value| message(on_input(value)))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

pub fn edit_web_bookmark_modal<'a, M: Clone + 'a>(input: Input<'a, M>) -> Option<(Element<'a, M>, M)> {
    let idx = input.editing_web_bookmark.unwrap_or(0);
    let i18n = input.i18n;
    let message = input.message;
    let close = (message)(Event::WebBookmarkEditCancel);
    let header = settings_modal_header(
        text(i18n.__("编辑网址")).size(20),
        settings_close_button(close.clone()),
    );
    let fields = settings_group_card(
        column![
            bookmark_field(
                i18n.__("标题"),
                input.edit_web_bookmark_title_input,
                Event::WebBookmarkEditTitleChanged,
                message,
            ),
            bookmark_field(
                i18n.__("网址"),
                input.edit_web_bookmark_url_input,
                Event::WebBookmarkEditUrlChanged,
                message,
            ),
            row![
                bookmark_field(
                    i18n.__("宽度（可选）"),
                    input.edit_web_bookmark_width_input,
                    Event::WebBookmarkEditWidthChanged,
                message,
                ),
                bookmark_field(
                    i18n.__("高度（可选）"),
                    input.edit_web_bookmark_height_input,
                    Event::WebBookmarkEditHeightChanged,
                message,
                ),
            ]
            .spacing(10),
        ]
        .spacing(12)
        .padding([12, 0]),
    );

    let cookie = settings_group_card(
        column![
            row![
                input.cookie_help,
                Tooltip::new(
                    button(text(i18n.__("插入例子")).size(13))
                        .on_press((message)(
                            Event::WebBookmarkEditCookieConfigsInsertExample
                        ))
                        .style(rounded_action_btn_style)
                        .padding([6, 10]),
                    tooltip_bubble(i18n.__("插入预设 Cookie 配置示例")),
                    TooltipPosition::Top,
                )
                .gap(8.0),
            ]
            .align_y(Alignment::Center)
            .spacing(8)
            .wrap(),
            text_editor(input.edit_web_bookmark_cookie_configs_editor)
                .on_action(move |action| {
                    (message)(Event::WebBookmarkEditCookieConfigsChanged(action))
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
        input.heading,
        fields,
        text(i18n.__("Cookie 配置")).size(17),
        cookie,
    ]
    .spacing(12);
    let actions = row![
        button(text(i18n.__("删除")).size(13))
            .on_press((message)(Event::WebBookmarkRemove(idx)))
            .padding([8, 14])
            .style(rounded_action_btn_style),
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .on_press(close.clone())
            .padding([8, 14])
            .style(rounded_action_btn_style),
        button(text(i18n.__("保存")).size(13))
            .on_press((message)(Event::WebBookmarkEditSave))
            .padding([8, 14])
            .style(primary_action_btn_style),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(560).height(610);
    Some((container(card).padding(12).into(), close))
}

pub fn add_web_bookmark_modal<'a, M: Clone + 'a>(input: Input<'a, M>) -> Option<(Element<'a, M>, M)> {
    let i18n = input.i18n;
    let message = input.message;
    let close = (message)(Event::ToggleWebLinksMenu);
    let header = settings_modal_header(
        text(i18n.__("添加网址")).size(20),
        settings_close_button(close.clone()),
    );
    let fields = settings_group_card(
        column![
            bookmark_field(
                i18n.__("标题"),
                input.web_bookmark_title_input,
                Event::WebBookmarkTitleChanged,
                message,
            ),
            bookmark_field(
                i18n.__("网址"),
                input.web_bookmark_url_input,
                Event::WebBookmarkUrlChanged,
                message,
            ),
            row![
                bookmark_field(
                    i18n.__("宽度（可选）"),
                    input.web_bookmark_width_input,
                    Event::WebBookmarkWidthChanged,
                message,
                ),
                bookmark_field(
                    i18n.__("高度（可选）"),
                    input.web_bookmark_height_input,
                    Event::WebBookmarkHeightChanged,
                message,
                ),
            ]
            .spacing(10),
        ]
        .spacing(12)
        .padding([12, 0]),
    );
    let body = column![
        input.heading,
        fields,
    ]
    .spacing(12);
    let can_add = can_add_web_bookmark(input.web_bookmark_title_input, input.web_bookmark_url_input);
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .on_press(close.clone())
            .padding([8, 14])
            .style(rounded_action_btn_style),
        button(text(i18n.__("添加")).size(13))
            .on_press_maybe(can_add.then_some((message)(Event::WebBookmarkAddSave)))
            .padding([8, 14])
            .style(primary_action_btn_style),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(560).height(445);
    Some((container(card).padding(12).into(), close))
}

#[derive(Clone, Debug)]
pub enum Event { WebBookmarkEditCancel, WebBookmarkEditTitleChanged(String), WebBookmarkEditUrlChanged(String), WebBookmarkEditWidthChanged(String), WebBookmarkEditHeightChanged(String), WebBookmarkEditCookieConfigsInsertExample, WebBookmarkEditCookieConfigsChanged(iced::widget::text_editor::Action), WebBookmarkRemove(usize), WebBookmarkEditSave, ToggleWebLinksMenu, WebBookmarkTitleChanged(String), WebBookmarkUrlChanged(String), WebBookmarkWidthChanged(String), WebBookmarkHeightChanged(String), WebBookmarkAddSave }
pub struct Input<'a, M> {
    pub i18n: I18n,
    pub message: fn(Event) -> M,
    pub editing_web_bookmark: Option<usize>,
    pub heading: Element<'a, M>,
    pub cookie_help: Element<'a, M>,
    pub edit_web_bookmark_cookie_configs_editor: &'a text_editor::Content,
    pub web_bookmark_title_input: &'a str,
    pub web_bookmark_url_input: &'a str,
    pub web_bookmark_width_input: &'a str,
    pub web_bookmark_height_input: &'a str,
    pub edit_web_bookmark_title_input: &'a str,
    pub edit_web_bookmark_url_input: &'a str,
    pub edit_web_bookmark_width_input: &'a str,
    pub edit_web_bookmark_height_input: &'a str,
}
fn tooltip_bubble<'a, M: 'a>(tip: &'a str) -> iced::widget::Container<'a, M> {
    container(text(tip).size(12)).padding([6, 8]).style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Color::from_rgba8(24, 24, 24, 0.96).into()),
        text_color: Some(iced::Color::WHITE),
        border: iced::Border { width: 0.0, color: iced::Color::TRANSPARENT, radius: 8.0.into() },
        shadow: iced::Shadow { color: iced::Color::BLACK.scale_alpha(0.40), offset: iced::Vector::new(0.0, 6.0), blur_radius: 18.0 }, snap: false,
    })
}

#[cfg(test)]
#[path = "bookmark_modals_tests.rs"]
mod tests;
