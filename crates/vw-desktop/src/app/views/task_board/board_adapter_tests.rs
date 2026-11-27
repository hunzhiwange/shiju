// Task 20 ownership: Retain host BoardSlots injection: agent/model/executor selectors are assembled by the App adapter.
//! Host composition coverage for selectors injected into the domain board.
use crate::app::App;
use crate::app::task::{Task, TaskStatus};
#[test]
fn board_composes_host_bulk_selectors() {
    let (mut app, _) = App::new();
    for status in [TaskStatus::PrSubmitted] {
        let mut task = Task::new(20);
        task.status = status;
        app.task_board.task_board_selected_tasks.insert(task.id.clone());
        app.task_board.task_board_tasks = vec![task];
        app.task_board.task_board_bulk_active_status = Some(status);
        app.task_board.task_board_bulk_priority_input = "42".into();
        app.task_board.task_board_bulk_model_input = "openai/gpt-4.1".into();
        app.task_board.task_board_bulk_agent = "agent-specialist".into();
        app.task_board.task_board_bulk_acp_agent = Some("codex".into());
        let _ = super::build_kanban_board(&app);
    }
}

use crate::app::message::TaskBoardMessage;
use desktop_task_board::work_views::{BuiltinWorkView, SidebarWorkView};
fn dispatch(app: &mut App, message: TaskBoardMessage) {
    let _ = crate::app::message::task_board::update(app, message);
}
#[test]
fn work_sidebar_personal_marks_are_project_scoped() {
    let (mut app, _) = App::new();
    app.navigation_tabs.project_path = Some("/project-a".into());
    let task = Task::new(20);
    app.task_board.task_board_tasks = vec![task.clone()];
    dispatch(&mut app, TaskBoardMessage::TogglePersonalTask(task.id.clone()));
    dispatch(&mut app, TaskBoardMessage::SelectWorkView(BuiltinWorkView::MyTasks));
    assert!(app.task_board.work_task_visible("/project-a", &task, false));
    assert!(!app.task_board.work_task_visible("/project-b", &task, false));
    dispatch(&mut app, TaskBoardMessage::TogglePersonalTask(task.id.clone()));
    assert!(!app.task_board.work_task_visible("/project-a", &task, false));
}
#[test]
fn work_sidebar_switch_clears_transient_selection() {
    let (mut app, _) = App::new();
    app.task_board.task_board_selected_tasks.insert("hidden".into());
    app.task_board.task_board_dragging = Some(("hidden".into(), TaskStatus::Pending));
    dispatch(&mut app, TaskBoardMessage::SelectWorkView(BuiltinWorkView::MyTasks));
    assert!(app.task_board.task_board_selected_tasks.is_empty());
    assert!(app.task_board.task_board_dragging.is_none());
}

#[test]
fn work_sidebar_subscribed_view_matches_the_current_viewer_and_expands_independently() {
    let (mut app, _) = App::new();
    let task = Task::new(20);
    let board = &mut app.task_board;
    board.work_viewer_id = Some("viewer-a".into());
    board.work_subscriptions.insert("viewer-a".into(), [task.id.clone()].into());
    let view = SidebarWorkView::Builtin(BuiltinWorkView::Subscribed);
    dispatch(&mut app, TaskBoardMessage::ToggleWorkSidebarView(view.clone()));
    assert!(app.task_board.work_sidebar_view_expanded(&view));
    assert_eq!(app.task_board.work_view, BuiltinWorkView::AllTasks);
    dispatch(&mut app, TaskBoardMessage::SelectWorkView(BuiltinWorkView::Subscribed));
    assert!(app.task_board.work_task_visible("/project-a", &task, false));
    app.task_board.work_viewer_id = Some("viewer-b".into());
    assert!(!app.task_board.work_task_visible("/project-a", &task, false));
}
#[test]
fn work_sidebar_select_all_excludes_hidden_tasks() {
    let (mut app, _) = App::new();
    app.navigation_tabs.project_path = Some("/project-a".into());
    let visible = Task::new(20);
    let mut hidden = visible.clone();
    hidden.id = "hidden".into();
    app.task_board.task_board_tasks = vec![visible.clone(), hidden.clone()];
    dispatch(&mut app, TaskBoardMessage::TogglePersonalTask(visible.id.clone()));
    dispatch(&mut app, TaskBoardMessage::SelectWorkView(BuiltinWorkView::MyTasks));
    dispatch(&mut app, TaskBoardMessage::SelectAllTasksInStatus(visible.status));
    assert!(app.task_board.task_board_selected_tasks.contains(&visible.id));
    assert!(!app.task_board.task_board_selected_tasks.contains(&hidden.id));
}
#[test]
fn work_sidebar_project_message_opens_work_and_clears_snapshot() {
    let (mut app, _) = App::new();
    let path = std::env::current_dir().unwrap().to_string_lossy().into_owned();
    app.navigation_tabs.project_path = Some("/previous-project".into());
    app.task_board.task_board_tasks = vec![Task::new(20)];
    dispatch(&mut app, TaskBoardMessage::OpenWorkProject(path.clone()));
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some(path.as_str()));
    assert!(app.task_board.show_task_board);
    assert_eq!(app.navigation_tabs.screen, crate::app::Screen::TaskBoard);
    assert!(app.task_board.task_board_tasks.is_empty());
    dispatch(&mut app, TaskBoardMessage::OpenWorkProject("/missing-work-sidebar-project".into()));
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some(path.as_str()));
}

#[test]
fn task_pool_reuses_its_main_tab_and_restores_its_project() {
    use crate::app::message::{ViewMessage, task_board};
    use crate::app::{Message, Screen};

    let (mut app, _) = App::new();
    let path = std::env::current_dir().unwrap().to_string_lossy().into_owned();
    dispatch(&mut app, TaskBoardMessage::OpenWorkProject(path.clone()));
    dispatch(&mut app, TaskBoardMessage::ToggleBoard);
    let id = task_board::TAB_ID;
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some(id));
    assert_eq!(app.navigation_tabs.open_tabs.iter().filter(|tab| tab.id == id).count(), 1);

    let (project_tab_id, _) = desktop_workspace::projects::project_tab(&path);
    let _ = app.update(Message::View(ViewMessage::TabSelected(project_tab_id)));
    assert_eq!(app.navigation_tabs.screen, Screen::Project);
    assert!(!app.task_board.show_task_board);
    assert!(app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == id));

    let _ = app.update(Message::View(ViewMessage::GoHome));
    assert!(app.navigation_tabs.project_path.is_none());
    let _ = app.update(Message::View(ViewMessage::TabSelected(id.into())));
    assert_eq!(app.navigation_tabs.screen, Screen::TaskBoard);
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some(path.as_str()));

    let _ = app.update(Message::View(ViewMessage::TabClosed(id.into())));
    assert_ne!(app.navigation_tabs.screen, Screen::TaskBoard);
    assert!(!app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == id));
}
