#[test]
fn normalize_task_model_input_uses_auto_for_blank_values() {
    assert_eq!(super::normalize_task_model_input("  "), super::TASK_MODEL_AUTO);
    assert_eq!(super::normalize_task_model_input(" gpt-5 "), "gpt-5");
}

#[test]
fn claude_model_alias_accepts_supported_aliases_only() {
    assert_eq!(super::claude_model_alias(""), Some(super::CLAUDE_DEFAULT_MODEL_ALIAS));
    assert_eq!(super::claude_model_alias("AUTO"), Some(super::CLAUDE_DEFAULT_MODEL_ALIAS));
    assert_eq!(super::claude_model_alias(" default "), Some(super::CLAUDE_DEFAULT_MODEL_ALIAS));
    assert_eq!(super::claude_model_alias("sonnet"), Some("sonnet"));
    assert_eq!(super::claude_model_alias("OPUS"), Some("opus"));
    assert_eq!(super::claude_model_alias(" Haiku "), Some("haiku"));
    assert_eq!(super::claude_model_alias("unknown"), None);
}

#[test]
fn executor_backend_round_trips_stable_ids() {
    for backend in super::TaskExecutorBackend::all() {
        assert_eq!(super::TaskExecutorBackend::from_id(backend.id()), Some(backend));
        assert!(!backend.label().is_empty());
    }

    assert_eq!(super::TaskExecutorBackend::from_id("missing"), None);
}

#[test]
fn executor_backend_accepts_legacy_display_ids() {
    assert_eq!(
        super::TaskExecutorBackend::from_id("Internal"),
        Some(super::TaskExecutorBackend::Internal)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("内置执行器"),
        Some(super::TaskExecutorBackend::Internal)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("OpenCode"),
        Some(super::TaskExecutorBackend::OpenCode)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("Claude"),
        Some(super::TaskExecutorBackend::Claude)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("Claude Code"),
        Some(super::TaskExecutorBackend::Claude)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("Codex"),
        Some(super::TaskExecutorBackend::Codex)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("Codex CLI"),
        Some(super::TaskExecutorBackend::Codex)
    );
}

#[test]
fn task_status_order_and_keys_are_stable() {
    let transitions = [
        (super::TaskStatus::Pool, Some(super::TaskStatus::Pending)),
        (super::TaskStatus::Pending, Some(super::TaskStatus::Running)),
        (super::TaskStatus::Running, Some(super::TaskStatus::CodeComplete)),
        (super::TaskStatus::Failed, Some(super::TaskStatus::Pending)),
        (super::TaskStatus::Paused, None),
        (super::TaskStatus::CodeComplete, Some(super::TaskStatus::CodeReview)),
        (super::TaskStatus::CodeReview, Some(super::TaskStatus::PrSubmitted)),
        (super::TaskStatus::PrSubmitted, Some(super::TaskStatus::Completed)),
        (super::TaskStatus::Completed, Some(super::TaskStatus::Archived)),
        (super::TaskStatus::Archived, None),
    ];

    for (status, next) in transitions {
        assert_eq!(status.next(), next);
        assert_eq!(super::TaskStatus::parse_key(status.to_string_key()), Some(status));
        assert!(!status.label().is_empty());
    }

    assert_eq!(super::TaskStatus::parse_key("missing"), None);
}

#[test]
fn task_status_accepts_pascal_case_keys() {
    let pairs = [
        ("Pool", super::TaskStatus::Pool),
        ("Pending", super::TaskStatus::Pending),
        ("Running", super::TaskStatus::Running),
        ("Failed", super::TaskStatus::Failed),
        ("Paused", super::TaskStatus::Paused),
        ("CodeComplete", super::TaskStatus::CodeComplete),
        ("PrSubmitted", super::TaskStatus::PrSubmitted),
        ("CodeReview", super::TaskStatus::CodeReview),
        ("Completed", super::TaskStatus::Completed),
        ("Archived", super::TaskStatus::Archived),
    ];

    for (key, status) in pairs {
        assert_eq!(super::TaskStatus::parse_key(key), Some(status));
    }
}

