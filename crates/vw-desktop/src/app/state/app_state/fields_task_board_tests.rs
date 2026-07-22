#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("fields_task_board_tests"));
}

#[test]
fn task_board_matrix_inbound_state_defaults_are_empty() {
    let app = crate::app::App::new().0;

    assert_eq!(app.task_board_matrix_event_cursor, 0);
    assert!(app.task_board_processed_matrix_event_ids.is_empty());
}
