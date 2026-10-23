//! 工作模式侧栏的测试，覆盖空间分组、任务状态、分页和上下文菜单。
//! fixture 使用真实项目与父子会话，验证长标题和运行态投影等边界。
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, Theme};
use vw_shared::session::info::{Info, TimeInfo};

use crate::app::assets::Icon;
use crate::app::message::{ProjectMessage, ShijuAppsMessage, SkillHubMessage, ViewMessage};
use crate::app::state::{
    WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime, WorkSidebarTab,
};
use crate::app::{App, Message};

use super::work_sidebar;

const VIEWPORT: Size = Size::new(320.0, 800.0);

#[test]
fn set_thread_pinned_round_trip_and_project_order() {
    let temp = tempfile::tempdir().unwrap();
    let project = "project";
    vw_shared::session::ui_store::set_thread_pinned(temp.path(), project, "older", true).unwrap();
    let pinned = vw_shared::session::ui_store::load_pinned_threads(temp.path(), project).unwrap();
    assert!(pinned.contains("older"));
    let older = session("older", "Older", "/workspace/project", 10);
    let newer = session("newer", "Newer", "/workspace/project", 20);
    let pinned_keys = pinned.iter().map(|id| (project.to_string(), id.clone())).collect();
    assert!(
        work_sidebar::compare_sessions(
            &("/workspace/project", &older),
            &("/workspace/project", &newer),
            &pinned_keys
        )
        .is_lt()
    );
    let mut other_project = older.clone();
    other_project.project_id = "other".into();
    assert!(
        work_sidebar::compare_sessions(
            &("/workspace/project", &other_project),
            &("/workspace/project", &newer),
            &pinned_keys
        )
        .is_gt()
    );
    vw_shared::session::ui_store::set_thread_pinned(temp.path(), project, "older", false).unwrap();
    let unpinned = vw_shared::session::ui_store::load_pinned_threads(temp.path(), project).unwrap();
    assert!(!unpinned.contains("older"));
    assert!(
        work_sidebar::compare_sessions(
            &("/workspace/project", &older),
            &("/workspace/project", &newer),
            &Default::default()
        )
        .is_gt()
    );
}

fn session(id: &str, title: &str, directory: &str, updated: u64) -> Info {
    Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}

fn child_session(id: &str, parent_id: &str, title: &str, directory: &str, updated: u64) -> Info {
    let mut session = session(id, title, directory, updated);
    session.parent_id = Some(parent_id.to_string());
    session
}

#[test]
fn work_sidebar_hides_archived_task_and_shows_restored_task() {
    let mut app = App::new().0;
    let path = "/workspace/archive";
    app.recent_projects = vec![path.into()];
    app.project_path = Some(path.into());
    let mut task = child_session("task", "root", "Task", path, 20);
    task.slug = Info::visible_task_slug(&task.slug);
    task.time.archived = Some(30);
    app.project_sessions.insert(path.into(), vec![session("root", "Root", path, 10), task]);
    assert_eq!(
        work_sidebar::model(&app).tasks.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["root"]
    );
    app.project_sessions.get_mut(path).expect("project")[1].time.archived = None;
    assert_eq!(
        work_sidebar::model(&app).tasks.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["task", "root"]
    );
}