#[test]
fn task_board_settings_sanitized_clamps_ranges() {
    let settings = super::TaskBoardSettings {
        max_concurrent: 0,
        refresh_interval_seconds: 0,
        scheduler_tick_interval_seconds: 99,
        auto_promote_tick_interval_seconds: 0,
        failed_retry_minutes: 0,
        failed_retry_max_attempts: 99,
        running_timeout_minutes: 99_999,
        pr_submitted_stall_timeout_seconds: 1,
        ..super::TaskBoardSettings::default()
    }
    .sanitized();

    assert_eq!(settings.max_concurrent, 1);
    assert_eq!(settings.refresh_interval_seconds, 1);
    assert_eq!(settings.scheduler_tick_interval_seconds, 60);
    assert_eq!(settings.auto_promote_tick_interval_seconds, 1);
    assert_eq!(settings.failed_retry_minutes, 1);
    assert_eq!(settings.failed_retry_max_attempts, 10);
    assert_eq!(settings.running_timeout_minutes, 1440);
    assert_eq!(settings.pr_submitted_stall_timeout_seconds, 5);
}

#[test]
fn task_board_settings_sanitized_clamps_upper_ranges() {
    let settings = super::TaskBoardSettings {
        max_concurrent: 99,
        refresh_interval_seconds: 99_999,
        scheduler_tick_interval_seconds: 99,
        auto_promote_tick_interval_seconds: 99_999,
        failed_retry_minutes: 99_999,
        failed_retry_max_attempts: 99,
        running_timeout_minutes: 99_999,
        pr_submitted_stall_timeout_seconds: 99_999,
        ..super::TaskBoardSettings::default()
    }
    .sanitized();

    assert_eq!(settings.max_concurrent, 10);
    assert_eq!(settings.refresh_interval_seconds, 3600);
    assert_eq!(settings.scheduler_tick_interval_seconds, 60);
    assert_eq!(settings.auto_promote_tick_interval_seconds, 3600);
    assert_eq!(settings.failed_retry_minutes, 1440);
    assert_eq!(settings.failed_retry_max_attempts, 10);
    assert_eq!(settings.running_timeout_minutes, 1440);
    assert_eq!(settings.pr_submitted_stall_timeout_seconds, 3600);
}

#[test]
fn task_board_settings_deserialize_applies_new_field_defaults() {
    let settings: super::TaskBoardSettings = serde_json::from_str(
        r#"{
            "auto_execute": false,
            "max_concurrent": 2,
            "default_priority": 5,
            "auto_promote_pool_tasks": false,
            "auto_promote_delay_seconds": 7
        }"#,
    )
    .expect("settings should deserialize with serde defaults");

    assert!(!settings.auto_execute);
    assert!(!settings.code_review_enabled);
    assert!(!settings.task_split_enabled);
    assert_eq!(settings.max_concurrent, 2);
    assert_eq!(settings.default_priority, 5);
    assert!(!settings.auto_promote_pool_tasks);
    assert_eq!(settings.auto_promote_delay_seconds, 7);
    assert!(settings.auto_refresh);
    assert_eq!(settings.refresh_interval_seconds, 60);
    assert_eq!(settings.scheduler_tick_interval_seconds, 1);
    assert_eq!(settings.auto_promote_tick_interval_seconds, 30);
    assert_eq!(settings.failed_retry_minutes, 20);
    assert_eq!(settings.failed_retry_max_attempts, 10);
    assert_eq!(settings.running_timeout_minutes, 20);
    assert!(!settings.recycle_worktree_on_task_finish);
    assert_eq!(settings.pr_submitted_stall_timeout_seconds, 30);
}

#[test]
fn task_default_initializes_persistent_fields() {
    let task = super::Task::default();

    assert!(task.id.starts_with('T'));
    assert_eq!(task.priority, 999);
    assert_eq!(task.assignee, "VibeWindow");
    assert_eq!(task.model, super::TASK_MODEL_AUTO);
    assert_eq!(task.executor, super::TaskExecutorBackend::Internal);
    assert_eq!(task.status, super::TaskStatus::Pool);
    assert!(task.logs.is_empty());
    assert!(!task.deleted);
    assert!(!task.archived);
    assert_eq!(task.retry_count, 0);
    assert_eq!(task.execution_started_at_ms, None);
    assert_eq!(task.last_execution_duration_ms, None);
    assert_eq!(task.merge_source_branch, None);
    assert_eq!(task.merge_target_branch, None);
    assert_eq!(task.selected_worktree_path, None);
    assert_eq!(task.task_split_mode, super::TaskSplitMode::FollowSystem);
}

