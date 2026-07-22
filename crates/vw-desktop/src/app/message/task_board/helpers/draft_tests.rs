#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("draft_tests"));
}

#[test]
fn create_task_draft_normalizes_saved_preferences_and_preserves_defaults() {
    let draft = super::create_task_draft_with_preferences(
        "  gpt-5  ".to_string(),
        Some("  claude  ".to_string()),
    );

    assert_eq!(draft.model, "gpt-5");
    assert_eq!(draft.acp_agent.as_deref(), Some("claude"));
    assert_eq!(draft.priority, "999");
    assert!(draft.prompt.is_empty());

    let default_draft =
        super::create_task_draft_with_preferences("   ".to_string(), Some(" default ".to_string()));
    assert_eq!(default_draft.model, crate::app::task::TASK_MODEL_AUTO);
    assert_eq!(default_draft.acp_agent, None);
}

#[test]
fn reset_create_draft_rebuilds_state_and_synchronizes_prompt_editor() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_last_model = "  gpt-5  ".to_string();
    app.task_board_last_acp_agent = Some("  codex  ".to_string());
    app.task_board_draft.prompt = "stale draft".to_string();
    app.task_board_prompt_editor = iced::widget::text_editor::Content::with_text("stale editor");

    super::reset_create_draft(&mut app);

    assert_eq!(app.task_board_draft.model, "gpt-5");
    assert_eq!(app.task_board_draft.acp_agent.as_deref(), Some("codex"));
    assert!(app.task_board_draft.prompt.is_empty());
    assert!(app.task_board_prompt_editor.text().is_empty());
}

#[test]
fn executor_selection_helpers_trim_custom_values_and_clear_default_aliases() {
    let mut draft = crate::app::task::TaskDraft::default();
    super::set_draft_executor_selection(&mut draft, Some(" custom-agent ".to_string()));
    assert_eq!(draft.acp_agent.as_deref(), Some("custom-agent"));

    super::set_draft_executor_selection(&mut draft, Some(" ACP 网关 ".to_string()));
    assert_eq!(draft.acp_agent, None);

    let mut task = crate::app::task::Task::new(10);
    super::set_task_executor_selection(&mut task, Some(" opencode ".to_string()));
    assert_eq!(task.acp_agent.as_deref(), Some("opencode"));

    super::set_task_executor_selection(&mut task, None);
    assert_eq!(task.acp_agent, None);
    assert_eq!(super::normalize_task_acp_agent(None), None);
}

#[test]
fn task_acp_agent_label_maps_known_unknown_and_default_selections() {
    assert_eq!(super::task_acp_agent_label(Some(" claude ")), "Claude Code");
    assert_eq!(super::task_acp_agent_label(Some(" custom-agent ")), "custom-agent");
    assert_eq!(super::task_acp_agent_label(Some(" default ")), "未使用 ACP");
    assert_eq!(super::task_acp_agent_label(None), "未使用 ACP");
}

#[test]
fn parse_priority_accepts_positive_values_and_rejects_invalid_boundaries() {
    assert_eq!(super::parse_priority_or_default(" 42 ", 999), 42);
    assert_eq!(super::parse_priority_or_default("0", 999), 999);
    assert_eq!(super::parse_priority_or_default("-1", 999), 999);
    assert_eq!(super::parse_priority_or_default("not-a-number", 999), 999);
    assert_eq!(super::parse_priority_or_default("4294967296", 999), 999);
}

#[test]
fn import_demo_content_returns_each_supported_format_and_rejects_unknown_names() {
    let json = super::import_demo_content("JSON 示例").expect("JSON demo should exist");
    assert!(json.starts_with('['));
    assert!(json.contains("\"reasoning_effort\": \"high\""));

    let csv = super::import_demo_content("CSV 示例").expect("CSV demo should exist");
    assert!(csv.starts_with("priority,prompt,model,acp_agent,reasoning_effort,task_split_mode\n"));
    assert!(csv.contains(",auto,claude,max,follow_system"));

    let tsv = super::import_demo_content("TSV 示例").expect("TSV demo should exist");
    assert!(
        tsv.starts_with("priority\tprompt\tmodel\tacp_agent\treasoning_effort\ttask_split_mode\n")
    );
    assert!(tsv.contains("\tauto\topencode\thigh\tfollow_system"));

    assert_eq!(super::import_demo_content("YAML 示例"), None);
}

#[test]
fn json_import_prompt_uses_selected_values_and_includes_reasoning_effort() {
    let prompt = super::import_prompt_template(
        crate::app::task::TaskImportPromptFormat::Json,
        321,
        "  gpt-5  ",
        Some("claude"),
    );

    assert!(prompt.contains("priority 使用数字，建议优先使用 321"));
    assert!(prompt.contains("model 默认使用 gpt-5"));
    assert!(prompt.contains("acp_agent 默认使用 claude"));
    assert!(prompt.contains("\"reasoning_effort\": null"));
    assert!(prompt.contains("reasoning_effort 可留空表示默认"));
}

