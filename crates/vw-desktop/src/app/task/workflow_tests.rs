use super::*;
use crate::app::task::models::TaskStatus;

#[test]
fn default_task_workflow_preserves_existing_status_contract() {
    let workflow = default_task_workflow();
    assert_eq!(workflow.id, DEFAULT_TASK_WORKFLOW_ID);
    assert_eq!(workflow.id, "dev_lifecycle");

    // 工作流状态顺序必须与现有 TaskStatus::all() 完全一致。
    let workflow_statuses: Vec<TaskStatus> = workflow.statuses.iter().map(|d| d.status).collect();
    assert_eq!(workflow_statuses, TaskStatus::all().to_vec());
    assert_eq!(workflow.statuses.len(), 11);

    // 每个状态定义的 key、label 与现有枚举映射一致。
    for definition in workflow.statuses.iter() {
        assert_eq!(definition.key, definition.status.to_string_key());
        assert_eq!(definition.label, definition.status.label());
        assert_eq!(TaskStatus::parse_key(definition.key), Some(definition.status));
    }

    // 终态标记与现有 next() == None 的状态一致。
    for definition in workflow.statuses.iter() {
        let has_no_next = definition.status.next().is_none();
        assert_eq!(
            definition.is_terminal, has_no_next,
            "终态标记应与 next() 是否为 None 对齐: {:?}",
            definition.status
        );
    }

    // 自动执行候选分类必须覆盖 Pool（自动提升起点）与 Running（执行中），
    // 并排除终态。
    let pool = workflow.status_definition(TaskStatus::Pool).expect("pool definition");
    assert!(pool.auto_execution_candidate);
    let archived = workflow.status_definition(TaskStatus::Archived).expect("archived definition");
    assert!(!archived.auto_execution_candidate);
    assert!(archived.is_terminal);
}

#[test]
fn task_workflow_by_id_resolves_default_and_rejects_unknown() {
    let resolved = task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID).expect("default workflow");
    assert_eq!(resolved.id(), DEFAULT_TASK_WORKFLOW_ID);
    assert!(task_workflow_by_id("does_not_exist").is_none());
}

#[test]
fn builtin_gateway_snapshot_includes_failed_retry_transition() {
    let workflow = TaskWorkflow::from(default_task_workflow());

    let snapshot = task_workflow_snapshot_for_gateway(&workflow);

    let failed = snapshot
        .statuses
        .iter()
        .find(|status| status.key == "failed")
        .expect("default workflow should contain failed status");
    assert!(
        failed
            .transitions
            .iter()
            .any(|transition| { transition.outcome == "retry" && transition.target == "pending" })
    );
}

#[test]
fn default_task_workflow_next_matches_existing_status_next() {
    let workflow = default_task_workflow();
    for status in TaskStatus::all() {
        assert_eq!(workflow.next_status(status), status.next());
    }
}

