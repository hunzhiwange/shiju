//! 本机个人工作偏好；同步小文件事务避免跨项目异步结果串写。
use super::TaskBoardMessage;
use crate::app::{App, Message};
use chrono::{Datelike, Local, Months};
use desktop_task_board::work_views::{
    BuiltinWorkView, ExecutionAgent, SavedWorkView, SidebarWorkView, WorkDateCalendar,
    WorkDateFilter, WorkDateRange, WorkPriorityLevel, WorkToolbarMenu, WorkViewFilter,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectPrefs {
    views: Vec<SavedWorkView>,
    personal_task_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    read_task_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    subscriptions: BTreeMap<String, BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_sidebar_views: Vec<SidebarWorkView>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    sidebar_show_counts: bool,
}
type Preferences = BTreeMap<String, ProjectPrefs>;

impl ProjectPrefs {
    fn current(app: &App, project: &str) -> Self {
        let board = &app.task_board;
        Self {
            views: board.saved_work_views.clone(),
            personal_task_ids: board.personal_tasks.get(project).cloned().unwrap_or_default(),
            read_task_ids: board.work_read_task_ids.clone(),
            subscriptions: board.work_subscriptions.clone(),
            hidden_sidebar_views: board.hidden_work_sidebar_views.clone(),
            sidebar_show_counts: board.work_sidebar_show_counts,
        }
    }
}

fn unique_id() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

fn read(path: &Path) -> Result<Preferences, desktop_i18n::workflow::Feedback> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| desktop_i18n::workflow::Feedback::new("工作视图配置损坏：%s", vec![e.to_string()])),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::new()),
        Err(e) => Err(desktop_i18n::workflow::Feedback::new("读取工作视图失败：%s", vec![e.to_string()])),
    }
}

fn save(path: &Path, project: &str, prefs: ProjectPrefs) -> Result<(), desktop_i18n::workflow::Feedback> {
    // 每次先读取：损坏的原文件不能被默认值覆盖。
    let mut all = read(path)?;
    all.insert(project.to_owned(), prefs);
    let bytes = serde_json::to_vec_pretty(&all).map_err(|e| e.to_string())?;
    let parent = path.parent().ok_or("工作视图保存路径无效")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!("work-views-{}.tmp", unique_id()));
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result.map_err(Into::into)
}

fn store_view(
    app: &mut App,
    project: &str,
    path: &Path,
    identity: &str,
    view: SavedWorkView,
) -> Result<(), desktop_i18n::workflow::Feedback> {
    let mut prefs = ProjectPrefs::current(app, project);
    if let Some(existing) = prefs.views.iter_mut().find(|existing| existing.id == view.id) {
        *existing = view;
    } else {
        prefs.views.push(view);
    }
    let views = prefs.views.clone();
    save(path, identity, prefs)?;
    app.task_board.saved_work_views = views;
    Ok(())
}

pub(super) fn mark_task_read(app: &mut App, id: &str) -> iced::Task<Message> {
    let Some(project) = app.navigation_tabs.project_path.as_deref() else {
        return iced::Task::none();
    };
    let board = &app.task_board;
    if board.work_prefs_project.as_deref() != Some(project) || !board.work_task_unread(id) {
        return iced::Task::none();
    }
    let Some(dirs) = crate::app::project_dirs() else {
        return iced::Task::none();
    };
    let identity = std::fs::canonicalize(project)
        .unwrap_or_else(|_| project.into())
        .to_string_lossy()
        .into_owned();
    let mut prefs = ProjectPrefs::current(app, project);
    prefs.read_task_ids.insert(id.to_owned());
    match save(&dirs.config_dir().join("work_views.json"), &identity, prefs) {
        Ok(()) => {
            app.task_board.work_read_task_ids.insert(id.to_owned());
            iced::Task::none()
        }
        Err(error) => app.show_message_toast(crate::app::state::ToastKind::Error, error.message()),
    }
}

