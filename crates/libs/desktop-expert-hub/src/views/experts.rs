//! 使用公共市场的 Suite 状态展示专家包。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Font, Length};

use super::styles;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_text_input_style,
};
use crate::message::public::PublicSkillMessage;



pub fn view<'a, M: Clone + 'a>(
    locale: config_ui::DesktopLocale,
    keyword: &'a str,
    content: Element<'a, M>,
    source_selector: Element<'a, M>,
    public_message: impl Fn(PublicSkillMessage) -> M + Copy + 'a,
) -> Element<'a, M> {
    let heading = column![
        text(desktop_i18n::I18n::new(locale).__("专家"))
            .size(26)
            .font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
        text(desktop_i18n::I18n::new(locale).__("浏览 SkillHub 专家包"))
            .size(13)
            .style(styles::muted_text_style),
    ]
    .spacing(5)
    .width(Length::Fill);
    let search = text_input(
        desktop_i18n::I18n::new(locale).__("搜索专家包"),
        keyword,
    )
    .on_input(move |query| public_message(PublicSkillMessage::QueryDraftChanged(query)))
    .on_submit(public_message(PublicSkillMessage::SubmitQuery))
    .padding(SETTINGS_CONTROL_PADDING)
    .size(SETTINGS_CONTROL_TEXT_SIZE)
    .style(settings_text_input_style)
    .width(Length::Fixed(340.0));
    let header = column![
        row![
            heading,
            source_selector
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        row![
            search,
            button(text(desktop_i18n::I18n::new(locale).__("搜索")).size(13))
                .on_press(public_message(PublicSkillMessage::SubmitQuery))
                .padding([10, 18])
                .style(styles::action_button_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(14);
    scrollable(container(column![header, content].spacing(18)).padding([0, 7]))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(4)
                .scroller_width(4)
                .spacing(8),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