#[test]
fn state_machine_allows_default_lifecycle_intents() {
    let machine = TaskStateMachine::new(default_task_workflow());

    // ManualMove 保留看板自由移动：跨列任意移动都允许。
    assert!(machine.can_transition(
        TaskStatus::Pool,
        TaskStatus::Completed,
        TaskTransitionIntent::ManualMove
    ));
    assert!(machine.can_transition(
        TaskStatus::Completed,
        TaskStatus::Pool,
        TaskTransitionIntent::ManualMove
    ));
    assert!(machine.can_transition(
        TaskStatus::Running,
        TaskStatus::Failed,
        TaskTransitionIntent::ManualMove
    ));

    // 自动意图按规则允许其专属边。
    assert!(machine.can_transition(
        TaskStatus::Pool,
        TaskStatus::Pending,
        TaskTransitionIntent::AutoPromote
    ));
    assert!(machine.can_transition(
        TaskStatus::Pending,
        TaskStatus::Running,
        TaskTransitionIntent::StartExecution
    ));
    assert!(machine.can_transition(
        TaskStatus::Planning,
        TaskStatus::Running,
        TaskTransitionIntent::StartExecution
    ));
    assert!(machine.can_transition(
        TaskStatus::Failed,
        TaskStatus::Running,
        TaskTransitionIntent::StartExecution
    ));
    assert!(machine.can_transition(
        TaskStatus::Running,
        TaskStatus::CodeComplete,
        TaskTransitionIntent::ExecutionSucceeded
    ));
    assert!(machine.can_transition(
        TaskStatus::Running,
        TaskStatus::Failed,
        TaskTransitionIntent::ExecutionFailed
    ));
    assert!(machine.can_transition(
        TaskStatus::CodeReview,
        TaskStatus::PrSubmitted,
        TaskTransitionIntent::ReviewApproved
    ));
    assert!(machine.can_transition(
        TaskStatus::CodeReview,
        TaskStatus::Failed,
        TaskTransitionIntent::ReviewBlocked
    ));
    assert!(machine.can_transition(
        TaskStatus::PrSubmitted,
        TaskStatus::Completed,
        TaskTransitionIntent::MergeSucceeded
    ));
    assert!(machine.can_transition(
        TaskStatus::PrSubmitted,
        TaskStatus::Failed,
        TaskTransitionIntent::MergeFailed
    ));
    assert!(machine.can_transition(
        TaskStatus::Completed,
        TaskStatus::Archived,
        TaskTransitionIntent::Archive
    ));

    // validate_transition 对允许边返回 Ok。
    assert!(
        machine
            .validate_transition(
                TaskStatus::Pool,
                TaskStatus::Pending,
                TaskTransitionIntent::AutoPromote
            )
            .is_ok()
    );

    // next_for_intent 返回该意图在给定来源状态下的唯一目标。
    assert_eq!(
        machine.next_for_intent(TaskStatus::Pool, TaskTransitionIntent::AutoPromote),
        Some(TaskStatus::Pending)
    );
    assert_eq!(
        machine.next_for_intent(TaskStatus::PrSubmitted, TaskTransitionIntent::MergeSucceeded),
        Some(TaskStatus::Completed)
    );
}

#[test]
fn state_machine_rejects_invalid_automatic_transition() {
    let machine = TaskStateMachine::new(default_task_workflow());

    // AutoPromote 只允许 Pool -> Pending，其他来源或目标都拒绝。
    assert!(!machine.can_transition(
        TaskStatus::Pending,
        TaskStatus::Planning,
        TaskTransitionIntent::AutoPromote
    ));
    assert!(!machine.can_transition(
        TaskStatus::Pool,
        TaskStatus::Completed,
        TaskTransitionIntent::AutoPromote
    ));

    // MergeSucceeded 只允许 PrSubmitted -> Completed。
    assert!(!machine.can_transition(
        TaskStatus::Running,
        TaskStatus::Completed,
        TaskTransitionIntent::MergeSucceeded
    ));

    // 错误的目标状态也被拒绝。
    assert!(!machine.can_transition(
        TaskStatus::Pool,
        TaskStatus::Running,
        TaskTransitionIntent::AutoPromote
    ));

    // validate_transition 返回包含 from、to、intent 与 workflow id 的错误。
    let err = machine
        .validate_transition(
            TaskStatus::Pending,
            TaskStatus::Planning,
            TaskTransitionIntent::AutoPromote,
        )
        .expect_err("invalid automatic transition must be rejected");
    assert_eq!(err.from, TaskStatus::Pending);
    assert_eq!(err.to, TaskStatus::Planning);
    assert_eq!(err.intent, TaskTransitionIntent::AutoPromote);
    assert_eq!(err.workflow_id, DEFAULT_TASK_WORKFLOW_ID);

    // next_for_intent 对没有定义边的来源状态返回 None。
    assert_eq!(
        machine.next_for_intent(TaskStatus::Pending, TaskTransitionIntent::AutoPromote),
        None
    );
}

#[test]
fn transition_error_display_preserves_rejected_transition_context() {
    let machine = TaskStateMachine::default_lifecycle();
    let error = machine
        .validate_transition(
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskTransitionIntent::MergeSucceeded,
        )
        .expect_err("invalid transition must expose an error");

    assert_eq!(
        error.to_string(),
        "工作流 dev_lifecycle 拒绝转移: Running -> Completed (意图 MergeSucceeded)"
    );
    let source = std::error::Error::source(&error);
    assert!(source.is_none());
}

