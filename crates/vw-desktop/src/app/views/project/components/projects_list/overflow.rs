//! 代码模式的全部项目浮层。

use iced::widget::{Space, column, container, mouse_area, row, scrollable, svg, text};
use iced::{Element, Length, Radians, Theme};

use crate::app::assets::{Icon, get_icon};
use crate::app::components::overlays::SideOverlay;
use crate::app::i18n::I18n;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};

use super::super::super::styles::content_panel_style;
use super::{
    PROJECT_SCROLLBAR_GUTTER, TooltipPosition, navigation_badge, project_badge_button,
    project_scroll_direction, recent_project_badge, title_tooltip,
};

const PROJECTS_PER_ROW: usize = 4;

pub(super) fn view(app: &App, open: bool) -> Element<'static, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let open_menu = Message::Project(ProjectMessage::CodeProjectsMenuChanged(true));
    let icon = svg(get_icon(Icon::DotsThreeVertical))
        .width(20)
        .height(20)
        .rotation(Radians(std::f32::consts::FRAC_PI_2))
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) });
    let trigger = navigation_badge(container(icon).center(Length::Fill), open_menu.clone());
    if !open {
        return mouse_area(trigger).on_enter(open_menu).into();
    }

    let mut grid = column![].spacing(6);
    for (row_index, names) in app.recent_projects_edits.chunks(PROJECTS_PER_ROW).enumerate() {
        let mut projects = row![].spacing(6);
        for (column_index, name) in names.iter().enumerate() {
            let index = row_index * PROJECTS_PER_ROW + column_index;
            let project = recent_project_badge(app, index, name);
            let title = project.title.clone();
            projects = projects.push(title_tooltip(
                project_badge_button(project, true),
                title,
                TooltipPosition::Bottom,
            ));
        }
        for _ in names.len()..PROJECTS_PER_ROW {
            projects = projects.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(projects);
    }

    let scroll =
        scrollable(container(grid).width(Length::Fill).padding([0, PROJECT_SCROLLBAR_GUTTER]))
            .direction(project_scroll_direction());
    let panel = container(
        column![container(text(i18n.__("全部项目")).size(14)).center_x(Length::Fill), scroll]
            .spacing(8),
    )
    .width(440)
    .max_height(390)
    .padding([12, 6])
    .style(content_panel_style);

    let close_menu = Message::Project(ProjectMessage::CodeProjectsMenuChanged(false));
    SideOverlay::new(trigger, panel)
        .show(true)
        .gap(10.0)
        .on_close(close_menu.clone())
        .on_mouse_exit(close_menu)
        .into()
}
