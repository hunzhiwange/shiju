#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("models_tests"));
}

#[test]
fn normalize_model_and_agent_inputs_trim_and_map_defaults() {
    assert_eq!(super::normalize_task_model_input("  "), super::TASK_MODEL_AUTO);
    assert_eq!(super::normalize_task_model_input(" gpt-5 "), "gpt-5");

    assert_eq!(super::claude_model_alias(""), Some(super::CLAUDE_DEFAULT_MODEL_ALIAS));
    assert_eq!(super::claude_model_alias(" AUTO "), Some(super::CLAUDE_DEFAULT_MODEL_ALIAS));
    assert_eq!(super::claude_model_alias("Sonnet"), Some("sonnet"));
    assert_eq!(super::claude_model_alias("unknown"), None);

    assert_eq!(super::normalize_task_acp_agent_input(" default "), None);
    assert_eq!(super::normalize_task_acp_agent_input("ACP 网关"), None);
    assert_eq!(
        super::normalize_task_acp_agent_input(" custom-agent "),
        Some("custom-agent".into())
    );
}

#[test]
fn executor_backend_ids_labels_and_legacy_mapping_are_stable() {
    let backends = super::TaskExecutorBackend::all();
    assert_eq!(backends.len(), 4);
    assert_eq!(super::TaskExecutorBackend::Internal.id(), "internal");
    assert_eq!(super::TaskExecutorBackend::OpenCode.label(), "OpenCode");
    assert_eq!(
        super::TaskExecutorBackend::from_id("Claude Code"),
        Some(super::TaskExecutorBackend::Claude)
    );
    assert_eq!(super::TaskExecutorBackend::from_id("missing"), None);

    assert_eq!(
        super::legacy_executor_to_task_acp_agent(super::TaskExecutorBackend::Internal),
        None
    );
    assert_eq!(
        super::legacy_executor_to_task_acp_agent(super::TaskExecutorBackend::Codex),
        Some("codex".into())
    );
}

#[test]
fn executor_backend_from_id_accepts_all_legacy_aliases() {
    for alias in ["internal", "Internal", "内置执行器", "ACP 智能体", "ACP 网关", "acp_gateway"]
    {
        assert_eq!(
            super::TaskExecutorBackend::from_id(alias),
            Some(super::TaskExecutorBackend::Internal)
        );
    }
    assert_eq!(
        super::TaskExecutorBackend::from_id("OpenCode"),
        Some(super::TaskExecutorBackend::OpenCode)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("Claude"),
        Some(super::TaskExecutorBackend::Claude)
    );
    assert_eq!(
        super::TaskExecutorBackend::from_id("Codex CLI"),
        Some(super::TaskExecutorBackend::Codex)
    );
}

#[test]
fn task_status_keys_labels_and_next_states_cover_all_variants() {
    let statuses = super::TaskStatus::all();
    assert_eq!(statuses.len(), 11);
    for status in statuses {
        let key = status.to_string_key();
        assert_eq!(super::TaskStatus::parse_key(key), Some(status));
        assert!(!status.label().is_empty());
    }

    assert_eq!(super::TaskStatus::Pool.next(), Some(super::TaskStatus::Pending));
    assert_eq!(super::TaskStatus::Failed.next(), Some(super::TaskStatus::Pending));
    assert_eq!(super::TaskStatus::Paused.next(), None);
    assert_eq!(super::TaskStatus::Archived.next(), None);
    // 未知非空 key 现落 Custom（支持自定义状态）；仅空字符串无法表达状态返回 None。
    assert_eq!(super::TaskStatus::parse_key("bogus"), Some(super::TaskStatus::Custom("bogus")));
    assert_eq!(super::TaskStatus::parse_key(""), None);
}

