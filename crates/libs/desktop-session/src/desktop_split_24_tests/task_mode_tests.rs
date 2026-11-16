use crate::message::chat::input::task_mode::*;
use iced::widget::text_editor;
#[test]
fn task_mode_subtask_text_and_editor_stay_in_sync() {
    let mut tasks = vec![String::new(); 3];
    let mut editors = (0..3).map(|_| text_editor::Content::new()).collect::<Vec<_>>();

    handle_task_mode_subtask_changed(&mut tasks, &mut editors, 1, "write tests".to_string());
    assert_eq!(tasks[1], "write tests");
    assert_eq!(editors[1].text(), "write tests");

    handle_task_mode_subtask_changed(&mut tasks, &mut editors, 99, "ignored".to_string());
    assert_eq!(tasks.len(), 3);

    handle_task_mode_subtask_editor_action(
        &mut tasks,
        &mut editors,
        1,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " now".to_string(),
        ))),
    );
    assert_eq!(editors[1].text(), "write tests now");
    assert_eq!(tasks[1], "write tests now");

    let _task = handle_task_mode_subtask_editor_action(
        &mut tasks,
        &mut editors,
        99,
        text_editor::Action::SelectAll,
    );
}

#[test]
fn task_mode_subtask_add_remove_and_move_preserve_editor_order() {
    let mut tasks: Vec<String>;
    let mut editors: Vec<text_editor::Content>;
    {
        tasks = vec!["one".into(), "two".into(), "three".into()];
        editors = tasks.iter().map(|value| text_editor::Content::with_text(value)).collect();
    }

    handle_task_mode_add_subtask(&mut tasks, &mut editors);
    assert_eq!(tasks, ["one", "two", "three", ""]);

    handle_task_mode_move_subtask_up(&mut tasks, &mut editors, 2);
    assert_eq!(tasks, ["one", "three", "two", ""]);
    assert_eq!(editors[1].text(), "three");

    handle_task_mode_move_subtask_down(&mut tasks, &mut editors, 1);
    assert_eq!(tasks, ["one", "two", "three", ""]);
    assert_eq!(editors[2].text(), "three");

    handle_task_mode_remove_subtask(&mut tasks, &mut editors, 1);
    assert_eq!(tasks, ["one", "three", ""]);

    handle_task_mode_remove_subtask(&mut tasks, &mut editors, 99);
    assert_eq!(tasks, ["one", "three", ""]);

    for _ in 0..3 {
        handle_task_mode_remove_subtask(&mut tasks, &mut editors, 0);
    }
    assert_eq!(tasks, [""]);
    assert_eq!(editors.len(), 1);
}

#[test]
fn task_mode_subtask_change_updates_available_storage_independently() {
    let mut tasks: Vec<String>;
    let mut editors: Vec<text_editor::Content>;
    {
        tasks = vec!["old task".into()];
        editors.clear();
    }

    handle_task_mode_subtask_changed(&mut tasks, &mut editors, 0, "new task".to_string());
    assert_eq!(tasks, ["new task"]);
    assert!(editors.is_empty());

    {
        tasks.clear();
        editors = vec![text_editor::Content::with_text("old editor")];
    }

    handle_task_mode_subtask_changed(&mut tasks, &mut editors, 0, "new editor".to_string());
    assert!(tasks.is_empty());
    assert_eq!(editors[0].text(), "new editor");
}

#[test]
fn task_mode_editor_action_backfills_missing_editors_before_editing() {
    let mut tasks: Vec<String>;
    let mut editors = Vec::new();
    {
        tasks = vec!["first".into(), "second".into()];
        editors.clear();
    }

    handle_task_mode_subtask_editor_action(
        &mut tasks,
        &mut editors,
        1,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            "replacement".to_string(),
        ))),
    );
    assert_eq!(editors.len(), 2);
    assert_eq!(editors[0].text(), "");
    assert_eq!(editors[1].text(), "replacement");
    assert_eq!(tasks, ["first", "replacement"]);
}

#[test]
fn task_mode_remove_subtask_tolerates_missing_editor() {
    let mut tasks: Vec<String>;
    let mut editors = Vec::new();
    {
        tasks = vec!["first".into(), "second".into()];
        editors.clear();
    }

    handle_task_mode_remove_subtask(&mut tasks, &mut editors, 0);
    assert_eq!(tasks, ["second"]);
    assert!(editors.is_empty());
}

#[test]
fn task_mode_move_subtask_respects_boundaries_with_missing_editors() {
    let mut tasks: Vec<String>;
    let mut editors: Vec<text_editor::Content>;
    {
        tasks = vec!["one".into(), "two".into(), "three".into()];
        editors = vec![text_editor::Content::with_text("one")];
    }

    handle_task_mode_move_subtask_up(&mut tasks, &mut editors, 0);
    handle_task_mode_move_subtask_up(&mut tasks, &mut editors, 99);
    handle_task_mode_move_subtask_down(&mut tasks, &mut editors, 2);
    assert_eq!(tasks, ["one", "two", "three"]);

    handle_task_mode_move_subtask_up(&mut tasks, &mut editors, 2);
    assert_eq!(tasks, ["one", "three", "two"]);

    handle_task_mode_move_subtask_down(&mut tasks, &mut editors, 0);
    assert_eq!(tasks, ["three", "one", "two"]);
    assert_eq!(editors.len(), 1);
    assert_eq!(editors[0].text(), "one");
}
