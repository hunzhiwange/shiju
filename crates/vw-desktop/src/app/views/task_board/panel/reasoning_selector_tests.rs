// Task 20 ownership: Retain cross-domain contract: task editing/save and bulk update -> chat session runtime -> AddTaskFromInputWithOptions. Independent reasoning widget events moved to desktop-task-board::desktop_split_20_tests.
//! 任务详情推理强度选择器的测试，覆盖默认值和各档位交互。
//! 用例核对禁用及选中样式，并保证消息携带预期的强度值。
use crate::app::App;
use crate::app::message::{self, ChatMessage, TaskBoardMessage};
use crate::app::task::{Task, TaskStatus};
use vw_api_chat_identity::ReasoningEffortDto;

#[test]
fn task_reasoning_effort_selector_updates_draft_edit_bulk_and_task_mode() {
    let mut app = App::new().0;

    let _ = message::task_board::update(
        &mut app,
        TaskBoardMessage::UpdateDraftReasoningEffort(Some(ReasoningEffortDto::High)),
    );
    assert_eq!(app.task_board_draft.reasoning_effort, Some(ReasoningEffortDto::High));

    let mut editing_task = Task::new(1);
    editing_task.id = "reasoning-edit".to_string();
    editing_task.status = TaskStatus::Pending;
    app.task_board_tasks.push(editing_task);
    let _ = message::task_board::update(
        &mut app,
        TaskBoardMessage::EditTask("reasoning-edit".to_string()),
    );
    let _ = message::task_board::update(
        &mut app,
        TaskBoardMessage::UpdateEditingTaskReasoningEffort(Some(ReasoningEffortDto::Xhigh)),
    );
    let _ = message::task_board::update(&mut app, TaskBoardMessage::SaveEditingTask);
    app.task_board_draft.reasoning_effort = None;
    let _ = message::task_board::update(
        &mut app,
        TaskBoardMessage::EditTask("reasoning-edit".to_string()),
    );
    assert_eq!(app.task_board_draft.reasoning_effort, Some(ReasoningEffortDto::Xhigh));

    app.task_board_selected_tasks.insert("reasoning-edit".to_string());
    let _ = message::task_board::update(
        &mut app,
        TaskBoardMessage::UpdateBulkReasoningEffort(Some(ReasoningEffortDto::Low)),
    );
    let bulk_reasoning_effort = app.task_board_bulk_reasoning_effort;
    let _ = message::task_board::update(
        &mut app,
        TaskBoardMessage::BulkSetReasoningEffortInStatus {
            status: TaskStatus::Pending,
            reasoning_effort: bulk_reasoning_effort,
        },
    );
    assert_eq!(app.task_board_tasks[0].reasoning_effort, Some(ReasoningEffortDto::Low));

    let _ = message::chat::update(
        &mut app,
        ChatMessage::TaskModeExecutorChanged(Some("codex".to_string())),
    );
    let _ = message::chat::update(
        &mut app,
        ChatMessage::TaskModeReasoningEffortChanged(Some(ReasoningEffortDto::Medium)),
    );
    assert_eq!(
        app.current_session_runtime().task_mode_reasoning_effort,
        Some(ReasoningEffortDto::Medium)
    );

    let message = TaskBoardMessage::AddTaskFromInputWithOptions {
        content: "task".to_string(),
        priority: "1".to_string(),
        model: "auto".to_string(),
        reasoning_effort: app.current_session_runtime().task_mode_reasoning_effort,
        task_split_mode: Default::default(),
        subtasks: Vec::new(),
    };
    match message {
        TaskBoardMessage::AddTaskFromInputWithOptions { reasoning_effort, .. } => {
            assert_eq!(reasoning_effort, Some(ReasoningEffortDto::Medium));
        }
        other => panic!("expected task mode add message, got {other:?}"),
    }
}
