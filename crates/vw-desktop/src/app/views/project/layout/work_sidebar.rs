//! 工作模式专属左栏，使用现有项目、会话与业务消息构建工作台导航。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, Stack, button, column, container, mouse_area, row, scrollable, svg, text,
};
use iced::{Background, Border, Color, Element, Length, Point, Shadow, Theme, Vector};

use crate::app::assets::{self, Icon};
use crate::app::components::input_panel::styles::tooltip_dark_style;
use crate::app::components::overlays::{BelowOverlay, PointBelowOverlay, SideOverlay};
use crate::app::components::status_animation::spinner_frame;
use crate::app::components::system_settings::SystemTab;
use crate::app::components::widgets::RightClickArea;
use crate::app::message::{ProjectMessage, SearchMessage, TaskBoardMessage, ViewMessage};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::message::{ShijuAppsMessage, SkillHubMessage};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::skill_hub::ExpertHubSection;
use crate::app::state::{
    WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE, WorkSidebarFilterScope, WorkSidebarFilterStatus,
    WorkSidebarFilterTime, WorkSidebarTab,
};
use crate::app::{App, Message};

use super::super::components::{build_session_menu, session_status_icon};
use super::super::utils::truncate_display_width;

pub(super) const WORK_SIDEBAR_SCROLLBAR_WIDTH: u32 = 4;
pub(super) const WORK_SIDEBAR_SCROLLBAR_GUTTER: f32 = 8.0;
const WORK_TASK_ROW_FIXED_WIDTH: f32 = 46.0;
const WORK_TASK_TITLE_END_GUTTER_WIDTH: f32 = 18.0;
const WORK_TASK_TIME_RESERVED_WIDTH: f32 = 62.0;
const WORK_TASK_DISPLAY_UNIT_WIDTH: f32 = 6.5;
const WORK_NAVIGATION_ICON_COLUMN_WIDTH: f32 = 22.0;
const WORK_HEADER_ICON_SIZE: f32 = 16.0;
const WORK_HEADER_CONTROL_SIZE: f32 = 32.0;
const WORK_FILTER_CONTROL_SIZE: f32 = 30.0;
const MILLIS_PER_DAY: u64 = 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkTaskStatus {
    Idle,
    Running,
    Queued(usize),
    Updated,
}

pub(super) struct WorkTaskItem {
    pub(super) id: String,
    title: String,
    project_label: String,
    pub(super) updated: u64,
    pub(super) status: WorkTaskStatus,
    pub(super) message: Message,
}

pub(super) struct WorkSpaceSession {
    pub(super) id: String,
    title: String,
    updated: u64,
    pub(super) status: WorkTaskStatus,
    pub(super) message: Message,
}

pub(super) struct WorkSpaceItem {
    path: String,
    pub(super) label: String,
    pub(super) selected: bool,
    pub(super) expanded: bool,
    pub(super) hovered: bool,
    pub(super) tools_open: bool,
    pub(super) creating: bool,
    pub(super) sessions: Vec<WorkSpaceSession>,
    pub(super) session_total: usize,
    pub(super) message: Message,
    pub(super) toggle_message: Message,
    pub(super) create_message: Message,
    pub(super) tools_message: Message,
}

pub(super) struct WorkSidebarModel {
    pub(super) tasks: Vec<WorkTaskItem>,
    pub(super) task_total: usize,
    pub(super) spaces: Vec<WorkSpaceItem>,
    pub(super) space_total: usize,
}

pub(super) fn navigation_specs() -> [(&'static str, Icon, Option<&'static str>); 8] {
    [
        ("新建任务", Icon::FileEarmarkPlus, None),
        ("助手", Icon::Robot, None),
        ("项目", Icon::FolderOpen, None),
        ("识矩应用", Icon::Grid1x2, None),
        ("识矩技能", Icon::Star, None),
        ("专家 · 技能 · 连接器", Icon::GearWideConnected, None),
        ("自动化", Icon::ArrowRepeat, None),
        ("更多", Icon::Grid1x2, Some("知识库 · 应用")),
    ]
}

pub(super) fn more_menu_item_specs() -> [(&'static str, Icon); 4] {
    [
        ("添加网址", Icon::Link45Deg),
        ("知识库", Icon::Journals),
        ("工作流", Icon::GitBranch),
        ("应用中心", Icon::Grid1x2),
    ]
}

pub(super) fn header_icon_specs(collapsed: bool) -> [(Icon, &'static str); 5] {
    [
        if collapsed {
            (Icon::LayoutSidebarReverse, "展开侧边栏")
        } else {
            (Icon::LayoutSidebar, "收起侧边栏")
        },
        (Icon::ArrowRepeat, "刷新全部任务"),
        (Icon::Grid1x2, "任务池"),
        (Icon::Search, "搜索"),
        (Icon::Funnel, "过滤"),
    ]
}

fn project_label(app: &App, path: &str) -> String {
    app.recent_projects
        .iter()
        .position(|candidate| candidate == path)
        .and_then(|index| app.recent_projects_edits.get(index))
        .map(|label| label.trim())
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .or_else(|| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| path.to_string())
}

type SessionRef<'a> = (&'a str, &'a vw_shared::session::info::Info);

fn collect_top_level_sessions<'a>(
    sessions: impl Iterator<Item = (&'a str, &'a vw_shared::session::info::Info)>,
) -> Vec<SessionRef<'a>> {
    let sessions = sessions.collect::<Vec<_>>();
    sessions
        .iter()
        .copied()
        .filter(|(path, session)| {
            if session.time.archived.is_some() {
                return false;
            }
            let Some(parent_id) = session.parent_id.as_deref().filter(|id| !id.trim().is_empty())
            else {
                return true;
            };
            session.is_visible_task()
                && session.directory == *path
                && sessions.iter().any(|(parent_path, parent)| {
                    *parent_path == *path
                        && parent.id == parent_id
                        && parent.project_id == session.project_id
                        && parent.directory == session.directory
                })
        })
        .collect()
}

