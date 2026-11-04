//! 只读识矩知识库入口；目录与文档分别保留服务端搜索和分页状态。
use crate::app::{App, Message, i18n::I18n, skill_hub::LoadState};
use iced::widget::{column, text};
use iced::{Element, Length};
mod catalog;

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.expert_hub.knowledge;
    if matches!(state.bases.status, LoadState::NeedsLogin) {
        return iced::widget::container(text(i18n.__("请先登录识矩账号")))
            .center(Length::Fill)
            .into();
    }
    column![text(i18n.__("资料和知识库")).size(22), catalog::bases(app), catalog::documents(app),]
        .spacing(12)
        .height(Length::Fill)
        .into()
}
