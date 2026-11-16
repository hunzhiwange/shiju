use desktop_session::message::chat::input::task_mode as domain;
use desktop_task_model::TaskSplitMode;
#[test]
fn task_mode_tests_module_is_wired() {
    assert!(module_path!().ends_with("task_mode_tests"));
}

#[test]
fn task_mode_scalar_settings_update_current_runtime() {
    let mut app = desktop_session::session::agent::SessionRuntimeState::<()>::new();

    domain::handle_task_mode_toggled(&mut app.task_mode_enabled, true);
    domain::handle_workflow_mode_toggled(&mut app.workflow_mode_enabled, true);
    domain::handle_task_mode_priority_changed(&mut app.task_mode_priority, "10".to_string());
    domain::handle_task_mode_model_changed(
        &mut app.task_mode_executor,
        &mut app.task_mode_reasoning_effort,
        &mut app.task_mode_model,
        "  gpt-5  ".to_string(),
        &[],
    );
    domain::handle_task_mode_executor_changed(
        &mut app.task_mode_executor,
        Some("codex".to_string()),
    );
    domain::handle_task_mode_reasoning_effort_changed(
        &mut app.task_mode_reasoning_effort,
        Some(vw_api_chat_identity::ReasoningEffortDto::High),
    );
    domain::handle_task_mode_task_split_mode_changed(
        &mut app.task_mode_task_split_mode,
        TaskSplitMode::Enabled,
    );

    let runtime = &app;
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

    domain::handle_task_mode_model_changed(
        &mut app.task_mode_executor,
        &mut app.task_mode_reasoning_effort,
        &mut app.task_mode_model,
        "   ".to_string(),
        &[],
    );
    domain::handle_task_mode_executor_changed(&mut app.task_mode_executor, None);
    domain::handle_task_mode_task_split_mode_changed(
        &mut app.task_mode_task_split_mode,
        TaskSplitMode::Disabled,
    );

    let runtime = &app;
    assert_eq!(runtime.task_mode_model, "auto");
    assert_eq!(runtime.task_mode_executor, None);
    assert_eq!(runtime.task_mode_task_split_mode, TaskSplitMode::Disabled);
}

#[test]
fn goal_mode_stays_enabled_and_independent_from_workflow() {
    let mut app = desktop_session::session::agent::SessionRuntimeState::<()>::new();

    // 默认目标开启；开启工作流不关闭目标
    assert!(app.goal_mode_enabled);
    domain::handle_workflow_mode_toggled(&mut app.workflow_mode_enabled, true);
    assert!(app.workflow_mode_enabled);
    assert!(app.goal_mode_enabled);

    // 关闭请求被忽略，目标模式保持开启
    domain::handle_goal_mode_toggled(&mut app.goal_mode_enabled, false);
    assert!(app.workflow_mode_enabled);
    assert!(app.goal_mode_enabled);

    domain::handle_goal_mode_toggled(&mut app.goal_mode_enabled, true);
    assert!(app.goal_mode_enabled);
    assert!(app.workflow_mode_enabled);

    // 再次开关工作流，目标保持开启
    domain::handle_workflow_mode_toggled(&mut app.workflow_mode_enabled, false);
    domain::handle_workflow_mode_toggled(&mut app.workflow_mode_enabled, true);
    assert!(app.workflow_mode_enabled);
    assert!(app.goal_mode_enabled);
}

#[test]
fn task_mode_reasoning_effort_can_be_cleared() {
    let mut app = desktop_session::session::agent::SessionRuntimeState::<()>::new();
    app.task_mode_reasoning_effort = Some(vw_api_chat_identity::ReasoningEffortDto::High);

    domain::handle_task_mode_reasoning_effort_changed(&mut app.task_mode_reasoning_effort, None);

    assert_eq!(app.task_mode_reasoning_effort, None);
}
