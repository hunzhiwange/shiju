#[test]
fn reset_create_draft_rebuilds_state_and_synchronizes_prompt_editor() {
    let mut draft = desktop_task_model::TaskDraft::default();
    let mut prompt_editor = iced::widget::text_editor::Content::new();
    draft.prompt = "stale draft".to_string();
    prompt_editor = iced::widget::text_editor::Content::with_text("stale editor");

    crate::draft::reset_create_draft(
        &mut draft,
        &mut prompt_editor,
        "  gpt-5  ".to_string(),
        Some("  codex  ".to_string()),
    );

    assert_eq!(draft.model, "gpt-5");
    assert_eq!(draft.acp_agent.as_deref(), Some("codex"));
    assert!(draft.prompt.is_empty());
    assert!(prompt_editor.text().is_empty());
}
