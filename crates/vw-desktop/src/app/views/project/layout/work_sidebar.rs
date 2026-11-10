//! 工作模式专属左栏，使用现有项目、会话与业务消息构建工作台导航。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, Stack, button, column, container, mouse_area, row, scrollable, svg, text,
};
use iced::{Background, Border, Element, Length, Point, Shadow, Theme, Vector};

use crate::app::assets::{self, Icon};
use crate::app::components::input_panel::styles::tooltip_dark_style;
use crate::app::components::overlays::{BelowOverlay, PointBelowOverlay, SideOverlay};
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

use super::super::components::{
    SessionRowDetails, SessionRowStatus as WorkTaskStatus, build_session_menu,
    project_sessions_header_with_tools, running_draft_sessions, session_row,
};
use super::super::styles::work_surface_style;

mod rail;

pub(super) const WORK_SIDEBAR_SCROLLBAR_WIDTH: u32 = 4;
pub(super) const WORK_SIDEBAR_SCROLLBAR_GUTTER: f32 = 8.0;
const WORK_NAVIGATION_ICON_COLUMN_WIDTH: f32 = 22.0;
const WORK_HEADER_ICON_SIZE: f32 = 16.0;
const WORK_HEADER_CONTROL_SIZE: f32 = 32.0;
const WORK_FILTER_CONTROL_SIZE: f32 = 30.0;
const MILLIS_PER_DAY: u64 = 24 * 60 * 60 * 1_000;

pub(super) struct WorkTaskItem {
    pub(super) id: String,
    pub(super) details: SessionRowDetails,
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
    pub(super) sessions: Vec<WorkTaskItem>,
    pub(super) session_total: usize,
    pub(super) message: Message,
    pub(super) toggle_message: Message,
    pub(super) tools_message: Message,
}

pub(crate) struct WorkSidebarModel {
    pub(super) tasks: Vec<WorkTaskItem>,
    pub(super) task_total: usize,
    pub(super) spaces: Vec<WorkSpaceItem>,
    pub(super) space_total: usize,
}