#[test]
fn csv_import_prompt_uses_auto_and_default_fallbacks_with_extended_header() {
    let prompt = super::import_prompt_template(
        crate::app::task::TaskImportPromptFormat::Csv,
        700,
        "   ",
        None,
    );

    assert!(prompt.contains("priority,prompt,model,acp_agent,reasoning_effort,task_split_mode"));
    assert!(prompt.contains("model 默认使用 auto"));
    assert!(prompt.contains("acp_agent 默认使用 default"));
    assert!(prompt.contains(",auto,default,,follow_system"));
}

#[test]
fn tsv_import_prompt_inserts_empty_reasoning_effort_column() {
    let prompt = super::import_prompt_template(
        crate::app::task::TaskImportPromptFormat::Tsv,
        900,
        "auto",
        Some("opencode"),
    );

    assert!(prompt.contains(
        "priority<TAB>prompt<TAB>model<TAB>acp_agent<TAB>reasoning_effort<TAB>task_split_mode"
    ));
    assert!(
        prompt.contains("priority\tprompt\tmodel\tacp_agent\treasoning_effort\ttask_split_mode")
    );
    assert!(prompt.contains("\tauto\topencode\t\tfollow_system"));
}

#[test]
fn initial_task_status_uses_first_status_for_selected_workflow() {
    use crate::app::task::TaskStatus;

    let _cache_lock = crate::app::task::runtime_task_lifecycle_cache_test_lock();
    assert_eq!(
        super::initial_task_status_for_workflow_id("marketing_campaign"),
        TaskStatus::Custom("campaign_ideas")
    );
    assert_eq!(
        super::initial_task_status_for_workflow_id("recruiting_pipeline"),
        TaskStatus::Custom("recruiting_pool")
    );
    assert_eq!(
        super::initial_task_status_for_workflow_id(crate::app::task::DEFAULT_TASK_WORKFLOW_ID),
        TaskStatus::Pool
    );
}

#[test]
fn task_lifecycle_cache_updates_status_helpers() {
    use crate::app::task::{TaskStatus, update_runtime_task_lifecycle_cache};
    use vw_gateway_client::vw_api_types::task_lifecycle::{
        TaskLifecycleStatusRoleDto, TaskLifecycleWorkflowDto,
    };

    let _cache_lock = crate::app::task::runtime_task_lifecycle_cache_test_lock();
    struct CacheGuard;

    impl Drop for CacheGuard {
        fn drop(&mut self) {
            crate::app::task::reset_runtime_task_lifecycle_cache()
                .expect("runtime lifecycle cache should reset");
        }
    }

    let _guard = CacheGuard;
    update_runtime_task_lifecycle_cache(vec![TaskLifecycleWorkflowDto {
        id: "cache_ops".to_string(),
        label: "Cache Ops".to_string(),
        built_in: false,
        statuses: vec![
            lifecycle_status(
                "cache_intake",
                "Cache Intake",
                TaskLifecycleStatusRoleDto::Backlog,
                Some("cache_execute"),
            ),
            lifecycle_status(
                "cache_execute",
                "Cache Execute",
                TaskLifecycleStatusRoleDto::Executing,
                Some("cache_done"),
            ),
            lifecycle_status("cache_done", "Cache Done", TaskLifecycleStatusRoleDto::Done, None),
        ],
    }])
    .expect("runtime lifecycle cache should update");

    assert_eq!(
        TaskStatus::all_for_workflow_id("cache_ops"),
        vec![
            TaskStatus::Custom("cache_intake"),
            TaskStatus::Custom("cache_execute"),
            TaskStatus::Custom("cache_done"),
        ]
    );
    assert_eq!(
        TaskStatus::Custom("cache_execute").label_for_workflow_id("cache_ops"),
        "Cache Execute"
    );
    assert_eq!(
        TaskStatus::Custom("cache_execute").next_for_workflow_id("cache_ops"),
        Some(TaskStatus::Custom("cache_done"))
    );
    assert_eq!(
        super::initial_task_status_for_workflow_id("cache_ops"),
        TaskStatus::Custom("cache_intake")
    );
}

fn lifecycle_status(
    key: &str,
    label: &str,
    role: vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusRoleDto,
    next: Option<&str>,
) -> vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusDto {
    use vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusRoleDto;

    vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusDto {
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        terminal: next.is_none(),
        auto_execution_candidate: !matches!(
            role,
            TaskLifecycleStatusRoleDto::Done | TaskLifecycleStatusRoleDto::Archived
        ),
        occupies_execution_slot: matches!(
            role,
            TaskLifecycleStatusRoleDto::Planning | TaskLifecycleStatusRoleDto::Executing
        ),
        is_scheduling_backlog: matches!(
            role,
            TaskLifecycleStatusRoleDto::Backlog
                | TaskLifecycleStatusRoleDto::Queued
                | TaskLifecycleStatusRoleDto::Planning
        ),
        next: next.map(str::to_string),
        transitions: Vec::new(),
    }
}