pub(super) fn compare_sessions(
    left: &SessionRef<'_>,
    right: &SessionRef<'_>,
    pinned: &std::collections::HashSet<(String, String)>,
) -> std::cmp::Ordering {
    let left_pinned = pinned.contains(&(left.1.project_id.clone(), left.1.id.clone()));
    let right_pinned = pinned.contains(&(right.1.project_id.clone(), right.1.id.clone()));
    right_pinned.cmp(&left_pinned).then_with(|| {
        right
            .1
            .time
            .updated
            .cmp(&left.1.time.updated)
            .then_with(|| left.1.id.cmp(&right.1.id))
            .then_with(|| left.0.cmp(right.0))
    })
}

fn sort_and_truncate_sessions(
    sessions: &mut Vec<SessionRef<'_>>,
    limit: usize,
    pinned: &std::collections::HashSet<(String, String)>,
) {
    if limit == 0 {
        sessions.clear();
        return;
    }

    if sessions.len() > limit {
        sessions.select_nth_unstable_by(limit, |left, right| compare_sessions(left, right, pinned));
        sessions.truncate(limit);
    }
    sessions.sort_unstable_by(|left, right| compare_sessions(left, right, pinned));
}

fn pinned_sessions(
    sessions: impl Iterator<Item = (String, String)>,
) -> std::collections::HashSet<(String, String)> {
    let Some(home) = directories::BaseDirs::new() else {
        return Default::default();
    };
    let data_dir = vw_config_types::paths::home_config_dir(home.home_dir());
    let tenant = vw_gateway_client::vw_api_types::id::TenantScope::default();
    let mut result = std::collections::HashSet::new();
    for project_id in
        sessions.map(|(project_id, _)| project_id).collect::<std::collections::HashSet<_>>()
    {
        let scope = vw_shared::session::ui_store::pinned_threads_scope(
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            &project_id,
        );
        if let Ok(ids) = vw_shared::session::ui_store::load_pinned_threads(&data_dir, &scope) {
            result.extend(ids.into_iter().map(|id| (project_id.clone(), id)));
        }
    }
    result
}

fn project_matches_filter(app: &App, path: &str) -> bool {
    match app.work_sidebar_filter_scope {
        WorkSidebarFilterScope::All => true,
        WorkSidebarFilterScope::CurrentProject => app.project_path.as_deref() == Some(path),
    }
}

fn session_matches_time_filter(app: &App, updated: u64) -> bool {
    let days = match app.work_sidebar_filter_time {
        WorkSidebarFilterTime::All => return true,
        WorkSidebarFilterTime::LastDay => 1,
        WorkSidebarFilterTime::LastSevenDays => 7,
        WorkSidebarFilterTime::LastThirtyDays => 30,
    };
    let cutoff = vw_shared::time::now_ms().saturating_sub(days * MILLIS_PER_DAY);
    updated >= cutoff
}

fn session_matches_status_filter(app: &App, session_id: &str) -> bool {
    let runtime = app.session_runtime_states.get(session_id);
    let is_requesting = runtime.is_some_and(|runtime| runtime.is_requesting);
    let has_pending = runtime.is_some_and(|runtime| !runtime.queue.is_empty());
    match app.work_sidebar_filter_status {
        WorkSidebarFilterStatus::All => true,
        WorkSidebarFilterStatus::InProgress => is_requesting,
        WorkSidebarFilterStatus::Completed => !is_requesting && !has_pending,
        WorkSidebarFilterStatus::Pending => !is_requesting && has_pending,
    }
}

fn task_status(app: &App, session_id: &str) -> WorkTaskStatus {
    let Some(runtime) = app.session_runtime_states.get(session_id) else {
        return WorkTaskStatus::Idle;
    };
    if runtime.is_requesting {
        WorkTaskStatus::Running
    } else if !runtime.queue.is_empty() {
        WorkTaskStatus::Queued(runtime.queue.len())
    } else if runtime.has_unseen_success {
        WorkTaskStatus::Updated
    } else {
        WorkTaskStatus::Idle
    }
}

