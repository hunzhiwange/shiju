// 验证任务执行生命周期、日志推进与合并停滞保护。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("execution_tests"));
}

#[test]
fn task_board_does_not_touch_legacy_task_database() {
    let execution_source = include_str!("execution.rs");
    let update_source = include_str!("../update.rs");
    let execution_results_source = include_str!("../update/execution_results.rs");

    for forbidden in [
        "crate::app::task::load_all_tasks(",
        "crate::app::task::update_task(",
        "write_task_execution_result_log(",
        "save_task_execution_result_artifact(",
        "write_task_code_review_result_log(",
        "save_task_code_review_result_artifact(",
        "备份到 SQLite",
        ".agent_scheduler_import_legacy_tasks(",
    ] {
        assert!(
            !execution_source.contains(forbidden)
                && !update_source.contains(forbidden)
                && !execution_results_source.contains(forbidden),
            "task board must not touch the legacy task database through {forbidden}"
        );
    }
}

#[test]
fn non_git_project_schedules_logical_pool_refresh() {
    let (mut app, _task) = crate::app::App::new();
    app.navigation_tabs.project_path = Some("/tmp/non-git-project".to_string());
    app.project_is_git_repo = false;
    app.task_board_worktree_snapshot_loading = false;
    app.task_board_worktree_snapshot = Some(crate::app::task::WorktreePoolSnapshot {
        repo_root: "/tmp/stale".to_string(),
        pool_root: "/tmp/stale/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 1,
        busy_count: 0,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: Vec::new(),
        slots: Vec::new(),
    });
    app.task_board_last_worktree_snapshot_at_ms = 99;

    let refresh = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);

    assert_eq!(refresh.units(), 1);
    assert!(app.task_board_worktree_snapshot_loading);
    assert!(app.task_board_worktree_snapshot.is_some());
    assert_eq!(app.task_board_last_worktree_snapshot_at_ms, 99);

    let maintain = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/non-git-project", 0);
    assert_eq!(maintain.units(), 1);
    assert!(app.task_board_worktree_maintenance_in_flight);

    let recycle = super::recycle_assigned_task_worktree_for_removal(&app, "T-1");
    assert_eq!(recycle.units(), 1);
}

#[test]
fn git_project_still_schedules_worktree_snapshot_refresh() {
    let (mut app, _task) = crate::app::App::new();
    app.navigation_tabs.project_path = Some("/tmp/git-project".to_string());
    app.project_is_git_repo = true;
    app.task_board_worktree_snapshot_loading = false;

    let refresh = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);

    assert!(iced_runtime::task::into_stream(refresh.collect()).is_some());
    assert!(app.task_board_worktree_snapshot_loading);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn persistence_and_removal_helpers_stay_idle_without_local_ownership() {
    let (app, _task) = crate::app::App::new();
    let task = crate::app::task::Task::new(1);

    assert!(super::build_persist_tasks("/project", &[task]).is_empty());

    let recycle = super::recycle_assigned_task_worktree_for_removal(&app, "missing-project");
    assert_eq!(recycle.units(), 0);
}

#[test]
fn task_pool_desktop_scheduler_prepares_logical_capacity_without_worktrees() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_string_lossy().into_owned();
    let (mut app, _) = crate::app::App::new();
    app.navigation_tabs.project_path = Some(path.clone());
    app.project_is_git_repo = false;
    app.task_board_settings.max_concurrent = 7;
    let task = super::schedule_worktree_pool_maintenance(&mut app, &path, 0);
    assert_eq!(task.units(), 1);
    assert!(app.task_board_worktree_maintenance_in_flight);
    assert_eq!(app.task_board_settings.max_concurrent, 7);
    assert!(!directory.path().join(".git").exists());
    assert!(!directory.path().join(".worktrees").exists());
}
