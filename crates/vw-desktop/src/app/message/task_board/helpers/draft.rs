pub(crate) use desktop_task_board::draft::*;

pub(crate) fn reset_create_draft(app: &mut crate::app::App) {
    desktop_task_board::draft::reset_create_draft(
        &mut app.task_board_draft,
        &mut app.task_board_prompt_editor,
        app.task_board_last_model.clone(),
        app.task_board_last_acp_agent.clone(),
    );
}
