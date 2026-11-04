//! 只读识矩知识库入口；目录与文档分别保留服务端搜索和分页状态。
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
    let base = column![
        text(i18n.__("资料和知识库")).size(22),
        row![
            container(catalog::bases(app)).width(Length::FillPortion(1)),
            container(catalog::documents(app)).width(Length::FillPortion(1)),
        ]
        .spacing(12)
        .height(Length::FillPortion(1)),
        container(search::view(app)).height(Length::FillPortion(1)),
    ]
    .spacing(12)
    .height(Length::Fill)
    .into();
    reader::with_reader(app, base)
}
