use crate::views::project::layout::work_sidebar::{
    self, ModelInput, WorkSidebarModel, events as sidebar_events,
};
use session_info::{Info, TimeInfo};
use std::collections::{HashMap, HashSet};
use work_sidebar::presentation::{
    WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime,
};

struct SidebarFixture {
    recent_projects: Vec<String>,
    recent_projects_edits: Vec<String>,
    project_path: Option<String>,
    project_sessions: HashMap<String, Vec<Info>>,
    work_expanded_spaces: HashSet<String>,
    work_sidebar_space_task_visible: HashMap<String, usize>,
    hovered_recent_project: Option<String>,
    project_tools_menu_path: Option<String>,
    work_sidebar_filter_scope: WorkSidebarFilterScope,
    work_sidebar_filter_time: WorkSidebarFilterTime,
    work_sidebar_visible_tasks: usize,
    work_sidebar_visible_spaces: usize,
}
impl Default for SidebarFixture {
    fn default() -> Self {
        Self {
            recent_projects: vec![],
            recent_projects_edits: vec![],
            project_path: None,
            project_sessions: HashMap::new(),
            work_expanded_spaces: HashSet::new(),
            work_sidebar_space_task_visible: HashMap::new(),
            hovered_recent_project: None,
            project_tools_menu_path: None,
            work_sidebar_filter_scope: WorkSidebarFilterScope::All,
            work_sidebar_filter_time: WorkSidebarFilterTime::All,
            work_sidebar_visible_tasks: 10,
            work_sidebar_visible_spaces: 10,
        }
    }
}
impl SidebarFixture {
    fn model(&self) -> WorkSidebarModel {
        work_sidebar::model(&ModelInput {
            recent_projects: &self.recent_projects,
            recent_projects_edits: &self.recent_projects_edits,
            project_path: &self.project_path,
            project_sessions: &self.project_sessions,
            pinned: &HashSet::new(),
            drafts: &[],
            activity: &|_| {
                (
                    false,
                    false,
                    crate::views::project::components::session_row::SessionRowStatus::Idle,
                )
            },
            work_sidebar_filter_scope: self.work_sidebar_filter_scope,
            work_sidebar_filter_status: WorkSidebarFilterStatus::All,
            work_sidebar_filter_time: self.work_sidebar_filter_time,
            work_sidebar_visible_tasks: self.work_sidebar_visible_tasks,
            work_sidebar_visible_spaces: self.work_sidebar_visible_spaces,
            work_expanded_spaces: &self.work_expanded_spaces,
            work_sidebar_space_task_visible: &self.work_sidebar_space_task_visible,
            hovered_recent_project: &self.hovered_recent_project,
            project_tools_menu_path: &self.project_tools_menu_path,
        })
    }
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
        fork_execution_started: None,
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
    let mut app = SidebarFixture::default();
    let path = "/workspace/archive";
    app.recent_projects = vec![path.into()];
    app.project_path = Some(path.into());
    let mut task = child_session("task", "root", "Task", path, 20);
    task.slug = Info::visible_task_slug(&task.slug);
    task.time.archived = Some(30);
    app.project_sessions.insert(path.into(), vec![session("root", "Root", path, 10), task]);
    assert_eq!(app.model().tasks.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["root"]);
    app.project_sessions.get_mut(path).expect("project")[1].time.archived = None;
    assert_eq!(
        app.model().tasks.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["task", "root"]
    );
}

#[test]
fn work_sidebar_shows_visible_task_in_own_project() {
    let mut app = SidebarFixture::default();
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
        let model = app.model();
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
    let mut app = SidebarFixture::default();
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

    let model = app.model();

    assert_eq!(
        model
            .tasks
            .iter()
            .map(|task| match &task.message {
                sidebar_events::Message::Project(
                    sidebar_events::ProjectMessage::OpenProjectSessionPressed(_, id),
                ) => id.as_str(),
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
    assert!(!model.spaces[1].expanded);
    assert!(model.spaces[1].sessions.is_empty());
    assert!(!model.spaces[1].hovered);
    assert!(!model.spaces[1].tools_open);
    assert_eq!(model.tasks[0].details.updated, 40);

    let sidebar_events::Message::Project(sidebar_events::ProjectMessage::SwitchWorkSpacePressed(
        path,
    )) = model.spaces[0].message.clone()
    else {
        panic!("space should use the work-mode project switch message");
    };
    assert_eq!(path, "/workspace/alpha");

    let sidebar_events::Message::View(sidebar_events::ViewMessage::WorkSpaceToggled(path)) =
        model.spaces[0].toggle_message.clone()
    else {
        panic!("space disclosure should preserve the existing expansion message");
    };
    assert_eq!(path, "/workspace/alpha");

    let sidebar_events::Message::Project(
        sidebar_events::ProjectMessage::OpenProjectSessionPressed(directory, id),
    ) = model.spaces[0].sessions[0].message.clone()
    else {
        panic!("expanded space task should use the existing project session open message");
    };
    assert_eq!(directory, "/workspace/alpha");
    assert_eq!(id, "newest");

    let sidebar_events::Message::Project(sidebar_events::ProjectMessage::ProjectToolsMenuToggled(
        path,
    )) = model.spaces[0].tools_message.clone()
    else {
        panic!("space tools action should toggle that space menu");
    };
    assert_eq!(path, "/workspace/alpha");
}

#[test]
fn work_sidebar_paginates_tasks_and_spaces_from_ten_in_steps_of_five() {
    let mut app = SidebarFixture::default();
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

    let initial = app.model();
    assert_eq!(initial.tasks.len(), 10);
    assert_eq!(initial.task_total, 13);
    assert_eq!(initial.spaces.len(), 10);
    assert_eq!(initial.space_total, 13);

    app.work_sidebar_visible_tasks += 5;
    app.work_sidebar_visible_spaces += 5;
    let loaded = app.model();
    assert_eq!(loaded.tasks.len(), 13);
    assert_eq!(loaded.spaces.len(), 13);
}

#[test]
fn work_sidebar_paginates_expanded_space_tasks_from_twenty() {
    let mut app = SidebarFixture::default();
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

    let initial = app.model();
    assert_eq!(initial.spaces.len(), 1);
    assert_eq!(initial.spaces[0].session_total, 25);
    assert_eq!(initial.spaces[0].sessions.len(), 20);
    assert_eq!(initial.spaces[0].sessions[0].id, "session-00");
    assert_eq!(initial.spaces[0].sessions[19].id, "session-19");

    app.work_sidebar_space_task_visible.insert(path, 40);
    let loaded = app.model();
    assert_eq!(loaded.spaces[0].sessions.len(), 25);
    assert_eq!(loaded.spaces[0].session_total, 25);
}

#[test]
fn work_sidebar_filters_sessions_by_project_and_update_time() {
    let mut app = SidebarFixture::default();
    let now = runtime_time::now_ms();
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
    let current_project = app.model();
    assert_eq!(current_project.tasks.len(), 1);
    assert_eq!(current_project.spaces.len(), 1);
    assert_eq!(current_project.spaces[0].label, "alpha");

    app.work_sidebar_filter_scope = WorkSidebarFilterScope::All;
    app.work_sidebar_filter_time = WorkSidebarFilterTime::LastSevenDays;
    let recent = app.model();
    assert_eq!(recent.tasks.len(), 1);
    assert_eq!(recent.spaces.len(), 1);
    assert_eq!(recent.spaces[0].label, "alpha");
}
