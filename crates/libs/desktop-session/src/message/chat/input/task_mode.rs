pub fn handle_task_mode_toggled(task_mode_enabled: &mut bool, enabled: bool) {

    (*task_mode_enabled) = enabled;
}
pub fn handle_workflow_mode_toggled(workflow_mode_enabled: &mut bool, enabled: bool) {
    (*workflow_mode_enabled) = enabled;
}
pub fn handle_goal_mode_toggled(goal_mode_enabled: &mut bool, _enabled: bool) {
    (*goal_mode_enabled) = true;
}
pub fn handle_task_mode_priority_changed(task_mode_priority: &mut String, value: String) {

    (*task_mode_priority) = value;
}
pub fn handle_task_mode_model_changed(task_mode_executor: &mut Option<String>, task_mode_reasoning_effort: &mut Option<vw_api_chat_identity::ReasoningEffortDto>, task_mode_model: &mut String, model: String, options: &[Option<vw_api_chat_identity::ReasoningEffortDto>]) {


    let trimmed = model.trim();
    if (*task_mode_executor).is_none()
        && !options.contains(&(*task_mode_reasoning_effort))
    {
        (*task_mode_reasoning_effort) = None;
    }
    (*task_mode_model) =
        if trimmed.is_empty() { "auto".to_string() } else { trimmed.to_string() };
}
pub fn handle_task_mode_executor_changed(
    task_mode_executor: &mut Option<String>,
    executor: Option<String>,
) {

    (*task_mode_executor) = executor;
}
pub fn handle_task_mode_reasoning_effort_changed(
    task_mode_reasoning_effort: &mut Option<vw_api_chat_identity::ReasoningEffortDto>,
    reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
) {
    (*task_mode_reasoning_effort) = reasoning_effort;
}
