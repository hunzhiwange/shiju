//! 使用公共市场的 Suite 状态展示专家包。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Font, Length};

use super::{public_skill_detail, public_skills, styles};
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_text_input_style,
};
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use crate::app::skill_hub::LoadState;
use crate::app::{App, Message};

fn public_message(message: PublicSkillMessage) -> Message {
    Message::SkillHub(SkillHubMessage::Public(message))
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.expert_hub.public_market;
    if state.detail.is_some() {
        return public_skill_detail::view(app);
    }

    let content = match &state.sources {
        LoadState::Idle | LoadState::Loading { .. } => public_skills::loading_view(),
        LoadState::Error(error) => public_skills::error_view(error),
        LoadState::NeedsLogin => public_skills::error_view("公开市场不需要登录，请重试"),
        LoadState::Loaded(_) => public_skills::suites_content(state),
    };
    let heading = column![
        text("专家").size(26).font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
        text("浏览 SkillHub 专家包").size(13).style(styles::muted_text_style),
    ]
    .spacing(5)
    .width(Length::Fill);
    let search = text_input("搜索专家包", &state.query.keyword_draft)
        .on_input(|query| public_message(PublicSkillMessage::QueryDraftChanged(query)))
        .on_submit(public_message(PublicSkillMessage::SubmitQuery))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fixed(340.0));
    let header = column![
        row![heading, public_skills::source_selector(state)].spacing(12).align_y(Alignment::Center),
        row![
            search,
            button(text("搜索").size(13))
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
                .width(public_skills::SCROLLBAR_WIDTH)
                .scroller_width(public_skills::SCROLLBAR_WIDTH)
                .spacing(8),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