#[test]
fn task_new_initializes_creation_log_and_priority() {
    let task = super::Task::new(10);

    assert_eq!(task.priority, 10);
    assert_eq!(task.status, super::TaskStatus::Pool);
    assert_eq!(task.logs.len(), 1);
    assert_eq!(task.logs[0].message, "任务创建");
    assert!(task.created_at_ms <= task.updated_at_ms);
}

#[test]
fn task_set_status_records_transition_and_clears_irrelevant_errors() {
    let mut task = super::Task::new(10);
    task.last_error = Some("old error".to_string());
    task.pause_reason = Some("old pause".to_string());

    task.set_status(super::TaskStatus::Failed);
    assert_eq!(task.last_error.as_deref(), Some("old error"));
    assert_eq!(task.pause_reason, None);
    assert_eq!(task.logs.last().expect("status log").status_from, Some(super::TaskStatus::Pool));
    assert_eq!(task.logs.last().expect("status log").status_to, Some(super::TaskStatus::Failed));

    task.pause_reason = Some("manual pause".to_string());
    task.set_status(super::TaskStatus::Paused);
    assert_eq!(task.last_error, None);
    assert_eq!(task.pause_reason.as_deref(), Some("manual pause"));

    task.set_status(super::TaskStatus::Pending);
    assert_eq!(task.pause_reason, None);
}

#[test]
fn task_add_log_updates_activity_and_message() {
    let mut task = super::Task::new(10);
    let previous_log_count = task.logs.len();

    task.add_log("hello".to_string());

    assert_eq!(task.logs.len(), previous_log_count + 1);
    assert_eq!(task.logs.last().expect("message log").message, "hello");
    assert_eq!(task.logs.last().expect("message log").status_from, None);
    assert_eq!(task.logs.last().expect("message log").status_to, None);
    assert!(task.last_active_at_ms >= task.created_at_ms);
}

#[test]
fn task_start_execution_resets_runtime_fields_and_records_retry() {
    let mut task = super::Task::new(10);
    task.last_error = Some("previous".to_string());
    task.pause_reason = Some("paused".to_string());
    task.merge_source_branch = Some("feature".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.selected_worktree_path = Some("/tmp/worktree".to_string());
    task.last_execution_duration_ms = Some(42);

    task.start_execution("trigger".to_string());

    assert_eq!(task.status, super::TaskStatus::Running);
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.last_error, None);
    assert_eq!(task.pause_reason, None);
    assert_eq!(task.merge_source_branch, None);
    assert_eq!(task.merge_target_branch, None);
    assert_eq!(task.selected_worktree_path, None);
    assert!(task.execution_started_at_ms.is_some());
    assert_eq!(task.last_execution_duration_ms, None);
    assert!(task.logs.iter().any(|entry| entry.message == "进入重试，第 1 次"));
    assert_eq!(task.logs.last().expect("trigger log").message, "trigger");
}

