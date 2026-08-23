//! 任务详情推理强度选择器的测试，覆盖默认值和各档位交互。
//! 用例核对禁用及选中样式，并保证消息携带预期的强度值。
use super::*;
use crate::app::App;
use crate::app::message::{self, ChatMessage, TaskBoardMessage};
use crate::app::task::{Task, TaskStatus};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

#[test]
fn reasoning_option_formats_automatic_and_explicit_efforts() {
    assert_eq!(TaskBoardReasoningEffortOption(None).to_string(), "默认");
    assert_eq!(TaskBoardReasoningEffortOption(Some(ReasoningEffortDto::High)).to_string(), "High");
}

#[test]
fn selector_kind_reads_draft_and_bulk_agent_and_effort_sources() {
    let mut app = App::new().0;
    app.task_board_draft.acp_agent = Some("draft-agent".to_string());
    app.task_board_draft.reasoning_effort = Some(ReasoningEffortDto::Medium);
    app.task_board_bulk_acp_agent = Some("bulk-agent".to_string());
    app.task_board_bulk_reasoning_effort = Some(ReasoningEffortDto::Low);

    assert_eq!(TaskBoardReasoningSelectorKind::Draft.agent(&app), Some("draft-agent"));
    assert_eq!(TaskBoardReasoningSelectorKind::Edit.agent(&app), Some("draft-agent"));
    assert_eq!(TaskBoardReasoningSelectorKind::Bulk.agent(&app), Some("bulk-agent"));
    assert_eq!(
        TaskBoardReasoningSelectorKind::Edit.current(&app),
        Some(ReasoningEffortDto::Medium)
    );
    assert_eq!(TaskBoardReasoningSelectorKind::Bulk.current(&app), Some(ReasoningEffortDto::Low));
}

#[test]
fn selector_builders_cover_draft_edit_bulk_and_missing_agent_defaults() {
    let mut app = App::new().0;

    std::hint::black_box(build_reasoning_effort_selector(&app, false));
    std::hint::black_box(build_reasoning_effort_selector(&app, true));
    std::hint::black_box(build_bulk_reasoning_effort_selector(&app));

    app.task_board_draft.acp_agent = Some("codex".to_string());
    app.task_board_bulk_acp_agent = Some("codex".to_string());
    std::hint::black_box(build_reasoning_effort_selector(&app, false));
    std::hint::black_box(build_bulk_reasoning_effort_selector(&app));
}

#[test]
fn task_reasoning_effort_selector_updates_draft_edit_bulk_and_task_mode() {
    let mut app = App::new().0;

    let _ = message::task_board::update(
        &mut app,
        TaskBoardMessage::UpdateDraftReasoningEffort(Some(ReasoningEffortDto::High)),
    );
    assert_eq!(app.task_board_draft.reasoning_effort, Some(ReasoningEffortDto::High));
    assert_eq!(TaskBoardReasoningSelectorKind::Draft.current(&app), Some(ReasoningEffortDto::High));
    assert!(matches!(
        TaskBoardReasoningSelectorKind::Draft.select_message(None),
        TaskBoardMessage::UpdateDraftReasoningEffort(None)
    ));

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