#[test]
fn builtin_statuses_bind_expected_roles() {
    let workflow = default_task_workflow();

    // 内置 11 状态各自绑定到唯一且语义正确的角色。
    let expected = [
        (TaskStatus::Pool, StatusRole::Backlog),
        (TaskStatus::Pending, StatusRole::Queued),
        (TaskStatus::Planning, StatusRole::Planning),
        (TaskStatus::Running, StatusRole::Executing),
        (TaskStatus::Failed, StatusRole::Failed),
        (TaskStatus::Paused, StatusRole::Paused),
        (TaskStatus::CodeComplete, StatusRole::ReviewReady),
        (TaskStatus::CodeReview, StatusRole::Review),
        (TaskStatus::PrSubmitted, StatusRole::Merging),
        (TaskStatus::Completed, StatusRole::Done),
        (TaskStatus::Archived, StatusRole::Archived),
    ];

    for (status, role) in expected {
        assert_eq!(
            workflow.status_definition(status).expect("status definition").role,
            role,
            "状态 {status:?} 必须绑定角色 {role:?}"
        );
        assert_eq!(
            workflow.status_role(status),
            Some(role),
            "status_role 查询必须返回状态 {status:?} 的角色 {role:?}"
        );
    }

    // 引擎角色互不相同：11 个内置状态映射出 11 个互异角色。
    let mut roles: Vec<StatusRole> = expected.iter().map(|(_, role)| *role).collect();
    let total = roles.len();
    roles.sort_unstable_by_key(|role| format!("{role:?}"));
    roles.dedup();
    assert_eq!(roles.len(), total, "内置状态绑定的角色必须互异");

    // Custom 角色存在，且与所有引擎角色区分。
    assert!(
        !expected.iter().any(|(_, role)| *role == StatusRole::Custom),
        "Custom 角色必须独立于内置引擎角色，不被任何内置状态使用"
    );
}

#[test]
fn status_role_lookup_resolves_known_role() {
    let workflow = default_task_workflow();

    // 按角色反查能定位到唯一状态。
    assert_eq!(workflow.status_role(TaskStatus::Running), Some(StatusRole::Executing));
    assert_eq!(workflow.status_role(TaskStatus::Pool), Some(StatusRole::Backlog));
    assert_eq!(workflow.status_role(TaskStatus::Archived), Some(StatusRole::Archived));

    // Custom 角色作为预留语义存在且可比较。
    let custom = StatusRole::Custom;
    assert_eq!(custom, StatusRole::Custom);
    assert_ne!(custom, StatusRole::Executing);
}