pub(super) fn model(app: &App) -> WorkSidebarModel {
    let pinned = pinned_sessions(
        app.project_sessions
            .values()
            .flatten()
            .map(|session| (session.project_id.clone(), session.id.clone())),
    );
    let mut task_sessions =
        collect_top_level_sessions(app.recent_projects.iter().flat_map(|path| {
            app.project_sessions
                .get(path)
                .into_iter()
                .flatten()
                .map(move |session| (path.as_str(), session))
        }))
        .into_iter()
        .filter(|(path, session)| {
            project_matches_filter(app, path)
                && session_matches_status_filter(app, &session.id)
                && session_matches_time_filter(app, session.time.updated)
        })
        .collect::<Vec<_>>();
    let task_total = task_sessions.len();
    sort_and_truncate_sessions(&mut task_sessions, app.work_sidebar_visible_tasks, &pinned);
    let tasks = task_sessions
        .into_iter()
        .map(|(path, session)| WorkTaskItem {
            id: session.id.clone(),
            title: session.title.clone(),
            project_label: project_label(app, path),
            updated: session.time.updated,
            status: task_status(app, &session.id),
            message: Message::Project(ProjectMessage::OpenProjectSessionPressed(
                session.directory.clone(),
                session.id.clone(),
            )),
        })
        .collect::<Vec<_>>();

    let mut spaces = app
        .recent_projects
        .iter()
        .filter(|path| project_matches_filter(app, path))
        .filter_map(|path| {
            let mut filtered_sessions = collect_top_level_sessions(
                app.project_sessions
                    .get(path)
                    .into_iter()
                    .flatten()
                    .map(|session| (path.as_str(), session)),
            )
            .into_iter()
            .filter(|(_, session)| {
                session_matches_status_filter(app, &session.id)
                    && session_matches_time_filter(app, session.time.updated)
            })
            .collect::<Vec<_>>();
            if (app.work_sidebar_filter_status != WorkSidebarFilterStatus::All
                || app.work_sidebar_filter_time != WorkSidebarFilterTime::All)
                && filtered_sessions.is_empty()
            {
                return None;
            }

            let expanded = app.work_expanded_spaces.contains(path);
            let session_total = filtered_sessions.len();
            let sessions = if expanded {
                let visible = app
                    .work_sidebar_space_task_visible
                    .get(path)
                    .copied()
                    .unwrap_or(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE);
                sort_and_truncate_sessions(&mut filtered_sessions, visible, &pinned);
                filtered_sessions
                    .into_iter()
                    .map(|(_, session)| WorkSpaceSession {
                        id: session.id.clone(),
                        title: session.title.clone(),
                        updated: session.time.updated,
                        status: task_status(app, &session.id),
                        message: Message::Project(ProjectMessage::OpenProjectSessionPressed(
                            session.directory.clone(),
                            session.id.clone(),
                        )),
                    })
                    .collect()
            } else {
                Vec::new()
            };

            Some(WorkSpaceItem {
                path: path.clone(),
                label: project_label(app, path),
                selected: app.project_path.as_deref() == Some(path.as_str()),
                expanded,
                hovered: app.hovered_recent_project.as_deref() == Some(path.as_str()),
                tools_open: app.project_tools_menu_path.as_deref() == Some(path.as_str()),
                creating: app.new_session_creating,
                sessions,
                session_total,
                message: Message::Project(ProjectMessage::SwitchWorkSpacePressed(path.clone())),
                toggle_message: Message::View(ViewMessage::WorkSpaceToggled(path.clone())),
                create_message: Message::Project(ProjectMessage::ProjectCreateSession(
                    path.clone(),
                )),
                tools_message: Message::Project(ProjectMessage::ProjectToolsMenuToggled(
                    path.clone(),
                )),
            })
        })
        .collect::<Vec<_>>();
    let space_total = spaces.len();
    spaces.truncate(app.work_sidebar_visible_spaces);

    WorkSidebarModel { tasks, task_total, spaces, space_total }
}

#[cfg(not(target_arch = "wasm32"))]
fn expert_hub_message(section: ExpertHubSection) -> Message {
    Message::Batch(vec![
        Message::SkillHub(SkillHubMessage::Open),
        Message::SkillHub(SkillHubMessage::SelectSection(section)),
    ])
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn expert_hub_button() -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[5];
    navigation_button(label, icon, Message::SkillHub(SkillHubMessage::Open), trailing)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn shiju_apps_button() -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[3];
    navigation_button(label, icon, Message::ShijuApps(ShijuAppsMessage::Open), trailing)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn shiju_skills_button() -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[4];
    navigation_button(label, icon, Message::SkillHub(SkillHubMessage::OpenShijuSkills), trailing)
}

pub(super) fn sidebar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    let background = if palette.is_dark {
        palette.background.weak.color
    } else {
        crate::app::theme_catalog::panel_surface_color(theme, 0.995)
    };
    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(theme.palette().text),
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.72),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn navigation_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => Some(palette.background.strong.color.scale_alpha(0.68)),
        button::Status::Pressed => Some(palette.primary.weak.color.scale_alpha(0.42)),
        _ => None,
    };
    button::Style {
        background: background.map(Background::Color),
        text_color: palette.background.base.text,
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn header_icon(icon: Icon) -> svg::Svg<'static> {
    header_icon_enabled(icon, true)
}

fn header_icon_enabled(icon: Icon, enabled: bool) -> svg::Svg<'static> {
    svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(WORK_HEADER_ICON_SIZE))
        .height(Length::Fixed(WORK_HEADER_ICON_SIZE))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text.scale_alpha(if enabled { 0.82 } else { 0.32 })),
        })
}