pub(super) fn navigation_specs() -> [(&'static str, Icon, Option<&'static str>); 9] {
    [
        ("新建任务", Icon::FileEarmarkPlus, None),
        ("助手", Icon::Robot, None),
        ("项目", Icon::FolderOpen, None),
        ("识矩应用", Icon::Grid1x2, None),
        ("识矩技能", Icon::Star, None),
        ("专家 · 技能 · 连接器", Icon::GearWideConnected, None),
        ("定时任务", Icon::ArrowRepeat, None),
        ("识矩知识库", Icon::Journals, None),
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
    // Most projects contain only root conversations. Build the parent index only
    // when a visible child actually needs a lookup, and hash the directory once.
    let mut parents = None;
    sessions
        .iter()
        .copied()
        .filter(|(path, session)| {
            if session.time.archived.is_some() || !session.is_user_navigable() {
                return false;
            }
            let Some(parent_id) = session.parent_id.as_deref().filter(|id| !id.trim().is_empty())
            else {
                return true;
            };
            session.is_visible_task()
                && session.directory == *path
                && parents
                    .get_or_insert_with(|| {
                        sessions
                            .iter()
                            .filter(|(path, parent)| parent.directory == *path)
                            .map(|(path, parent)| {
                                (*path, parent.id.as_str(), parent.project_id.as_str())
                            })
                            .collect::<std::collections::HashSet<_>>()
                    })
                    .contains(&(*path, parent_id, session.project_id.as_str()))
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
    right
        .1
        .is_first_response_pending()
        .cmp(&left.1.is_first_response_pending())
        .then_with(|| right_pinned.cmp(&left_pinned))
        .then_with(|| {
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

pub(super) fn cached_model(app: &App) -> std::sync::Arc<WorkSidebarModel> {
    let mut cached = app.session_ui_projection.sidebar.borrow_mut();
    cached.get_or_insert_with(|| std::sync::Arc::new(model(app))).clone()
}

pub(super) fn model(app: &App) -> WorkSidebarModel {
    let pinned = &app.session_ui_projection.pinned;
    let mut drafts_by_project = std::collections::HashMap::<_, Vec<_>>::new();
    for (path, draft) in running_draft_sessions(app) {
        drafts_by_project.entry(path).or_default().push(draft);
    }
    // Share the filtered projection between the task and space tabs. Building
    // both independently repeats all parent lookups and runtime filters.
    let mut sessions_by_project = std::collections::HashMap::new();
    let mut task_sessions = Vec::new();
    for path in app.recent_projects.iter().filter(|path| project_matches_filter(app, path)) {
        let mut sessions = collect_top_level_sessions(
            app.project_sessions
                .get(path)
                .into_iter()
                .flatten()
                .map(|session| (path.as_str(), session)),
        );
        for draft in drafts_by_project.get(path.as_str()).into_iter().flatten() {
            if !sessions.iter().any(|(_, session)| session.id == draft.id) {
                sessions.push((path, draft));
            }
        }
        sessions.retain(|(_, session)| {
            session_matches_status_filter(app, &session.id)
                && session_matches_time_filter(app, session.time.updated)
        });
        task_sessions.extend(sessions.iter().copied());
        sessions_by_project.insert(path.as_str(), sessions);
    }
    let task_total = task_sessions.len();
    sort_and_truncate_sessions(&mut task_sessions, app.work_sidebar_visible_tasks, pinned);
    let tasks = task_sessions
        .into_iter()
        .map(|(path, session)| WorkTaskItem {
            id: session.id.clone(),
            details: SessionRowDetails::new(session, project_label(app, path)),
            status: WorkTaskStatus::for_session(app, &session.id),
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
            let mut filtered_sessions =
                sessions_by_project.remove(path.as_str()).unwrap_or_default();
            if (app.work_sidebar_filter_status != WorkSidebarFilterStatus::All
                || app.work_sidebar_filter_time != WorkSidebarFilterTime::All)
                && filtered_sessions.is_empty()
            {
                return None;
            }

            let label = project_label(app, path);
            let expanded = app.work_expanded_spaces.contains(path);
            let session_total = filtered_sessions.len();
            let sessions = if expanded {
                let visible = app
                    .work_sidebar_space_task_visible
                    .get(path)
                    .copied()
                    .unwrap_or(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE);
                sort_and_truncate_sessions(&mut filtered_sessions, visible, pinned);
                filtered_sessions
                    .into_iter()
                    .map(|(_, session)| WorkTaskItem {
                        id: session.id.clone(),
                        details: SessionRowDetails::new(session, label.clone()),
                        status: WorkTaskStatus::for_session(app, &session.id),
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
                label,
                selected: app.project_path.as_deref() == Some(path.as_str()),
                expanded,
                hovered: app.hovered_recent_project.as_deref() == Some(path.as_str()),
                tools_open: app.project_tools_menu_path.as_deref() == Some(path.as_str()),
                sessions,
                session_total,
                message: Message::Project(ProjectMessage::SwitchWorkSpacePressed(path.clone())),
                toggle_message: Message::View(ViewMessage::WorkSpaceToggled(path.clone())),
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
pub(super) fn expert_hub_button(i18n: crate::app::i18n::I18n) -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[5];
    navigation_button(i18n, label, icon, Message::SkillHub(SkillHubMessage::Open), trailing)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn shiju_apps_button(i18n: crate::app::i18n::I18n) -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[3];
    navigation_button(i18n, label, icon, Message::ShijuApps(ShijuAppsMessage::Open), trailing)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn shiju_skills_button(i18n: crate::app::i18n::I18n) -> Element<'static, Message> {
    let (label, icon, trailing) = navigation_specs()[4];
    navigation_button(
        i18n,
        label,
        icon,
        Message::SkillHub(SkillHubMessage::OpenShijuSkills),
        trailing,
    )
}

pub(super) fn sidebar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(theme.palette().text),
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.72),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..work_surface_style(theme)
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
    svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(WORK_HEADER_ICON_SIZE))
        .height(Length::Fixed(WORK_HEADER_ICON_SIZE))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text.scale_alpha(0.82)),
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
    i18n: crate::app::i18n::I18n,
    icon: Icon,
    label: &'static str,
    message: Message,
    active: bool,
) -> Element<'static, Message> {
    header_icon_button_sized(
        i18n,
        header_icon(icon),
        label,
        message,
        active,
        WORK_HEADER_CONTROL_SIZE,
    )
}

fn header_icon_button_sized(
    i18n: crate::app::i18n::I18n,
    icon: svg::Svg<'static>,
    label: &'static str,
    message: Message,
    active: bool,
    control_size: f32,
) -> Element<'static, Message> {
    let control = icon_control(icon, Some(message), active, control_size);
    Tooltip::new(
        control,
        container(text(i18n.__(label)).size(12)).padding([6, 8]).style(tooltip_dark_style),
        TooltipPosition::Bottom,
    )
    .gap(6)
    .into()
}

fn icon_control(
    icon: svg::Svg<'static>,
    message: Option<Message>,
    active: bool,
    control_size: f32,
) -> button::Button<'static, Message> {
    let mut control = button(
        container(icon)
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

    control
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
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    let mut content = row![text(i18n.__(label)).size(14), Space::new().width(Length::Fill)]
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
        text(crate::app::i18n::I18n::new(app.desktop_locale).__("筛选状态")).size(12).style(
            |theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.52)) }
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("全部状态"),
            status == WorkSidebarFilterStatus::All,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::All,
            )),
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("进行中"),
            status == WorkSidebarFilterStatus::InProgress,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::InProgress,
            )),
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("已完成"),
            status == WorkSidebarFilterStatus::Completed,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::Completed,
            )),
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("待处理"),
            status == WorkSidebarFilterStatus::Pending,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::Pending,
            )),
        ),
        filter_divider(),
        text(crate::app::i18n::I18n::new(app.desktop_locale).__("筛选范围")).size(12).style(
            |theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.52)) }
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("全部项目"),
            scope == WorkSidebarFilterScope::All,
            Message::View(
                ViewMessage::WorkSidebarFilterScopeSelected(WorkSidebarFilterScope::All,)
            ),
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("当前项目"),
            scope == WorkSidebarFilterScope::CurrentProject,
            Message::View(ViewMessage::WorkSidebarFilterScopeSelected(
                WorkSidebarFilterScope::CurrentProject,
            )),
        ),
        filter_divider(),
        text(crate::app::i18n::I18n::new(app.desktop_locale).__("筛选时间")).size(12).style(
            |theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.52)) }
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("全部时间"),
            time == WorkSidebarFilterTime::All,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(WorkSidebarFilterTime::All,)),
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("最近 24 小时"),
            time == WorkSidebarFilterTime::LastDay,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastDay,
            )),
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("最近 7 天"),
            time == WorkSidebarFilterTime::LastSevenDays,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastSevenDays,
            )),
        ),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("最近 30 天"),
            time == WorkSidebarFilterTime::LastThirtyDays,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastThirtyDays,
            )),
        ),
        filter_divider(),
        filter_option(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("重置筛选条件"),
            false,
            Message::View(ViewMessage::WorkSidebarFilterReset),
        ),
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
        crate::app::i18n::I18n::new(app.desktop_locale),
        header_icon(icon),
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

