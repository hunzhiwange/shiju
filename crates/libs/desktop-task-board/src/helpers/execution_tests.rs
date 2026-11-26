#[test]
fn start_command_skips_created_in_flight_task() {
    use vw_api_agent_scheduler::{AgentSchedulerCommandKindDto, AgentTaskStatusDto};

    assert!(super::should_send_agent_scheduler_command_after_create(
        AgentSchedulerCommandKindDto::Start,
        &AgentTaskStatusDto::Queued
    ));
    assert!(super::should_send_agent_scheduler_command_after_create(
        AgentSchedulerCommandKindDto::Start,
        &AgentTaskStatusDto::Draft
    ));

    for status in [
        AgentTaskStatusDto::Assigned,
        AgentTaskStatusDto::Dispatched,
        AgentTaskStatusDto::Acknowledged,
        AgentTaskStatusDto::Running,
        AgentTaskStatusDto::Submitted,
    ] {
        assert!(!super::should_send_agent_scheduler_command_after_create(
            AgentSchedulerCommandKindDto::Start,
            &status
        ));
    }

    assert!(super::should_send_agent_scheduler_command_after_create(
        AgentSchedulerCommandKindDto::Retry,
        &AgentTaskStatusDto::Running
    ));
}

#[test]
fn retry_backoff_minutes_grows_exponentially_and_caps_at_one_day() {
    assert_eq!(super::retry_backoff_minutes(1, 0), 1);
    assert_eq!(super::retry_backoff_minutes(1, 1), 2);
    assert_eq!(super::retry_backoff_minutes(1, 4), 16);
    assert_eq!(super::retry_backoff_minutes(60, 5), 1440);
    assert_eq!(super::retry_backoff_minutes(0, 1), 2);
}

#[test]
fn truncate_for_ui_counts_unicode_characters_and_preserves_short_values() {
    assert_eq!(super::truncate_for_ui("任务日志", 4), "任务日志");
    assert_eq!(super::truncate_for_ui("任务日志", 2), "任务 ...(已截断，共4字符)");
    assert_eq!(super::truncate_for_ui("abc", 0), " ...(已截断，共3字符)");
}

#[test]
fn format_task_log_stream_for_ui_covers_empty_text_and_exit_variants() {
    use desktop_task_executor::TaskLogStream;

    assert_eq!(
        super::format_task_log_stream_for_ui(&TaskLogStream::Stdout("  \n".into()), 20),
        None
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(&TaskLogStream::Stderr("abcdef".into()), 3),
        Some("abc ...(已截断，共6字符)".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::ExitStatus { success: true, code: Some(0), signal: None },
            1,
        ),
        Some("执行成功 code=Some(0)".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::ExitStatus { success: false, code: None, signal: Some(9) },
            1,
        ),
        Some("执行失败 code=None signal=9".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::ExitStatus { success: false, code: Some(2), signal: None },
            1,
        ),
        Some("执行失败 code=Some(2)".to_string())
    );
}

#[test]
fn worktree_recycle_failure_log_explains_gateway_cleanup_ownership() {
    let mut task = desktop_task_model::Task::new(1);

    super::record_task_worktree_recycle_after_failure(&mut task, "execution failed");

    assert_eq!(
        task.logs.last().map(|entry| entry.message.as_str()),
        Some("execution failed，等待网关运行时清理 worktree 槽位")
    );
}

#[test]
fn scheduler_status_key_preserves_unknown_step_keys() {
    use desktop_task_model::TaskStatus;
    assert_eq!(super::agent_scheduler_status_key_for_task_status(
        TaskStatus::Custom("ops-review"), "missing"), "ops-review");
}

#[test]
fn wake_scheduler_requests_register_all_nonterminal_steps() {
    use desktop_task_model::{Task, TaskStatus};
    use vw_api_agent_scheduler::AgentTaskKindDto;

    let mut pending = Task::new(1);
    pending.id = "pending".to_string();
    pending.status = TaskStatus::Pending;
    let mut running = Task::new(3);
    running.id = "running".to_string();
    running.status = TaskStatus::Running;
    let mut pool = Task::new(4);
    pool.id = "pool".to_string();
    pool.status = TaskStatus::Pool;

    let requests = super::agent_scheduler_wake_create_task_requests(
        &[pool, pending, running],
        desktop_task_model::workflow::DEFAULT_TASK_WORKFLOW_ID,
    );

    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].task_id, "pool");
    assert!(requests[0].status.is_none());
    assert_eq!(requests[1].task_id, "pending");
    assert!(requests.iter().all(|request| request.kind == AgentTaskKindDto::Finite));
    assert!(
        requests[1..].iter().all(|request| { request.status.is_none() })
    );
}