fn header_control_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style + Copy {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if active {
            Some(palette.background.strong.color.scale_alpha(0.82))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(palette.background.strong.color.scale_alpha(0.62))
        } else {
            None
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: palette.background.base.text,
            border: Border { radius: 10.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

fn header_icon_button(
    icon: Icon,
    label: &'static str,
    message: Message,
    active: bool,
) -> Element<'static, Message> {
    header_icon_button_sized(icon, label, message, active, WORK_HEADER_CONTROL_SIZE)
}

fn header_icon_button_sized(
    icon: Icon,
    label: &'static str,
    message: Message,
    active: bool,
    control_size: f32,
) -> Element<'static, Message> {
    header_icon_button_sized_maybe(icon, label, Some(message), active, control_size)
}

fn header_icon_button_sized_maybe(
    icon: Icon,
    label: &'static str,
    message: Option<Message>,
    active: bool,
    control_size: f32,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let mut control = button(
        container(header_icon_enabled(icon, enabled))
            .width(Length::Fixed(control_size))
            .height(Length::Fixed(control_size))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(header_control_style(active));
    if let Some(message) = message {
        control = control.on_press(message);
    }

    Tooltip::new(
        control,
        container(text(label).size(12)).padding([6, 8]).style(tooltip_dark_style),
        TooltipPosition::Bottom,
    )
    .gap(6)
    .into()
}

fn filter_active(app: &App) -> bool {
    app.work_sidebar_filter_scope != WorkSidebarFilterScope::All
        || app.work_sidebar_filter_status != WorkSidebarFilterStatus::All
        || app.work_sidebar_filter_time != WorkSidebarFilterTime::All
}

fn filter_option_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style + Copy {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if selected {
            Some(palette.background.strong.color.scale_alpha(0.86))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(palette.background.strong.color.scale_alpha(0.54))
        } else {
            None
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: palette.background.base.text,
            border: Border { radius: 9.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

fn filter_option(
    label: &'static str,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    let mut content = row![text(label).size(14), Space::new().width(Length::Fill)]
        .align_y(iced::Alignment::Center);
    if selected {
        content = content
            .push(header_icon(Icon::Check).width(Length::Fixed(15.0)).height(Length::Fixed(15.0)));
    }
    button(content)
        .on_press(message)
        .padding([9, 11])
        .width(Length::Fill)
        .style(filter_option_style(selected))
        .into()
}

fn filter_divider() -> Element<'static, Message> {
    container(Space::new().height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color.scale_alpha(0.52),
            )),
            ..Default::default()
        })
        .into()
}

fn filter_popover_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(palette.background.weak.text),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.86),
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: iced::Color::BLACK.scale_alpha(0.28),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}

fn filter_popover(app: &App) -> Element<'static, Message> {
    let scope = app.work_sidebar_filter_scope;
    let status = app.work_sidebar_filter_status;
    let time = app.work_sidebar_filter_time;
    let content = column![
        text("筛选状态").size(12).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().text.scale_alpha(0.52)),
        }),
        filter_option(
            "全部状态",
            status == WorkSidebarFilterStatus::All,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::All,
            )),
        ),
        filter_option(
            "进行中",
            status == WorkSidebarFilterStatus::InProgress,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::InProgress,
            )),
        ),
        filter_option(
            "已完成",
            status == WorkSidebarFilterStatus::Completed,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::Completed,
            )),
        ),
        filter_option(
            "待处理",
            status == WorkSidebarFilterStatus::Pending,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::Pending,
            )),
        ),
        filter_divider(),
        text("筛选范围").size(12).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().text.scale_alpha(0.52)),
        }),
        filter_option(
            "全部项目",
            scope == WorkSidebarFilterScope::All,
            Message::View(
                ViewMessage::WorkSidebarFilterScopeSelected(WorkSidebarFilterScope::All,)
            ),
        ),
        filter_option(
            "当前项目",
            scope == WorkSidebarFilterScope::CurrentProject,
            Message::View(ViewMessage::WorkSidebarFilterScopeSelected(
                WorkSidebarFilterScope::CurrentProject,
            )),
        ),
        filter_divider(),
        text("筛选时间").size(12).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().text.scale_alpha(0.52)),
        }),
        filter_option(
            "全部时间",
            time == WorkSidebarFilterTime::All,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(WorkSidebarFilterTime::All,)),
        ),
        filter_option(
            "最近 24 小时",
            time == WorkSidebarFilterTime::LastDay,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastDay,
            )),
        ),
        filter_option(
            "最近 7 天",
            time == WorkSidebarFilterTime::LastSevenDays,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastSevenDays,
            )),
        ),
        filter_option(
            "最近 30 天",
            time == WorkSidebarFilterTime::LastThirtyDays,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastThirtyDays,
            )),
        ),
        filter_divider(),
        filter_option("重置筛选条件", false, Message::View(ViewMessage::WorkSidebarFilterReset),),
    ]
    .spacing(4);

    container(content)
        .padding([12, 14])
        .width(Length::Fixed(264.0))
        .style(filter_popover_style)
        .into()
}

fn filter_control(app: &App) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[4];
    let trigger = header_icon_button_sized(
        icon,
        label,
        Message::View(ViewMessage::WorkSidebarFilterToggled),
        app.work_sidebar_filter_open || filter_active(app),
        WORK_FILTER_CONTROL_SIZE,
    );
    BelowOverlay::new(trigger, filter_popover(app))
        .show(app.work_sidebar_filter_open)
        .gap(6.0)
        .on_close(Message::View(ViewMessage::WorkSidebarFilterClosed))
        .into()
}

fn search_control() -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(false)[3];
    header_icon_button(
        icon,
        label,
        Message::Batch(vec![
            Message::View(ViewMessage::WorkSidebarFilterClosed),
            Message::View(ViewMessage::WorkSidebarMoreClosed),
            Message::Search(SearchMessage::Open),
        ]),
        false,
    )
}

fn sidebar_toggle_control(app: &App) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[0];
    header_icon_button(icon, label, Message::View(ViewMessage::WorkSidebarToggled), false)
}