#[test]
fn task_start_merge_execution_keeps_merge_branches_and_logs() {
    let mut task = super::Task::new(10);
    task.merge_source_branch = Some("feature".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.last_execution_duration_ms = Some(42);

    task.start_merge_execution("merge".to_string());

    assert_eq!(task.merge_source_branch.as_deref(), Some("feature"));
    assert_eq!(task.merge_target_branch.as_deref(), Some("main"));
    assert!(task.execution_started_at_ms.is_some());
    assert_eq!(task.last_execution_duration_ms, None);
    assert_eq!(task.logs.last().expect("merge log").message, "merge");
}

#[test]
fn task_should_auto_merge_requires_non_blank_branches() {
    let mut task = super::Task::new(10);
    assert!(!task.should_auto_merge());

    task.merge_source_branch = Some("feature".to_string());
    task.merge_target_branch = Some(" ".to_string());
    assert!(!task.should_auto_merge());

    task.merge_target_branch = Some("main".to_string());
    assert!(task.should_auto_merge());
}

#[test]
fn task_mark_execution_failed_records_duration_status_and_error_log() {
    let mut task = super::Task::new(10);
    task.execution_started_at_ms = Some(u64::MAX);

    task.mark_execution_failed("boom".to_string());

    assert_eq!(task.status, super::TaskStatus::Failed);
    assert_eq!(task.last_error.as_deref(), Some("boom"));
    assert_eq!(task.execution_started_at_ms, None);
    assert_eq!(task.last_execution_duration_ms, Some(0));
    assert!(task.logs.iter().any(|entry| entry.message == "失败原因: boom"));
}

#[test]
fn task_mark_execution_succeeded_clears_runtime_error_state() {
    let mut task = super::Task::new(10);
    task.last_error = Some("old".to_string());
    task.pause_reason = Some("paused".to_string());
    task.execution_started_at_ms = Some(u64::MAX);

    task.mark_execution_succeeded();

    assert_eq!(task.last_error, None);
    assert_eq!(task.pause_reason, None);
    assert_eq!(task.execution_started_at_ms, None);
    assert_eq!(task.last_execution_duration_ms, Some(0));
}

#[test]
fn task_mark_paused_records_reason_and_duration() {
    let mut task = super::Task::new(10);
    task.execution_started_at_ms = Some(u64::MAX);

    task.mark_paused("waiting".to_string());

    assert_eq!(task.status, super::TaskStatus::Paused);
    assert_eq!(task.pause_reason.as_deref(), Some("waiting"));
    assert_eq!(task.execution_started_at_ms, None);
    assert_eq!(task.last_execution_duration_ms, Some(0));
    assert!(task.logs.iter().any(|entry| entry.message == "暂停原因: waiting"));
}

#[test]
fn task_running_duration_uses_started_time_and_saturates() {
    let mut task = super::Task::new(10);
    task.status = super::TaskStatus::Running;
    task.execution_started_at_ms = Some(100);

    assert_eq!(task.running_duration_ms(150), Some(50));
    assert_eq!(task.running_duration_ms(50), Some(0));

    task.status = super::TaskStatus::Paused;
    assert_eq!(task.running_duration_ms(150), None);
}

#[test]
fn task_running_duration_falls_back_to_last_active_time() {
    let mut task = super::Task::new(10);
    task.status = super::TaskStatus::Running;
    task.execution_started_at_ms = None;
    task.last_active_at_ms = 100;

    assert_eq!(task.running_duration_ms(125), Some(25));
}

#[test]
fn task_display_execution_duration_prefers_running_duration() {
    let mut task = super::Task::new(10);
    task.last_execution_duration_ms = Some(20);
    assert_eq!(task.display_execution_duration_ms(125), Some(20));

    task.status = super::TaskStatus::Running;
    task.execution_started_at_ms = Some(100);
    assert_eq!(task.display_execution_duration_ms(125), Some(25));
}

#[test]
fn task_deserialize_applies_added_field_defaults() {
    let task: super::Task = serde_json::from_str(
        r#"{
            "id": "T20260610.0001",
            "priority": 1,
            "assignee": "user",
            "model": "auto",
            "executor": "Internal",
            "description": "desc",
            "prompt": "prompt",
            "status": "Pool",
            "created_at_ms": 10,
            "updated_at_ms": 11,
            "logs": [],
            "order": 2,
            "deleted": false,
            "archived": false,
            "subtasks": [],
            "auto_promote_delay_ms": null
        }"#,
    )
    .expect("task should deserialize with serde defaults");

    assert_eq!(task.last_error, None);
    assert_eq!(task.pause_reason, None);
    assert_eq!(task.retry_count, 0);
    assert!(task.last_active_at_ms > 0);
    assert_eq!(task.execution_started_at_ms, None);
    assert_eq!(task.last_execution_duration_ms, None);
    assert_eq!(task.merge_source_branch, None);
    assert_eq!(task.merge_target_branch, None);
    assert_eq!(task.selected_worktree_path, None);
}

#[test]
fn task_log_entry_builders_set_expected_fields() {
    let change = super::TaskLogEntry::new_status_change(
        super::TaskStatus::Pending,
        super::TaskStatus::Running,
    );
    assert_eq!(change.status_from, Some(super::TaskStatus::Pending));
    assert_eq!(change.status_to, Some(super::TaskStatus::Running));
    assert_eq!(change.message, "状态变更: 待执行 → 执行中");

    let message = super::TaskLogEntry::new_message("note".to_string());
    assert_eq!(message.status_from, None);
    assert_eq!(message.status_to, None);
    assert_eq!(message.message, "note");
}

