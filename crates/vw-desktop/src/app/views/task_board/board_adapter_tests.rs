// Task 20 ownership: Retain host BoardSlots injection: agent/model/executor selectors are assembled by the App adapter.
//! Host composition coverage for selectors injected into the domain board.
use crate::app::App;
use crate::app::task::{Task, TaskStatus};
#[test]
fn board_composes_host_bulk_selectors() {
    let (mut app, _) = App::new();
    for status in [TaskStatus::Planning, TaskStatus::PrSubmitted] {
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