pub(super) fn refresh_all_message(app: &App) -> Message {
    let mut seen = std::collections::HashSet::new();
    Message::Batch(
        app.recent_projects
            .iter()
            .filter(|path| seen.insert((*path).clone()))
            .cloned()
            .map(|path| Message::Project(ProjectMessage::ProjectLoadSessions(path)))
            .collect(),
    )
}

pub(super) fn refresh_all_control(app: &App) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[1];
    let is_refreshing =
        app.recent_projects.iter().any(|path| app.project_sessions_loading.contains(path));
    header_icon_button(icon, label, refresh_all_message(app), is_refreshing)
}

pub(super) fn task_pool_control(app: &App) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[2];
    header_icon_button(icon, label, Message::TaskBoard(TaskBoardMessage::ToggleBoard), false)
}

fn expanded_header(app: &App) -> Element<'static, Message> {
    row![
        container(text("识矩").size(17))
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Center),
        sidebar_toggle_control(app),
        refresh_all_control(app),
        task_pool_control(app),
        search_control(),
        filter_control(app),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn collapsed_header(app: &App) -> Element<'static, Message> {
    column![
        container(text("V").size(20))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
        sidebar_toggle_control(app),
        refresh_all_control(app),
        task_pool_control(app),
        search_control(),
        filter_control(app),
    ]
    .spacing(6)
    .align_x(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn navigation_icon(icon: Icon) -> svg::Svg<'static> {
    navigation_icon_enabled(icon, true)
}

fn navigation_icon_enabled(icon: Icon, enabled: bool) -> svg::Svg<'static> {
    let size = match icon {
        Icon::Plus | Icon::Grid1x2 => 15.0,
        _ => 18.0,
    };
    svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text.scale_alpha(if enabled { 0.78 } else { 0.32 })),
        })
}

fn navigation_button(
    label: &'static str,
    icon: Icon,
    message: Message,
    trailing: Option<&'static str>,
) -> Element<'static, Message> {
    navigation_button_maybe(label, icon, Some(message), trailing)
}

fn navigation_button_maybe(
    label: &'static str,
    icon: Icon,
    message: Option<Message>,
    trailing: Option<&'static str>,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let mut content = row![
        container(navigation_icon_enabled(icon, enabled))
            .width(Length::Fixed(WORK_NAVIGATION_ICON_COLUMN_WIDTH)),
        text(label).size(14).style(move |theme: &Theme| text::Style {
            color: (!enabled).then(|| theme.palette().text.scale_alpha(0.32)),
        }),
        Space::new().width(Length::Fill),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    if let Some(trailing) = trailing {
        content = content.push(text(trailing).size(11).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().text.scale_alpha(0.48)),
        }));
    }

    let mut control =
        button(content).padding([9, 11]).width(Length::Fill).style(navigation_button_style);
    if let Some(message) = message {
        control = control.on_press(message);
    }
    control.into()
}

fn more_menu_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(palette.background.weak.text),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.72),
            radius: 18.0.into(),
        },
        shadow: Shadow {
            color: iced::Color::BLACK.scale_alpha(if palette.is_dark { 0.34 } else { 0.16 }),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 30.0,
        },
        ..Default::default()
    }
}

fn more_menu_divider() -> Element<'static, Message> {
    container(Space::new().height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color.scale_alpha(0.52),
            )),
            ..Default::default()
        })
        .into()
}

fn close_more_then(message: Message) -> Message {
    Message::Batch(vec![Message::View(ViewMessage::WorkSidebarMoreClosed), message])
}

fn more_menu_action(
    label: &'static str,
    icon: Icon,
    message: Message,
) -> Element<'static, Message> {
    button(
        row![
            container(navigation_icon(icon)).width(Length::Fixed(28.0)),
            text(label).size(15),
            Space::new().width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .on_press(close_more_then(message))
    .padding([11, 12])
    .width(Length::Fill)
    .style(navigation_button_style)
    .into()
}

fn more_menu() -> Element<'static, Message> {
    let specs = more_menu_item_specs();
    let add_url = Message::Batch(vec![
        Message::View(ViewMessage::OpenApps),
        Message::View(ViewMessage::ToggleWebLinksMenu),
    ]);
    let content = column![
        more_menu_action(specs[0].0, specs[0].1, add_url),
        more_menu_divider(),
        more_menu_action(specs[1].0, specs[1].1, Message::View(ViewMessage::OpenKnowledge),),
        more_menu_action(specs[2].0, specs[2].1, Message::View(ViewMessage::OpenWorkflowTool),),
        more_menu_divider(),
        more_menu_action(specs[3].0, specs[3].1, Message::View(ViewMessage::OpenApps),),
    ]
    .spacing(3);

    container(content).padding([12, 14]).width(Length::Fixed(252.0)).style(more_menu_style).into()
}

fn more_control(app: &App) -> Element<'static, Message> {
    let (_, icon, trailing) = navigation_specs()[7];
    let trigger = navigation_button(
        "更多",
        icon,
        Message::View(ViewMessage::WorkSidebarMoreOpened),
        trailing,
    );
    let hover_trigger =
        mouse_area(trigger).on_enter(Message::View(ViewMessage::WorkSidebarMoreOpened));
    SideOverlay::new(hover_trigger, more_menu())
        .show(app.work_sidebar_more_open)
        .gap(8.0)
        .on_close(Message::View(ViewMessage::WorkSidebarMoreClosed))
        .on_mouse_exit(Message::View(ViewMessage::WorkSidebarMoreClosed))
        .into()
}

