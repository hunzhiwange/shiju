//! 搜索领域与桌面数据、翻译及全局消息的适配。
use crate::app::{App, Message};
use iced::Element;
pub(crate) use desktop_search::components::search_panel::input_id;
use desktop_search::components::search_panel::{self as panel, ViewData};

fn data(app: &App) -> ViewData<'_> {
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    ViewData {
        search_text: &app.search.search_text,
        files: app.cached_search_panel_file_results(),
        recent_projects: &app.workspace_state.recent_projects,
        sessions: panel::matching_sessions(&app.session_state.sessions, &app.session_state.session_previews,
            |session| crate::app::i18n::other::session_title(app, &session.id, &session.title),
            &app.search.search_text),
        labels: [i18n.__("搜索文件 / 项目 / 历史会话"), i18n.__("关闭"),
            i18n.__("文件"), i18n.__("项目"), i18n.__("历史会话")],
    }
}

pub fn overlay(app: &App) -> Element<'_, Message> {
    panel::overlay(data(app)).map(Message::Search)
}

#[cfg(test)]
pub(super) fn view(app: &App) -> Element<'_, Message> {
    panel::view(data(app)).map(Message::Search)
}