#[test]
fn builtin_status_serialization_is_unchanged() {
    // 内置变体的 serde 线格式必须与历史派生实现逐字节一致（PascalCase 变体名），
    // 否则旧 JSON 任务存档会读不回来。同时校验 to_string_key/parse_key 的稳定 key 不变。
    let cases = [
        (super::TaskStatus::Pool, "\"Pool\"", "pool"),
        (super::TaskStatus::Pending, "\"Pending\"", "pending"),
        (super::TaskStatus::Planning, "\"Planning\"", "planning"),
        (super::TaskStatus::Running, "\"Running\"", "running"),
        (super::TaskStatus::Failed, "\"Failed\"", "failed"),
        (super::TaskStatus::Paused, "\"Paused\"", "paused"),
        (super::TaskStatus::CodeComplete, "\"CodeComplete\"", "code_complete"),
        (super::TaskStatus::CodeReview, "\"CodeReview\"", "code_review"),
        (super::TaskStatus::PrSubmitted, "\"PrSubmitted\"", "pr_submitted"),
        (super::TaskStatus::Completed, "\"Completed\"", "completed"),
        (super::TaskStatus::Archived, "\"Archived\"", "archived"),
    ];

    for (status, expected_json, expected_key) in cases {
        let json = serde_json::to_string(&status).expect("serialize builtin status");
        assert_eq!(json, expected_json, "{status:?} 的 serde 线格式不能变化");

        let decoded: super::TaskStatus =
            serde_json::from_str(&json).expect("deserialize builtin status");
        assert_eq!(decoded, status, "{status:?} serde 往返必须稳定");

        assert_eq!(status.to_string_key(), expected_key, "{status:?} 稳定 key 不能变化");
        assert_eq!(super::TaskStatus::parse_key(expected_key), Some(status));
    }

    // 旧 SQLite/日志列以 snake_case key 持久化，PascalCase 变体名也曾被接受，二者都要继续解析。
    assert_eq!(super::TaskStatus::parse_key("Pool"), Some(super::TaskStatus::Pool));
    assert_eq!(
        super::TaskStatus::parse_key("code_complete"),
        Some(super::TaskStatus::CodeComplete)
    );
}

#[test]
fn unknown_status_key_round_trips_as_custom() {
    // 未知 key 经 parse_key 落 Custom 且原始 key 无损保留。
    let custom = super::TaskStatus::parse_key("qa_signoff").expect("unknown key 必须落 Custom");
    assert_ne!(custom, super::TaskStatus::Pool, "Custom 不能塌缩成内置变体");
    assert_eq!(custom.to_string_key(), "qa_signoff", "Custom 必须保留原始 key");
    assert!(!custom.label().is_empty(), "Custom 至少回退到自身 key 作为标签");

    // 相同 key 的两次解析必须相等（interner 去重，可比较）。
    let again = super::TaskStatus::parse_key("qa_signoff").expect("再次解析同一 key");
    assert_eq!(custom, again, "相同自定义 key 解析结果必须相等");

    // serde 往返稳定：Custom 序列化为其原始 key 字符串，再反序列化回相同 Custom。
    let json = serde_json::to_string(&custom).expect("serialize custom status");
    assert_eq!(json, "\"qa_signoff\"", "Custom 的 serde 线格式为原始 key 字符串");
    let decoded: super::TaskStatus =
        serde_json::from_str(&json).expect("deserialize custom status");
    assert_eq!(decoded, custom, "Custom serde 往返必须无损");
    assert_eq!(decoded.to_string_key(), "qa_signoff");

    // 不在内置集合内，all() 不应包含自定义状态。
    assert!(!super::TaskStatus::all().contains(&custom));
}