pub(super) fn task_session_context_menu(
    app: &App,
    id: String,
    content: Element<'static, Message>,
) -> Element<'static, Message> {
    let id_for_right_click = std::rc::Rc::new(id.clone());
    let right_click = Element::new(RightClickArea::new(
        content,
        Box::new(move |position| {
            Message::Project(ProjectMessage::SessionRightClicked(
                (*id_for_right_click).clone(),
                position.x,
                position.y,
            ))
        }),
    ));

    if app.session_menu_id.as_ref() == Some(&id) {
        let project_path = app
            .known_session_directory(&id)
            .or_else(|| app.project_path.clone())
            .unwrap_or_default();
        let compacting = app
            .sessions
            .iter()
            .chain(app.project_sessions.values().flatten())
            .find(|session| session.id == id)
            .is_some_and(|session| session.time.compacting.is_some());
        PointBelowOverlay::new(
            right_click,
            build_session_menu(
                id.clone(),
                project_path,
                app.session_is_requesting(&id) || compacting,
            ),
        )
        .show(true)
        .anchor(app.session_menu_anchor.unwrap_or(Point::ORIGIN))
        .on_close(Message::Project(ProjectMessage::SessionMenuClose))
        .into()
    } else {
        right_click
    }
}

fn task_button(app: &App, task: &WorkTaskItem) -> Element<'static, Message> {
    let raw_title = if task.title.trim().is_empty() {
        "未命名任务".to_string()
    } else {
        task.title.clone()
    };
    let title = truncate_work_task_text(&raw_title, app.work_sidebar_width, false);
    let project_label = truncate_work_task_text(&task.project_label, app.work_sidebar_width, true);
    let updated = crate::app::components::chat_panel::utils::relative_time_label(task.updated);
    let mut meta = row![
        text(project_label)
            .size(11)
            .wrapping(text::Wrapping::None)
            .style(muted_task_text_style)
            .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);
    if let Some(status) = task_status_badge(task.status, app.status_animation_frame) {
        meta = meta.push(status);
    }
    meta = meta.push(text(updated).size(11).style(muted_task_text_style));
    let title_row = row![
        work_task_status_icon(&task.id, task.status),
        text(title).size(13).wrapping(text::Wrapping::None).width(Length::Fill),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);
    let content = button(column![title_row, meta].spacing(2))
        .on_press(task.message.clone())
        .padding([7, 9])
        .width(Length::Fill)
        .style(navigation_button_style)
        .into();
    task_session_context_menu(app, task.id.clone(), content)
}

fn muted_task_text_style(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.palette().text.scale_alpha(0.52)) }
}

fn work_task_status_icon(id: &str, status: WorkTaskStatus) -> Element<'static, Message> {
    let (is_running, queued, has_unseen_success) = match status {
        WorkTaskStatus::Idle => (false, 0, false),
        WorkTaskStatus::Running => (true, 0, false),
        WorkTaskStatus::Queued(count) => (false, count, false),
        WorkTaskStatus::Updated => (false, 0, true),
    };
    let now_s = crate::app::time::now_ms() as f64 / 1000.0;
    session_status_icon(id, is_running, queued, has_unseen_success, now_s)
}

fn task_status_badge(
    status: WorkTaskStatus,
    animation_frame: usize,
) -> Option<Element<'static, Message>> {
    let (content, color): (Element<'static, Message>, Color) = match status {
        WorkTaskStatus::Idle => return None,
        WorkTaskStatus::Running => (
            row![
                text(spinner_frame(animation_frame))
                    .size(10)
                    .line_height(text::LineHeight::Relative(1.0)),
                text("运行中").size(10),
            ]
            .spacing(3)
            .align_y(iced::Alignment::Center)
            .into(),
            Color::from_rgb8(239, 68, 68),
        ),
        WorkTaskStatus::Queued(count) => {
            (text(format!("排队 {count}")).size(10).into(), Color::from_rgb8(245, 158, 11))
        }
        WorkTaskStatus::Updated => (text("有更新").size(10).into(), Color::from_rgb8(46, 184, 114)),
    };
    Some(
        container(content)
            .padding([1, 5])
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(color.scale_alpha(0.16))),
                border: Border { radius: 999.0.into(), ..Default::default() },
                text_color: Some(color),
                ..Default::default()
            })
            .into(),
    )
}

pub(super) fn truncate_work_task_text(
    value: &str,
    sidebar_width: f32,
    reserve_time: bool,
) -> String {
    let trailing_width =
        if reserve_time { WORK_TASK_TIME_RESERVED_WIDTH } else { WORK_TASK_TITLE_END_GUTTER_WIDTH };
    let available_width = (sidebar_width - WORK_TASK_ROW_FIXED_WIDTH - trailing_width).max(65.0);
    let max_display_width = (available_width / WORK_TASK_DISPLAY_UNIT_WIDTH).max(10.0) as usize;
    truncate_display_width(value, max_display_width)
}

