use super::*;

pub struct WorkTaskItem {
    pub id: String,
    pub details: SessionRowDetails,
    pub status: WorkTaskStatus,
    pub message: Message,
}

pub struct WorkSpaceItem {
    pub(super) path: String,
    pub label: String,
    pub selected: bool,
    pub expanded: bool,
    pub hovered: bool,
    pub tools_open: bool,
    pub sessions: Vec<WorkTaskItem>,
    pub session_total: usize,
    pub message: Message,
    pub toggle_message: Message,
    pub tools_message: Message,
}

pub struct WorkSidebarModel {
    pub tasks: Vec<WorkTaskItem>,
    pub task_total: usize,
    pub spaces: Vec<WorkSpaceItem>,
    pub space_total: usize,
}

pub fn navigation_specs() -> [(&'static str, Icon, Option<&'static str>); 9] {
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

pub fn more_menu_item_specs() -> [(&'static str, Icon); 4] {
    [
        ("添加网址", Icon::Link45Deg),
        ("知识库", Icon::Journals),
        ("工作流", Icon::GitBranch),
        ("应用中心", Icon::Grid1x2),
    ]
}

pub fn header_icon_specs(collapsed: bool) -> [(Icon, &'static str); 5] {
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

pub fn project_label(recent_projects: &[String], recent_projects_edits: &[String], path: &str) -> String {
    recent_projects
        .iter()
        .position(|candidate| candidate == path)
        .and_then(|index| recent_projects_edits.get(index))
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

type SessionRef<'a> = (&'a str, &'a session_info::Info);

fn collect_top_level_sessions<'a>(
    sessions: impl Iterator<Item = (&'a str, &'a session_info::Info)>,
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

pub fn compare_sessions(
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

fn project_matches_filter(app: &ModelInput<'_>, path: &str) -> bool {
    match app.work_sidebar_filter_scope {
        WorkSidebarFilterScope::All => true,
        WorkSidebarFilterScope::CurrentProject => app.project_path.as_deref() == Some(path),
    }
}

fn session_matches_time_filter(app: &ModelInput<'_>, updated: u64) -> bool {
    let days = match app.work_sidebar_filter_time {
        WorkSidebarFilterTime::All => return true,
        WorkSidebarFilterTime::LastDay => 1,
        WorkSidebarFilterTime::LastSevenDays => 7,
        WorkSidebarFilterTime::LastThirtyDays => 30,
    };
    let cutoff = runtime_time::now_ms().saturating_sub(days * MILLIS_PER_DAY);
    updated >= cutoff
}

fn session_matches_status_filter(app: &ModelInput<'_>, session_id: &str) -> bool {
    let (is_requesting, has_pending, _) = (app.activity)(session_id);
    match app.work_sidebar_filter_status {
        WorkSidebarFilterStatus::All => true,
        WorkSidebarFilterStatus::InProgress => is_requesting,
        WorkSidebarFilterStatus::Completed => !is_requesting && !has_pending,
        WorkSidebarFilterStatus::Pending => !is_requesting && has_pending,
    }
}

pub fn model(app: &ModelInput<'_>) -> WorkSidebarModel {
    let pinned = app.pinned;
    let mut drafts_by_project = std::collections::HashMap::<_, Vec<_>>::new();
    for (path, draft) in app.drafts.iter().map(|(path, info)| (*path, info)) {
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
            details: SessionRowDetails::new(session, project_label(app.recent_projects, app.recent_projects_edits, path)),
            status: (app.activity)(&session.id).2,
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

            let label = project_label(app.recent_projects, app.recent_projects_edits, path);
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
                        status: (app.activity)(&session.id).2,
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

pub struct ModelInput<'a> {
 pub recent_projects: &'a [String],
 pub recent_projects_edits: &'a [String],
 pub project_path: &'a Option<String>,
 pub project_sessions: &'a std::collections::HashMap<String, Vec<session_info::Info>>,
 pub pinned: &'a std::collections::HashSet<(String, String)>,
 pub drafts: &'a [(&'a str, session_info::Info)],
 pub activity: &'a dyn Fn(&str) -> (bool, bool, WorkTaskStatus),
 pub work_sidebar_filter_scope: WorkSidebarFilterScope,
 pub work_sidebar_filter_status: WorkSidebarFilterStatus,
 pub work_sidebar_filter_time: WorkSidebarFilterTime,
 pub work_sidebar_visible_tasks: usize,
 pub work_sidebar_visible_spaces: usize,
 pub work_expanded_spaces: &'a std::collections::HashSet<String>,
 pub work_sidebar_space_task_visible: &'a std::collections::HashMap<String, usize>,
 pub hovered_recent_project: &'a Option<String>,
 pub project_tools_menu_path: &'a Option<String>,
}