#[test]
fn subtask_new_initializes_defaults() {
    let subtask = super::SubTask::new("do work".to_string());

    assert!(subtask.id.starts_with("SUB-"));
    assert_eq!(subtask.content, "do work");
    assert_eq!(subtask.boundary, "");
    assert!(subtask.acceptance_criteria.is_empty());
    assert!(subtask.target_files.is_empty());
    assert_eq!(subtask.order, 0);
    assert!(!subtask.completed);
    assert_eq!(subtask.status, super::SubTaskStatus::Pending);
    assert_eq!(subtask.execution_started_at_ms, None);
    assert_eq!(subtask.last_execution_duration_ms, None);
}

#[test]
fn subtask_execution_lifecycle_updates_status_and_duration() {
    let mut subtask = super::SubTask::new("do work".to_string());

    subtask.start_execution();
    assert_eq!(subtask.status, super::SubTaskStatus::Running);
    assert!(!subtask.completed);
    assert!(subtask.execution_started_at_ms.is_some());
    assert_eq!(subtask.last_execution_duration_ms, None);
    assert_eq!(subtask.display_execution_duration_ms(0), Some(0));

    subtask.execution_started_at_ms = Some(u64::MAX);
    subtask.mark_completed();
    assert_eq!(subtask.status, super::SubTaskStatus::Completed);
    assert!(subtask.completed);
    assert_eq!(subtask.execution_started_at_ms, None);
    assert_eq!(subtask.last_execution_duration_ms, Some(0));
    assert_eq!(subtask.display_execution_duration_ms(500), Some(0));

    subtask.start_execution();
    subtask.execution_started_at_ms = Some(u64::MAX);
    subtask.mark_failed();
    assert_eq!(subtask.status, super::SubTaskStatus::Failed);
    assert!(!subtask.completed);
    assert_eq!(subtask.execution_started_at_ms, None);
    assert_eq!(subtask.last_execution_duration_ms, Some(0));
}

#[test]
fn subtask_deserialize_applies_defaults() {
    let subtask: super::SubTask = serde_json::from_str(
        r#"{
            "id": "SUB-1.00001",
            "content": "do work",
            "created_at_ms": 10,
            "order": 1,
            "completed": true
        }"#,
    )
    .expect("subtask should deserialize with serde defaults");

    assert_eq!(subtask.boundary, "");
    assert!(subtask.acceptance_criteria.is_empty());
    assert!(subtask.target_files.is_empty());
    assert_eq!(subtask.status, super::SubTaskStatus::Pending);
    assert_eq!(subtask.execution_started_at_ms, None);
    assert_eq!(subtask.last_execution_duration_ms, None);
}

#[test]
fn task_index_initializes_every_status_bucket() {
    let index = super::TaskIndex::new();

    for status in super::TaskStatus::all() {
        assert!(index.order_by_status.contains_key(status.to_string_key()));
    }

    assert!(index.tasks.is_empty());
    assert_eq!(index.last_task_date, None);
    assert_eq!(index.last_task_seq, 0);
}

#[test]
fn task_draft_default_initializes_editable_strings() {
    let draft = super::TaskDraft::default();

    assert_eq!(draft.priority, "999");
    assert_eq!(draft.assignee, "VibeWindow");
    assert_eq!(draft.model, super::TASK_MODEL_AUTO);
    assert_eq!(draft.executor, super::TaskExecutorBackend::Internal);
    assert_eq!(draft.description, "");
    assert_eq!(draft.prompt, "");
    assert_eq!(draft.subtasks, vec![String::new(), String::new(), String::new()]);
    assert_eq!(draft.task_split_mode, super::TaskSplitMode::FollowSystem);
    assert_eq!(draft.auto_promote_delay_seconds, "0");
}

#[test]
fn task_import_prompt_format_default_is_json() {
    assert_eq!(super::TaskImportPromptFormat::default(), super::TaskImportPromptFormat::Json);
    assert_ne!(super::TaskImportPromptFormat::Csv, super::TaskImportPromptFormat::Tsv);
}

#[test]
fn task_split_mode_all_labels_and_keys_are_stable() {
    use super::TaskSplitMode;
    let all = TaskSplitMode::all();
    assert_eq!(all, [TaskSplitMode::FollowSystem, TaskSplitMode::Enabled, TaskSplitMode::Disabled]);

    assert_eq!(TaskSplitMode::FollowSystem.label(), "遵循系统");
    assert_eq!(TaskSplitMode::Enabled.label(), "开启");
    assert_eq!(TaskSplitMode::Disabled.label(), "关闭");

    assert_eq!(TaskSplitMode::FollowSystem.to_string_key(), "follow_system");
    assert_eq!(TaskSplitMode::Enabled.to_string_key(), "enabled");
    assert_eq!(TaskSplitMode::Disabled.to_string_key(), "disabled");

    // Display 复用 label。
    assert_eq!(TaskSplitMode::Enabled.to_string(), "开启");
}

