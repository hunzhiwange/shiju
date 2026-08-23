//! 工作树池测试，覆盖 Git 列表解析、分支令牌净化和受管路径判定。
//! 空记录、游离头及旧版目录均有明确兼容边界，非槽位路径必须拒绝。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("worktree_pool_tests"));
}

#[test]
fn desktop_task_executor_no_local_worktree_pool_exports() {
    let executor_mod = include_str!("mod.rs");
    let state_mod = include_str!("state.rs");
    let worktree_admin_mod = include_str!("worktree_admin.rs");

    assert!(!executor_mod.contains("pub use worktree_pool"));
    assert!(!state_mod.contains("WORKTREE_POOLS"));
    assert!(!state_mod.contains("RepoWorktreePool"));
    assert!(!worktree_admin_mod.contains("worktree_pools"));
}

fn legacy_worktree_path(repo_root: &str, leaf: &str) -> String {
    format!(
        "{repo_root}/{}/{}/{leaf}",
        vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME,
        vw_config_types::paths::TASK_WORKTREES_DIR_NAME
    )
}

#[test]
fn parse_worktree_list_handles_branches_detached_heads_and_blank_records() {
    let slot_path = legacy_worktree_path("/repo", "slot-1");
    let output = format!(
        "\
worktree /repo
HEAD abc
branch refs/heads/main

worktree {slot_path}
HEAD def
branch refs/heads/vw/task/one

worktree /repo/detached
HEAD 123
detached
"
    );

    let entries = super::parse_worktree_list(&output);

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].path, "/repo");
    assert_eq!(entries[0].branch.as_deref(), Some("main"));
    assert_eq!(entries[1].branch.as_deref(), Some("vw/task/one"));
    assert_eq!(entries[2].path, "/repo/detached");
    assert_eq!(entries[2].branch, None);
}

#[test]
fn parse_worktree_list_returns_empty_without_worktree_records() {
    let entries = super::parse_worktree_list(
        "branch refs/heads/orphan\nHEAD abc123\ndetached\nunknown value\n\n",
    );

    assert!(entries.is_empty());
}

#[test]
fn parse_worktree_list_flushes_adjacent_records_and_preserves_branch_forms() {
    let entries = super::parse_worktree_list(
        "worktree   /repo/one  \nbranch topic/raw\n\
         worktree /repo/two\nbranch refs/heads/first\nbranch refs/heads/final\n\
         worktree /repo/detached\nHEAD abc123",
    );

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].path, "/repo/one");
    assert_eq!(entries[0].branch.as_deref(), Some("topic/raw"));
    assert_eq!(entries[1].path, "/repo/two");
    assert_eq!(entries[1].branch.as_deref(), Some("final"));
    assert_eq!(entries[2].path, "/repo/detached");
    assert_eq!(entries[2].branch, None);
}

#[test]
fn sanitize_branch_token_replaces_unsafe_chars_and_trims_dashes() {
    assert_eq!(super::sanitize_branch_token(" task/ABC.123 "), "task-ABC.123");
    assert_eq!(super::sanitize_branch_token("///"), "");
    assert_eq!(super::sanitize_branch_token("a_b-c.d"), "a_b-c.d");
}

#[test]
fn sanitize_branch_token_handles_empty_unicode_and_repeated_separators() {
    assert_eq!(super::sanitize_branch_token(""), "");
    assert_eq!(super::sanitize_branch_token("任务"), "");
    assert_eq!(super::sanitize_branch_token("a//b"), "a--b");
    assert_eq!(super::sanitize_branch_token(".-_"), ".-_");
}

