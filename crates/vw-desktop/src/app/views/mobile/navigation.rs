//! 从既有项目和会话数据构建单列导航，不复制业务状态。
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Length};
use iced::widget::{column, container, scrollable, text};
pub fn view(app: &App) -> Element<'_, Message> {
    let mut items = column![super::action("关闭项目列表", ViewMessage::MobileDrawerToggled)].spacing(8);
    for (index, path) in app.recent_projects.iter().enumerate() {
        let label = app.recent_projects_edits.get(index).filter(|s| !s.trim().is_empty()).unwrap_or(path);
        items = items.push(super::action(label, ViewMessage::MobileProjectSelected(path.clone())))
            .push(super::action("新建会话", ViewMessage::MobileNewSession(path.clone())));
        if let Some(sessions) = app.project_sessions.get(path) {
            for session in sessions {
                items = items.push(super::action(if session.title.trim().is_empty() { &session.id } else { &session.title },
                    ViewMessage::MobileSessionSelected(session.directory.clone(), session.id.clone())));
            }
        }
        if app.project_sessions_loading.contains(path) {
            items = items.push(text("正在加载会话…"));
        }
    }
    if app.recent_projects.is_empty() { items = items.push(text("暂无项目，请先在设置中配置网关或项目。")); }
    scrollable(container(items).padding([0, 7]).width(Length::Fill))
        .width(Length::Fill).height(Length::Fill)
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4))).into()
}
