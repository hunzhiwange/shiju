use iced::widget::text_editor;
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
pub fn handle_task_mode_task_split_mode_changed(
    task_mode_task_split_mode: &mut desktop_task_model::TaskSplitMode,
    mode: desktop_task_model::TaskSplitMode,
) {

    (*task_mode_task_split_mode) = mode;
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
pub fn handle_task_mode_subtask_changed(
    task_mode_subtasks: &mut Vec<String>, task_mode_subtask_editors: &mut Vec<text_editor::Content>,
    index: usize,
    value: String,
) {

    if let Some(item) = (*task_mode_subtasks).get_mut(index) {
        *item = value.clone();
    }
    if let Some(editor) = (*task_mode_subtask_editors).get_mut(index) {
        *editor = text_editor::Content::with_text(&value);
        editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    }
}
pub fn handle_task_mode_subtask_editor_action(
    task_mode_subtasks: &mut Vec<String>, task_mode_subtask_editors: &mut Vec<text_editor::Content>,
    index: usize,
    action: text_editor::Action,
) {

    if index >= (*task_mode_subtasks).len() {
        return;
    }

    while (*task_mode_subtask_editors).len() < (*task_mode_subtasks).len() {
        (*task_mode_subtask_editors).push(text_editor::Content::new());
    }

    if let Some(editor) = (*task_mode_subtask_editors).get_mut(index) {
        let editor: &mut text_editor::Content = editor;
        let action: text_editor::Action = action;
        editor.perform(action);
        let value = editor.text().to_string();
        if let Some(item) = (*task_mode_subtasks).get_mut(index) {
            *item = value;
        }
    }

}
pub fn handle_task_mode_add_subtask(task_mode_subtasks: &mut Vec<String>, task_mode_subtask_editors: &mut Vec<text_editor::Content>) {

    (*task_mode_subtasks).push(String::new());
    (*task_mode_subtask_editors).push(text_editor::Content::new());
}
pub fn handle_task_mode_remove_subtask(task_mode_subtasks: &mut Vec<String>, task_mode_subtask_editors: &mut Vec<text_editor::Content>, index: usize) {

    if index < (*task_mode_subtasks).len() {
        (*task_mode_subtasks).remove(index);
        if index < (*task_mode_subtask_editors).len() {
            (*task_mode_subtask_editors).remove(index);
        }
    }

    if (*task_mode_subtasks).is_empty() {
        (*task_mode_subtasks).push(String::new());
        (*task_mode_subtask_editors).push(text_editor::Content::new());
    }

}
pub fn handle_task_mode_move_subtask_up(task_mode_subtasks: &mut Vec<String>, task_mode_subtask_editors: &mut Vec<text_editor::Content>, index: usize) {

    if index > 0 && index < (*task_mode_subtasks).len() {
        (*task_mode_subtasks).swap(index - 1, index);
        if index < (*task_mode_subtask_editors).len() {
            (*task_mode_subtask_editors).swap(index - 1, index);
        }
    }
}
pub fn handle_task_mode_move_subtask_down(task_mode_subtasks: &mut Vec<String>, task_mode_subtask_editors: &mut Vec<text_editor::Content>, index: usize) {

    if index + 1 < (*task_mode_subtasks).len() {
        (*task_mode_subtasks).swap(index, index + 1);
        if index + 1 < (*task_mode_subtask_editors).len() {
            (*task_mode_subtask_editors).swap(index, index + 1);
        }
    }
}