fn search_control(i18n: crate::app::i18n::I18n) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(false)[3];
    header_icon_button(
        i18n,
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
    header_icon_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        icon,
        label,
        Message::View(ViewMessage::WorkSidebarToggled),
        false,
    )
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
    header_icon_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        icon,
        label,
        refresh_all_message(app),
        is_refreshing,
    )
}

pub(super) fn task_pool_control(app: &App) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[2];
    header_icon_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        icon,
        label,
        Message::TaskBoard(TaskBoardMessage::ToggleBoard),
        false,
    )
}

fn expanded_header(app: &App) -> Element<'static, Message> {
    row![
        container(text(crate::app::i18n::I18n::new(app.desktop_locale).__("识矩")).size(17))
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Center),
        sidebar_toggle_control(app),
        refresh_all_control(app),
        task_pool_control(app),
        search_control(crate::app::i18n::I18n::new(app.desktop_locale)),
        filter_control(app),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn current_project_card(app: &App, path: &str) -> Element<'static, Message> {
    container(project_sessions_header_with_tools(
        crate::app::i18n::I18n::new(app.desktop_locale),
        project_label(app, path),
        path.to_owned(),
        false,
        ProjectMessage::CurrentProjectMenuToggled(path.to_owned()),
        app.work_current_project_menu.as_deref() == Some(path),
        app.project_sessions_loading.contains(path),
    ))
    .padding([6, 10])
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
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Message,
    trailing: Option<&'static str>,
) -> Element<'static, Message> {
    navigation_button_maybe(i18n, label, icon, Some(message), trailing)
}