#[test]
fn work_sidebar_shows_visible_task_in_own_project() {
    let mut app = App::new().0;
    let alpha = "/workspace/alpha";
    let beta = "/workspace/beta";
    app.recent_projects = vec![alpha.into(), beta.into()];
    app.project_path = Some(alpha.into());
    app.work_expanded_spaces.insert(alpha.into());
    app.work_expanded_spaces.insert(beta.into());

    let mut visible = child_session("visible", "alpha-root", "Visible", alpha, 20);
    visible.slug = Info::visible_task_slug(&visible.slug);
    let mut wrong_project = child_session("wrong-project", "alpha-root", "Wrong", beta, 30);
    wrong_project.slug = Info::visible_task_slug(&wrong_project.slug);
    let mut wrong_project_id =
        child_session("wrong-project-id", "alpha-root", "Wrong ID", alpha, 35);
    wrong_project_id.slug = Info::visible_task_slug(&wrong_project_id.slug);
    wrong_project_id.project_id = "other-project".into();
    let mut plan_run = child_session("plan-run", "alpha-root", "Plan", alpha, 40);
    plan_run.slug = "plan-run-child".into();
    app.project_sessions.insert(
        alpha.into(),
        vec![
            session("alpha-root", "Root", alpha, 10),
            visible.clone(),
            child_session("subagent", "alpha-root", "Internal", alpha, 50),
            plan_run,
            wrong_project,
            wrong_project_id,
        ],
    );
    app.project_sessions.insert(beta.into(), vec![session("beta-root", "Beta", beta, 5)]);

    for _ in 0..2 {
        let model = work_sidebar::model(&app);
        assert_eq!(
            model.tasks.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
            ["visible", "alpha-root", "beta-root"]
        );
        assert_eq!(
            model.spaces[0].sessions.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
            ["visible", "alpha-root"]
        );
        assert_eq!(
            model.spaces[1].sessions.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
            ["beta-root"]
        );
        app.project_sessions.get_mut(alpha).unwrap().retain(|item| item.id != "visible");
        app.project_sessions.get_mut(alpha).unwrap().push(visible.clone());
    }
}

#[test]
fn work_sidebar_builds_workbuddy_sections_from_real_projects() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/workspace/alpha".into(), "/workspace/beta".into()];
    app.recent_projects_edits = vec!["阿尔法空间".into(), "Beta".into()];
    app.project_sessions.insert(
        "/workspace/alpha".into(),
        vec![
            session("older", "较早任务", "/workspace/alpha", 10),
            child_session("newest-child", "newest", "不应显示的子任务", "/workspace/alpha", 50),
            session("newest", "最新任务", "/workspace/alpha", 40),
        ],
    );
    app.project_sessions.insert(
        "/workspace/beta".into(),
        vec![session("middle", "Middle task", "/workspace/beta", 20)],
    );
    app.project_path = Some("/workspace/alpha".into());
    app.work_expanded_spaces.insert("/workspace/alpha".into());
    app.hovered_recent_project = Some("/workspace/alpha".into());
    app.project_tools_menu_path = Some("/workspace/alpha".into());

    let model = work_sidebar::model(&app);

    assert_eq!(
        model
            .tasks
            .iter()
            .map(|task| match &task.message {
                Message::Project(ProjectMessage::OpenProjectSessionPressed(_, id)) => id.as_str(),
                _ => panic!("task should use the existing project session open message"),
            })
            .collect::<Vec<_>>(),
        ["newest", "middle", "older"]
    );
    assert_eq!(
        model.tasks.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        ["newest", "middle", "older"],
        "top-level tasks should retain their real session IDs"
    );
    assert_eq!(
        model.spaces.iter().map(|space| space.label.as_str()).collect::<Vec<_>>(),
        ["阿尔法空间", "Beta"]
    );
    assert!(model.spaces[0].selected);
    assert!(!model.spaces[1].selected);
    assert!(model.spaces[0].expanded);
    assert_eq!(model.spaces[0].sessions.len(), 2);
    assert_eq!(
        model.spaces[0].sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        ["newest", "older"],
        "expanded-space tasks should retain their real session IDs"
    );
    assert!(model.spaces[0].hovered);
    assert!(model.spaces[0].tools_open);
    assert!(!model.spaces[0].creating);
    assert!(!model.spaces[1].expanded);
    assert!(model.spaces[1].sessions.is_empty());
    assert!(!model.spaces[1].hovered);
    assert!(!model.spaces[1].tools_open);
    assert_eq!(model.tasks[0].updated, 40);

    let Message::Project(ProjectMessage::SwitchWorkSpacePressed(path)) =
        model.spaces[0].message.clone()
    else {
        panic!("space should use the work-mode project switch message");
    };
    assert_eq!(path, "/workspace/alpha");

    let Message::View(ViewMessage::WorkSpaceToggled(path)) = model.spaces[0].toggle_message.clone()
    else {
        panic!("space disclosure should preserve the existing expansion message");
    };
    assert_eq!(path, "/workspace/alpha");

    let Message::Project(ProjectMessage::OpenProjectSessionPressed(directory, id)) =
        model.spaces[0].sessions[0].message.clone()
    else {
        panic!("expanded space task should use the existing project session open message");
    };
    assert_eq!(directory, "/workspace/alpha");
    assert_eq!(id, "newest");

    let Message::Project(ProjectMessage::ProjectCreateSession(path)) =
        model.spaces[0].create_message.clone()
    else {
        panic!("space add action should create a task in that space");
    };
    assert_eq!(path, "/workspace/alpha");

    let Message::Project(ProjectMessage::ProjectToolsMenuToggled(path)) =
        model.spaces[0].tools_message.clone()
    else {
        panic!("space tools action should toggle that space menu");
    };
    assert_eq!(path, "/workspace/alpha");
}