#[test]
fn role_derived_scheduling_semantics_match_builtin_flags() {
    let workflow = default_task_workflow();

    // 角色派生的调度语义必须对内置每个状态与既有布尔字段逐状态一致：
    // occupies_execution_slot / is_scheduling_backlog 已有布尔字段，角色派生方法
    // 必须返回相同结果，证明判定来源从变体/字段切到角色后行为零变化。
    for definition in workflow.statuses.iter() {
        let status = definition.status;
        assert_eq!(
            workflow.role_occupies_execution_slot(status),
            definition.occupies_execution_slot,
            "状态 {status:?} 的角色派生占槽判定必须与布尔字段一致",
        );
        assert_eq!(
            workflow.role_is_scheduling_backlog(status),
            definition.is_scheduling_backlog,
            "状态 {status:?} 的角色派生调度积压判定必须与布尔字段一致",
        );
    }

    // 重试候选语义由 Failed 角色推导：仅 Failed 状态可重试，其余状态不可。
    for definition in workflow.statuses.iter() {
        let status = definition.status;
        let expected_retry = workflow.status_role(status) == Some(StatusRole::Failed);
        assert_eq!(
            workflow.role_is_retry_candidate(status),
            expected_retry,
            "状态 {status:?} 的可重试判定必须由 Failed 角色推导",
        );
    }
    assert!(workflow.role_is_retry_candidate(TaskStatus::Failed));
    assert!(!workflow.role_is_retry_candidate(TaskStatus::Running));

    // 超时监控语义由 Executing 角色推导：仅 Running（执行中）状态被超时监控。
    for definition in workflow.statuses.iter() {
        let status = definition.status;
        let expected_timeout = workflow.status_role(status) == Some(StatusRole::Executing);
        assert_eq!(
            workflow.role_is_timeout_monitored(status),
            expected_timeout,
            "状态 {status:?} 的超时监控判定必须由 Executing 角色推导",
        );
    }
    assert!(workflow.role_is_timeout_monitored(TaskStatus::Running));
    assert!(!workflow.role_is_timeout_monitored(TaskStatus::Planning));

    // 占槽角色覆盖 Executing 与 Planning：Running 与 Planning 占槽，其余不占。
    assert!(workflow.role_occupies_execution_slot(TaskStatus::Running));
    assert!(workflow.role_occupies_execution_slot(TaskStatus::Planning));
    assert!(!workflow.role_occupies_execution_slot(TaskStatus::Pool));

    // 调度积压角色覆盖 Backlog/Queued/Planning：Pool、Pending、Planning 属积压。
    assert!(workflow.role_is_scheduling_backlog(TaskStatus::Pool));
    assert!(workflow.role_is_scheduling_backlog(TaskStatus::Pending));
    assert!(workflow.role_is_scheduling_backlog(TaskStatus::Planning));
    assert!(!workflow.role_is_scheduling_backlog(TaskStatus::Running));
}

#[test]
fn registered_workflow_ids_are_unique_and_documented() {
    let workflows = all_task_workflows();

    // 至少注册默认工作流。
    assert!(!workflows.is_empty(), "必须至少注册一个工作流");

    // 工作流 id 全局唯一。
    let mut ids: Vec<&str> = workflows.iter().map(|workflow| workflow.id()).collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "工作流 id 必须唯一");

    // 默认工作流必须存在且可通过 id 解析。
    assert!(
        workflows.iter().any(|workflow| workflow.id() == DEFAULT_TASK_WORKFLOW_ID),
        "默认工作流 {DEFAULT_TASK_WORKFLOW_ID} 必须在注册表中"
    );
    let default = task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID).expect("默认工作流可解析");
    assert_eq!(default.id(), DEFAULT_TASK_WORKFLOW_ID);

    for workflow in workflows {
        // 每个注册工作流都能按 id 反查到同一定义。
        let resolved = task_workflow_by_id(workflow.id())
            .unwrap_or_else(|| panic!("注册工作流 {} 必须能按 id 解析", workflow.id()));
        assert_eq!(resolved.id(), workflow.id());

        // 工作流非空且每个状态 key 唯一、可被 TaskStatus 解析、与枚举映射一致。
        assert!(!workflow.statuses().is_empty(), "工作流 {} 必须包含状态", workflow.id());
        let mut keys: Vec<&str> = Vec::with_capacity(workflow.statuses().len());
        for definition in workflow.statuses() {
            assert_eq!(
                TaskStatus::parse_key(&definition.key),
                Some(definition.status),
                "工作流 {} 的状态 key {} 必须可解析为对应枚举",
                workflow.id(),
                definition.key
            );
            assert_eq!(
                definition.key,
                definition.status.to_string_key(),
                "工作流 {} 的状态 key 必须与枚举 to_string_key 一致",
                workflow.id()
            );
            if workflow.id() == DEFAULT_TASK_WORKFLOW_ID {
                assert_eq!(
                    definition.label,
                    definition.status.label(),
                    "默认工作流 {} 的状态 label 必须与枚举 label 一致",
                    workflow.id()
                );
            }
            // 终态必须没有默认下一状态，非终态必须有。
            assert_eq!(
                definition.is_terminal,
                definition.next.is_none(),
                "工作流 {} 状态 {:?} 的终态标记必须与 next 是否为 None 对齐",
                workflow.id(),
                definition.status
            );
            keys.push(&definition.key);
        }
        let key_total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), key_total, "工作流 {} 的状态 key 必须唯一", workflow.id());

        // 默认工作流的下一状态链必须从 next() 解析，保证状态机可前向推进。
        for definition in workflow.statuses() {
            if let Some(next) = definition.next {
                assert!(
                    workflow.status_definition(next).is_some(),
                    "工作流 {} 状态 {:?} 的 next {:?} 必须在工作流内定义",
                    workflow.id(),
                    definition.status,
                    next
                );
            }
        }
    }
}