fn navigation_button_maybe(
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Option<Message>,
    trailing: Option<&'static str>,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let mut content = row![
        container(navigation_icon_enabled(icon, enabled))
            .width(Length::Fixed(WORK_NAVIGATION_ICON_COLUMN_WIDTH)),
        text(i18n.__(label)).size(14).style(move |theme: &Theme| text::Style {
            color: (!enabled).then(|| theme.palette().text.scale_alpha(0.32)),
        }),
        Space::new().width(Length::Fill),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    if let Some(trailing) = trailing {
        content = content.push(text(i18n.__(trailing)).size(11).style(|theme: &Theme| {
            text::Style { color: Some(theme.palette().text.scale_alpha(0.48)) }
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
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Message,
    width: Length,
) -> Element<'static, Message> {
    button(
        row![
            container(navigation_icon(icon)).width(Length::Fixed(28.0)),
            text(i18n.__(label)).size(15),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .on_press(close_more_then(message))
    .padding([11, 12])
    .width(width)
    .style(navigation_button_style)
    .into()
}

fn more_menu_items(
    i18n: crate::app::i18n::I18n,
    width: Length,
) -> iced::widget::Column<'static, Message> {
    let specs = more_menu_item_specs();
    let add_url = Message::Batch(vec![
        Message::View(ViewMessage::OpenApps),
        Message::View(ViewMessage::ToggleWebLinksMenu),
    ]);
    column![
        more_menu_action(i18n, specs[0].0, specs[0].1, add_url, width),
        more_menu_divider(),
        more_menu_action(
            i18n,
            specs[1].0,
            specs[1].1,
            Message::View(ViewMessage::OpenKnowledge),
            width
        ),
        more_menu_action(
            i18n,
            specs[2].0,
            specs[2].1,
            Message::View(ViewMessage::OpenWorkflowTool),
            width,
        ),
        more_menu_divider(),
        more_menu_action(i18n, specs[3].0, specs[3].1, Message::View(ViewMessage::OpenApps), width),
    ]
    .spacing(3)
    .width(width)
}

fn more_control(app: &App) -> Element<'static, Message> {
    let (_, icon, trailing) = navigation_specs()[8];
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let trigger = navigation_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        crate::app::i18n::I18n::new(app.desktop_locale).__("更多"),
        icon,
        Message::View(ViewMessage::WorkSidebarMoreOpened),
        trailing,
    );
    let hover_trigger =
        mouse_area(trigger).on_enter(Message::View(ViewMessage::WorkSidebarMoreOpened));
    let menu = container(more_menu_items(i18n, Length::Fill))
        .padding([12, 14])
        .width(Length::Fixed(252.0))
        .style(more_menu_style);
    SideOverlay::new(hover_trigger, menu)
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
                crate::app::i18n::I18n::new(app.desktop_locale),
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
    let content = session_row(app, &task.id, &task.details, task.status, task.message.clone());
    if task.details.pending {
        return content;
    }
    task_session_context_menu(app, task.id.clone(), content)
}

fn space_tools_menu_action(
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Message,
) -> Element<'static, Message> {
    const MENU_ICON_SIZE: f32 = 15.0;
    button(
        row![
            container(
                navigation_icon(icon)
                    .width(Length::Fixed(MENU_ICON_SIZE))
                    .height(Length::Fixed(MENU_ICON_SIZE)),
            )
            .width(Length::Fixed(28.0)),
            text(i18n.__(label)).size(15),
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

fn space_tools_menu(i18n: crate::app::i18n::I18n, path: String) -> Element<'static, Message> {
    container(
        column![
            space_tools_menu_action(
                i18n,
                i18n.__("刷新任务"),
                Icon::ArrowRepeat,
                Message::Project(ProjectMessage::ProjectLoadSessions(path.clone())),
            ),
            space_tools_menu_action(
                i18n,
                i18n.__("编辑项目"),
                Icon::Sliders,
                Message::Project(ProjectMessage::ProjectEditOpened(path.clone())),
            ),
            more_menu_divider(),
            space_tools_menu_action(
                i18n,
                i18n.__("打开文件夹"),
                Icon::FolderOpen,
                Message::Project(ProjectMessage::RecentRevealPressed(path.clone())),
            ),
            space_tools_menu_action(
                i18n,
                i18n.__("从列表中移除"),
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

fn space_button(i18n: crate::app::i18n::I18n, space: &WorkSpaceItem) -> Element<'static, Message> {
    const SPACE_FOLDER_ICON_SIZE: f32 = 16.0;
    const SPACE_ACTION_ICON_SIZE: f32 = 14.0;
    let action_icon = |icon| {
        header_icon(icon)
            .width(Length::Fixed(SPACE_ACTION_ICON_SIZE))
            .height(Length::Fixed(SPACE_ACTION_ICON_SIZE))
    };

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
        i18n,
        action_icon(if space.expanded { Icon::ChevronUp } else { Icon::ChevronDown }),
        if space.expanded { i18n.__("收起任务") } else { i18n.__("展开任务") },
        space.toggle_message.clone(),
        false,
        28.0,
    );

    let actions: Element<'static, Message> = if space.hovered || space.tools_open {
        let tools_trigger = header_icon_button_sized(
            i18n,
            action_icon(Icon::DotsThreeVertical),
            i18n.__("空间操作"),
            space.tools_message.clone(),
            space.tools_open,
            30.0,
        );
        BelowOverlay::new(tools_trigger, space_tools_menu(i18n, space.path.clone()))
            .show(space.tools_open)
            .gap(6.0)
            .on_close(Message::Project(ProjectMessage::ProjectToolsMenuClosed))
            .into()
    } else {
        Space::new().width(Length::Fixed(30.0)).into()
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

fn new_task_button(app: &App) -> Element<'static, Message> {
    let (label, icon, _) = navigation_specs()[0];
    navigation_button_maybe(
        crate::app::i18n::I18n::new(app.desktop_locale),
        if app.new_session_creating { "创建中..." } else { label },
        icon,
        (!app.new_session_creating).then_some(Message::View(ViewMessage::ProjectFileNewSession)),
        None,
    )
}

fn navigation_section(app: &App) -> Element<'static, Message> {
    let specs = navigation_specs();
    let navigation = column![new_task_button(app)].spacing(2);

    #[cfg(not(target_arch = "wasm32"))]
    let navigation = navigation
        .push(navigation_button(
            crate::app::i18n::I18n::new(app.desktop_locale),
            specs[1].0,
            specs[1].1,
            expert_hub_message(ExpertHubSection::Experts),
            specs[1].2,
        ))
        .push(project_navigation_control(app))
        .push(shiju_apps_button(crate::app::i18n::I18n::new(app.desktop_locale)))
        .push(shiju_skills_button(crate::app::i18n::I18n::new(app.desktop_locale)))
        .push(expert_hub_button(crate::app::i18n::I18n::new(app.desktop_locale)));

    #[cfg(target_arch = "wasm32")]
    let navigation = navigation.push(project_navigation_control(app));

    let navigation = navigation.push(navigation_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        specs[6].0,
        specs[6].1,
        Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Cron)),
        specs[6].2,
    ));
    #[cfg(not(target_arch = "wasm32"))]
    let navigation = navigation.push(navigation_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        specs[7].0,
        specs[7].1,
        expert_hub_message(ExpertHubSection::Knowledge),
        specs[7].2,
    ));
    navigation.push(more_control(app)).into()
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
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("项目")).size(14),
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
        crate::app::i18n::I18n::new(app.desktop_locale),
        header_icon(Icon::Plus),
        crate::app::i18n::I18n::new(app.desktop_locale).__("新建项目"),
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