fn space_tools_menu_action(
    label: &'static str,
    icon: Icon,
    message: Message,
) -> Element<'static, Message> {
    button(
        row![
            container(navigation_icon(icon)).width(Length::Fixed(28.0)),
            text(label).size(15),
            Space::new().width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .on_press(Message::Batch(vec![
        Message::Project(ProjectMessage::ProjectToolsMenuClosed),
        message,
    ]))
    .padding([11, 12])
    .width(Length::Fill)
    .style(navigation_button_style)
    .into()
}

fn space_tools_menu(path: String) -> Element<'static, Message> {
    container(
        column![
            space_tools_menu_action(
                "刷新任务",
                Icon::ArrowRepeat,
                Message::Project(ProjectMessage::ProjectLoadSessions(path.clone())),
            ),
            space_tools_menu_action(
                "编辑项目",
                Icon::Sliders,
                Message::Project(ProjectMessage::ProjectEditOpened(path.clone())),
            ),
            more_menu_divider(),
            space_tools_menu_action(
                "打开文件夹",
                Icon::FolderOpen,
                Message::Project(ProjectMessage::RecentRevealPressed(path.clone())),
            ),
            space_tools_menu_action(
                "从列表中移除",
                Icon::Trash,
                Message::Project(ProjectMessage::RecentRemovePressed(path)),
            ),
        ]
        .spacing(3),
    )
    .padding([10, 12])
    .width(Length::Fixed(220.0))
    .style(more_menu_style)
    .into()
}

fn space_row_style(selected: bool) -> impl Fn(&Theme) -> container::Style + Copy {
    move |theme| container::Style {
        background: selected.then(|| {
            Background::Color(theme.extended_palette().background.strong.color.scale_alpha(0.72))
        }),
        border: Border { radius: 9.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn space_button(space: &WorkSpaceItem) -> Element<'static, Message> {
    const SPACE_FOLDER_ICON_SIZE: f32 = 16.0;
    let switch = button(
        row![
            navigation_icon(Icon::FolderOpen)
                .width(Length::Fixed(SPACE_FOLDER_ICON_SIZE))
                .height(Length::Fixed(SPACE_FOLDER_ICON_SIZE)),
            text(space.label.clone()).size(14).wrapping(text::Wrapping::None),
        ]
        .spacing(9)
        .align_y(iced::Alignment::Center),
    )
    .on_press(space.message.clone())
    .padding([7, 5])
    .width(Length::Shrink)
    .style(navigation_button_style);

    let toggle = header_icon_button_sized(
        if space.expanded { Icon::ChevronUp } else { Icon::ChevronDown },
        if space.expanded { "收起任务" } else { "展开任务" },
        space.toggle_message.clone(),
        false,
        28.0,
    );

    let actions: Element<'static, Message> = if space.hovered || space.tools_open {
        let tools_trigger = header_icon_button_sized(
            Icon::DotsThreeVertical,
            "空间操作",
            space.tools_message.clone(),
            space.tools_open,
            30.0,
        );
        let tools: Element<'static, Message> =
            BelowOverlay::new(tools_trigger, space_tools_menu(space.path.clone()))
                .show(space.tools_open)
                .gap(6.0)
                .on_close(Message::Project(ProjectMessage::ProjectToolsMenuClosed))
                .into();
        let create = if space.creating {
            header_icon_button_sized_maybe(Icon::Plus, "任务创建中", None, false, 30.0)
        } else {
            header_icon_button_sized(
                Icon::Plus,
                "新建任务",
                Message::Batch(vec![
                    Message::Project(ProjectMessage::ProjectToolsMenuClosed),
                    space.create_message.clone(),
                ]),
                false,
                30.0,
            )
        };
        row![tools, create]
            .spacing(3)
            .align_y(iced::Alignment::Center)
            .width(Length::Fixed(63.0))
            .into()
    } else {
        Space::new().width(Length::Fixed(63.0)).into()
    };

    let path = space.path.clone();
    mouse_area(
        container(
            row![switch, toggle, Space::new().width(Length::Fill), actions,]
                .spacing(2)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .style(space_row_style(space.selected)),
    )
    .on_enter(Message::Project(ProjectMessage::RecentHovered(Some(path))))
    .on_exit(Message::Project(ProjectMessage::RecentHovered(None)))
    .into()
}

fn space_session_button(app: &App, session: &WorkSpaceSession) -> Element<'static, Message> {
    let raw_title = if session.title.trim().is_empty() {
        "未命名任务".to_string()
    } else {
        session.title.clone()
    };
    let title = truncate_work_task_text(&raw_title, app.work_sidebar_width, true);
    let content_row = row![
        work_task_status_icon(&session.id, session.status),
        text(title).size(12).wrapping(text::Wrapping::None).width(Length::Fill),
        text(crate::app::components::chat_panel::utils::relative_time_label(session.updated))
            .size(11)
            .style(muted_task_text_style),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);
    let content = button(content_row)
        .on_press(session.message.clone())
        .padding([7, 9])
        .width(Length::Fill)
        .style(navigation_button_style)
        .into();
    task_session_context_menu(app, session.id.clone(), content)
}

fn navigation_section(app: &App) -> Element<'static, Message> {
    let specs = navigation_specs();
    let create_task_button = if app.new_session_creating {
        navigation_button_maybe("创建中...", specs[0].1, None, specs[0].2)
    } else {
        navigation_button(
            specs[0].0,
            specs[0].1,
            Message::View(ViewMessage::ProjectFileNewSession),
            specs[0].2,
        )
    };
    let navigation = column![create_task_button].spacing(2);

    #[cfg(not(target_arch = "wasm32"))]
    let navigation = navigation
        .push(navigation_button(
            specs[1].0,
            specs[1].1,
            expert_hub_message(ExpertHubSection::Experts),
            specs[1].2,
        ))
        .push(project_navigation_control(app))
        .push(shiju_apps_button())
        .push(shiju_skills_button())
        .push(expert_hub_button());

    #[cfg(target_arch = "wasm32")]
    let navigation = navigation.push(project_navigation_control(app));

    navigation
        .push(navigation_button(
            specs[6].0,
            specs[6].1,
            Message::View(ViewMessage::OpenWorkflowManager),
            specs[6].2,
        ))
        .push(more_control(app))
        .into()
}

