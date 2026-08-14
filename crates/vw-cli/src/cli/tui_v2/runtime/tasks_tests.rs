use vw_shared::todo::Todo;

use super::super::model::background_task::{
    BackgroundTask, BackgroundTaskKind, BackgroundTaskPanel, BackgroundTaskStatus,
};
use super::super::model::{UiTodoItem, UiTodoOverlay};
use super::super::state::TuiState;
use super::tasks::{TasksRuntime, refresh_background_tasks, save_todos, stop_background_task};

#[derive(Default)]
struct RecordingTasks {
    tasks: Vec<BackgroundTask>,
    saved: Vec<(String, Vec<Todo>)>,
    stopped: Vec<String>,
    fail: bool,
}

impl TasksRuntime for RecordingTasks {
    fn list_background_tasks(&mut self) -> Result<Vec<BackgroundTask>, String> {
        if self.fail { Err("scheduler offline".to_string()) } else { Ok(self.tasks.clone()) }
    }

    fn save_todos(&mut self, session_id: &str, todos: Vec<Todo>) -> Result<(), String> {
        if self.fail {
            return Err("todo save failed".to_string());
        }
        self.saved.push((session_id.to_string(), todos));
        Ok(())
    }

    fn stop_task(&mut self, task_id: &str) -> Result<(), String> {
        if self.fail {
            return Err("stop failed".to_string());
        }
        self.stopped.push(task_id.to_string());
        Ok(())
    }
}

fn task(id: &str, supports_stop: bool) -> BackgroundTask {
    BackgroundTask {
        id: id.to_string(),
        session_id: "session-a".to_string(),
        kind: BackgroundTaskKind::Agent,
        title: "Background agent".to_string(),
        status: BackgroundTaskStatus::Running,
        detail: "working".to_string(),
        log: vec!["started".to_string()],
        supports_stop,
    }
}

#[test]
fn tui_todos_and_background_tasks_match_reference_state_and_gateway_actions() {
    let mut runtime = RecordingTasks {
        tasks: vec![task("task-a", true), task("task-b", false)],
        ..RecordingTasks::default()
    };
    let mut panel = BackgroundTaskPanel::default();
    refresh_background_tasks(&mut panel, &mut runtime);
    assert_eq!(panel.tasks.len(), 2);
    assert_eq!(panel.tasks[0].log, ["started"]);

    runtime.fail = true;
    refresh_background_tasks(&mut panel, &mut runtime);
    assert_eq!(panel.tasks.len(), 2, "refresh failure preserves verified task list");
    assert_eq!(panel.error.as_deref(), Some("scheduler offline"));

    let mut todos = UiTodoOverlay {
        session_id: Some("session-a".to_string()),
        items: vec![UiTodoItem {
            id: "todo-a".to_string(),
            content: "verify".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
        }],
        dirty: true,
        ..UiTodoOverlay::default()
    };
    assert!(save_todos(&mut todos, &mut runtime).is_err());
    assert!(todos.dirty);
    runtime.fail = false;
    save_todos(&mut todos, &mut runtime).unwrap();
    assert!(!todos.dirty);

    assert!(stop_background_task(&mut panel.tasks[0], "session-a", false, &mut runtime).is_err());
    stop_background_task(&mut panel.tasks[0], "session-a", true, &mut runtime).unwrap();
    assert_eq!(panel.tasks[0].status, BackgroundTaskStatus::Cancelled);
    assert_eq!(runtime.stopped, ["task-a"]);
    assert!(
        stop_background_task(&mut panel.tasks[1], "session-a", true, &mut runtime)
            .unwrap_err()
            .contains("does not expose")
    );
}

#[test]
fn task_and_agent_panels_load_scheduler_data_and_gate_mutations_by_scope() {
    let state = TuiState::default();
    let tasks = state.command_registry.resolve("tasks").expect("/tasks");
    assert!(tasks.aliases.iter().any(|alias| alias == "bashes"));
    assert!(state.command_registry.resolve("agents").is_some());
    for blocked in ["fork", "subtask"] {
        let spec = state.command_registry.resolve(blocked).expect("blocked command visible");
        assert!(!spec.availability.is_available());
        assert!(spec.availability.unavailable_reason().is_some());
    }
}
