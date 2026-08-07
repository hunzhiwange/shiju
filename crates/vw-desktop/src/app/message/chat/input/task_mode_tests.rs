#![allow(unused_must_use)]
use super::{
    handle_task_mode_add_subtask, handle_task_mode_executor_changed,
    handle_task_mode_model_changed, handle_task_mode_move_subtask_down,
    handle_task_mode_move_subtask_up, handle_task_mode_priority_changed,
    handle_task_mode_reasoning_effort_changed, handle_task_mode_remove_subtask,
    handle_task_mode_subtask_changed, handle_task_mode_subtask_editor_action,
    handle_task_mode_task_split_mode_changed, handle_task_mode_toggled,
    handle_goal_mode_toggled, handle_subagent_mode_toggled, handle_workflow_mode_toggled,
};
use crate::app::App;
use crate::app::task::TaskSplitMode;
use iced::widget::text_editor;

fn test_app() -> App {
    App::new().0
}

#[test]
fn task_mode_tests_module_is_wired() {
    assert!(module_path!().ends_with("task_mode_tests"));
}

#[test]
fn task_mode_scalar_settings_update_current_runtime() {
    let mut app = test_app();

    handle_task_mode_toggled(&mut app, true);
    handle_workflow_mode_toggled(&mut app, true);
    handle_task_mode_priority_changed(&mut app, "10".to_string());
    handle_task_mode_model_changed(&mut app, "  gpt-5  ".to_string());
    handle_task_mode_executor_changed(&mut app, Some("codex".to_string()));
    handle_task_mode_reasoning_effort_changed(
        &mut app,
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High),
    );
    handle_task_mode_task_split_mode_changed(&mut app, TaskSplitMode::Enabled);

    let runtime = app.current_session_runtime();
    assert!(runtime.task_mode_enabled);
    assert!(runtime.workflow_mode_enabled);
    assert_eq!(runtime.task_mode_priority, "10");
    assert_eq!(runtime.task_mode_model, "gpt-5");
    assert_eq!(runtime.task_mode_executor.as_deref(), Some("codex"));
    assert_eq!(
        runtime.task_mode_reasoning_effort,
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High)
    );
    assert_eq!(runtime.task_mode_task_split_mode, TaskSplitMode::Enabled);

    handle_task_mode_model_changed(&mut app, "   ".to_string());
    handle_task_mode_executor_changed(&mut app, None);
    handle_task_mode_task_split_mode_changed(&mut app, TaskSplitMode::Disabled);

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_model, "auto");
    assert_eq!(runtime.task_mode_executor, None);
    assert_eq!(runtime.task_mode_task_split_mode, TaskSplitMode::Disabled);
}

#[test]
fn goal_and_workflow_modes_can_both_be_enabled() {
    let mut app = test_app();

    // 默认目标开启；开启工作流不关闭目标
    assert!(app.current_session_runtime().goal_mode_enabled);
    handle_workflow_mode_toggled(&mut app, true);
    assert!(app.current_session_runtime().workflow_mode_enabled);
    assert!(app.current_session_runtime().goal_mode_enabled);

    // 关闭目标后重新开启，不关闭工作流
    handle_goal_mode_toggled(&mut app, false);
    assert!(app.current_session_runtime().workflow_mode_enabled);
    assert!(!app.current_session_runtime().goal_mode_enabled);

    handle_goal_mode_toggled(&mut app, true);
    assert!(app.current_session_runtime().goal_mode_enabled);
    assert!(app.current_session_runtime().workflow_mode_enabled);

    // 再次开关工作流，目标保持开启
    handle_workflow_mode_toggled(&mut app, false);
    handle_workflow_mode_toggled(&mut app, true);
    assert!(app.current_session_runtime().workflow_mode_enabled);
    assert!(app.current_session_runtime().goal_mode_enabled);
}

#[test]
fn subagent_mode_toggle_is_independent_from_workflow_and_goal_modes() {
    let mut app = test_app();

    handle_goal_mode_toggled(&mut app, true);
    handle_subagent_mode_toggled(&mut app, true);
    let runtime = app.current_session_runtime();
    assert!(runtime.goal_mode_enabled);
    assert!(runtime.subagent_mode_enabled);

    handle_subagent_mode_toggled(&mut app, false);
    let runtime = app.current_session_runtime();
    assert!(runtime.goal_mode_enabled);
    assert!(!runtime.subagent_mode_enabled);
}

#[test]
fn task_mode_reasoning_effort_can_be_cleared() {
    let mut app = test_app();
    app.current_session_runtime_mut().task_mode_reasoning_effort =
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High);

    handle_task_mode_reasoning_effort_changed(&mut app, None);

    assert_eq!(app.current_session_runtime().task_mode_reasoning_effort, None);
}