#[test]
fn subtask_execution_lifecycle_tracks_status_and_duration() {
    let mut subtask = super::SubTask::new("write tests".into());
    assert!(subtask.id.starts_with("SUB-"));
    assert_eq!(subtask.status, super::SubTaskStatus::Pending);
    assert_eq!(subtask.display_execution_duration_ms(subtask.created_at_ms + 10), None);

    subtask.start_execution();
    let started_at = subtask.execution_started_at_ms.expect("start timestamp");
    assert_eq!(subtask.status, super::SubTaskStatus::Running);
    assert_eq!(subtask.display_execution_duration_ms(started_at + 25), Some(25));

    subtask.mark_completed();
    assert_eq!(subtask.status, super::SubTaskStatus::Completed);
    assert!(subtask.completed);
    assert!(subtask.last_execution_duration_ms.is_some());
    let completed_duration = subtask.last_execution_duration_ms;

    subtask.start_execution();
    assert_eq!(subtask.last_execution_duration_ms, completed_duration);
    subtask.mark_failed();
    assert_eq!(subtask.status, super::SubTaskStatus::Failed);
    assert!(!subtask.completed);
}

#[test]
fn subtask_batch_fields_default_for_legacy_payloads() {
    let legacy = r#"{
        "id": "SUB-1",
        "content": "legacy",
        "created_at_ms": 1,
        "order": 0,
        "completed": false
    }"#;
    let subtask: super::SubTask =
        serde_json::from_str(legacy).expect("legacy subtask payload should deserialize");
    assert_eq!(subtask.batch, 1);
    assert!(subtask.depends_on_batches.is_empty());

    let fresh = super::SubTask::new("fresh".into());
    assert_eq!(fresh.batch, 1);
    assert!(fresh.depends_on_batches.is_empty());
}

#[test]
fn task_lifecycle_preserves_and_sets_execution_fields() {
    let mut task = super::Task::new(7);
    assert_eq!(task.priority, 7);
    assert_eq!(task.status, super::TaskStatus::Pool);
    assert_eq!(task.logs.len(), 1);

    task.set_status(super::TaskStatus::Pending);
    task.last_error = Some("previous".into());
    task.start_execution("run now".into()).expect("pending start should be allowed");
    assert_eq!(task.status, super::TaskStatus::Running);
    assert_eq!(task.retry_count, 1);
    assert!(task.execution_started_at_ms.is_some());
    assert!(task.logs.iter().any(|log| log.message == "run now"));

    task.mark_execution_failed("boom".into());
    assert_eq!(task.status, super::TaskStatus::Failed);
    assert_eq!(task.last_error.as_deref(), Some("boom"));
    assert!(task.last_execution_duration_ms.is_some());
    let failed_duration = task.last_execution_duration_ms;
    assert!(task.logs.iter().any(|log| log.message == "失败原因: boom"));

    task.start_execution("retry".into()).expect("failed retry start should be allowed");
    assert_eq!(task.last_execution_duration_ms, failed_duration);
    task.mark_execution_succeeded();
    task.apply_task_transition(
        super::TaskStatus::CodeComplete,
        super::super::workflow::TaskTransitionIntent::ExecutionSucceeded,
    )
    .expect("running to code complete should be allowed");
    assert_eq!(task.last_error, None);
    assert_eq!(task.pause_reason, None);
    assert_eq!(task.status, super::TaskStatus::CodeComplete);
}

#[test]
fn apply_task_transition_rejects_illegal_edge_and_keeps_status() {
    use super::super::workflow::TaskTransitionIntent;

    let mut task = super::Task::new(1);
    task.set_status(super::TaskStatus::Running);
    let log_count_before = task.logs.len();

    // Running -> Completed 不是 MergeSucceeded 的合法边，必须被拒绝且不改状态。
    let error = task
        .apply_task_transition(super::TaskStatus::Completed, TaskTransitionIntent::MergeSucceeded)
        .expect_err("illegal edge must be rejected");
    assert_eq!(error.from, super::TaskStatus::Running);
    assert_eq!(error.to, super::TaskStatus::Completed);
    assert_eq!(task.status, super::TaskStatus::Running);
    assert!(task.logs.len() > log_count_before);
    assert!(task.logs.iter().any(|log| log.message.contains("状态转移被状态机拒绝")));
}