fn load_more_button(
    i18n: crate::app::i18n::I18n,
    remaining: usize,
    message: Message,
) -> Element<'static, Message> {
    button(
        text(
            i18n.format("加载更多 · 还有 %u 条", &[remaining.into()])
                .expect("valid pagination template"),
        )
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
        tasks = tasks.push(
            container(
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("暂无任务")).size(12),
            )
            .padding([7, 9])
            .width(Length::Fill),
        );
    } else {
        for task in &model.tasks {
            tasks = tasks.push(task_button(app, task));
        }
        if model.tasks.len() < model.task_total {
            tasks = tasks.push(load_more_button(
                crate::app::i18n::I18n::new(app.desktop_locale),
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
        spaces = spaces.push(
            container(
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("暂无空间")).size(12),
            )
            .padding([7, 9])
            .width(Length::Fill),
        );
    } else {
        for space in &model.spaces {
            spaces =
                spaces.push(space_button(crate::app::i18n::I18n::new(app.desktop_locale), space));
            for session in &space.sessions {
                spaces = spaces.push(task_button(app, session));
            }
            if space.expanded && space.sessions.len() < space.session_total {
                spaces = spaces.push(load_more_button(
                    crate::app::i18n::I18n::new(app.desktop_locale),
                    space.session_total - space.sessions.len(),
                    Message::View(ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(
                        space.path.clone(),
                    )),
                ));
            }
        }
        if model.spaces.len() < model.space_total {
            spaces = spaces.push(load_more_button(
                crate::app::i18n::I18n::new(app.desktop_locale),
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
    i18n: crate::app::i18n::I18n,
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
    i18n: crate::app::i18n::I18n,
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

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let model = cached_model(app);

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

    let mut navigation = column![].spacing(9);
    if !app.work_sidebar_collapsed {
        navigation = navigation.push(expanded_header(app));
    }
    if let Some(path) = app.project_path.as_deref() {
        navigation = navigation.push(current_project_card(app, path));
    }
    let navigation = navigation.push(if app.work_sidebar_collapsed {
        new_task_button(app)
    } else {
        navigation_section(app)
    });
    let sidebar = container(
        column![
            navigation,
            tabs(crate::app::i18n::I18n::new(app.desktop_locale), &model, app.work_sidebar_tab),
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
