#[test]
fn reset_create_draft_rebuilds_state_and_synchronizes_prompt_editor() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_last_model = "  gpt-5  ".to_string();
    app.task_board_last_acp_agent = Some("  codex  ".to_string());
    app.task_board_draft.prompt = "stale draft".to_string();
    app.task_board_prompt_editor = iced::widget::text_editor::Content::with_text("stale editor");

    super::reset_create_draft(&mut app);

    assert_eq!(app.task_board_draft.model, "gpt-5");
    assert_eq!(app.task_board_draft.acp_agent.as_deref(), Some("codex"));
    assert!(app.task_board_draft.prompt.is_empty());
    assert!(app.task_board_prompt_editor.text().is_empty());
}
