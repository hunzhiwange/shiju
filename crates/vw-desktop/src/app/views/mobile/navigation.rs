//! 项目分组和轻量会话列表，复用已有项目及会话操作。
use crate::app::{
    App, Message,
    assets::{Icon, get_icon},
    message::ViewMessage,
};
use iced::widget::{button, column, container, row, scrollable, svg, text, tooltip};
use iced::{Element, Length, Theme};

fn icon(icon: Icon) -> iced::widget::Svg<'static> {
    svg(get_icon(icon))
        .width(18)
        .height(18)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}

fn list_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let alpha = match status {
        button::Status::Pressed => 0.16,
        button::Status::Hovered => 0.10,
        _ if selected => 0.12,
        _ => 0.0,
    };
    button::Style {
        background: Some(theme.palette().text.scale_alpha(alpha).into()),
        text_color: theme.palette().text,
        border: iced::Border { radius: 12.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn session_row(label: &str, message: ViewMessage, selected: bool) -> Element<'static, Message> {
    button(
        text(label.to_string())
            .size(15)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .width(Length::Fill),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .on_press(Message::View(message))
    .style(move |theme, status| list_style(theme, status, selected))
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let mut items = column![].spacing(6).width(Length::Fill);
    // 主操作仅使用已有项目，避免移动端新增第二条项目创建流程。
    if let Some(path) = app
        .navigation_tabs
        .project_path
        .as_ref()
        .or_else(|| app.workspace_state.recent_projects.first())
    {
        let new = button(
            container(
                row![icon(Icon::Plus), text("新建会话").size(15)]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
            )
            .center_x(Length::Fill),
        )
        .padding([12, 14])
        .width(Length::Fill)
        .on_press(Message::View(ViewMessage::MobileNewSession(path.clone())))
        .style(|theme, status| {
            let mut style = list_style(theme, status, false);
            style.border.width = 1.0;
            style.border.color = theme.extended_palette().background.strong.color;
            style
        });
        items = items.push(new).push(
            container(
                text("项目与会话").size(12).style(
                    crate::app::components::system_settings_common::settings_muted_text_style,
                ),
            )
            .padding(iced::Padding { top: 18.0, right: 14.0, bottom: 6.0, left: 14.0 }),
        );
    }
    for (index, path) in app.workspace_state.recent_projects.iter().enumerate() {
        let custom =
            app.workspace_state.recent_projects_edits.get(index).filter(|s| !s.trim().is_empty());
        let label = custom.map(String::as_str).unwrap_or_else(|| {
            path.trim_end_matches(['/', '\\'])
                .rsplit(['/', '\\'])
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or(path)
        });
        let project_selected = app.navigation_tabs.project_path.as_deref() == Some(path.as_str());
        let project = button(
            row![
                icon(if project_selected { Icon::FolderOpen } else { Icon::FolderClosed }),
                text(label).size(15).width(Length::Fill),
                icon(Icon::ChevronRight)
            ]
            .spacing(9)
            .align_y(iced::Alignment::Center),
        )
        .padding([12, 14])
        .width(Length::Fill)
        .style(move |theme, status| list_style(theme, status, project_selected))
        .on_press(Message::View(ViewMessage::MobileProjectSelected(path.clone())));
        let add = button(container(icon(Icon::Plus)).center_x(32).center_y(32))
            .padding(0)
            .style(|theme, status| list_style(theme, status, false))
            .on_press(Message::View(ViewMessage::MobileNewSession(path.clone())));
        let add = tooltip(
            add,
            container(text("新建会话").size(12))
                .padding([6, 10])
                .style(desktop_ui_controls::tooltip::tooltip_dark_style),
            tooltip::Position::Bottom,
        )
        .gap(6);
        items = items.push(row![project, add].spacing(4).align_y(iced::Alignment::Center));
        if project_selected {
            let visible = app.navigation_sidebar.work_sidebar_space_task_visible.get(path)
                .copied().unwrap_or(desktop_workspace::views::project::layout::work_sidebar::presentation::WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE);
            if let Some(sessions) = app.workspace_state.project_sessions.get(path) {
                for session in sessions.iter().take(visible) {
                    let selected = app.navigation_tabs.project_path.as_deref()
                        == Some(path.as_str())
                        && app.session_state.active_session_id.as_deref()
                            == Some(session.id.as_str());
                    items = items.push(session_row(
                        if session.title.trim().is_empty() { &session.id } else { &session.title },
                        ViewMessage::MobileSessionSelected(
                            session.directory.clone(),
                            session.id.clone(),
                        ),
                        selected,
                    ));
                }
            }
            if let Some(sessions) = app.workspace_state.project_sessions.get(path) {
                if sessions.len() > visible {
                    items = items.push(session_row(
                        &format!("加载更多 · 还有 {} 条", sessions.len() - visible),
                        ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(path.clone()),
                        false,
                    ));
                }
            }
            if app.workspace_state.project_sessions_loading.contains(path) {
                items = items.push(
                    container(text("正在加载会话…").size(13).style(
                        crate::app::components::system_settings_common::settings_muted_text_style,
                    ))
                    .padding([8, 14]),
                );
            }
        }
    }
    if app.workspace_state.recent_projects.is_empty() {
        items =
            items.push(
                container(text("暂无项目，请先在设置中配置网关或项目。").size(14).style(
                    crate::app::components::system_settings_common::settings_muted_text_style,
                ))
                .padding(14),
            );
    }
    scrollable(container(items).padding([8, 7]).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .into()
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;
