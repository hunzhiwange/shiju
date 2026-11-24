//! 本机个人工作偏好；同步小文件事务避免跨项目异步结果串写。
use super::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_task_board::work_views::{ExecutionAgent, SavedWorkView, WorkViewFilter};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectPrefs {
    views: Vec<SavedWorkView>,
    personal_task_ids: Vec<String>,
}
type Preferences = BTreeMap<String, ProjectPrefs>;

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

fn filter(app: &App) -> Result<WorkViewFilter, desktop_i18n::workflow::Feedback> {
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
    for value in board.work_property_values.values() {
        if let vw_api_agent_scheduler::AgentSchedulerTaskPropertyValue::Date(date) = value {
            let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| "日期筛选须填写有效日期（YYYY-MM-DD）")?;
            if date.len() != 10 || date.starts_with("0000") || parsed.to_string() != *date {
                return Err("日期筛选须填写有效日期（YYYY-MM-DD）".into());
            }
        }
    }
    Ok(WorkViewFilter {
        status_keys: board
            .work_status_input
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        priority_min: min,
        priority_max: max,
        agents,
        label_ids: board.work_label_ids.clone(),
        properties: board.work_property_values.clone(),
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
        app.task_board.personal_tasks.remove(&project);
        app.task_board.work_prefs_project = Some(project.clone());
        match read(&path) {
            Ok(mut all) => {
                let prefs = all.remove(&identity).unwrap_or_default();
                app.task_board.saved_work_views = prefs.views;
                app.task_board.personal_tasks.insert(project.clone(), prefs.personal_task_ids);
            }
            Err(error) => {
                // 显示读取错误，仍允许当前业务消息继续处理。
                let _ = app.show_message_toast(crate::app::state::ToastKind::Error, error.message());
            }
        }
    }
    let result: Result<(), desktop_i18n::workflow::Feedback> = (|| {
        match message {
            TaskBoardMessage::OpenWorkViewEditor => {
                app.task_board.work_editor_open = !app.task_board.work_editor_open;
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
            TaskBoardMessage::WorkViewPropertyChanged(id, value) => {
                let values = &mut app.task_board.work_property_values;
                if let Some(value) = value {
                    values.insert(id.clone(), value.clone());
                } else {
                    values.remove(id);
                }
            }
            TaskBoardMessage::ApplyWorkFilter => {
                let filter = filter(app)?;
                app.task_board.task_board_selected_tasks.clear();
                app.task_board.work_filter = filter;
            }
            TaskBoardMessage::ClearWorkFilter => {
                let view = app.task_board.work_view;
                app.task_board.switch_work_view(view);
            }
            TaskBoardMessage::OpenSavedWorkView(id) => {
                if let Some(view) =
                    app.task_board.saved_work_views.iter().find(|v| &v.id == id).cloned()
                {
                    app.task_board.switch_work_view(
                        desktop_task_board::work_views::BuiltinWorkView::AllTasks,
                    );
                    let board = &mut app.task_board;
                    board.work_status_input = view.filter.status_keys.join(", ");
                    board.work_label_ids.clone_from(&view.filter.label_ids);
                    board.work_property_values.clone_from(&view.filter.properties);
                    board.work_min_input =
                        view.filter.priority_min.map(|n| n.to_string()).unwrap_or_default();
                    board.work_max_input =
                        view.filter.priority_max.map(|n| n.to_string()).unwrap_or_default();
                    board.work_agent_input = view
                        .filter
                        .agents
                        .iter()
                        .map(|a| match a {
                            ExecutionAgent::Builtin { id } => format!("builtin:{id}"),
                            ExecutionAgent::Acp { id } => format!("acp:{id}"),
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    board.selected_work_view = Some(view.id);
                    board.work_view_name = view.name;
                    board.work_filter = view.filter;
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
                let mut views = app.task_board.saved_work_views.clone();
                views.retain(|v| v.id != id);
                views.push(view);
                save(
                    &path,
                    &identity,
                    ProjectPrefs {
                        views: views.clone(),
                        personal_task_ids: app
                            .task_board
                            .personal_tasks
                            .get(&project)
                            .cloned()
                            .unwrap_or_default(),
                    },
                )?;
                let board = &mut app.task_board;
                board.task_board_selected_tasks.clear();
                board.work_view = desktop_task_board::work_views::BuiltinWorkView::AllTasks;
                board.saved_work_views = views;
                board.selected_work_view = Some(id);
                board.work_view_name = name;
                board.work_filter = filter;
            }
            TaskBoardMessage::DeleteWorkView => {
                let id =
                    app.task_board.selected_work_view.as_ref().ok_or("请先打开一个保存视图")?;
                let mut views = app.task_board.saved_work_views.clone();
                views.retain(|v| &v.id != id);
                save(
                    &path,
                    &identity,
                    ProjectPrefs {
                        views: views.clone(),
                        personal_task_ids: app
                            .task_board
                            .personal_tasks
                            .get(&project)
                            .cloned()
                            .unwrap_or_default(),
                    },
                )?;
                app.task_board.saved_work_views = views;
                app.task_board
                    .switch_work_view(desktop_task_board::work_views::BuiltinWorkView::AllTasks);
            }
            TaskBoardMessage::TogglePersonalTask(id) => {
                let mut ids =
                    app.task_board.personal_tasks.get(&project).cloned().unwrap_or_default();
                if ids.contains(id) {
                    ids.retain(|item| item != id);
                } else {
                    ids.push(id.clone());
                }
                save(
                    &path,
                    &identity,
                    ProjectPrefs {
                        views: app.task_board.saved_work_views.clone(),
                        personal_task_ids: ids.clone(),
                    },
                )?;
                app.task_board.personal_tasks.insert(project, ids);
                app.task_board.task_board_selected_tasks.remove(id);
            }
            _ => return Ok(()),
        }
        Ok(())
    })();
    let handled = matches!(
        message,
        TaskBoardMessage::OpenWorkViewEditor
            | TaskBoardMessage::WorkViewFieldChanged(..)
            | TaskBoardMessage::ToggleWorkViewLabel(_)
            | TaskBoardMessage::WorkViewPropertyChanged(..)
            | TaskBoardMessage::ApplyWorkFilter
            | TaskBoardMessage::ClearWorkFilter
            | TaskBoardMessage::OpenSavedWorkView(_)
            | TaskBoardMessage::SaveWorkView(_)
            | TaskBoardMessage::DeleteWorkView
            | TaskBoardMessage::TogglePersonalTask(_)
    );
    if let Err(error) = result {
        return Some(app.show_message_toast(crate::app::state::ToastKind::Error, error.message()));
    }
    handled.then(iced::Task::none)
}
