//! 在当前知识库内检索正文；结果始终使用服务端身份打开原文。
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_muted_text_style, settings_text_input_style,
};
use crate::app::message::skill_hub::knowledge::{KnowledgeMessage, message};
use crate::app::{App, Message, i18n::I18n, skill_hub::LoadState};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.expert_hub.knowledge;
    let i18n = I18n::new(app.desktop_locale);
    if state.selected_base.is_none() {
        return container(text(i18n.__("请选择知识库"))).center(Length::Fill).into();
    }
    let busy = matches!(state.search, LoadState::Loading { .. });
    let submit = (!busy && !state.search_query.trim().is_empty())
        .then(|| message(KnowledgeMessage::Search(state.search_query.clone())));
    let input = text_input(i18n.__("检索正文"), &state.search_query)
        .on_input(|query| message(KnowledgeMessage::SearchQueryChanged(query)))
        .on_submit_maybe(submit.clone())
        .style(settings_text_input_style)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE);
    let mut results = column![].spacing(8);
    match &state.search {
        LoadState::Idle => {}
        LoadState::NeedsLogin => {
            results = results.push(text(i18n.__("请先登录识矩账号")));
        }
        LoadState::Loading { .. } => {
            results = results.push(text(i18n.__("加载中…")));
        }
        LoadState::Error(_) => {
            results = results.push(
                row![
                    text(i18n.__("检索失败，请重试")),
                    button(text(i18n.__("重试")))
                        .padding([6, 10])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(submit.clone()),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            );
        }
        LoadState::Loaded(result) if result.records.is_empty() => {
            results = results.push(text(i18n.__("暂无检索结果")));
        }
        LoadState::Loaded(result) => {
            for record in &result.records {
                results = results.push(
                    container(
                        column![
                            text(&record.title).size(15),
                            text(&record.content).size(13),
                            row![
                                text(format!("{}：{:.3}", i18n.__("分数"), record.score))
                                    .size(12)
                                    .style(settings_muted_text_style),
                                button(text(i18n.__("打开原文")))
                                    .padding([6, 10])
                                    .style(rounded_action_btn_style)
                                    .on_press(message(KnowledgeMessage::OpenReader {
                                        kb: record.kb_zid.clone(),
                                        doc: record.doc_zid.clone()
                                    })),
                            ]
                            .spacing(12)
                            .align_y(iced::Alignment::Center),
                        ]
                        .spacing(6),
                    )
                    .padding(10)
                    .width(Length::Fill)
                    .style(super::super::styles::card_style),
                );
            }
        }
    }
    column![
        row![
            input,
            button(text(i18n.__("检索")))
                .padding([7, 12])
                .style(primary_action_btn_style)
                .on_press_maybe(submit)
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        scrollable(container(results).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}