#[test]
fn work_sidebar_marks_space_creation_action_disabled_while_session_is_creating() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/workspace/alpha".into()];
    app.new_session_creating = true;

    let model = work_sidebar::model(&app);

    assert!(model.spaces[0].creating);
}

#[test]
fn work_sidebar_projects_runtime_status_for_main_tasks_only() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/workspace/alpha".into()];
    app.project_sessions.insert(
        "/workspace/alpha".into(),
        vec![
            session("idle", "空闲任务", "/workspace/alpha", 10),
            session("running", "运行任务", "/workspace/alpha", 30),
            child_session("running-child", "running", "运行中的子任务", "/workspace/alpha", 40),
        ],
    );
    app.work_expanded_spaces.insert("/workspace/alpha".into());
    app.get_session_runtime_mut("running").is_requesting = true;
    app.get_session_runtime_mut("running-child").is_requesting = true;

    let model = work_sidebar::model(&app);

    assert_eq!(
        model.tasks.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        ["running", "idle"]
    );
    assert_eq!(model.tasks[0].status, work_sidebar::WorkTaskStatus::Running);
    assert_eq!(model.tasks[1].status, work_sidebar::WorkTaskStatus::Idle);
    assert_eq!(
        model.spaces[0].sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        ["running", "idle"]
    );
    assert_eq!(model.spaces[0].sessions[0].status, work_sidebar::WorkTaskStatus::Running);
}

#[test]
fn work_sidebar_truncates_long_task_titles_without_using_time_space() {
    let long_title = "查询DHB企业证件审核任务并整理全部结果";
    let full_width = work_sidebar::truncate_work_task_text(long_title, 280.0, false);
    let with_time = work_sidebar::truncate_work_task_text(long_title, 280.0, true);

    assert!(full_width.ends_with('…'));
    assert!(with_time.ends_with('…'));
    assert!(with_time.chars().count() < full_width.chars().count());
    assert_eq!(work_sidebar::truncate_work_task_text("短任务", 280.0, true), "短任务");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_task_context_menu_routes_session_id_and_pointer_position() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = App::new().0;
    let content = iced::widget::container(iced::widget::text("中文任务")).into();
    let mut task = work_sidebar::task_session_context_menu(&app, "session-42".into(), content);
    let messages = right_click(&mut task, &renderer, Point::new(19.0, 13.0));

    assert!(matches!(
        messages.as_slice(),
        [Message::Project(ProjectMessage::SessionRightClicked(id, x, y))]
            if id == "session-42" && *x == 19.0 && *y == 13.0
    ));
}

#[test]
fn work_sidebar_paginates_tasks_and_spaces_from_ten_in_steps_of_five() {
    let mut app = App::new().0;
    app.recent_projects.clear();
    app.recent_projects_edits.clear();
    app.project_sessions.clear();
    for index in 0..13 {
        let path = format!("/workspace/project-{index:02}");
        app.recent_projects.push(path.clone());
        app.project_sessions.insert(
            path.clone(),
            vec![session(
                &format!("session-{index:02}"),
                &format!("任务 {index:02}"),
                &path,
                index as u64,
            )],
        );
    }

    let initial = work_sidebar::model(&app);
    assert_eq!(initial.tasks.len(), 10);
    assert_eq!(initial.task_total, 13);
    assert_eq!(initial.spaces.len(), 10);
    assert_eq!(initial.space_total, 13);

    app.work_sidebar_visible_tasks += 5;
    app.work_sidebar_visible_spaces += 5;
    let loaded = work_sidebar::model(&app);
    assert_eq!(loaded.tasks.len(), 13);
    assert_eq!(loaded.spaces.len(), 13);
}