pub(super) fn project_navigation_message() -> Message {
    Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Projects))
}

pub(super) fn project_create_message() -> Message {
    Message::View(ViewMessage::ProjectFileNewProject)
}

fn project_navigation_control(app: &App) -> Element<'static, Message> {
    const CREATE_CONTROL_SIZE: f32 = 30.0;
    let (_, icon, _) = navigation_specs()[2];
    let switch = button(
        row![
            container(navigation_icon(icon))
                .width(Length::Fixed(WORK_NAVIGATION_ICON_COLUMN_WIDTH)),
            text("项目").size(14),
            Space::new().width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .on_press(project_navigation_message())
    .padding([9, 11])
    .width(Length::Fill)
    .style(navigation_button_style);

    let create = header_icon_button_sized(
        Icon::Plus,
        "新建项目",
        project_create_message(),
        false,
        CREATE_CONTROL_SIZE,
    );

    let content: Element<'static, Message> = if app.work_sidebar_project_nav_hovered {
        Stack::new()
            .push(switch.width(Length::Fill))
            .push(
                container(create)
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding([9, 12]),
            )
            .width(Length::Fill)
            .into()
    } else {
        switch.width(Length::Fill).into()
    };

    mouse_area(content)
        .on_enter(Message::View(ViewMessage::WorkSidebarProjectNavHovered(true)))
        .on_move(|_| Message::View(ViewMessage::WorkSidebarProjectNavHovered(true)))
        .on_exit(Message::View(ViewMessage::WorkSidebarProjectNavHovered(false)))
        .into()
}

fn load_more_button(remaining: usize, message: Message) -> Element<'static, Message> {
    button(
        text(format!("加载更多 · 还有 {remaining} 条"))
            .size(12)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(message)
    .padding([8, 9])
    .width(Length::Fill)
    .style(navigation_button_style)
    .into()
}

fn tasks_section(app: &App, model: &WorkSidebarModel) -> Element<'static, Message> {
    let mut tasks = column![].spacing(3);
    if model.tasks.is_empty() {
        tasks =
            tasks.push(container(text("暂无任务").size(12)).padding([7, 9]).width(Length::Fill));
    } else {
        for task in &model.tasks {
            tasks = tasks.push(task_button(app, task));
        }
        if model.tasks.len() < model.task_total {
            tasks = tasks.push(load_more_button(
                model.task_total - model.tasks.len(),
                Message::View(ViewMessage::WorkSidebarLoadMorePressed),
            ));
        }
    }
    tasks.into()
}

fn spaces_section(app: &App, model: &WorkSidebarModel) -> Element<'static, Message> {
    let mut spaces = column![].spacing(2);
    if model.spaces.is_empty() {
        spaces =
            spaces.push(container(text("暂无空间").size(12)).padding([7, 9]).width(Length::Fill));
    } else {
        for space in &model.spaces {
            spaces = spaces.push(space_button(space));
            for session in &space.sessions {
                spaces = spaces.push(space_session_button(app, session));
            }
            if space.expanded && space.sessions.len() < space.session_total {
                spaces = spaces.push(load_more_button(
                    space.session_total - space.sessions.len(),
                    Message::View(ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(
                        space.path.clone(),
                    )),
                ));
            }
        }
        if model.spaces.len() < model.space_total {
            spaces = spaces.push(load_more_button(
                model.space_total - model.spaces.len(),
                Message::View(ViewMessage::WorkSidebarLoadMorePressed),
            ));
        }
    }
    spaces.into()
}

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
    label: &'static str,
    count: usize,
    tab: WorkSidebarTab,
    selected: bool,
) -> Element<'static, Message> {
    button(
        row![text(label).size(13), text(format!("({count})")).size(11)]
            .spacing(5)
            .align_y(iced::Alignment::Center),
    )
    .on_press(Message::View(ViewMessage::WorkSidebarTabSelected(tab)))
    .padding([7, 10])
    .width(Length::Fill)
    .style(sidebar_tab_style(selected))
    .into()
}

fn tabs(model: &WorkSidebarModel, selected: WorkSidebarTab) -> Element<'static, Message> {
    row![
        sidebar_tab(
            "任务",
            model.task_total,
            WorkSidebarTab::Tasks,
            selected == WorkSidebarTab::Tasks,
        ),
        sidebar_tab(
            "空间",
            model.space_total,
            WorkSidebarTab::Spaces,
            selected == WorkSidebarTab::Spaces,
        ),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    if app.work_sidebar_collapsed {
        return container(collapsed_header(app))
            .width(Length::Fixed(app.work_sidebar_width))
            .height(Length::Fill)
            .padding([10, 9])
            .style(sidebar_style)
            .into();
    }

    let model = model(app);
    let header = expanded_header(app);

    let active_list = match app.work_sidebar_tab {
        WorkSidebarTab::Tasks => tasks_section(app, &model),
        WorkSidebarTab::Spaces => spaces_section(app, &model),
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

    container(
        column![header, navigation_section(app), tabs(&model, app.work_sidebar_tab), scroll,]
            .spacing(9)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(app.work_sidebar_width))
    .height(Length::Fill)
    .padding([10, 10])
    .style(sidebar_style)
    .into()
}