#[test]
fn start_execution_model_rejects_invalid_source_status() {
    let mut task = super::Task::new(1);
    // 默认 Pool 不是 StartExecution 的合法来源（须先提升到 Pending）。
    assert_eq!(task.status, super::TaskStatus::Pool);
    let error = task
        .start_execution("启动".into())
        .expect_err("pool source must be rejected for StartExecution");
    assert_eq!(error.from, super::TaskStatus::Pool);
    assert_eq!(error.to, super::TaskStatus::Running);
    assert_eq!(task.status, super::TaskStatus::Pool);
    assert!(task.execution_started_at_ms.is_none());
    assert_eq!(task.retry_count, 0);
    assert!(task.logs.iter().any(|log| log.message.contains("拒绝开始执行")));

    // Completed 同样非法。
    task.set_status(super::TaskStatus::Completed);
    assert!(task.start_execution("再启动".into()).is_err());
    assert_eq!(task.status, super::TaskStatus::Completed);
}

#[test]
fn task_pause_merge_and_duration_helpers_use_expected_fallbacks() {
    let mut task = super::Task::default();
    assert!(!task.should_auto_merge());
    task.merge_source_branch = Some(" feature ".into());
    task.merge_target_branch = Some("main".into());
    assert!(task.should_auto_merge());

    task.start_merge_execution("merge".into());
    assert!(task.execution_started_at_ms.is_some());
    let merge_started_duration = task.last_execution_duration_ms;

    task.mark_paused("waiting".into());
    assert_eq!(task.status, super::TaskStatus::Paused);
    assert_eq!(task.pause_reason.as_deref(), Some("waiting"));
    assert!(task.last_execution_duration_ms.is_some());
    assert_ne!(task.last_execution_duration_ms, merge_started_duration);
    assert_eq!(task.running_duration_ms(super::now_ms()), None);
    assert!(task.display_execution_duration_ms(super::now_ms()).is_some());

    task.status = super::TaskStatus::Running;
    task.execution_started_at_ms = None;
    task.last_active_at_ms = 100;
    assert_eq!(task.running_duration_ms(125), Some(25));
    assert_eq!(task.running_duration_ms(75), Some(0));
}

#[test]
fn task_index_and_board_settings_defaults_are_sanitized() {
    let index = super::TaskIndex::new();
    assert_eq!(index.order_by_status.len(), super::TaskStatus::all().len());
    assert!(index.order_by_status.contains_key("pending"));

    let settings = super::TaskBoardSettings {
        max_concurrent: 99,
        refresh_interval_seconds: 0,
        scheduler_tick_interval_seconds: 99,
        auto_promote_tick_interval_seconds: 0,
        failed_retry_minutes: 0,
        failed_retry_max_attempts: 99,
        running_timeout_minutes: 9999,
        pr_submitted_stall_timeout_seconds: 1,
        ..super::TaskBoardSettings::new()
    }
    .sanitized();

    assert_eq!(settings.max_concurrent, 10);
    assert_eq!(settings.refresh_interval_seconds, 1);
    assert_eq!(settings.scheduler_tick_interval_seconds, 60);
    assert_eq!(settings.auto_promote_tick_interval_seconds, 1);
    assert_eq!(settings.failed_retry_minutes, 1);
    assert_eq!(settings.failed_retry_max_attempts, 10);
    assert_eq!(settings.running_timeout_minutes, 1440);
    assert_eq!(settings.pr_submitted_stall_timeout_seconds, 5);
}