#[test]
fn task_split_mode_parse_key_accepts_aliases_and_rejects_unknown() {
    use super::TaskSplitMode;
    for value in ["follow_system", "follow-system", "system", "default", " INHERIT "] {
        assert_eq!(TaskSplitMode::parse_key(value), Some(TaskSplitMode::FollowSystem));
    }
    for value in ["enabled", "enable", "on", "true", "yes", "1"] {
        assert_eq!(TaskSplitMode::parse_key(value), Some(TaskSplitMode::Enabled));
    }
    for value in ["disabled", "disable", "off", "false", "no", "0"] {
        assert_eq!(TaskSplitMode::parse_key(value), Some(TaskSplitMode::Disabled));
    }
    assert_eq!(TaskSplitMode::parse_key("unknown"), None);
}

#[test]
fn task_split_mode_enabled_with_system_resolves_override() {
    use super::TaskSplitMode;
    assert!(TaskSplitMode::FollowSystem.enabled_with_system(true));
    assert!(!TaskSplitMode::FollowSystem.enabled_with_system(false));
    assert!(TaskSplitMode::Enabled.enabled_with_system(false));
    assert!(!TaskSplitMode::Disabled.enabled_with_system(true));
}

#[test]
fn task_status_next_advances_through_pipeline() {
    use super::TaskStatus;
    assert_eq!(TaskStatus::Pool.next(), Some(TaskStatus::Pending));
    assert_eq!(TaskStatus::Pending.next(), Some(TaskStatus::Running));
    assert_eq!(TaskStatus::Running.next(), Some(TaskStatus::CodeComplete));
    assert_eq!(TaskStatus::Failed.next(), Some(TaskStatus::Pending));
    assert_eq!(TaskStatus::Paused.next(), None);
    assert_eq!(TaskStatus::CodeComplete.next(), Some(TaskStatus::CodeReview));
    assert_eq!(TaskStatus::CodeReview.next(), Some(TaskStatus::PrSubmitted));
    assert_eq!(TaskStatus::PrSubmitted.next(), Some(TaskStatus::Completed));
    assert_eq!(TaskStatus::Completed.next(), Some(TaskStatus::Archived));
    assert_eq!(TaskStatus::Archived.next(), None);
}

#[test]
fn generated_task_id_uses_date_prefixed_format() {
    // generate_task_id 经 Task::new/default 间接触发，验证稳定格式 `T{8位日期}.{4位序号}`。
    let task = super::Task::default();
    let id = task.id;
    assert!(id.starts_with('T'), "id should start with T: {id}");
    let rest = &id[1..];
    let (date, seq) = rest.split_once('.').expect("id must contain a dot");
    assert_eq!(date.len(), 8, "date segment should be 8 chars: {date}");
    assert!(date.chars().all(|c| c.is_ascii_digit()), "date must be digits: {date}");
    assert_eq!(seq.len(), 4, "seq segment should be 4 chars: {seq}");
    assert!(seq.chars().all(|c| c.is_ascii_digit()), "seq must be digits: {seq}");
}

#[test]
fn subtask_display_duration_reports_none_when_idle() {
    // 非 Running 且无历史时长时返回 None。
    let subtask = super::SubTask::new("内容".to_string());
    assert_eq!(subtask.status, super::SubTaskStatus::Pending);
    assert_eq!(subtask.display_execution_duration_ms(1000), None);
}

#[test]
fn subtask_mark_failed_without_start_keeps_duration_none() {
    let mut subtask = super::SubTask::new("内容".to_string());
    subtask.mark_failed();
    assert_eq!(subtask.status, super::SubTaskStatus::Failed);
    assert!(!subtask.completed);
    assert_eq!(subtask.last_execution_duration_ms, None);
}

#[test]
fn subtask_default_status_is_pending() {
    assert_eq!(super::SubTaskStatus::default(), super::SubTaskStatus::Pending);
}
