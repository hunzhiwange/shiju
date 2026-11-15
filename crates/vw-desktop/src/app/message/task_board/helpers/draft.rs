pub(crate) use desktop_task_board::draft::*;

pub(crate) fn reset_create_draft(app: &mut crate::app::App) {
    app.task_board_draft = create_task_draft_with_preferences(
        app.task_board_last_model.clone(),
        app.task_board_last_acp_agent.clone(),
    );
    app.task_board_prompt_editor =
        iced::widget::text_editor::Content::with_text(&app.task_board_draft.prompt);
}

#[cfg(test)]
#[path = "draft_adapter_tests.rs"]
mod draft_adapter_tests;
