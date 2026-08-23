//! 任务执行器状态测试，验证并发容量、运行任务集合和日志通道生命周期。
//! 启动与结束操作需保持幂等，轮询只能消费对应任务的已注册日志。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("state_tests"));
}

fn legacy_worktree_root(repo_root: &str) -> String {
    format!(
        "{repo_root}/{}/{}",
        vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME,
        vw_config_types::paths::TASK_WORKTREES_DIR_NAME
    )
}

#[test]
fn default_state_uses_expected_limits_and_empty_collections() {
    let state = super::TaskExecutorState::new();

    assert!(state.running_tasks.is_empty());
    assert_eq!(state.max_concurrent, 3);
    assert_eq!(state.simulation_delay_ms, 2000);
    assert!(state.log_receivers.is_empty());
    assert!(state.can_start_more());
}

#[test]
fn start_task_is_idempotent_and_can_start_more_respects_limit() {
    let mut state = super::TaskExecutorState::new();
    state.max_concurrent = 2;

    state.start_task("a");
    state.start_task("a");
    assert_eq!(state.running_tasks, vec!["a".to_string()]);
    assert!(state.is_running("a"));
    assert!(state.can_start_more());

    state.start_task("b");
    assert!(!state.can_start_more());
}

#[test]
fn zero_concurrency_reports_no_capacity_even_without_running_tasks() {
    let mut state = super::TaskExecutorState::new();
    state.max_concurrent = 0;

    assert!(state.running_tasks.is_empty());
    assert!(!state.can_start_more());
}

#[test]
fn finish_task_removes_running_entry_and_log_channels() {
    let mut state = super::TaskExecutorState::new();
    state.start_task("a");
    state.register_log_channel("a".to_string());

    assert!(state.get_log_sender("a").is_some());
    state.finish_task("a");

    assert!(!state.is_running("a"));
    assert!(state.get_log_sender("a").is_none());
    assert!(!state.log_receivers.contains_key("a"));
}

#[test]
fn finish_unknown_task_preserves_other_task_and_channel() {
    let mut state = super::TaskExecutorState::new();
    state.start_task("active");
    state.register_log_channel("active".to_string());

    state.finish_task("missing");

    assert!(state.is_running("active"));
    assert!(state.get_log_sender("active").is_some());
    assert!(state.log_receivers.contains_key("active"));
}

#[test]
fn registering_channel_again_replaces_both_channel_ends() {
    let mut state = super::TaskExecutorState::new();
    state.register_log_channel("a".to_string());
    let stale_sender = state.get_log_sender("a").expect("first sender should exist");

    state.register_log_channel("a".to_string());
    let current_sender = state.get_log_sender("a").expect("replacement sender should exist");

    assert!(stale_sender.send(super::TaskLogStream::Stdout("stale".to_string())).is_err());
    current_sender
        .send(super::TaskLogStream::Stdout("current".to_string()))
        .expect("replacement channel should receive");
    let logs = state.poll_task_logs_all("a");
    assert!(matches!(&logs[..], [super::TaskLogStream::Stdout(value)] if value == "current"));
}

#[test]
fn poll_task_logs_drains_registered_channel() {
    let mut state = super::TaskExecutorState::new();
    state.register_log_channel("a".to_string());
    let sender = state.get_log_sender("a").expect("sender should exist");
    sender.send(super::TaskLogStream::Stdout("one".to_string())).expect("stdout should send");
    sender.send(super::TaskLogStream::Stderr("two".to_string())).expect("stderr should send");

    let logs = state.poll_task_logs("a");

    assert_eq!(logs.len(), 2);
    assert!(matches!(&logs[0], super::TaskLogStream::Stdout(value) if value == "one"));
    assert!(matches!(&logs[1], super::TaskLogStream::Stderr(value) if value == "two"));
    assert!(state.poll_task_logs("a").is_empty());
}

#[test]
fn poll_task_logs_for_unknown_task_is_empty() {
    let mut state = super::TaskExecutorState::new();

    assert!(state.poll_task_logs("missing").is_empty());
}

#[test]
fn get_all_running_logs_returns_only_tasks_with_logs() {
    let mut state = super::TaskExecutorState::new();
    state.start_task("a");
    state.start_task("b");
    state.register_log_channel("a".to_string());
    state.register_log_channel("b".to_string());
    state
        .get_log_sender("b")
        .expect("sender should exist")
        .send(super::TaskLogStream::ExitStatus { success: true, code: Some(0), signal: None })
        .expect("exit status should send");

    let logs = state.get_all_running_logs();

    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].0, "b");
    assert!(matches!(
        logs[0].1[0],
        super::TaskLogStream::ExitStatus { success: true, code: Some(0), signal: None }
    ));
}

