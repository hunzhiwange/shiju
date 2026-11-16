#[test]
fn main_window_closed_exits_even_when_task_pet_is_open() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = Some(main_window_id);
    let mut task_pet_window = Some(task_pet_window_id);

    let outcome = crate::window_lifecycle::mark_window_closed(
        &mut main_window,
        &mut task_pet_window,
        main_window_id,
    );

    assert_eq!(outcome, crate::window_lifecycle::WindowClosedOutcome::Exit);
    assert_eq!(main_window, None);
    assert_eq!(task_pet_window, None);
}

#[test]
fn task_pet_window_closed_keeps_main_window_running() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = Some(main_window_id);
    let mut task_pet_window = Some(task_pet_window_id);

    let outcome = crate::window_lifecycle::mark_window_closed(
        &mut main_window,
        &mut task_pet_window,
        task_pet_window_id,
    );

    assert_eq!(outcome, crate::window_lifecycle::WindowClosedOutcome::Continue);
    assert_eq!(main_window, Some(main_window_id));
    assert_eq!(task_pet_window, None);
}

#[test]
fn closing_last_or_already_absent_window_exits() {
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = None;
    let mut task_pet_window = Some(task_pet_window_id);

    let task_pet_outcome = crate::window_lifecycle::mark_window_closed(
        &mut main_window,
        &mut task_pet_window,
        task_pet_window_id,
    );
    let absent_outcome = crate::window_lifecycle::mark_window_closed(
        &mut main_window,
        &mut task_pet_window,
        iced::window::Id::unique(),
    );

    assert_eq!(task_pet_outcome, crate::window_lifecycle::WindowClosedOutcome::Exit);
    assert_eq!(absent_outcome, crate::window_lifecycle::WindowClosedOutcome::Exit);
    assert_eq!(main_window, None);
    assert_eq!(task_pet_window, None);
}

#[test]
fn closing_unknown_window_preserves_open_windows() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = Some(main_window_id);
    let mut task_pet_window = Some(task_pet_window_id);

    let outcome = crate::window_lifecycle::mark_window_closed(
        &mut main_window,
        &mut task_pet_window,
        iced::window::Id::unique(),
    );

    assert_eq!(outcome, crate::window_lifecycle::WindowClosedOutcome::Continue);
    assert_eq!(main_window, Some(main_window_id));
    assert_eq!(task_pet_window, Some(task_pet_window_id));
}
#[test]
fn main_window_close_request_closes_task_pet_too() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();

    let windows = crate::window_lifecycle::close_requested_windows(
        Some(main_window_id),
        Some(task_pet_window_id),
        main_window_id,
    );

    assert_eq!(windows.main_window_id, Some(main_window_id));
    assert_eq!(windows.task_pet_window_id, Some(task_pet_window_id));
}

#[test]
fn task_pet_close_request_only_closes_task_pet() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();

    let windows = crate::window_lifecycle::close_requested_windows(
        Some(main_window_id),
        Some(task_pet_window_id),
        task_pet_window_id,
    );

    assert_eq!(windows.main_window_id, None);
    assert_eq!(windows.task_pet_window_id, Some(task_pet_window_id));
}

#[test]
fn unknown_window_close_request_closes_nothing() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();

    let windows = crate::window_lifecycle::close_requested_windows(
        Some(main_window_id),
        Some(task_pet_window_id),
        iced::window::Id::unique(),
    );

    assert_eq!(windows.main_window_id, None);
    assert_eq!(windows.task_pet_window_id, None);
}

#[test]
fn shared_window_id_is_closed_only_once() {
    let shared_window_id = iced::window::Id::unique();

    let windows = crate::window_lifecycle::close_requested_windows(
        Some(shared_window_id),
        Some(shared_window_id),
        shared_window_id,
    );

    assert_eq!(windows.main_window_id, Some(shared_window_id));
    assert_eq!(windows.task_pet_window_id, None);
}