#[test]
fn task_board_settings_new_and_default_keep_exact_runtime_defaults() {
    let settings = super::TaskBoardSettings::new();
    let default_settings = super::TaskBoardSettings::default();

    assert_eq!(settings.max_concurrent, 3);
    assert_eq!(settings.default_priority, 999);
    assert_eq!(settings.auto_promote_delay_seconds, 30);
    assert!(settings.auto_refresh);
    assert_eq!(settings.refresh_interval_seconds, 60);
    assert_eq!(settings.scheduler_tick_interval_seconds, 1);
    assert_eq!(settings.auto_promote_tick_interval_seconds, 30);
    assert_eq!(settings.failed_retry_minutes, 20);
    assert_eq!(settings.failed_retry_max_attempts, 10);
    assert_eq!(settings.running_timeout_minutes, 20);
    assert!(!settings.recycle_worktree_on_task_finish);
    assert_eq!(settings.pr_submitted_stall_timeout_seconds, 30);
    assert_eq!(settings.workflow_id, crate::app::task::DEFAULT_TASK_WORKFLOW_ID);
    assert_eq!(default_settings.workflow_id, settings.workflow_id);

    let zero_retry_attempts = super::TaskBoardSettings {
        failed_retry_max_attempts: 0,
        ..super::TaskBoardSettings::new()
    }
    .sanitized();
    assert_eq!(zero_retry_attempts.failed_retry_max_attempts, 0);
}

#[test]
fn task_board_settings_sanitizes_workflow_id() {
    // 新建配置默认使用 dev_lifecycle。
    assert_eq!(
        super::TaskBoardSettings::new().workflow_id,
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID
    );

    // 未知 workflow id 被 sanitize 回默认值。
    let settings = super::TaskBoardSettings {
        workflow_id: "does_not_exist".to_string(),
        ..super::TaskBoardSettings::new()
    }
    .sanitized();
    assert_eq!(settings.workflow_id, crate::app::task::DEFAULT_TASK_WORKFLOW_ID);

    // 旧配置缺字段时反序列化落默认值。
    let legacy: super::TaskBoardSettings = serde_json::from_str(
        "{\"auto_execute\":true,\"max_concurrent\":3,\"default_priority\":999,\"auto_promote_pool_tasks\":true,\"auto_promote_delay_seconds\":30}",
    )
    .expect("legacy settings without workflow_id should decode");
    assert_eq!(legacy.workflow_id, crate::app::task::DEFAULT_TASK_WORKFLOW_ID);
    assert_eq!(
        legacy.agent_scheduler_dispatch_mode,
        vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto::Local
    );
    let encoded = serde_json::to_value(&legacy).expect("legacy settings should serialize");
    assert!(encoded.get("auto_execute").is_none());
    assert!(encoded.get("auto_promote_pool_tasks").is_none());
}

#[test]
fn task_draft_defaults_match_task_creation_defaults() {
    let draft = super::TaskDraft::default();
    assert_eq!(draft.priority, "999");
    assert_eq!(draft.assignee, "VibeWindow");
    assert_eq!(draft.model, super::TASK_MODEL_AUTO);
    assert_eq!(draft.agent.as_deref(), Some(super::TASK_AGENT_MAIN));
    assert_eq!(draft.subtasks.len(), 3);
    assert_eq!(draft.task_split_mode, super::TaskSplitMode::FollowSystem);
    assert_eq!(draft.auto_promote_delay_seconds, "0");
}