#[test]
fn task_mode_subtask_text_and_editor_stay_in_sync() {
    let mut app = test_app();

    handle_task_mode_subtask_changed(&mut app, 1, "write tests".to_string());

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_subtasks[1], "write tests");
    assert_eq!(runtime.task_mode_subtask_editors[1].text(), "write tests");

    handle_task_mode_subtask_changed(&mut app, 99, "ignored".to_string());
    assert_eq!(app.current_session_runtime().task_mode_subtasks.len(), 3);

    handle_task_mode_subtask_editor_action(
        &mut app,
        1,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " now".to_string(),
        ))),
    );

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_subtask_editors[1].text(), "write tests now");
    assert_eq!(runtime.task_mode_subtasks[1], "write tests now");

    let _task =
        handle_task_mode_subtask_editor_action(&mut app, 99, text_editor::Action::SelectAll);
}

#[test]
fn task_mode_subtask_add_remove_and_move_preserve_editor_order() {
    let mut app = test_app();
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks = vec!["one".into(), "two".into(), "three".into()];
        runtime.task_mode_subtask_editors = runtime
            .task_mode_subtasks
            .iter()
            .map(|value| text_editor::Content::with_text(value))
            .collect();
    }

    handle_task_mode_add_subtask(&mut app);
    assert_eq!(app.current_session_runtime().task_mode_subtasks, ["one", "two", "three", ""]);

    handle_task_mode_move_subtask_up(&mut app, 2);
    assert_eq!(app.current_session_runtime().task_mode_subtasks, ["one", "three", "two", ""]);
    assert_eq!(app.current_session_runtime().task_mode_subtask_editors[1].text(), "three");

    handle_task_mode_move_subtask_down(&mut app, 1);
    assert_eq!(app.current_session_runtime().task_mode_subtasks, ["one", "two", "three", ""]);
    assert_eq!(app.current_session_runtime().task_mode_subtask_editors[2].text(), "three");

    handle_task_mode_remove_subtask(&mut app, 1);
    assert_eq!(app.current_session_runtime().task_mode_subtasks, ["one", "three", ""]);

    handle_task_mode_remove_subtask(&mut app, 99);
    assert_eq!(app.current_session_runtime().task_mode_subtasks, ["one", "three", ""]);

    for _ in 0..3 {
        handle_task_mode_remove_subtask(&mut app, 0);
    }

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_subtasks, [""]);
    assert_eq!(runtime.task_mode_subtask_editors.len(), 1);
}

#[test]
fn task_mode_subtask_change_updates_available_storage_independently() {
    let mut app = test_app();
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks = vec!["old task".into()];
        runtime.task_mode_subtask_editors.clear();
    }

    handle_task_mode_subtask_changed(&mut app, 0, "new task".to_string());

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_subtasks, ["new task"]);
    assert!(runtime.task_mode_subtask_editors.is_empty());

    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks.clear();
        runtime.task_mode_subtask_editors = vec![text_editor::Content::with_text("old editor")];
    }

    handle_task_mode_subtask_changed(&mut app, 0, "new editor".to_string());

    let runtime = app.current_session_runtime();
    assert!(runtime.task_mode_subtasks.is_empty());
    assert_eq!(runtime.task_mode_subtask_editors[0].text(), "new editor");
}

#[test]
fn task_mode_editor_action_backfills_missing_editors_before_editing() {
    let mut app = test_app();
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks = vec!["first".into(), "second".into()];
        runtime.task_mode_subtask_editors.clear();
    }

    handle_task_mode_subtask_editor_action(
        &mut app,
        1,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            "replacement".to_string(),
        ))),
    );

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_subtask_editors.len(), 2);
    assert_eq!(runtime.task_mode_subtask_editors[0].text(), "");
    assert_eq!(runtime.task_mode_subtask_editors[1].text(), "replacement");
    assert_eq!(runtime.task_mode_subtasks, ["first", "replacement"]);
}

#[test]
fn task_mode_remove_subtask_tolerates_missing_editor() {
    let mut app = test_app();
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks = vec!["first".into(), "second".into()];
        runtime.task_mode_subtask_editors.clear();
    }

    handle_task_mode_remove_subtask(&mut app, 0);

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_subtasks, ["second"]);
    assert!(runtime.task_mode_subtask_editors.is_empty());
}

#[test]
fn task_mode_move_subtask_respects_boundaries_with_missing_editors() {
    let mut app = test_app();
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks = vec!["one".into(), "two".into(), "three".into()];
        runtime.task_mode_subtask_editors = vec![text_editor::Content::with_text("one")];
    }

    handle_task_mode_move_subtask_up(&mut app, 0);
    handle_task_mode_move_subtask_up(&mut app, 99);
    handle_task_mode_move_subtask_down(&mut app, 2);
    assert_eq!(app.current_session_runtime().task_mode_subtasks, ["one", "two", "three"]);

    handle_task_mode_move_subtask_up(&mut app, 2);
    assert_eq!(app.current_session_runtime().task_mode_subtasks, ["one", "three", "two"]);

    handle_task_mode_move_subtask_down(&mut app, 0);
    let runtime = app.current_session_runtime();
    assert_eq!(runtime.task_mode_subtasks, ["three", "one", "two"]);
    assert_eq!(runtime.task_mode_subtask_editors.len(), 1);
    assert_eq!(runtime.task_mode_subtask_editors[0].text(), "one");
}