#[test]
fn work_sidebar_paginates_expanded_space_tasks_from_twenty() {
    let mut app = App::new().0;
    app.recent_projects.clear();
    app.recent_projects_edits.clear();
    app.project_sessions.clear();
    let path = "/workspace/alpha".to_string();
    app.recent_projects.push(path.clone());
    app.work_expanded_spaces.insert(path.clone());
    app.project_sessions.insert(
        path.clone(),
        (0..25)
            .map(|index| {
                session(
                    &format!("session-{index:02}"),
                    &format!("任务 {index:02}"),
                    &path,
                    (25 - index) as u64,
                )
            })
            .collect(),
    );

    let initial = work_sidebar::model(&app);
    assert_eq!(initial.spaces.len(), 1);
    assert_eq!(initial.spaces[0].session_total, 25);
    assert_eq!(initial.spaces[0].sessions.len(), 20);
    assert_eq!(initial.spaces[0].sessions[0].id, "session-00");
    assert_eq!(initial.spaces[0].sessions[19].id, "session-19");

    app.work_sidebar_space_task_visible.insert(path, 40);
    let loaded = work_sidebar::model(&app);
    assert_eq!(loaded.spaces[0].sessions.len(), 25);
    assert_eq!(loaded.spaces[0].session_total, 25);
}

#[test]
fn work_sidebar_navigation_matches_reference_grouping_and_icons() {
    assert_eq!(
        work_sidebar::navigation_specs(),
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
    );
    assert_eq!(
        work_sidebar::header_icon_specs(false),
        [
            (Icon::LayoutSidebar, "收起侧边栏"),
            (Icon::ArrowRepeat, "刷新全部任务"),
            (Icon::Grid1x2, "任务池"),
            (Icon::Search, "搜索"),
            (Icon::Funnel, "过滤"),
        ]
    );
    assert_eq!(
        work_sidebar::header_icon_specs(true)[0],
        (Icon::LayoutSidebarReverse, "展开侧边栏")
    );
    assert_eq!(
        work_sidebar::more_menu_item_specs(),
        [
            ("添加网址", Icon::Link45Deg),
            ("知识库", Icon::Journals),
            ("工作流", Icon::GitBranch),
            ("应用中心", Icon::Grid1x2),
        ]
    );
    assert!(matches!(
        work_sidebar::project_navigation_message(),
        Message::View(ViewMessage::OpenSystemSettingsTab(
            crate::app::components::system_settings::SystemTab::Projects
        ))
    ));
    assert!(matches!(
        work_sidebar::project_create_message(),
        Message::View(ViewMessage::ProjectFileNewProject)
    ));
}

#[test]
fn work_sidebar_surface_is_pale_in_light_theme_and_dark_in_dark_theme() {
    let surface = |theme: &Theme| match work_sidebar::sidebar_style(theme).background {
        Some(iced::Background::Color(color)) => color,
        _ => panic!("work sidebar should have a background"),
    };
    let luminance = |color: iced::Color| 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;

    assert!(luminance(surface(&Theme::Light)) > 0.90);
    assert!(luminance(surface(&Theme::Dark)) < 0.50);
}