#[test]
fn get_all_running_logs_leaves_non_running_task_logs_queued() {
    let mut state = super::TaskExecutorState::new();
    state.register_log_channel("queued".to_string());
    state
        .get_log_sender("queued")
        .expect("sender should exist")
        .send(super::TaskLogStream::Stdout("later".to_string()))
        .expect("log should send");

    assert!(state.get_all_running_logs().is_empty());
    assert!(matches!(
        &state.poll_task_logs("queued")[..],
        [super::TaskLogStream::Stdout(value)] if value == "later"
    ));
}

#[test]
fn task_log_stream_variants_keep_payloads() {
    let variants = vec![
        super::TaskLogStream::SubTaskStarted {
            subtask_id: "s1".to_string(),
            content: "do work".to_string(),
            timestamp_ms: 10,
        },
        super::TaskLogStream::SubTaskCompleted { subtask_id: "s2".to_string(), timestamp_ms: 20 },
        super::TaskLogStream::SubTaskFailed {
            subtask_id: "s3".to_string(),
            error: "bad".to_string(),
            timestamp_ms: 30,
        },
    ];

    assert!(matches!(
        &variants[0],
        super::TaskLogStream::SubTaskStarted { subtask_id, content, timestamp_ms }
            if subtask_id == "s1" && content == "do work" && *timestamp_ms == 10
    ));
    assert!(matches!(
        &variants[1],
        super::TaskLogStream::SubTaskCompleted { subtask_id, timestamp_ms }
            if subtask_id == "s2" && *timestamp_ms == 20
    ));
    assert!(matches!(
        &variants[2],
        super::TaskLogStream::SubTaskFailed { subtask_id, error, timestamp_ms }
            if subtask_id == "s3" && error == "bad" && *timestamp_ms == 30
    ));
}

#[test]
fn worktree_state_is_copy_and_comparable() {
    let state = super::WorktreeState::Idle;
    let copied = state;

    assert_eq!(copied, super::WorktreeState::Idle);
    assert_ne!(super::WorktreeState::Busy, super::WorktreeState::Dead);
}

#[test]
fn worktree_state_maps_every_gateway_variant() {
    use vw_gateway_client::vw_api_types::task::TaskPoolWorktreeSlotStateDto as GatewayState;

    let cases = [
        (GatewayState::Idle, super::WorktreeState::Idle),
        (GatewayState::Busy, super::WorktreeState::Busy),
        (GatewayState::Tainted, super::WorktreeState::Tainted),
        (GatewayState::Recycling, super::WorktreeState::Recycling),
        (GatewayState::Dead, super::WorktreeState::Dead),
    ];

    for (gateway, expected) in cases {
        assert_eq!(super::WorktreeState::from_gateway(gateway), expected);
    }
}

#[test]
fn gateway_snapshot_conversion_preserves_empty_boundary() {
    use vw_gateway_client::vw_api_types::id::ProjectId;
    use vw_gateway_client::vw_api_types::task::TaskPoolWorktreeSnapshotResponse;

    let snapshot =
        super::WorktreePoolSnapshot::from_gateway_response(TaskPoolWorktreeSnapshotResponse {
            project_id: ProjectId("project-empty".to_string()),
            repo_root: "/repo".to_string(),
            pool_root: "/pool".to_string(),
            base_branch: "main".to_string(),
            idle_count: 0,
            busy_count: 0,
            tainted_count: 0,
            recycling_count: 0,
            dead_count: 0,
            merge_target_locks: Vec::new(),
            slots: Vec::new(),
        });

    assert_eq!(snapshot.repo_root, "/repo");
    assert_eq!(snapshot.pool_root, "/pool");
    assert_eq!(snapshot.base_branch, "main");
    assert_eq!(
        (
            snapshot.idle_count,
            snapshot.busy_count,
            snapshot.tainted_count,
            snapshot.recycling_count,
            snapshot.dead_count,
        ),
        (0, 0, 0, 0, 0)
    );
    assert!(snapshot.merge_target_locks.is_empty());
    assert!(snapshot.slots.is_empty());
}