pub(super) fn load_filter_draft(
    board: &mut desktop_task_board::state::State,
    filter: &WorkViewFilter,
) {
    board.work_status_input = filter.status_keys.join(", ");
    board.work_status_excluded = filter.status_excluded;
    board.work_filter_agent_running = filter.agent_running;
    board.work_unread_only = filter.unread_only;
    board.work_label_ids.clone_from(&filter.label_ids);
    board.work_labels_excluded = filter.labels_excluded;
    board.work_property_values.clone_from(&filter.properties);
    board.work_unset_properties.clone_from(&filter.unset_properties);
    board.work_min_input = filter.priority_min.map(|n| n.to_string()).unwrap_or_default();
    board.work_max_input = filter.priority_max.map(|n| n.to_string()).unwrap_or_default();
    board.work_priority_levels.clone_from(&filter.priority_levels);
    board.work_priority_excluded = filter.priority_excluded;
    board.work_date_field = filter.date.map(|date| date.field).unwrap_or_default();
    board.work_date_range = filter.date.map(|date| date.range);
    board.work_date_calendar = None;
    board.work_scope_selections.clone_from(&filter.scopes);
    board.work_agent_input = filter
        .agents
        .iter()
        .map(|agent| match agent {
            ExecutionAgent::Builtin { id } => format!("builtin:{id}"),
            ExecutionAgent::Acp { id } => format!("acp:{id}"),
        })
        .collect::<Vec<_>>()
        .join(", ");
}

fn select_saved_view(app: &mut App, view: SavedWorkView) {
    let board = &mut app.task_board;
    board.switch_work_view(BuiltinWorkView::AllTasks);
    load_filter_draft(board, &view.filter);
    board.selected_work_view = Some(view.id);
    board.work_view_name = view.name;
    board.work_filter = view.filter;
}

pub(in crate::app) fn filter(app: &App) -> Result<WorkViewFilter, desktop_i18n::workflow::Feedback> {
    let board = &app.task_board;
    let number = |value: &str| -> Result<Option<u32>, desktop_i18n::workflow::Feedback> {
        if value.trim().is_empty() {
            Ok(None)
        } else {
            value.trim().parse().map(Some).map_err(|_| "优先级须为非负整数".into())
        }
    };
    let min = number(&board.work_min_input)?;
    let max = number(&board.work_max_input)?;
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        return Err("最低优先级不能大于最高优先级".into());
    }
    let mut agents = Vec::new();
    for value in board.work_agent_input.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (kind, id) = value.split_once(':').ok_or("智能体格式为 builtin:标识 或 acp:标识")?;
        if id.trim().is_empty() {
            return Err("智能体标识不能为空".into());
        }
        agents.push(match kind {
            "builtin" => ExecutionAgent::Builtin { id: id.trim().into() },
            "acp" => ExecutionAgent::Acp { id: id.trim().into() },
            _ => return Err("智能体类型须为 builtin 或 acp".into()),
        });
    }
    use vw_api_agent_scheduler::AgentSchedulerTaskPropertyValue as Property;
    let mut properties = board.work_property_values.clone();
    for value in properties.values_mut() {
        match value {
            Property::Date(date) => {
                let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .map_err(|_| "日期筛选须填写有效日期（YYYY-MM-DD）")?;
                if date.len() != 10 || date.starts_with("0000") || parsed.to_string() != *date {
                    return Err("日期筛选须填写有效日期（YYYY-MM-DD）".into());
                }
            }
            Property::Number(number) => {
                *number = number.trim().parse::<serde_json::Number>()
                    .map_err(|_| "数字筛选须填写有效数字")?.to_string();
            }
            Property::Member(id) | Property::Link(id) => *id = id.trim().to_owned(),
            Property::MultiMember(items) | Property::LinkList(items) => {
                for item in items {
                    *item = item.trim().to_owned();
                    if item.is_empty() {
                        return Err("列表筛选项不能为空".into());
                    }
                }
            }
            Property::TextList(items) if items.iter().any(|item| item.trim().is_empty()) => {
                return Err("列表筛选项不能为空".into());
            }
            _ => {}
        }
    }
    let status_keys: Vec<_> = board
        .work_status_input
        .split(',')
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(WorkViewFilter {
        status_excluded: !status_keys.is_empty() && board.work_status_excluded,
        status_keys,
        agent_running: board.work_filter_agent_running,
        unread_only: board.work_unread_only,
        priority_min: min,
        priority_max: max,
        priority_levels: WorkPriorityLevel::ALL.into_iter()
            .filter(|level| board.work_priority_levels.contains(level))
            .collect(),
        priority_excluded: !board.work_priority_levels.is_empty() && board.work_priority_excluded,
        date: board.work_date_range.map(|range| WorkDateFilter { field: board.work_date_field, range }),
        scopes: board.work_scope_selections.iter()
            .filter(|(_, selection)| !selection.values.is_empty())
            .map(|(field, selection)| (*field, selection.clone()))
            .collect(),
        agents,
        label_ids: board.work_label_ids.clone(),
        labels_excluded: !board.work_label_ids.is_empty() && board.work_labels_excluded,
        properties,
        unset_properties: board.work_unset_properties.clone(),
        advanced: board.work_filter.advanced.clone(),
    })
}

