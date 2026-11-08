//! 只读识矩知识库入口；目录与文档分别保留服务端搜索和分页状态。
use crate::app::components::system_settings_common::settings_select;
use crate::app::message::skill_hub::knowledge::{KnowledgeMessage, message};
use crate::app::skill_hub::knowledge::KnowledgeSearchMode;
use crate::app::{App, Message, i18n::I18n, skill_hub::LoadState};
use iced::widget::{column, container, row, text};
use iced::{Element, Length};
mod catalog;
mod reader;
mod search;

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.expert_hub.knowledge;
    if matches!(state.bases.status, LoadState::NeedsLogin) {
        return iced::widget::container(text(i18n.__("请先登录识矩账号")))
            .center(Length::Fill)
            .into();
    }
    let catalog_label = i18n.__("检索知识库");
    let content_label = i18n.__("召回正文");
    let mode = settings_select(
        [catalog_label, content_label],
        Some(match state.search_mode {
            KnowledgeSearchMode::Catalog => catalog_label,
            KnowledgeSearchMode::Content => content_label,
        }),
        move |selected| {
            message(KnowledgeMessage::SelectSearchMode(if selected == catalog_label {
                KnowledgeSearchMode::Catalog
            } else {
                KnowledgeSearchMode::Content
            }))
        },
    )
    .width(160);
    let results = match state.search_mode {
        KnowledgeSearchMode::Catalog => catalog::documents(app),
        KnowledgeSearchMode::Content => search::view(app),
    };
    let base = column![
        row![container(text(i18n.__("识矩知识库")).size(22)).width(Length::Fill), mode,]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        row![
            container(catalog::bases(app)).width(Length::FillPortion(1)),
            container(results).width(Length::FillPortion(3)),
        ]
        .spacing(12)
        .height(Length::Fill),
    ]
    .spacing(12)
    .height(Length::Fill)
    .into();
    reader::with_reader(app, base)
}
