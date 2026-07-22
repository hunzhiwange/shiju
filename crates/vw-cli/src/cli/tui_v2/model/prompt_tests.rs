use super::prompt::{
    PromptCursor, PromptHistoryState, PromptMode, PromptMotion, PromptState, PromptSubmission,
    PromptSubmissionStatus, QueuedPromptCommand, QueuedPromptCommandKind,
};

#[test]
fn prompt_cursor_counts_unicode_characters_at_end() {
    let cursor = PromptCursor::at_end("a你\nb");
    assert_eq!(cursor.char_index, 4);
    assert_eq!(cursor.preferred_column, None);
}

#[test]
fn prompt_history_push_trims_deduplicates_and_restores_draft() {
    let mut history = PromptHistoryState::default();
    history.push("   ");
    history.push("  first  ");
    history.push("first");
    history.push("second");
    assert_eq!(history.entries, vec!["first".to_string(), "second".to_string()]);

    assert_eq!(history.select_previous("draft").as_deref(), Some("second"));
    assert_eq!(history.select_previous("ignored").as_deref(), Some("first"));
    assert_eq!(history.select_previous("ignored").as_deref(), Some("first"));
    assert_eq!(history.select_next().as_deref(), Some("second"));
    assert_eq!(history.select_next().as_deref(), Some("draft"));
    assert_eq!(history.select_next(), None);
}

#[test]
fn prompt_state_editing_handles_unicode_and_mode_refresh() {
    let mut prompt = PromptState::new("a你");
    assert_eq!(prompt.cursor.char_index, 2);
    assert!(prompt.move_cursor(PromptMotion::Left));
    prompt.insert_text("中");
    assert_eq!(prompt.value, "a中你");
    prompt.backspace();
    assert_eq!(prompt.value, "a你");
    prompt.delete();
    assert_eq!(prompt.value, "a");

    prompt.set_value("  /help");
    assert_eq!(prompt.mode, PromptMode::SlashCommand);
    prompt.set_value("hello");
    assert_eq!(prompt.mode, PromptMode::Compose);
}

#[test]
fn prompt_state_horizontal_and_vertical_cursor_motion_clamps_to_lines() {
    let mut prompt = PromptState::new("abc\nde\nfghi");
    prompt.cursor.char_index = 1;
    assert!(prompt.move_cursor(PromptMotion::End));
    assert_eq!(prompt.cursor.char_index, 3);
    assert!(!prompt.can_move_cursor(PromptMotion::End));
    assert!(prompt.move_cursor(PromptMotion::Down));
    assert_eq!(prompt.cursor.char_index, 6);
    assert_eq!(prompt.cursor.preferred_column, Some(3));
    assert!(prompt.move_cursor(PromptMotion::Down));
    assert_eq!(prompt.cursor.char_index, 10);
    assert!(prompt.move_cursor(PromptMotion::Home));
    assert_eq!(prompt.cursor.char_index, 7);
    assert!(prompt.move_cursor(PromptMotion::Up));
    assert_eq!(prompt.cursor.char_index, 4);
}

#[test]
fn prompt_state_history_selection_replaces_value_and_restores_draft() {
    let mut prompt = PromptState::new("draft");
    prompt.history.push("one");
    prompt.history.push("two");
    prompt.set_value("draft");

    assert!(prompt.select_previous_history());
    assert_eq!(prompt.value, "two");
    assert!(prompt.select_previous_history());
    assert_eq!(prompt.value, "one");
    assert!(prompt.select_next_history());
    assert_eq!(prompt.value, "two");
    assert!(prompt.select_next_history());
    assert_eq!(prompt.value, "draft");
    assert!(!prompt.select_next_history());
}

#[test]
fn prompt_submission_lifecycle_records_history_and_terminal_status() {
    let mut prompt = PromptState::new("ready");
    let submission = PromptSubmission::new("run tests")
        .with_stream_id(7)
        .with_session_id("session-1")
        .with_root("/tmp/work")
        .with_model("gpt")
        .with_history_len(3);

    prompt.start_submission(submission);
    assert!(prompt.is_busy());
    assert_eq!(prompt.value, "");
    assert_eq!(prompt.history.entries, vec!["run tests".to_string()]);
    assert!(matches!(
        prompt.active_submission.as_ref().map(|item| &item.status),
        Some(PromptSubmissionStatus::Streaming)
    ));

    prompt.finish_submission(PromptSubmissionStatus::Done { finish_reason: Some("stop".into()) });
    assert_eq!(prompt.mode, PromptMode::Compose);
    assert!(prompt.active_submission.is_none());
    assert!(matches!(
        prompt.last_submission.as_ref().map(|item| &item.status),
        Some(PromptSubmissionStatus::Done { finish_reason }) if finish_reason.as_deref() == Some("stop")
    ));
}

#[test]
fn queued_prompt_commands_are_fifo() {
    let mut prompt = PromptState::default();
    prompt.queue_command(QueuedPromptCommand {
        raw: "first".to_string(),
        kind: QueuedPromptCommandKind::Submit,
        enqueued_ms: Some(1),
    });
    prompt.queue_command(QueuedPromptCommand {
        raw: "/help".to_string(),
        kind: QueuedPromptCommandKind::SlashCommand,
        enqueued_ms: Some(2),
    });

    assert_eq!(prompt.pop_queued_command().map(|command| command.raw), Some("first".to_string()));
    assert_eq!(prompt.pop_queued_command().map(|command| command.raw), Some("/help".to_string()));
    assert_eq!(prompt.pop_queued_command(), None);
}

#[test]
fn prompt_state_boundary_edits_and_vertical_moves_are_noops() {
    let mut prompt = PromptState::new("one\ntwo");
    prompt.cursor.char_index = 0;

    prompt.backspace();
    assert_eq!(prompt.value, "one\ntwo");
    assert!(!prompt.move_cursor(PromptMotion::Up));
    assert_eq!(prompt.cursor.char_index, 0);

    prompt.cursor = PromptCursor::at_end(&prompt.value);
    prompt.delete();
    assert_eq!(prompt.value, "one\ntwo");
    assert!(!prompt.move_cursor(PromptMotion::Down));
    assert_eq!(prompt.cursor.char_index, prompt.value.chars().count());
}
