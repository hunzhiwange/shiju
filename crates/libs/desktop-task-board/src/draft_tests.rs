// 验证任务草稿默认值、执行器选择与导入提示生成。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
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
    assert_eq!(default_draft.model, desktop_task_model::TASK_MODEL_AUTO);
    assert_eq!(default_draft.acp_agent, None);
}

#[test]
fn executor_selection_helpers_trim_custom_values_and_clear_default_aliases() {
    let mut draft = desktop_task_model::TaskDraft::default();
    super::set_draft_executor_selection(&mut draft, Some(" custom-agent ".to_string()));
    assert_eq!(draft.acp_agent.as_deref(), Some("custom-agent"));

    super::set_draft_executor_selection(&mut draft, Some(" ACP 网关 ".to_string()));
    assert_eq!(draft.acp_agent, None);

    let mut task = desktop_task_model::Task::new(10);
    super::set_task_executor_selection(&mut task, Some(" opencode ".to_string()));
    assert_eq!(task.acp_agent.as_deref(), Some("opencode"));

    super::set_task_executor_selection(&mut task, None);
    assert_eq!(task.acp_agent, None);
    assert_eq!(super::normalize_task_acp_agent(None), None);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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
    assert!(csv.contains(",auto,claude,max,follow_system"));

    let tsv = super::import_demo_content("TSV 示例").expect("TSV demo should exist");
    assert!(tsv.starts_with("priority\tprompt\tmodel\tacp_agent\treasoning_effort\n"));
    assert!(tsv.contains("\tauto\topencode\thigh\tfollow_system"));

    assert_eq!(super::import_demo_content("YAML 示例"), None);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

#[test]
fn initial_task_status_uses_first_status_for_selected_workflow() {
    use desktop_task_model::TaskStatus;

    let _cache_lock = desktop_task_model::runtime_task_lifecycle_cache_test_lock();
    assert_eq!(
        super::initial_task_status_for_workflow_id("marketing_campaign"),
        TaskStatus::Custom("campaign_ideas")
    );
    assert_eq!(
        super::initial_task_status_for_workflow_id("recruiting_pipeline"),
        TaskStatus::Custom("recruiting_pool")
    );
    assert_eq!(
        super::initial_task_status_for_workflow_id(desktop_task_model::DEFAULT_TASK_WORKFLOW_ID),
        TaskStatus::Pool
    );
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn task_lifecycle_cache_updates_status_helpers() {
    use desktop_task_model::{TaskStatus, update_runtime_task_lifecycle_cache};
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
                "cache_intake",
                "Cache Intake",
                TaskLifecycleStatusCategoryDto::Unstarted,
            ),
            lifecycle_status(
                "cache_execute",
                "Cache Execute",
                TaskLifecycleStatusCategoryDto::Started,
            ),
            lifecycle_status("cache_done", "Cache Done", TaskLifecycleStatusCategoryDto::Done),
        ];
        TaskLifecycleWorkflowDto {
            id: "cache_ops".to_string(),
            label: "Cache Ops".to_string(),
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
        None // 任务执行结果不再隐式推进看板步骤。
    );
    assert_eq!(
        super::initial_task_status_for_workflow_id("cache_ops"),
        TaskStatus::Custom("cache_intake")
    );
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