#[test]
fn task_reasoning_effort_imports_roundtrip() {
    use vw_gateway_client::vw_api_types::ReasoningEffortDto;

    let json = super::parse_task_import_content(
        r#"[
            {"prompt":"default"},
            {"prompt":"none","reasoning_effort":"none"},
            {"prompt":"high","reasoning_effort":"high"}
        ]"#,
    )
    .expect("JSON import should parse");
    assert_eq!(json[0].reasoning_effort, None);
    assert_eq!(json[1].reasoning_effort, Some(ReasoningEffortDto::None));
    assert_eq!(json[2].reasoning_effort, Some(ReasoningEffortDto::High));

    let csv = super::parse_task_import_content("prompt,reasoning_effort\ncsv task,xhigh")
        .expect("CSV import should parse");
    assert_eq!(csv[0].reasoning_effort, Some(ReasoningEffortDto::Xhigh));

    let tsv = super::parse_task_import_content("prompt\treasoning_effort\ntsv task\tmax")
        .expect("TSV import should parse");
    assert_eq!(tsv[0].reasoning_effort, Some(ReasoningEffortDto::Max));

    let error = super::parse_task_import_content("prompt,reasoning_effort\ninvalid,extreme")
        .expect_err("invalid reasoning effort should fail import");
    assert!(error.contains("reasoning_effort"));

    let mut task = super::Task::default();
    task.reasoning_effort = Some(ReasoningEffortDto::High);
    let encoded = serde_json::to_value(&task).expect("serialize task reasoning effort");
    assert_eq!(encoded["reasoning_effort"], "high");
    let decoded: super::Task =
        serde_json::from_value(encoded.clone()).expect("deserialize task reasoning effort");
    assert_eq!(decoded.reasoning_effort, Some(ReasoningEffortDto::High));

    let mut legacy = encoded;
    legacy.as_object_mut().expect("task JSON object").remove("reasoning_effort");
    let legacy: super::Task =
        serde_json::from_value(legacy).expect("legacy task without reasoning_effort should decode");
    assert_eq!(legacy.reasoning_effort, None);
}

#[test]
fn task_split_mode_aliases_labels_and_system_override_are_stable() {
    let cases = [
        (
            super::TaskSplitMode::FollowSystem,
            "follow_system",
            "遵循系统",
            &["follow_system", "follow-system", "system", "default", "inherit"] as &[&str],
        ),
        (
            super::TaskSplitMode::Enabled,
            "enabled",
            "开启",
            &["enabled", "enable", "on", "true", "yes", "1"] as &[&str],
        ),
        (
            super::TaskSplitMode::Disabled,
            "disabled",
            "关闭",
            &["disabled", "disable", "off", "false", "no", "0"] as &[&str],
        ),
    ];

    assert_eq!(super::TaskSplitMode::all().len(), cases.len());
    for (mode, key, label, aliases) in cases {
        assert_eq!(mode.to_string_key(), key);
        assert_eq!(mode.label(), label);
        assert_eq!(mode.to_string(), label);
        for alias in aliases {
            assert_eq!(super::TaskSplitMode::parse_key(&format!(" {alias} ")), Some(mode));
        }
    }

    assert_eq!(super::TaskSplitMode::parse_key("occasionally"), None);
    assert!(!super::TaskSplitMode::FollowSystem.enabled_with_system(false));
    assert!(super::TaskSplitMode::FollowSystem.enabled_with_system(true));
    assert!(super::TaskSplitMode::Enabled.enabled_with_system(false));
    assert!(!super::TaskSplitMode::Disabled.enabled_with_system(true));
}

#[test]
fn task_import_json_maps_optional_fields_and_rejects_invalid_shapes() {
    use vw_gateway_client::vw_api_types::ReasoningEffortDto;

    let records = super::parse_task_import_content(
        r#"[
            {
                "priority": 7,
                "prompt": "  ship it  ",
                "model": " gpt-5 ",
                "agent": " main ",
                "acp_agent": " codex ",
                "reasoning_effort": " HIGH ",
                "task_split": "ON"
            },
            {"priority": 4294967296, "prompt": 3, "reasoning_effort": null}
        ]"#,
    )
    .expect("valid JSON array should import");

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].priority, Some(7));
    assert_eq!(records[0].prompt, "ship it");
    assert_eq!(records[0].model.as_deref(), Some("gpt-5"));
    assert_eq!(records[0].agent.as_deref(), Some("main"));
    assert_eq!(records[0].acp_agent.as_deref(), Some("codex"));
    assert_eq!(records[0].reasoning_effort, Some(ReasoningEffortDto::High));
    assert_eq!(records[0].task_split_mode, Some(super::TaskSplitMode::Enabled));
    assert_eq!(records[1].priority, None);
    assert!(records[1].prompt.is_empty());
    assert_eq!(records[1].reasoning_effort, None);

    assert!(
        super::parse_task_import_content("{}").expect_err("object root must fail").contains("数组")
    );
    assert!(
        super::parse_task_import_content("[1]")
            .expect_err("scalar item must fail")
            .contains("对象")
    );
    assert!(
        super::parse_task_import_content(r#"[{"reasoning_effort":3}]"#)
            .expect_err("non-string reasoning effort must fail")
            .contains("字符串或 null")
    );
}

