use crate::project_message::edit::*;
#[derive(Default)]
struct Fixture {
    project_editor: Editor,
    hovered_recent_project: Option<String>,
    project_path: Option<String>,
    project_tools_menu_path: Option<String>,
    project_worktree_enabled: std::collections::HashMap<String, bool>,
    recent_project_remove_confirm_path: Option<String>,
    recent_projects: Vec<String>,
    recent_projects_edits: Vec<String>,
    recent_projects_meta: Vec<RecentProjectMeta>,
    session_menu_anchor: Option<iced::Point>,
    session_menu_id: Option<String>,
    task_board_settings: desktop_task_model::TaskBoardSettings,
    work_current_project_menu: Option<String>,
}
fn app() -> Fixture {
    Fixture::default()
}
fn apply(app: &mut Fixture, event: Event) {
    let _ = handle(
        &mut State {
            editor: &mut app.project_editor,
            hovered_recent_project: &mut app.hovered_recent_project,
            project_path: &mut app.project_path,
            project_tools_menu_path: &mut app.project_tools_menu_path,
            project_worktree_enabled: &mut app.project_worktree_enabled,
            recent_project_remove_confirm_path: &mut app.recent_project_remove_confirm_path,
            recent_projects: &mut app.recent_projects,
            recent_projects_edits: &mut app.recent_projects_edits,
            recent_projects_meta: &mut app.recent_projects_meta,
            session_menu_anchor: &mut app.session_menu_anchor,
            session_menu_id: &mut app.session_menu_id,
            task_board_settings: &mut app.task_board_settings,
            work_current_project_menu: &mut app.work_current_project_menu,
        },
        event,
    );
}
fn custom_status(key: &'static str) -> desktop_task_model::TaskStatus {
    desktop_task_model::TaskStatus::parse_key(key).expect("custom status key should parse")
}

fn owned_status(
    key: &'static str,
    label: &'static str,
    role: desktop_task_model::StatusRole,
    next: Option<desktop_task_model::TaskStatus>,
) -> desktop_task_model::OwnedStatusDefinition {
    desktop_task_model::OwnedStatusDefinition {
        status: custom_status(key),
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        is_terminal: next.is_none(),
        auto_execution_candidate: !matches!(
            role,
            desktop_task_model::StatusRole::Done | desktop_task_model::StatusRole::Archived
        ),
        occupies_execution_slot: matches!(
            role,
            desktop_task_model::StatusRole::Executing | desktop_task_model::StatusRole::Planning
        ),
        is_scheduling_backlog: matches!(
            role,
            desktop_task_model::StatusRole::Backlog
                | desktop_task_model::StatusRole::Queued
                | desktop_task_model::StatusRole::Planning
        ),
        next,
        transitions: Vec::new(),
    }
}

fn custom_workflow(id: &str) -> desktop_task_model::OwnedTaskWorkflow {
    let done = custom_status("project_custom_done");
    desktop_task_model::OwnedTaskWorkflow {
        id: id.to_string(),
        label: "Project Custom Workflow".to_string(),
        statuses: vec![
            owned_status(
                "project_custom_queued",
                "Queued",
                desktop_task_model::StatusRole::Queued,
                Some(done),
            ),
            owned_status("project_custom_done", "Done", desktop_task_model::StatusRole::Done, None),
        ],
    }
}

struct TempHome {
    _dir: tempfile::TempDir,
    previous_home: Option<std::ffi::OsString>,
}

impl TempHome {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp home should be created");
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", dir.path()) };
        Self { _dir: dir, previous_home }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        match &self.previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}

struct RuntimeWorkflowGuard {
    _cache_lock: std::sync::MutexGuard<'static, ()>,
    _home: TempHome,
}

impl RuntimeWorkflowGuard {
    fn with_workflows(workflows: Vec<desktop_task_model::OwnedTaskWorkflow>) -> Self {
        let cache_lock = desktop_task_model::runtime_task_lifecycle_cache_test_lock();
        let home = TempHome::new();
        desktop_task_model::replace_runtime_task_lifecycle_workflows_for_tests(workflows)
            .expect("runtime workflows should be isolated");
        Self { _cache_lock: cache_lock, _home: home }
    }
}

impl Drop for RuntimeWorkflowGuard {
    fn drop(&mut self) {
        let _ = desktop_task_model::reset_runtime_task_lifecycle_cache();
    }
}

#[test]
fn project_accepts_custom_registered_workflow() {
    let workflow_id = "project_custom_workflow";
    let _guard = RuntimeWorkflowGuard::with_workflows(vec![custom_workflow(workflow_id)]);
    let mut app = app();
    let path = "/tmp/project-custom-workflow".to_string();
    app.project_path = Some(path.clone());
    app.recent_projects = vec![path.clone()];
    app.project_editor.project_edit_path = Some(path.clone());
    app.project_editor.project_edit_name = "Project".to_string();

    let _ = apply(&mut app, Event::ProjectEditWorkflowIdChanged(workflow_id.to_string()));

    assert_eq!(app.project_editor.project_edit_task_board_settings.workflow_id, workflow_id);
    assert_eq!(
        app.project_editor.project_edit_task_board_settings.clone().sanitized().workflow_id,
        workflow_id
    );

    let _ = apply(&mut app, Event::ProjectEditSaved);
    let saved = app.recent_projects_meta.iter().find(|meta| meta.path == path).unwrap();
    assert_eq!(
        saved.task_board_settings.as_ref().map(|settings| settings.workflow_id.as_str()),
        Some(workflow_id)
    );
    assert_eq!(app.task_board_settings.workflow_id, workflow_id);
}
#[test]
fn project_rejects_unknown_workflow_id() {
    let _guard = RuntimeWorkflowGuard::with_workflows(Vec::new());
    let mut app = app();

    let _ = apply(&mut app, Event::ProjectEditWorkflowIdChanged("missing_workflow".to_string()));

    assert_eq!(
        app.project_editor.project_edit_task_board_settings.workflow_id,
        desktop_task_model::DEFAULT_TASK_WORKFLOW_ID
    );

    app.project_editor.project_edit_task_board_settings.workflow_id =
        "missing_workflow".to_string();
    assert_eq!(
        app.project_editor.project_edit_task_board_settings.clone().sanitized().workflow_id,
        desktop_task_model::DEFAULT_TASK_WORKFLOW_ID
    );
}
