//! 工作模式专属左栏，使用现有项目、会话与业务消息构建工作台导航。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, Stack, button, column, container, mouse_area, row, scrollable, svg, text,
};
use iced::{Background, Border, Element, Length, Point, Shadow, Theme, Vector};

use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::tooltip::tooltip_dark_style;
use desktop_ui_controls::overlays::{BelowOverlay, PointBelowOverlay, SideOverlay};
use desktop_ui_controls::right_click_area::RightClickArea;
use super::super::components::session_row::{SessionRowDetails, SessionRowStatus as WorkTaskStatus};
use super::super::components::sessions_panel::{self, build_session_menu};
use super::styles::work_surface_style;
pub mod presentation;
pub mod events;
use events::*;
use presentation::*;
mod rail;
pub const WORK_SIDEBAR_SCROLLBAR_WIDTH: u32 = 4;
pub const WORK_SIDEBAR_SCROLLBAR_GUTTER: f32 = 8.0;
const WORK_NAVIGATION_ICON_COLUMN_WIDTH: f32 = 22.0;
const WORK_HEADER_ICON_SIZE: f32 = 16.0;
const WORK_HEADER_CONTROL_SIZE: f32 = 32.0;
const WORK_FILTER_CONTROL_SIZE: f32 = 30.0;
const MILLIS_PER_DAY: u64 = 24 * 60 * 60 * 1_000;

mod model;
pub use model::{WorkTaskItem, WorkSpaceItem, WorkSidebarModel, ModelInput, model, compare_sessions, project_label, navigation_specs, more_menu_item_specs, header_icon_specs};

#[cfg(not(target_arch = "wasm32"))]
fn expert_hub_message(section: ExpertHubSection) -> Message {
    Message::Batch(vec![
        Message::SkillHub(SkillHubMessage::Open),
        Message::SkillHub(SkillHubMessage::SelectSection(section)),
    ])
}

#[cfg(not(target_arch = "wasm32"))]
pub fn expert_hub_button(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[5];
    navigation_button(i18n, label, icon, Message::SkillHub(SkillHubMessage::Open), trailing)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn shiju_apps_button(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[3];
    navigation_button(i18n, label, icon, Message::ShijuApps(ShijuAppsMessage::Open), trailing)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn shiju_skills_button(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[4];
    navigation_button(
        i18n,
        label,
        icon,
        Message::SkillHub(SkillHubMessage::OpenShijuSkills),
        trailing,
    )
}

mod controls;
use controls::*;
pub use controls::sidebar_style;

mod filters;
use filters::*;
pub use filters::{refresh_all_message, refresh_all_control, task_pool_control};

mod navigation;
use navigation::*;

pub struct TaskPresentation {
    pub row: super::super::components::session_row::Input,
    pub menu: Option<(String, bool, Point)>,
}
fn task_button(task: &WorkTaskItem, render: &dyn Fn(&WorkTaskItem) -> TaskPresentation) -> Element<'static, Message> {
    let presentation = render(task);
    let open = task.message.clone();
    let content = super::super::components::session_row::session_row(
        &presentation.row, &task.id, &task.details, task.status, sessions_panel::Message::Open,
    ).map(move |event| match event {
        sessions_panel::Message::Open => open.clone(),
        event => Message::Session(event),
    });
    if task.details.pending { return content; }
    let id = task.id.clone();
    let right_click: Element<'static, Message> = RightClickArea::new(content, Box::new(move |position| {
        Message::Project(ProjectMessage::SessionRightClicked(id.clone(), position.x, position.y))
    })).into();
    if let Some((path, disabled, anchor)) = presentation.menu {
        PointBelowOverlay::new(right_click, build_session_menu(desktop_i18n::I18n::new(presentation.row.desktop_locale), task.id.clone(), path, disabled).map(Message::Session))
            .show(true).anchor(anchor).on_close(Message::Project(ProjectMessage::SessionMenuClose)).into()
    } else { right_click }
}

mod lists;
use lists::*;
pub use lists::{project_navigation_message, project_create_message};