#[test]
fn task_import_delimited_content_supports_localized_headers_and_reports_missing_prompt() {
    use vw_gateway_client::vw_api_types::ReasoningEffortDto;

    let records = super::parse_task_import_content(
        "优先级\t提示词\t模型\t代理\t智能体\t推理强度\t任务拆分\n12\t 写测试 \t auto \t main \t codex \t low \t no\n\ninvalid\t second ",
    )
    .expect("localized TSV should import");

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].priority, Some(12));
    assert_eq!(records[0].prompt, "写测试");
    assert_eq!(records[0].model.as_deref(), Some("auto"));
    assert_eq!(records[0].agent.as_deref(), Some("main"));
    assert_eq!(records[0].acp_agent.as_deref(), Some("codex"));
    assert_eq!(records[0].reasoning_effort, Some(ReasoningEffortDto::Low));
    assert_eq!(records[0].task_split_mode, Some(super::TaskSplitMode::Disabled));
    assert_eq!(records[1].priority, None);
    assert_eq!(records[1].prompt, "second");
    assert_eq!(records[1].model, None);

    assert!(
        super::parse_task_import_content("").expect_err("empty import must fail").contains("空")
    );
    assert!(
        super::parse_task_import_content("model,priority\nauto,1")
            .expect_err("prompt column is required")
            .contains("缺少 prompt")
    );
}

#[test]
fn task_status_workflow_helpers_use_selected_workflow_and_default_fallback() {
    let content_idea = super::TaskStatus::parse_key("content_idea").expect("custom status");
    let content_queued = super::TaskStatus::parse_key("content_queued").expect("custom status");
    let content_statuses = super::TaskStatus::all_for_workflow_id("content_publishing");

    assert_eq!(content_statuses.first(), Some(&content_idea));
    assert_eq!(content_idea.label_for_workflow_id("content_publishing"), "选题池");
    assert_eq!(content_idea.next_for_workflow_id("content_publishing"), Some(content_queued));
    assert_eq!(
        super::TaskStatus::all_for_workflow_id("missing-workflow"),
        super::TaskStatus::all().to_vec()
    );
    assert_eq!(
        super::TaskStatus::Pool.label_for_workflow_id("missing-workflow"),
        super::TaskStatus::Pool.label()
    );
    assert_eq!(content_idea.next_for_workflow_id("missing-workflow"), None);
}