#[test]
fn industry_presets_are_registered_with_business_labels() {
    let content = task_workflow_by_id("content_publishing").expect("内容发布预设必须可按 id 解析");
    assert_eq!(content.label(), "内容创作与多平台发布");
    assert!(content.statuses().iter().any(|definition| definition.label == "多平台发布"));
    assert_eq!(
        content.status_role(TaskStatus::Custom("content_publishing")),
        Some(StatusRole::Merging)
    );
    assert_eq!(
        content.next_status(TaskStatus::Custom("content_publishing")),
        Some(TaskStatus::Custom("content_published"))
    );

    let workflow_ids: Vec<String> =
        all_task_workflows().into_iter().map(|workflow| workflow.id).collect();
    for id in
        ["content_publishing", "marketing_campaign", "customer_support", "recruiting_pipeline"]
    {
        assert!(workflow_ids.iter().any(|workflow_id| workflow_id == id), "缺少行业预设 {id}");
    }
}

#[test]
fn default_status_entrypoint_and_builtin_list_expose_stable_order() {
    let default_statuses = default_task_workflow_statuses();
    assert_eq!(default_statuses, default_task_workflow().statuses);

    let workflows = builtin_task_workflows();
    assert_eq!(workflows.first().map(|workflow| workflow.id), Some(DEFAULT_TASK_WORKFLOW_ID));
    assert_eq!(
        workflows.iter().map(|workflow| workflow.id).collect::<Vec<_>>(),
        vec![
            "dev_lifecycle",
            "content_publishing",
            "marketing_campaign",
            "customer_support",
            "recruiting_pipeline",
        ]
    );
}

#[test]
fn gateway_snapshot_maps_all_default_roles_and_transition_targets_to_keys() {
    let workflow = TaskWorkflow::from(default_task_workflow());

    let snapshot = task_workflow_snapshot_for_gateway(&workflow);

    let expected_roles = [
        ("pool", TaskPoolWorkflowStatusRoleDto::Backlog),
        ("pending", TaskPoolWorkflowStatusRoleDto::Queued),
        ("planning", TaskPoolWorkflowStatusRoleDto::Planning),
        ("running", TaskPoolWorkflowStatusRoleDto::Executing),
        ("failed", TaskPoolWorkflowStatusRoleDto::Failed),
        ("paused", TaskPoolWorkflowStatusRoleDto::Paused),
        ("code_complete", TaskPoolWorkflowStatusRoleDto::ReviewReady),
        ("code_review", TaskPoolWorkflowStatusRoleDto::Review),
        ("pr_submitted", TaskPoolWorkflowStatusRoleDto::Merging),
        ("completed", TaskPoolWorkflowStatusRoleDto::Done),
        ("archived", TaskPoolWorkflowStatusRoleDto::Archived),
    ];

    for (key, role) in expected_roles {
        let status = snapshot
            .statuses
            .iter()
            .find(|status| status.key == key)
            .unwrap_or_else(|| panic!("gateway snapshot should include status {key}"));
        assert_eq!(status.role, role, "gateway role mismatch for status {key}");
    }

    let completed = snapshot
        .statuses
        .iter()
        .find(|status| status.key == "completed")
        .expect("completed status");
    assert_eq!(completed.next.as_deref(), Some("archived"));

    let archived =
        snapshot.statuses.iter().find(|status| status.key == "archived").expect("archived status");
    assert!(archived.terminal);
    assert!(archived.next.is_none());
}
