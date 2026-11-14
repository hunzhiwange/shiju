use super::*;

#[test]
fn visible_history_rows_are_cached_for_resize_replay() {
    let mut state = TranscriptReflowState::default();

    state.set_visible_history_rows(/*rows*/ 19);

    assert_eq!(state.visible_history_rows(), Some(19));
}

#[test]
fn visible_history_rows_keep_one_row_for_a_full_height_composer() {
    let mut state = TranscriptReflowState::default();

    state.set_visible_history_rows(/*rows*/ 0);

    assert_eq!(state.visible_history_rows(), Some(1));
}

#[test]
fn schedule_debounced_postpones_existing_reflow() {
    let mut state = TranscriptReflowState::default();

    assert!(!state.schedule_debounced(/*target_width*/ None));
    let first_deadline = state.pending_until().expect("pending reflow");

    std::thread::sleep(Duration::from_millis(1));
    assert!(!state.schedule_debounced(/*target_width*/ None));

    assert!(
        state.pending_until().expect("pending reflow") > first_deadline,
        "a later resize should push the debounce deadline out"
    );
}

#[test]
fn schedule_debounced_postpones_due_existing_reflow() {
    let mut state = TranscriptReflowState::default();
    state.set_due_for_test();
    let before_reschedule = Instant::now();

    assert!(!state.schedule_debounced(/*target_width*/ None));
    assert!(
        state.pending_until().expect("pending reflow") > before_reschedule,
        "a resize after the old deadline should start a fresh quiet period"
    );
}

#[test]
fn first_observed_width_marks_reflow_baseline() {
    let mut state = TranscriptReflowState::default();

    let width = state.note_width(/*width*/ 80);

    assert!(width.initialized);
    assert_eq!(state.last_observed_width, Some(80));
    assert_eq!(state.last_reflow_width, Some(80));
    assert!(!state.reflow_needed_for_width(/*width*/ 80));
}

#[test]
fn mark_reflowed_width_records_actual_rebuild_width() {
    let mut state = TranscriptReflowState::default();
    state.note_width(/*width*/ 80);

    assert!(state.mark_reflowed_width(/*width*/ 100));

    assert_eq!(state.last_observed_width, Some(80));
    assert_eq!(state.last_reflow_width, Some(100));
}

#[test]
fn reflow_needed_compares_against_actual_rebuild_width() {
    let mut state = TranscriptReflowState::default();
    state.note_width(/*width*/ 80);
    state.mark_reflowed_width(/*width*/ 90);
    state.note_width(/*width*/ 100);

    assert!(state.reflow_needed_for_width(/*width*/ 100));
}

#[test]
fn pending_reflow_target_prevents_repeated_reschedule() {
    let mut state = TranscriptReflowState::default();
    state.note_width(/*width*/ 80);

    assert!(state.reflow_needed_for_width(/*width*/ 100));
    state.schedule_debounced(/*target_width*/ Some(100));

    assert!(!state.reflow_needed_for_width(/*width*/ 100));
}

#[test]
fn clear_pending_reflow_allows_same_width_to_be_rescheduled() {
    let mut state = TranscriptReflowState::default();
    state.note_width(/*width*/ 80);
    state.schedule_debounced(/*target_width*/ Some(100));

    state.clear_pending_reflow();

    assert!(state.reflow_needed_for_width(/*width*/ 100));
}

#[test]
fn mark_reflowed_width_reports_unchanged_width() {
    let mut state = TranscriptReflowState::default();
    assert!(state.mark_reflowed_width(/*width*/ 100));

    assert!(!state.mark_reflowed_width(/*width*/ 100));
    assert_eq!(state.last_reflow_width, Some(100));
}

#[test]
fn take_stream_finish_reflow_needed_drains_resize_request() {
    let mut state = TranscriptReflowState::default();
    state.mark_resize_requested_during_stream();

    assert!(state.take_stream_finish_reflow_needed());
    assert!(!state.take_stream_finish_reflow_needed());
}

#[test]
fn take_stream_finish_reflow_needed_drains_ran_during_stream() {
    let mut state = TranscriptReflowState::default();
    state.mark_ran_during_stream();

    assert!(state.take_stream_finish_reflow_needed());
    assert!(!state.take_stream_finish_reflow_needed());
}

#[test]
fn clear_resets_stream_reflow_flags() {
    let mut state = TranscriptReflowState::default();
    state.mark_ran_during_stream();
    state.mark_resize_requested_during_stream();

    state.clear();

    assert!(!state.take_stream_finish_reflow_needed());
}