#[test]
fn task_status_deserialization_rejects_empty_and_non_string_values() {
    let empty = serde_json::from_str::<super::TaskStatus>(r#""""#)
        .expect_err("empty status key must be rejected");
    assert!(empty.to_string().contains("empty task status key"));

    let non_string = serde_json::from_str::<super::TaskStatus>("42")
        .expect_err("status must use its string wire format");
    assert!(non_string.to_string().contains("task status key string"));
}

#[test]
fn subtask_explicit_timestamps_cover_saturation_and_terminal_state_fields() {
    let mut subtask = super::SubTask::new("deterministic".into());
    subtask.start_execution_at(100);
    assert_eq!(subtask.status, super::SubTaskStatus::Running);
    assert_eq!(subtask.display_execution_duration_ms(90), Some(0));

    subtask.mark_completed_at(140);
    assert_eq!(subtask.last_execution_duration_ms, Some(40));
    assert_eq!(subtask.execution_started_at_ms, None);
    assert_eq!(subtask.status, super::SubTaskStatus::Completed);
    assert!(subtask.completed);

    subtask.start_execution_at(200);
    subtask.mark_failed_at(150);
    assert_eq!(subtask.last_execution_duration_ms, Some(0));
    assert_eq!(subtask.execution_started_at_ms, None);
    assert_eq!(subtask.status, super::SubTaskStatus::Failed);
    assert!(!subtask.completed);

    let mut never_started = super::SubTask::new("not started".into());
    never_started.mark_completed_at(500);
    assert_eq!(never_started.last_execution_duration_ms, None);
}

#[test]
fn task_log_constructors_and_auto_merge_blank_checks_preserve_semantics() {
    let status_change =
        super::TaskLogEntry::new_status_change(super::TaskStatus::Pool, super::TaskStatus::Pending);
    assert_eq!(status_change.status_from, Some(super::TaskStatus::Pool));
    assert_eq!(status_change.status_to, Some(super::TaskStatus::Pending));
    assert!(status_change.message.contains(super::TaskStatus::Pool.label()));
    assert!(status_change.message.contains(super::TaskStatus::Pending.label()));

    let message = super::TaskLogEntry::new_message("note".into());
    assert_eq!(message.status_from, None);
    assert_eq!(message.status_to, None);
    assert_eq!(message.message, "note");

    let mut task = super::Task::default();
    task.merge_source_branch = Some("  ".into());
    task.merge_target_branch = Some("main".into());
    assert!(!task.should_auto_merge());
    task.merge_source_branch = Some("feature".into());
    task.merge_target_branch = Some("\t".into());
    assert!(!task.should_auto_merge());
}

#[test]
fn task_private_date_formatter_and_task_legacy_defaults_are_stable() {
    assert_eq!(super::format_task_date_from_ms(0), "19700101");
    assert_eq!(super::format_task_date_from_ms(1_704_067_200_000), "20240101");

    let mut value = serde_json::to_value(super::Task::new(5)).expect("task should serialize");
    let object = value.as_object_mut().expect("serialized task must be object");
    for field in [
        "agent",
        "acp_agent",
        "reasoning_effort",
        "task_split_mode",
        "last_error",
        "pause_reason",
        "retry_count",
        "last_active_at_ms",
        "execution_started_at_ms",
        "last_execution_duration_ms",
        "merge_source_branch",
        "merge_target_branch",
        "selected_worktree_slot_id",
        "selected_worktree_path",
        "selected_worktree_branch",
    ] {
        object.remove(field);
    }

    let decoded: super::Task =
        serde_json::from_value(value).expect("legacy task missing defaults should decode");
    assert_eq!(decoded.agent.as_deref(), None);
    assert_eq!(decoded.acp_agent, None);
    assert_eq!(decoded.reasoning_effort, None);
    assert_eq!(decoded.task_split_mode, super::TaskSplitMode::FollowSystem);
    assert_eq!(decoded.retry_count, 0);
    assert!(decoded.last_active_at_ms > 0);
    assert_eq!(decoded.execution_started_at_ms, None);
    assert_eq!(decoded.selected_worktree_path, None);
}

#[test]
fn task_import_rejects_unknown_reasoning_effort_in_json_and_delimited_content() {
    let json_error =
        super::parse_task_import_content(r#"[{"prompt":"ship","reasoning_effort":"ultra"}]"#)
            .expect_err("unknown JSON reasoning effort must fail");
    assert!(json_error.contains("非法 reasoning_effort 值: ultra"));

    let csv_error = super::parse_task_import_content("prompt,reasoning_effort\nship,ultra")
        .expect_err("unknown CSV reasoning effort must fail");
    assert!(csv_error.contains("非法 reasoning_effort 值: ultra"));
}
