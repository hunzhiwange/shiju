use super::*;
use taskboard::update::execution_results::{ResultState, ResultEffect};
fn persist_result_task_via_gateway(
    project_path: String,
    workflow_id: String,
    task: Task,
    action_label: &'static str,
) -> iced::Task<crate::app::Message> {
    let task_id = task.id.clone();
    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            persist_updated_tasks_via_gateway(&project_path, &[task], action_label, &workflow_id)
                .await
        },
        move |result| {
            Message::TaskBoard(TaskBoardMessage::TaskResultPersistenceCompleted {
                project_path: result_project_path.clone(),
                task_id: task_id.clone(),
                result,
            })
        },
    )
}

pub(super) fn update(app: &mut crate::app::App, message: TaskBoardMessage) -> iced::Task<Message> {
    let mut state = ResultState {
        project_path: &mut app.navigation_tabs.project_path,
        task_board_tasks: &mut app.task_board.task_board_tasks,
        task_board_executor: &mut app.task_board.task_board_executor,
        task_board_settings: &mut app.task_board.task_board_settings,
        task_board_tasks_pending_persistence: &mut app.task_board.task_board_tasks_pending_persistence,
        task_board_manual_merge_queue: &mut app.task_board.task_board_manual_merge_queue,
        effects: vec![],
    };
    let tasks = taskboard::update::execution_results::update(&mut state, message);
    let mut effects = std::mem::take(&mut state.effects);
    effects.extend(tasks);
    drop(state);
    let mut tasks = Vec::new();
    for effect in effects {
        match effect {
            ResultEffect::SyncLogs(task) => {
                if let Some(task) = task {
                    if app.task_board.task_board_viewing_logs.as_ref().is_some_and(|viewing| viewing.id == task.id) {
                        set_viewing_logs(app, Some(task));
                    } else {
                        refresh_task_log_cache(app, &task);
                    }
                }
            },
            ResultEffect::ScheduleTick => tasks.push(schedule_scheduler_tick_with_deadline(app)),
            ResultEffect::MergeStopped => app.push_localized_notification("批量合并已停止，请先处理当前合并失败", vec![]),
            ResultEffect::Message(message) => tasks.push(iced::Task::done(Message::TaskBoard(message))),
            ResultEffect::Persist { project_path, workflow_id, task, action_label } => tasks.push(persist_result_task_via_gateway(project_path, workflow_id, task, action_label)),
        }
    }
    iced::Task::batch(tasks)
}