#[test]
fn work_sidebar_filters_sessions_by_project_and_update_time() {
    let mut app = App::new().0;
    let now = vw_shared::time::now_ms();
    app.recent_projects = vec!["/workspace/alpha".into(), "/workspace/beta".into()];
    app.recent_projects_edits.clear();
    app.project_path = Some("/workspace/alpha".into());
    app.project_sessions.insert(
        "/workspace/alpha".into(),
        vec![session("current", "当前项目", "/workspace/alpha", now)],
    );
    app.project_sessions.insert(
        "/workspace/beta".into(),
        vec![session(
            "old",
            "旧项目",
            "/workspace/beta",
            now.saturating_sub(8 * 24 * 60 * 60 * 1_000),
        )],
    );

    app.work_sidebar_filter_scope = WorkSidebarFilterScope::CurrentProject;
    let current_project = work_sidebar::model(&app);
    assert_eq!(current_project.tasks.len(), 1);
    assert_eq!(current_project.spaces.len(), 1);
    assert_eq!(current_project.spaces[0].label, "alpha");

    app.work_sidebar_filter_scope = WorkSidebarFilterScope::All;
    app.work_sidebar_filter_time = WorkSidebarFilterTime::LastSevenDays;
    let recent = work_sidebar::model(&app);
    assert_eq!(recent.tasks.len(), 1);
    assert_eq!(recent.spaces.len(), 1);
    assert_eq!(recent.spaces[0].label, "alpha");
}