#[test]
fn managed_worktree_path_accepts_current_and_legacy_pool_roots() {
    let repo_root = "/repo";
    let current_slot = super::worktree_pool_root(repo_root).join("slot-1");
    let current_fork = super::worktree_pool_root(repo_root).join("fork-1");
    let current_sibling = super::worktree_pool_root(repo_root).with_file_name("task-worktrees-old");
    let legacy_root = std::path::Path::new(repo_root)
        .join(vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME)
        .join(vw_config_types::paths::TASK_WORKTREES_DIR_NAME);
    let legacy_slot = legacy_root.join("slot-2");
    let legacy_fork = legacy_root.join("fork-2");
    let unrelated_slot = std::path::Path::new(repo_root).join("other").join("slot-3");

    assert!(super::is_managed_worktree_path(repo_root, current_slot.to_string_lossy().as_ref()));
    assert!(super::is_managed_worktree_path(repo_root, legacy_slot.to_string_lossy().as_ref()));
    assert!(!super::is_managed_worktree_path(repo_root, current_fork.to_string_lossy().as_ref()));
    assert!(!super::is_managed_worktree_path(repo_root, legacy_fork.to_string_lossy().as_ref()));
    assert!(!super::is_managed_worktree_path(
        repo_root,
        current_sibling.join("slot-x").to_string_lossy().as_ref()
    ));
    assert!(!super::is_managed_worktree_path(repo_root, unrelated_slot.to_string_lossy().as_ref()));
}

#[test]
fn managed_worktree_path_requires_slot_dash_leaf_name() {
    let repo_root = "/repo";
    let legacy_root = std::path::Path::new(repo_root)
        .join(vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME)
        .join(vw_config_types::paths::TASK_WORKTREES_DIR_NAME);

    assert!(super::is_managed_worktree_path(
        repo_root,
        legacy_root.join("slot-").to_string_lossy().as_ref()
    ));
    assert!(!super::is_managed_worktree_path(
        repo_root,
        legacy_root.join("slot").to_string_lossy().as_ref()
    ));
    assert!(!super::is_managed_worktree_path(
        repo_root,
        legacy_root.join("prefix-slot-1").to_string_lossy().as_ref()
    ));
}

#[test]
fn desktop_worktree_snapshot_maps_gateway_dto() {
    use vw_gateway_client::vw_api_types::id::{ProjectId, TaskId};
    use vw_gateway_client::vw_api_types::task::{
        TaskPoolWorktreeMergeLockDto, TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto,
        TaskPoolWorktreeSnapshotResponse,
    };

    let snapshot =
        super::WorktreePoolSnapshot::from_gateway_response(TaskPoolWorktreeSnapshotResponse {
            project_id: ProjectId("project-1".to_string()),
            repo_root: "/repo".to_string(),
            pool_root: "/pool".to_string(),
            base_branch: "main".to_string(),
            idle_count: 1,
            busy_count: 1,
            tainted_count: 1,
            recycling_count: 1,
            dead_count: 1,
            merge_target_locks: vec![TaskPoolWorktreeMergeLockDto {
                target_branch: "main".to_string(),
                task_id: TaskId("T-lock".to_string()),
                age_ms: 0,
            }],
            slots: vec![TaskPoolWorktreeSlotDto {
                id: "slot-1".to_string(),
                path: "/pool/slot-1".to_string(),
                branch: "vw/task/slot-1".to_string(),
                state: TaskPoolWorktreeSlotStateDto::Busy,
                leased_task_id: Some(TaskId("T-busy".to_string())),
                taint_reason: Some("dirty".to_string()),
            }],
        });

    assert_eq!(snapshot.repo_root, "/repo");
    assert_eq!(snapshot.pool_root, "/pool");
    assert_eq!(snapshot.base_branch, "main");
    assert_eq!(snapshot.idle_count, 1);
    assert_eq!(snapshot.busy_count, 1);
    assert_eq!(snapshot.tainted_count, 1);
    assert_eq!(snapshot.recycling_count, 1);
    assert_eq!(snapshot.dead_count, 1);
    assert_eq!(snapshot.merge_target_locks, vec![("main".to_string(), "T-lock".to_string())]);
    assert_eq!(snapshot.slots[0].id, "slot-1");
    assert_eq!(snapshot.slots[0].state, super::WorktreeState::Busy);
    assert_eq!(snapshot.slots[0].leased_task_id.as_deref(), Some("T-busy"));
    assert_eq!(snapshot.slots[0].taint_reason.as_deref(), Some("dirty"));
}

#[cfg(unix)]
#[test]
fn pool_slot_path_rejects_non_utf8_leaf_names() {
    use std::os::unix::ffi::OsStringExt;

    let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![b's', b'l', 0xff]));

    assert!(!super::is_pool_slot_path(&path));
}