#[test]
fn public_worktree_snapshots_keep_counts_and_slots() {
    let slot = super::WorktreeSlotSnapshot {
        id: "slot-1".to_string(),
        path: "/slot".to_string(),
        branch: "vw/task/1".to_string(),
        state: super::WorktreeState::Tainted,
        leased_task_id: None,
        taint_reason: Some("conflict".to_string()),
    };
    let pool_root = legacy_worktree_root("/repo");
    let snapshot = super::WorktreePoolSnapshot {
        repo_root: "/repo".to_string(),
        pool_root: pool_root.clone(),
        base_branch: "main".to_string(),
        idle_count: 1,
        busy_count: 2,
        tainted_count: 3,
        recycling_count: 4,
        dead_count: 5,
        merge_target_locks: vec![("main".to_string(), "task-1".to_string())],
        slots: vec![slot],
    };

    assert_eq!(snapshot.repo_root, "/repo");
    assert_eq!(snapshot.pool_root, pool_root);
    assert_eq!(snapshot.base_branch, "main");
    assert_eq!(snapshot.idle_count + snapshot.busy_count + snapshot.tainted_count, 6);
    assert_eq!(snapshot.recycling_count, 4);
    assert_eq!(snapshot.dead_count, 5);
    assert_eq!(snapshot.merge_target_locks[0].1, "task-1");
    assert_eq!(snapshot.slots[0].state, super::WorktreeState::Tainted);
    assert_eq!(snapshot.slots[0].id, "slot-1");
    assert_eq!(snapshot.slots[0].path, "/slot");
    assert_eq!(snapshot.slots[0].branch, "vw/task/1");
    assert_eq!(snapshot.slots[0].leased_task_id, None);
    assert_eq!(snapshot.slots[0].taint_reason.as_deref(), Some("conflict"));
}

#[test]
fn selected_workspace_and_worktree_entry_keep_values() {
    let entry = super::WorktreeEntry {
        path: "/repo/worktree".to_string(),
        branch: Some("feature".to_string()),
    };
    assert_eq!(entry.path, "/repo/worktree");
    assert_eq!(entry.branch.as_deref(), Some("feature"));

    let workspace = super::SelectedExecutionWorkspace {
        slot_id: Some("slot-1".to_string()),
        execution_path: "/exec".to_string(),
        selected_worktree_path: Some("/exec".to_string()),
        selected_worktree_branch: Some("feature".to_string()),
        merge_target_branch: Some("main".to_string()),
        project_path: "/repo".to_string(),
    };
    assert_eq!(workspace.slot_id.as_deref(), Some("slot-1"));
    assert_eq!(workspace.execution_path, "/exec");
    assert_eq!(workspace.selected_worktree_path.as_deref(), Some("/exec"));
    assert_eq!(workspace.selected_worktree_branch.as_deref(), Some("feature"));
    assert_eq!(workspace.merge_target_branch.as_deref(), Some("main"));
    assert_eq!(workspace.project_path, "/repo");
}

#[test]
fn global_state_maps_are_initializable() {
    let claimed = super::claimed_worktrees();

    drop(claimed.lock().expect("claimed lock should open"));
}

#[test]
fn claim_guard_without_path_does_not_release_other_claims() {
    let path = "state-tests-unrelated-claim".to_string();
    {
        let mut claimed = super::claimed_worktrees().lock().expect("claimed lock should open");
        claimed.insert(path.clone());
    }

    drop(super::WorktreeClaimGuard::new(None));

    let mut claimed = super::claimed_worktrees().lock().expect("claimed lock should open");
    assert!(claimed.remove(&path));
}

#[test]
fn claim_guard_with_path_releases_matching_claim_on_drop() {
    let path = "state-tests-claimed-path".to_string();
    {
        let mut claimed = super::claimed_worktrees().lock().expect("claimed lock should open");
        claimed.insert(path.clone());
    }

    drop(super::WorktreeClaimGuard::new(Some(path.clone())));

    let claimed = super::claimed_worktrees().lock().expect("claimed lock should open");
    assert!(!claimed.contains(&path));
}

#[test]
fn git_state_constants_are_stable() {
    assert_eq!(super::GIT_MAINTENANCE_COMMAND_TIMEOUT_SECS, 30);
    assert_eq!(super::GIT_SUMMARY_TAG, "__VW_GIT_SUMMARY__");
    assert_eq!(super::GIT_SOURCE_BRANCH_TAG, "__VW_GIT_SOURCE_BRANCH__");
    assert_eq!(super::GIT_TARGET_BRANCH_TAG, "__VW_GIT_TARGET_BRANCH__");
    assert_eq!(super::GIT_WORKTREE_PATH_TAG, "__VW_GIT_WORKTREE_PATH__");
}