#[test]
fn work_sidebar_filters_sessions_by_runtime_status() {
    let mut app = App::new().0;
    let now = vw_shared::time::now_ms();
    app.recent_projects = vec!["/workspace/alpha".into()];
    app.project_sessions.insert(
        "/workspace/alpha".into(),
        vec![
            session("running", "进行中的任务", "/workspace/alpha", now),
            session("completed", "已完成的任务", "/workspace/alpha", now),
        ],
    );
    app.get_session_runtime_mut("running").is_requesting = true;

    app.work_sidebar_filter_status = WorkSidebarFilterStatus::InProgress;
    let running = work_sidebar::model(&app);
    assert_eq!(
        running
            .tasks
            .iter()
            .map(|task| match &task.message {
                Message::Project(ProjectMessage::OpenProjectSessionPressed(_, id)) => id.as_str(),
                _ => panic!("task should use the existing project session open message"),
            })
            .collect::<Vec<_>>(),
        ["running"]
    );

    app.work_sidebar_filter_status = WorkSidebarFilterStatus::Completed;
    let completed = work_sidebar::model(&app);
    assert_eq!(
        completed
            .tasks
            .iter()
            .map(|task| match &task.message {
                Message::Project(ProjectMessage::OpenProjectSessionPressed(_, id)) => id.as_str(),
                _ => panic!("task should use the existing project session open message"),
            })
            .collect::<Vec<_>>(),
        ["completed"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_routes_shiju_entries_and_expert_hub_separately() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");

    let mut shiju_apps = work_sidebar::shiju_apps_button();
    let mut shiju_messages = click(&mut shiju_apps, &renderer);
    assert_eq!(shiju_messages.len(), 1);
    assert!(matches!(shiju_messages.remove(0), Message::ShijuApps(ShijuAppsMessage::Open)));

    let mut shiju_skills = work_sidebar::shiju_skills_button();
    let mut skill_messages = click(&mut shiju_skills, &renderer);
    assert_eq!(skill_messages.len(), 1);
    assert!(matches!(
        skill_messages.remove(0),
        Message::SkillHub(SkillHubMessage::OpenShijuSkills)
    ));

    let mut expert_hub = work_sidebar::expert_hub_button();
    let mut expert_messages = click(&mut expert_hub, &renderer);
    assert_eq!(expert_messages.len(), 1);
    assert!(matches!(expert_messages.remove(0), Message::SkillHub(SkillHubMessage::Open)));
}

#[test]
fn work_sidebar_refresh_message_deduplicates_recent_projects() {
    let mut app = App::new().0;
    app.recent_projects =
        vec!["/workspace/alpha".into(), "/workspace/beta".into(), "/workspace/alpha".into()];

    let Message::Batch(refreshes) = work_sidebar::refresh_all_message(&app) else {
        panic!("refresh helper should batch project session loads");
    };
    assert_eq!(refreshes.len(), 2);
    assert!(matches!(
        refreshes.first(),
        Some(Message::Project(ProjectMessage::ProjectLoadSessions(path)))
            if path == "/workspace/alpha"
    ));
    assert!(matches!(
        refreshes.get(1),
        Some(Message::Project(ProjectMessage::ProjectLoadSessions(path)))
            if path == "/workspace/beta"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_refreshes_sessions_for_every_space() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.recent_projects = vec!["/workspace/alpha".into(), "/workspace/beta".into()];
    let mut button = work_sidebar::refresh_all_control(&app);
    let mut messages = click(&mut button, &renderer);
    assert_eq!(messages.len(), 1);
    let Message::Batch(refreshes) = messages.remove(0) else {
        panic!("refresh control should batch every space refresh");
    };
    assert_eq!(refreshes.len(), 2);
    assert!(matches!(
        refreshes.first(),
        Some(Message::Project(ProjectMessage::ProjectLoadSessions(path)))
            if path == "/workspace/alpha"
    ));
    assert!(matches!(
        refreshes.get(1),
        Some(Message::Project(ProjectMessage::ProjectLoadSessions(path)))
            if path == "/workspace/beta"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_task_pool_control_reuses_code_mode_entry() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = App::new().0;
    let mut button = work_sidebar::task_pool_control(&app);
    let messages = click(&mut button, &renderer);

    assert!(matches!(
        messages.as_slice(),
        [Message::TaskBoard(crate::app::message::TaskBoardMessage::ToggleBoard)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_replaces_search_row_with_compact_header() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = App::new().0;
    let mut sidebar = work_sidebar::view(&app);
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(sidebar.as_widget());
    let node = sidebar.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let column =
        Layout::new(&node).children().next().expect("sidebar container should contain a column");

    assert_eq!(app.work_sidebar_tab, WorkSidebarTab::Tasks);
    assert_eq!(
        column.children().count(),
        4,
        "header, navigation and tabs should stay outside the scrolling list"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_uses_resizable_width_from_app_state() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_sidebar_width = 420.0;
    let mut sidebar = work_sidebar::view(&app);
    let limits = layout::Limits::new(Size::ZERO, Size::new(520.0, 800.0));
    let mut tree = Tree::new(sidebar.as_widget());
    let node = sidebar.as_widget_mut().layout(&mut tree, &renderer, &limits);

    assert_eq!(node.bounds().width, 420.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_collapsed_state_keeps_compact_controls_visible() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_sidebar_collapsed = true;
    app.work_sidebar_width = 54.0;
    let mut sidebar = work_sidebar::view(&app);
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(sidebar.as_widget());
    let node = sidebar.as_widget_mut().layout(&mut tree, &renderer, &limits);

    assert_eq!(node.bounds().width, 54.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_draws_dark_chinese_without_monospace_panic() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.recent_projects = vec!["/workspace/chinese".into()];
    app.recent_projects_edits = vec!["中文空间".into()];
    app.project_sessions.insert(
        "/workspace/chinese".into(),
        vec![session("cn", "中文任务标题", "/workspace/chinese", 30)],
    );
    app.get_session_runtime_mut("cn").is_requesting = true;
    app.work_sidebar_more_open = true;
    app.session_menu_id = Some("cn".into());
    app.session_menu_anchor = Some(Point::new(120.0, 180.0));
    let mut element = work_sidebar::view(&app);
    draw(&mut element, &mut renderer, &Theme::Dark);
    drop(element);

    app.work_sidebar_more_open = false;
    app.work_sidebar_tab = WorkSidebarTab::Spaces;
    app.status_animation_frame = 1;
    app.work_expanded_spaces.insert("/workspace/chinese".into());
    app.hovered_recent_project = Some("/workspace/chinese".into());
    app.project_tools_menu_path = Some("/workspace/chinese".into());
    let mut space_menu = work_sidebar::view(&app);
    draw(&mut space_menu, &mut renderer, &Theme::Dark);

    assert!(
        !include_str!("work_sidebar.rs").contains("Font::MONOSPACE"),
        "Chinese sidebar text must never use the monospace font"
    );
}

fn click(element: &mut iced::Element<'_, Message>, renderer: &iced::Renderer) -> Vec<Message> {
    let limits = layout::Limits::new(Size::ZERO, Size::new(280.0, 48.0));
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let cursor =
        mouse::Cursor::Available(Point::new(node.bounds().center_x(), node.bounds().center_y()));
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::new();

    for event in [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            cursor,
            renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::INFINITE,
        );
    }

    messages
}

fn right_click(
    element: &mut iced::Element<'_, Message>,
    renderer: &iced::Renderer,
    position: Point,
) -> Vec<Message> {
    let limits = layout::Limits::new(Size::ZERO, Size::new(280.0, 48.0));
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let cursor = mouse::Cursor::Available(position);
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    element.as_widget_mut().update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        Layout::new(&node),
        cursor,
        renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    messages
}

fn draw(element: &mut iced::Element<'_, Message>, renderer: &mut iced::Renderer, theme: &Theme) {
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let viewport = Rectangle::with_size(VIEWPORT);

    element.as_widget().draw(
        &tree,
        renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
}

#[test]
fn work_sidebar_projection_reuses_scroll_snapshot_and_updates_after_session_change() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/workspace".into()];
    app.project_sessions.insert("/workspace".into(), vec![session("old", "Old", "/workspace", 1)]);
    let first = work_sidebar::cached_model(&app);
    let _ =
        app.prepare_session_ui_projection(&Message::View(ViewMessage::PointerMoved(20.0, 30.0)));
    let second = work_sidebar::cached_model(&app);
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    let frame = app.status_animation_frame;
    let tick = ViewMessage::ActivityAnimationTick;
    app.prepare_session_ui_projection(&Message::View(tick.clone()));
    let _ = crate::app::message::view::update(&mut app, tick);
    assert_eq!(app.status_animation_frame, frame.wrapping_add(1));
    assert!(std::sync::Arc::ptr_eq(&first, &work_sidebar::cached_model(&app)));
    let event = ProjectMessage::ProjectSessionsLoaded(
        "/workspace".into(),
        Ok(vec![session("new", "New", "/workspace", 2)]),
    );
    app.prepare_session_ui_projection(&Message::Project(event.clone()));
    let _ = crate::app::message::project::update(&mut app, event);
    let changed = work_sidebar::cached_model(&app);
    assert!(!std::sync::Arc::ptr_eq(&first, &changed));
    assert_eq!(changed.tasks[0].id, "new");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "explicit real-layout session open/scroll performance probe"]
async fn session_ui_projection_open_scroll_probe() {
    use crate::app::message::ChatMessage;
    use std::time::Instant;
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    let path = "/workspace/probe";
    app.screen = crate::app::Screen::Project;
    app.workspace_mode = crate::app::state::WorkspaceMode::Work;
    app.project_path = Some(path.into());
    app.recent_projects = vec![path.into()];
    app.active_session_id = Some("root".into());
    let mut sessions = vec![session("root", "中文会话", path, 1)];
    for index in 0..10_000 {
        let mut child =
            child_session(&format!("task-{index}"), "root", "中文历史任务", path, index + 2);
        child.slug = Info::visible_task_slug(&child.slug);
        sessions.push(child);
    }
    app.project_sessions.insert(path.into(), sessions);
    app.chat = (0..80)
        .map(|index| crate::app::models::ChatMessage {
            role: if index % 2 == 0 {
                crate::app::models::ChatRole::User
            } else {
                crate::app::models::ChatRole::Assistant
            },
            content: "中文会话滚动验证，检查布局与绘制。\n".repeat(8),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
    app.rebuild_chat_message_estimated_heights();
    let started = Instant::now();
    let mut element = crate::app::views::project::view(&app);
    let limits = layout::Limits::new(Size::ZERO, Size::new(1200.0, 800.0));
    let mut tree = Tree::new(element.as_widget());
    let _ = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    drop(element);
    let open_ms = started.elapsed().as_secs_f64() * 1000.0;
    let sidebar_snapshot = app.session_ui_projection.sidebar.borrow().as_ref().unwrap().clone();
    let agents_snapshot =
        app.session_ui_projection.agent_sessions.borrow().as_ref().unwrap().clone();
    let mut frames = Vec::new();
    for step in 0..60 {
        let started = Instant::now();
        // Native scrolling also crosses hover regions and read-only editors.
        // These messages must not discard unrelated session projections.
        for event in [
            ChatMessage::ToolHover(1, 0),
            ChatMessage::ToolHoverLeave,
            ChatMessage::ThinkHover(1, 0),
            ChatMessage::ThinkHoverLeave,
            ChatMessage::MarkdownBlockEditorAction(
                1,
                0,
                0,
                iced::widget::text_editor::Action::SelectAll,
            ),
        ] {
            app.prepare_session_ui_projection(&Message::Chat(event.clone()));
            let _ = crate::app::message::chat::update(&mut app, event);
        }
        let event = ChatMessage::ScrollChanged {
            offset_y: step as f32 / 60.0,
            absolute_offset_y: step as f32 * 30.0,
            viewport_h: 600.0,
        };
        app.prepare_session_ui_projection(&Message::Chat(event.clone()));
        let _ = crate::app::message::chat::update(&mut app, event);
        let mut element = crate::app::views::project::view(&app);
        element.as_widget().diff(&mut tree);
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &Theme::Dark,
            &renderer::Style { text_color: iced::Color::WHITE },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(Size::new(1200.0, 800.0)),
        );
        frames.push(started.elapsed().as_secs_f64() * 1000.0);
        assert!(std::sync::Arc::ptr_eq(
            app.session_ui_projection.sidebar.borrow().as_ref().unwrap(),
            &sidebar_snapshot,
        ));
        assert!(std::sync::Arc::ptr_eq(
            app.session_ui_projection.agent_sessions.borrow().as_ref().unwrap(),
            &agents_snapshot,
        ));
    }
    frames.sort_by(f64::total_cmp);
    println!(
        "SESSION_PROBE sessions=10001 messages=80 open_ms={open_ms:.3} scroll_p50_ms={:.3} scroll_p95_ms={:.3} scroll_max_ms={:.3}",
        frames[30], frames[57], frames[59]
    );
    assert!(app.session_ui_projection.sidebar.borrow().is_some());
    assert!(app.session_ui_projection.agent_sessions.borrow().is_some());
}

#[test]
fn browser_and_request_polling_reuse_sidebar_without_hiding_metadata_updates() {
    use crate::app::message::ChatMessage;
    let mut app = App::new().0;
    app.recent_projects = vec!["/workspace".into()];
    app.project_sessions
        .insert("/workspace".into(), vec![session("task", "Task", "/workspace", 1)]);
    app.active_session_id = Some("task".into());
    app.current_session_runtime_mut().is_requesting = true;
    let snapshot = work_sidebar::cached_model(&app);
    for _ in 0..30 {
        let tick = ViewMessage::WorkBrowserRuntimeTick;
        app.prepare_session_ui_projection(&Message::View(tick.clone()));
        let _ = crate::app::message::view::update(&mut app, tick);
        let frame = app.current_session_runtime_ref().unwrap().submit_anim;
        for event in [
            ChatMessage::SubmitTick,
            ChatMessage::SessionSaveAck,
            ChatMessage::QuestionPollTick,
            ChatMessage::PermissionPollTick,
            ChatMessage::TodoPollTick,
            ChatMessage::QuestionListLoaded(Ok(Vec::new())),
            ChatMessage::PermissionListLoaded(Ok(Vec::new())),
        ] {
            app.prepare_session_ui_projection(&Message::Chat(event.clone()));
            let _ = crate::app::message::chat::update(&mut app, event);
        }
        assert_eq!(app.current_session_runtime_ref().unwrap().submit_anim, (frame + 1) % 3);
        assert!(std::sync::Arc::ptr_eq(&snapshot, &work_sidebar::cached_model(&app)));
    }
    let event = ProjectMessage::ProjectSessionsLoaded(
        "/workspace".into(),
        Ok(vec![session("task", "Renamed", "/workspace", 2)]),
    );
    app.prepare_session_ui_projection(&Message::Project(event.clone()));
    let _ = crate::app::message::project::update(&mut app, event);
    let changed = work_sidebar::cached_model(&app);
    assert!(!std::sync::Arc::ptr_eq(&snapshot, &changed));
    assert_eq!(changed.tasks[0].updated, 2);
}