fn sidebar_tab_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style + Copy {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if selected {
            Some(palette.background.strong.color.scale_alpha(0.72))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(palette.background.strong.color.scale_alpha(0.42))
        } else {
            None
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: if selected {
                theme.palette().text
            } else {
                theme.palette().text.scale_alpha(0.62)
            },
            border: Border { radius: 8.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

fn sidebar_tab(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    count: usize,
    tab: WorkSidebarTab,
    selected: bool,
) -> Element<'static, Message> {
    button(
        row![text(i18n.__(label)).size(13), text(format!("({count})")).size(11)]
            .spacing(5)
            .align_y(iced::Alignment::Center),
    )
    .on_press(Message::View(ViewMessage::WorkSidebarTabSelected(tab)))
    .padding([7, 10])
    .width(Length::Fill)
    .style(sidebar_tab_style(selected))
    .into()
}

fn tabs(
    i18n: desktop_i18n::I18n,
    model: &WorkSidebarModel,
    selected: WorkSidebarTab,
) -> Element<'static, Message> {
    row![
        sidebar_tab(
            i18n,
            i18n.__("任务"),
            model.task_total,
            WorkSidebarTab::Tasks,
            selected == WorkSidebarTab::Tasks,
        ),
        sidebar_tab(
            i18n,
            i18n.__("空间"),
            model.space_total,
            WorkSidebarTab::Spaces,
            selected == WorkSidebarTab::Spaces,
        ),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

pub fn view(app: &ViewInput<'_>, model: &WorkSidebarModel, current_project: Option<Element<'static, Message>>, render: &dyn Fn(&WorkTaskItem) -> TaskPresentation) -> Element<'static, Message> {

    let active_list = match app.work_sidebar_tab {
        WorkSidebarTab::Tasks => tasks_section(app, model, render),
        WorkSidebarTab::Spaces => spaces_section(app, model, render),
    };
    let scroll = scrollable(
        container(active_list)
            .width(Length::Fill)
            .padding(iced::Padding::default().right(WORK_SIDEBAR_SCROLLBAR_GUTTER)),
    )
    .direction(Direction::Vertical(
        Scrollbar::new()
            .width(WORK_SIDEBAR_SCROLLBAR_WIDTH)
            .scroller_width(WORK_SIDEBAR_SCROLLBAR_WIDTH),
    ))
    .width(Length::Fill)
    .height(Length::Fill);

    let mut navigation = column![].spacing(9);
    if !app.work_sidebar_collapsed {
        navigation = navigation.push(expanded_header(app));
    }
    if let Some(card) = current_project { navigation = navigation.push(card); }
    let navigation = navigation.push(if app.work_sidebar_collapsed {
        new_task_button(app)
    } else {
        navigation_section(app)
    });
    let sidebar = container(
        column![
            navigation,
            tabs(desktop_i18n::I18n::new(app.desktop_locale), model, app.work_sidebar_tab),
            scroll,
        ]
        .spacing(9)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(app.work_sidebar_width))
    .height(Length::Fill)
    .padding([10, 10])
    .style(sidebar_style);

    let rail = if app.work_sidebar_collapsed { rail::view(app) } else { Space::new().into() };
    row![rail, sidebar].into()
}

pub struct ViewInput<'a> {
 pub desktop_locale: config_ui::DesktopLocale,
 pub recent_projects: &'a [String],
 pub project_sessions_loading: &'a std::collections::HashSet<String>,
 pub work_sidebar_collapsed: bool,
 pub work_sidebar_more_open: bool,
 pub work_sidebar_filter_open: bool,
 pub work_sidebar_project_nav_hovered: bool,
 pub work_sidebar_filter_scope: WorkSidebarFilterScope,
 pub work_sidebar_filter_status: WorkSidebarFilterStatus,
 pub work_sidebar_filter_time: WorkSidebarFilterTime,
 pub work_sidebar_tab: WorkSidebarTab,
 pub work_sidebar_width: f32,
 pub new_session_creating: bool,
}

#[cfg(test)]
#[path = "work_sidebar/tests.rs"]
mod tests;