pub(super) fn update(app: &mut App, message: &TaskBoardMessage) -> Option<iced::Task<Message>> {
    let project = app.navigation_tabs.project_path.clone()?;
    let path = match crate::app::project_dirs() {
        Some(dirs) => dirs.config_dir().join("work_views.json"),
        None => return Some(app.show_localized_toast(crate::app::state::ToastKind::Error, "无法定位工作视图配置目录", vec![])),
    };
    // 文件内以规范路径标识项目；界面标记仍以导航路径索引。
    let identity = std::fs::canonicalize(&project)
        .unwrap_or_else(|_| project.clone().into())
        .to_string_lossy()
        .into_owned();
    if app.task_board.work_prefs_project.as_ref() != Some(&project) {
        app.task_board.switch_work_view(desktop_task_board::work_views::BuiltinWorkView::AllTasks);
        app.task_board.work_editor_open = false;
        app.task_board.saved_work_views.clear();
        app.task_board.hidden_work_sidebar_views.clear();
        app.task_board.work_sidebar_show_counts = true;
        app.task_board.work_sidebar_hovered_view = None;
        app.task_board.work_sidebar_view_expansion.clear();
        app.task_board.work_sidebar_task_limits.clear();
        app.task_board.personal_tasks.remove(&project);
        app.task_board.work_read_task_ids.clear();
        app.task_board.work_snapshot_project_id = None;
        app.task_board.work_project_progress = None;
        app.task_board.work_viewer_id = None;
        app.task_board.work_subscriptions.clear();
        app.task_board.work_prefs_project = Some(project.clone());
        match read(&path) {
            Ok(mut all) => {
                let prefs = all.remove(&identity).unwrap_or(ProjectPrefs {
                    sidebar_show_counts: true,
                    ..Default::default()
                });
                app.task_board.saved_work_views = prefs.views;
                app.task_board.hidden_work_sidebar_views = prefs.hidden_sidebar_views;
                app.task_board.work_sidebar_show_counts = prefs.sidebar_show_counts;
                app.task_board.personal_tasks.insert(project.clone(), prefs.personal_task_ids);
                app.task_board.work_read_task_ids = prefs.read_task_ids;
                app.task_board.work_subscriptions = prefs.subscriptions;
            }
            Err(error) => {
                // 显示读取错误，仍允许当前业务消息继续处理。
                let _ = app.show_message_toast(crate::app::state::ToastKind::Error, error.message());
            }
        }
    }
    let result: Result<(), desktop_i18n::workflow::Feedback> = (|| {
        match message {
            TaskBoardMessage::CreateWorkView | TaskBoardMessage::DuplicateWorkView => {
                let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
                let (name, filter) = if matches!(message, TaskBoardMessage::DuplicateWorkView) {
                    let board = &app.task_board;
                    let view = board.saved_work_views.iter()
                        .find(|view| Some(&view.id) == board.selected_work_view.as_ref())
                        .ok_or("请先打开一个保存视图")?;
                    let name = i18n.format("%s（副本）", &[view.name.as_str().into()])
                        .expect("valid duplicate view title");
                    (name, view.filter.clone())
                } else {
                    (i18n.__("未命名视图").to_owned(), WorkViewFilter::default())
                };
                let view = SavedWorkView { id: unique_id(), name, filter };
                store_view(app, &project, &path, &identity, view.clone())?;
                select_saved_view(app, view);
                app.task_board.work_view_name_editing = true;
                app.task_board.work_editor_open = false;
            }
            TaskBoardMessage::EditWorkViewName | TaskBoardMessage::CancelWorkViewName => {
                let board = &mut app.task_board;
                if let Some(view) = board.saved_work_views.iter()
                    .find(|view| Some(&view.id) == board.selected_work_view.as_ref())
                {
                    board.work_view_name.clone_from(&view.name);
                    board.work_view_name_editing = matches!(message, TaskBoardMessage::EditWorkViewName);
                    board.work_toolbar_menu = None;
                }
            }
            TaskBoardMessage::SaveWorkViewName => {
                let board = &app.task_board;
                if !board.work_view_name_editing {
                    return Ok(());
                }
                let Some(mut view) = board.saved_work_views.iter()
                    .find(|view| Some(&view.id) == board.selected_work_view.as_ref())
                    .cloned()
                else {
                    return Ok(());
                };
                let name = board.work_view_name.trim().to_owned();
                if name.is_empty() {
                    return Err("视图名称不能为空".into());
                }
                if view.name != name {
                    view.name.clone_from(&name);
                    store_view(app, &project, &path, &identity, view)?;
                }
                app.task_board.work_view_name = name;
                app.task_board.work_view_name_editing = false;
            }
            TaskBoardMessage::ShowWorkSidebarCounts(visible) => {
                let mut prefs = ProjectPrefs::current(app, &project);
                prefs.sidebar_show_counts = *visible;
                save(&path, &identity, prefs)?;
                app.task_board.work_sidebar_show_counts = *visible;
            }
            TaskBoardMessage::SetSidebarWorkViewVisible(view, visible) => {
                let mut prefs = ProjectPrefs::current(app, &project);
                prefs.hidden_sidebar_views.retain(|hidden| hidden != view);
                if !*visible {
                    prefs.hidden_sidebar_views.push(view.clone());
                }
                let hidden = prefs.hidden_sidebar_views.clone();
                save(&path, &identity, prefs)?;
                app.task_board.hidden_work_sidebar_views = hidden;
            }
            TaskBoardMessage::OpenWorkViewEditor | TaskBoardMessage::EditWorkView => {
                let board = &mut app.task_board;
                board.work_editor_open =
                    matches!(message, TaskBoardMessage::EditWorkView) || !board.work_editor_open;
                board.work_toolbar_menu = None;
            }
            TaskBoardMessage::WorkViewFieldChanged(field, value) => {
                let target = match field {
                    0 => &mut app.task_board.work_view_name,
                    1 => &mut app.task_board.work_status_input,
                    2 => &mut app.task_board.work_min_input,
                    3 => &mut app.task_board.work_max_input,
                    _ => &mut app.task_board.work_agent_input,
                };
                *target = value.clone();
            }
            TaskBoardMessage::ToggleWorkViewLabel(id) => {
                let board = &mut app.task_board;
                if board.work_label_ids.contains(id) {
                    board.work_label_ids.retain(|selected| selected != id);
                } else if board.task_labels.iter().any(|label| !label.archived && &label.id == id) {
                    board.work_label_ids.push(id.clone());
                }
            }
            TaskBoardMessage::SetWorkFilterLabelsExcluded(excluded) => {
                app.task_board.work_labels_excluded = *excluded;
            }
            TaskBoardMessage::WorkViewPropertyChanged(id, value) => {
                let values = &mut app.task_board.work_property_values;
                if let Some(value) = value {
                    values.insert(id.clone(), value.clone());
                } else {
                    values.remove(id);
                }
            }
            TaskBoardMessage::ToggleWorkFilterUnsetProperty(id) => {
                let unset = &mut app.task_board.work_unset_properties;
                if !unset.remove(id) {
                    unset.insert(id.clone());
                }
            }
            TaskBoardMessage::ApplyWorkFilter
            | TaskBoardMessage::ApplyQuickWorkFilter
            | TaskBoardMessage::ApplyWorkFilterCalendar => {
                if matches!(message, TaskBoardMessage::ApplyWorkFilterCalendar) {
                    let board = &mut app.task_board;
                    let Some(calendar) = board.work_date_calendar else {
                        return Ok(());
                    };
                    let Some(start) = calendar.start else {
                        return Ok(());
                    };
                    board.work_date_range = Some(WorkDateRange::Custom {
                        start,
                        end: calendar.end.unwrap_or(start),
                    });
                    board.work_date_calendar = None;
                    if board.work_toolbar_menu != Some(WorkToolbarMenu::Filter) {
                        return Ok(());
                    }
                }
                let filter = filter(app)?;
                if !matches!(message, TaskBoardMessage::ApplyWorkFilter) {
                    app.task_board.work_toolbar_menu = None;
                }
                app.task_board.task_board_selected_tasks.clear();
                app.task_board.work_filter = filter;
            }
            TaskBoardMessage::SetWorkFilterStatusExcluded(excluded) => {
                app.task_board.work_status_excluded = *excluded;
            }
            TaskBoardMessage::SetWorkFilterAgentRunning(running) => {
                app.task_board.work_filter_agent_running = *running;
            }
            TaskBoardMessage::SetWorkFilterUnread(unread) => {
                app.task_board.work_unread_only = *unread;
            }
            TaskBoardMessage::ToggleWorkFilterPriority(level) => {
                let levels = &mut app.task_board.work_priority_levels;
                if levels.contains(level) {
                    levels.retain(|selected| selected != level);
                } else {
                    levels.push(*level);
                }
            }
            TaskBoardMessage::SetWorkFilterPriorityExcluded(excluded) => {
                app.task_board.work_priority_excluded = *excluded;
            }
            TaskBoardMessage::SetWorkFilterDateField(field) => {
                app.task_board.work_date_field = *field;
            }
            TaskBoardMessage::ToggleWorkScopeValue(field, value) => {
                let selection = app.task_board.work_scope_selections.entry(*field).or_default();
                if !selection.values.remove(value) {
                    selection.values.insert(value.clone());
                }
            }
            TaskBoardMessage::SetWorkScopeExcluded(field, excluded) => {
                app.task_board.work_scope_selections.entry(*field).or_default().excluded = *excluded;
            }
            TaskBoardMessage::SetWorkFilterDateRange(range) => {
                app.task_board.work_date_range = *range;
                app.task_board.work_date_calendar = None;
            }
            TaskBoardMessage::ShowWorkFilterCalendar(open) => {
                let board = &mut app.task_board;
                board.work_date_calendar = open.then(|| {
                    let (start, end) = match board.work_date_range {
                        Some(WorkDateRange::Custom { start, end }) => (Some(start), Some(end)),
                        _ => (None, None),
                    };
                    let date = start.unwrap_or_else(|| Local::now().date_naive());
                    WorkDateCalendar { month: date.with_day(1).unwrap_or(date), start, end }
                });
            }
            TaskBoardMessage::ShiftWorkFilterCalendar(offset) => {
                if let Some(calendar) = app.task_board.work_date_calendar.as_mut() {
                    let months = Months::new(offset.unsigned_abs());
                    let next = if *offset < 0 {
                        calendar.month.checked_sub_months(months)
                    } else {
                        calendar.month.checked_add_months(months)
                    };
                    if let Some(next) = next.filter(|date| (1..=9999).contains(&date.year())) {
                        calendar.month = next;
                    }
                }
            }
            TaskBoardMessage::PickWorkFilterDate(date) => {
                if let Some(calendar) = app.task_board.work_date_calendar.as_mut() {
                    if let (Some(start), None) = (calendar.start, calendar.end) {
                        calendar.start = Some(start.min(*date));
                        calendar.end = Some(start.max(*date));
                    } else {
                        calendar.start = Some(*date);
                        calendar.end = None;
                    }
                }
            }
            TaskBoardMessage::RemoveWorkFilter(field) => {
                let board = &mut app.task_board;
                let mut applied = board.work_filter.clone();
                applied.remove(field);
                load_filter_draft(board, &applied);
                board.work_filter = applied;
                board.work_toolbar_menu = None;
                board.task_board_selected_tasks.clear();
            }
            TaskBoardMessage::SaveAppliedWorkFilter => {
                let board = &app.task_board;
                let mut view = board.saved_work_views.iter()
                    .find(|view| Some(&view.id) == board.selected_work_view.as_ref())
                    .cloned()
                    .ok_or("请先打开一个保存视图")?;
                view.filter.clone_from(&board.work_filter);
                store_view(app, &project, &path, &identity, view)?;
            }
            TaskBoardMessage::ClearWorkFilter => {
                let board = &mut app.task_board;
                board.work_advanced_filter = None;
                let filter = WorkViewFilter::default();
                load_filter_draft(board, &filter);
                board.work_filter = filter;
                board.work_toolbar_menu = None;
                board.task_board_selected_tasks.clear();
            }
            TaskBoardMessage::OpenSavedWorkView(id) => {
                if let Some(view) =
                    app.task_board.saved_work_views.iter().find(|v| &v.id == id).cloned()
                {
                    select_saved_view(app, view);
                }
            }
            TaskBoardMessage::SaveWorkView(copy) => {
                let name = app.task_board.work_view_name.trim().to_owned();
                if name.is_empty() {
                    return Err("视图名称不能为空".into());
                }
                let filter = filter(app)?;
                let id = if *copy {
                    unique_id()
                } else {
                    app.task_board.selected_work_view.clone().ok_or("请先打开一个保存视图")?
                };
                let view =
                    SavedWorkView { id: id.clone(), name: name.clone(), filter: filter.clone() };
                store_view(app, &project, &path, &identity, view)?;
                let board = &mut app.task_board;
                board.task_board_selected_tasks.clear();
                board.work_view = desktop_task_board::work_views::BuiltinWorkView::AllTasks;
                board.selected_work_view = Some(id);
                board.work_view_name = name;
                board.work_view_name_editing = false;
                board.work_filter = filter;
            }
            TaskBoardMessage::DeleteWorkView => {
                let id =
                    app.task_board.selected_work_view.as_ref().ok_or("请先打开一个保存视图")?;
                let mut prefs = ProjectPrefs::current(app, &project);
                prefs.views.retain(|view| &view.id != id);
                prefs.hidden_sidebar_views.retain(
                    |hidden| !matches!(hidden, SidebarWorkView::Saved(hidden_id) if hidden_id == id),
                );
                let views = prefs.views.clone();
                let hidden = prefs.hidden_sidebar_views.clone();
                save(&path, &identity, prefs)?;
                app.task_board.saved_work_views = views;
                app.task_board.hidden_work_sidebar_views = hidden;
                app.task_board
                    .switch_work_view(desktop_task_board::work_views::BuiltinWorkView::AllTasks);
            }
            TaskBoardMessage::TogglePersonalTask(id) => {
                let mut prefs = ProjectPrefs::current(app, &project);
                let ids = &mut prefs.personal_task_ids;
                if ids.contains(id) {
                    ids.retain(|item| item != id);
                } else {
                    ids.push(id.clone());
                }
                let personal_task_ids = ids.clone();
                save(&path, &identity, prefs)?;
                app.task_board.personal_tasks.insert(project, personal_task_ids);
                app.task_board.task_board_selected_tasks.remove(id);
            }
            TaskBoardMessage::ToggleWorkSubscription(id) => {
                let board = &app.task_board;
                let Some(viewer) = board.work_viewer_id.as_ref() else {
                    return Ok(());
                };
                if !board.task_board_tasks.iter().any(|task| &task.id == id && !task.deleted) {
                    return Ok(());
                }
                let mut prefs = ProjectPrefs::current(app, &project);
                let ids = prefs.subscriptions.entry(viewer.clone()).or_default();
                if !ids.remove(id) {
                    ids.insert(id.clone());
                }
                if ids.is_empty() {
                    prefs.subscriptions.remove(viewer);
                }
                let subscriptions = prefs.subscriptions.clone();
                save(&path, &identity, prefs)?;
                app.task_board.work_subscriptions = subscriptions;
                app.task_board.work_sidebar_menu = None;
            }
            _ => return Ok(()),
        }
        Ok(())
    })();
    let handled = matches!(
        message,
        TaskBoardMessage::OpenWorkViewEditor
            | TaskBoardMessage::EditWorkView
            | TaskBoardMessage::CreateWorkView
            | TaskBoardMessage::DuplicateWorkView
            | TaskBoardMessage::EditWorkViewName
            | TaskBoardMessage::SaveWorkViewName
            | TaskBoardMessage::CancelWorkViewName
            | TaskBoardMessage::ShowWorkSidebarCounts(_)
            | TaskBoardMessage::SetSidebarWorkViewVisible(..)
            | TaskBoardMessage::WorkViewFieldChanged(..)
            | TaskBoardMessage::ToggleWorkViewLabel(_)
            | TaskBoardMessage::SetWorkFilterLabelsExcluded(_)
            | TaskBoardMessage::WorkViewPropertyChanged(..)
            | TaskBoardMessage::ToggleWorkFilterUnsetProperty(_)
            | TaskBoardMessage::ApplyWorkFilter
            | TaskBoardMessage::ApplyQuickWorkFilter
            | TaskBoardMessage::SetWorkFilterStatusExcluded(_)
            | TaskBoardMessage::SetWorkFilterAgentRunning(_)
            | TaskBoardMessage::SetWorkFilterUnread(_)
            | TaskBoardMessage::ToggleWorkFilterPriority(_)
            | TaskBoardMessage::SetWorkFilterPriorityExcluded(_)
            | TaskBoardMessage::SetWorkFilterDateField(_)
            | TaskBoardMessage::SetWorkFilterDateRange(_)
            | TaskBoardMessage::ShowWorkFilterCalendar(_)
            | TaskBoardMessage::ShiftWorkFilterCalendar(_)
            | TaskBoardMessage::PickWorkFilterDate(_)
            | TaskBoardMessage::ApplyWorkFilterCalendar
            | TaskBoardMessage::ToggleWorkScopeValue(..)
            | TaskBoardMessage::SetWorkScopeExcluded(..)
            | TaskBoardMessage::ToggleWorkSubscription(_)
            | TaskBoardMessage::RemoveWorkFilter(_)
            | TaskBoardMessage::SaveAppliedWorkFilter
            | TaskBoardMessage::ClearWorkFilter
            | TaskBoardMessage::OpenSavedWorkView(_)
            | TaskBoardMessage::SaveWorkView(_)
            | TaskBoardMessage::DeleteWorkView
            | TaskBoardMessage::TogglePersonalTask(_)
    );
    if let Err(error) = result {
        return Some(app.show_message_toast(crate::app::state::ToastKind::Error, error.message()));
    }
    if matches!(
        message,
        TaskBoardMessage::CreateWorkView
            | TaskBoardMessage::DuplicateWorkView
            | TaskBoardMessage::EditWorkViewName
    )
        && app.task_board.work_view_name_editing
    {
        use crate::app::views::task_board::WORK_VIEW_NAME_INPUT_ID;
        use iced::widget::operation::{focus, select_all};
        return Some(focus(WORK_VIEW_NAME_INPUT_ID).chain(select_all(WORK_VIEW_NAME_INPUT_ID)));
    }
    handled.then(iced::Task::none)
}