#[test]
fn scheduler_create_request_uses_prompt_title_and_saturates_timestamp() {
    use desktop_task_model::{Task, TaskStatus};

    let mut task = Task::new(7);
    task.id = "request-task".to_string();
    task.status = TaskStatus::Pending;
    task.prompt = format!("\n  {}  \nignored", "界".repeat(130));
    task.created_at_ms = u64::MAX;

    let request = super::agent_scheduler_create_task_request(&task, "");

    assert_eq!(request.title.chars().count(), 120);
    assert!(request.title.chars().all(|character| character == '界'));
    assert_eq!(request.spec_uri, "task-board/request-task/spec.md");
    assert_eq!(request.meta_uri, "task-board/request-task/meta.json");
    assert_eq!(request.created_at_ms.0, i64::MAX);

    task.prompt = " \n\t".to_string();
    let fallback = super::agent_scheduler_create_task_request(&task, "unknown-workflow");
    assert_eq!(fallback.title, "request-task");
    assert_eq!(
        fallback.metadata["task_board.workflow_id"],
        desktop_task_model::workflow::DEFAULT_TASK_WORKFLOW_ID
    );
}

#[test]
fn agent_scheduler_request_uses_lifecycle_cache_metadata() {
    use desktop_task_model::{Task, TaskStatus, update_runtime_task_lifecycle_cache};
    use vw_api_task_lifecycle::{TaskLifecycleStatusCategoryDto, TaskLifecycleWorkflowDto};

    let _cache_lock = desktop_task_model::runtime_task_lifecycle_cache_test_lock();
    struct CacheGuard;

    impl Drop for CacheGuard {
        fn drop(&mut self) {
            desktop_task_model::reset_runtime_task_lifecycle_cache()
                .expect("runtime lifecycle cache should reset");
        }
    }

    let _guard = CacheGuard;
    update_runtime_task_lifecycle_cache(vec![{
        let statuses: Vec<vw_api_task_lifecycle::TaskLifecycleStatusDto> = vec![
            lifecycle_status(
                "ops_backlog",
                "Ops Backlog",
                TaskLifecycleStatusCategoryDto::Unstarted,
            ),
            lifecycle_status("ops_ready", "Ops Ready", TaskLifecycleStatusCategoryDto::Unstarted),
            lifecycle_status("ops_active", "Ops Active", TaskLifecycleStatusCategoryDto::Started),
            lifecycle_status("ops_review", "Ops Review", TaskLifecycleStatusCategoryDto::Started),
            lifecycle_status("ops_blocked", "Ops Blocked", TaskLifecycleStatusCategoryDto::Started),
            lifecycle_status("ops_done", "Ops Done", TaskLifecycleStatusCategoryDto::Done),
        ];
        TaskLifecycleWorkflowDto {
            id: "scheduler_ops".to_string(),
            label: "Scheduler Ops".to_string(),
            built_in: false,
            initial_status_key: statuses
                .first()
                .map(|status| status.key.clone())
                .unwrap_or_default(),
            steps: statuses
                .iter()
                .map(|status| vw_api_task_lifecycle::TaskLifecycleStepDto {
                    status_key: status.key.clone(),
                    handoff: vw_api_task_lifecycle::TaskLifecycleStepHandoffDto::KeepAssignee,
                })
                .collect(),
            statuses,
        }
    }])
    .expect("runtime lifecycle cache should update");

    let mut task = Task::new(42);
    task.id = "task-cache-metadata".to_string();
    task.status = TaskStatus::Custom("ops_active");
    task.prompt = "Use cache metadata".to_string();

    let request = super::agent_scheduler_create_task_request(&task, "scheduler_ops");
    let metadata = request.metadata.as_object().expect("metadata should be an object");
    assert!(request.status.is_none());
    assert_eq!(metadata["task_board.workflow_id"], "scheduler_ops");
    assert_eq!(metadata["task_board.status_key"], "ops_active");
    assert!(!metadata.contains_key("task_board.status_role"));
    assert!(!metadata.contains_key("task_board.status_key_map"));
}

fn lifecycle_status(
    key: &str,
    label: &str,
    category: vw_api_task_lifecycle::TaskLifecycleStatusCategoryDto,
) -> vw_api_task_lifecycle::TaskLifecycleStatusDto {
    vw_api_task_lifecycle::TaskLifecycleStatusDto {
        key: key.to_string(),
        label: label.to_string(),
        enabled: true,
        category,
    }
}
