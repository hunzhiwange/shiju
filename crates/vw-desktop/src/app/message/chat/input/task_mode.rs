use crate::app::{App, Message};
use desktop_session::message::chat::input::task_mode as domain;
use iced::Task;

pub(super) fn handle_task_mode_toggled(app: &mut App, enabled: bool) -> Task<Message> {

 let runtime = app.current_session_runtime_mut();
 domain::handle_task_mode_toggled(&mut runtime.task_mode_enabled, enabled);
 Task::none()
}
pub(super) fn handle_workflow_mode_toggled(app: &mut App, enabled: bool) -> Task<Message> {

 let runtime = app.current_session_runtime_mut();
 domain::handle_workflow_mode_toggled(&mut runtime.workflow_mode_enabled, enabled);
 Task::none()
}
pub(super) fn handle_goal_mode_toggled(app: &mut App, _enabled: bool) -> Task<Message> {

 let runtime = app.current_session_runtime_mut();
 domain::handle_goal_mode_toggled(&mut runtime.goal_mode_enabled, _enabled);
 Task::none()
}
pub(super) fn handle_task_mode_priority_changed(app: &mut App, value: String) -> Task<Message> {

 let runtime = app.current_session_runtime_mut();
 domain::handle_task_mode_priority_changed(&mut runtime.task_mode_priority, value);
 Task::none()
}
pub(super) fn handle_task_mode_model_changed(app: &mut App, model: String) -> Task<Message> {
let options =
        crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(
            app,
            model.trim(),
        );
 let runtime = app.current_session_runtime_mut();
 domain::handle_task_mode_model_changed(&mut runtime.task_mode_executor, &mut runtime.task_mode_reasoning_effort, &mut runtime.task_mode_model, model, &options);
 Task::none()
}
pub(super) fn handle_task_mode_task_split_mode_changed(
    app: &mut App,
    mode: crate::app::task::TaskSplitMode,
) -> Task<Message> {

 let runtime = app.current_session_runtime_mut();
 domain::handle_task_mode_task_split_mode_changed(&mut runtime.task_mode_task_split_mode, mode);
 Task::none()
}
pub(super) fn handle_task_mode_executor_changed(
    app: &mut App,
    executor: Option<String>,
) -> Task<Message> {

 let runtime = app.current_session_runtime_mut();
 domain::handle_task_mode_executor_changed(&mut runtime.task_mode_executor, executor);
 if runtime.task_mode_executor.is_none() && runtime.task_mode_model == desktop_task_model::TASK_MODEL_ACP_NATIVE {
     runtime.task_mode_model = desktop_task_model::TASK_MODEL_AUTO.to_owned();
 }
 Task::none()
}
pub(super) fn handle_task_mode_reasoning_effort_changed(
    app: &mut App,
    reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
) -> Task<Message> {

 let runtime = app.current_session_runtime_mut();
 domain::handle_task_mode_reasoning_effort_changed(&mut runtime.task_mode_reasoning_effort, reasoning_effort);
 Task::none()
}
