#![allow(unused_must_use)]
use super::{
    handle_goal_mode_toggled, handle_task_mode_executor_changed, handle_task_mode_model_changed,
    handle_task_mode_priority_changed, handle_task_mode_reasoning_effort_changed,
    handle_task_mode_task_split_mode_changed, handle_task_mode_toggled,
    handle_workflow_mode_toggled,
};
use crate::app::App;
use crate::app::task::TaskSplitMode;

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
        Some(vw_api_chat_identity::ReasoningEffortDto::High),
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
        Some(vw_api_chat_identity::ReasoningEffortDto::High)
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
fn goal_mode_stays_enabled_and_independent_from_workflow() {
    let mut app = test_app();

    // 默认目标开启；开启工作流不关闭目标
    assert!(app.current_session_runtime().goal_mode_enabled);
    handle_workflow_mode_toggled(&mut app, true);
    assert!(app.current_session_runtime().workflow_mode_enabled);
    assert!(app.current_session_runtime().goal_mode_enabled);

    // 关闭请求被忽略，目标模式保持开启
    handle_goal_mode_toggled(&mut app, false);
    assert!(app.current_session_runtime().workflow_mode_enabled);
    assert!(app.current_session_runtime().goal_mode_enabled);

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
fn task_mode_reasoning_effort_can_be_cleared() {
    let mut app = test_app();
    app.current_session_runtime_mut().task_mode_reasoning_effort =
        Some(vw_api_chat_identity::ReasoningEffortDto::High);

    handle_task_mode_reasoning_effort_changed(&mut app, None);

    assert_eq!(app.current_session_runtime().task_mode_reasoning_effort, None);
}
