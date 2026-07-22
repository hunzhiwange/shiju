use super::*;

fn keep(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn custom_capability(id: &str) -> TaskBoardStaffCapability {
    let mut capability =
        TaskBoardStaffCapability::custom_saved_workflow(id.to_string(), "Custom Flow".to_string());
    capability.input_summary = TaskBoardStaffCapabilitySummary {
        description: "Inputs".to_string(),
        fields: vec!["task_id".to_string(), "prompt".to_string()],
    };
    capability.result_summary = TaskBoardStaffCapabilitySummary {
        description: "Result".to_string(),
        fields: vec!["status".to_string()],
    };
    capability
}

#[test]
fn summary_fields_trim_whitespace_and_drop_empty_entries() {
    assert_eq!(
        fields_from_summary_input(" task_id, , result ,task_id,  "),
        vec!["task_id", "result", "task_id"]
    );
    assert!(fields_from_summary_input(" , , ").is_empty());
}

#[test]
fn capability_kind_labels_distinguish_builtin_and_saved_workflow() {
    assert_eq!(capability_kind_label(TaskBoardStaffCapabilityKind::BuiltIn), "内置能力");
    assert_eq!(
        capability_kind_label(TaskBoardStaffCapabilityKind::SavedWorkflow),
        "已保存 Workflow"
    );
}

#[test]
fn stable_id_lookup_rejects_unknown_ids_and_preserves_registry_metadata() {
    assert!(stage_capability_by_stable_id("missing-capability").is_none());

    let registry_capability = stage_capability_by_stable_id(EXECUTE_TASK_CAPABILITY_ID)
        .expect("execute task capability should be registered");
    let panel_item = builtin_stage_capability_item(EXECUTE_TASK_CAPABILITY_ID, "默认 Agent 执行")
        .expect("registered capability should become a panel item");

    assert_eq!(panel_item.id, registry_capability.stable_id);
    assert_eq!(panel_item.label, "默认 Agent 执行");
    assert_eq!(panel_item.input_contract, registry_capability.input_contract);
    assert_eq!(panel_item.result_semantics, registry_capability.result_semantics);
    assert_eq!(panel_item.permission_hint, registry_capability.permission_hint);
    assert_eq!(panel_item.requires_workflow_uuid, registry_capability.requires_workflow_uuid);
}

#[test]
fn builtin_panel_includes_local_task_split_and_unique_stable_ids() {
    let items = builtin_capability_panel_items();
    let task_split = items
        .iter()
        .find(|item| item.id == "task_split")
        .expect("task split should be included alongside registered capabilities");

    assert_eq!(task_split.label, "任务拆分");
    assert!(!task_split.requires_workflow_uuid);
    assert!(task_split.permission_hint.contains("不扩大"));

    let mut ids = items.iter().map(|item| item.id).collect::<Vec<_>>();
    let item_count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), item_count);
}

#[test]
fn capability_binding_summary_describes_success_failure_and_blocked_paths() {
    let summary = capability_binding_summary();

    assert!(summary.contains("成功"));
    assert!(summary.contains("失败"));
    assert!(summary.contains("阻塞"));
    assert!(summary.contains("人工处理"));
}

#[test]
fn builtin_capability_cards_build_for_workflow_and_non_workflow_items() {
    let items = builtin_capability_panel_items();
    let non_workflow = items
        .iter()
        .copied()
        .find(|item| !item.requires_workflow_uuid)
        .expect("non-workflow built-in item");
    let workflow = items
        .iter()
        .copied()
        .find(|item| item.requires_workflow_uuid)
        .expect("workflow built-in item");

    keep(build_builtin_capability_card(non_workflow));
    keep(build_builtin_capability_card(workflow));
}

#[test]
fn custom_capability_card_and_summary_editors_build_editable_controls() {
    let capability = custom_capability("custom-flow");

    keep(build_capability_summary_editor(
        &capability.id,
        "输入摘要",
        TaskBoardStaffCapabilitySummaryKind::Input,
        &capability.input_summary,
    ));
    keep(build_capability_summary_editor(
        &capability.id,
        "结果摘要",
        TaskBoardStaffCapabilitySummaryKind::Result,
        &capability.result_summary,
    ));
    keep(build_custom_capability_card(&capability));
}

#[test]
fn capability_management_view_builds_loading_error_empty_and_custom_states() {
    let (mut app, _task) = App::new();

    keep(build_capability_management_view(&app));

    app.task_board_staff_capability_catalog_error = Some("gateway unavailable".to_string());
    keep(build_capability_management_view(&app));

    app.task_board_staff_capability_catalog_error = None;
    app.task_board_staff_capability_catalog_workflow_id =
        Some(crate::app::task::DEFAULT_TASK_WORKFLOW_ID.to_string());
    app.task_board_staff_capability_catalog = vec![custom_capability("catalog-custom")];
    app.task_board_staff_capability_draft = Vec::new();
    keep(build_capability_management_view(&app));

    app.task_board_staff_capability_saving = true;
    app.task_board_staff_capability_draft = vec![
        TaskBoardStaffCapability {
            built_in: true,
            capability_type: TaskBoardStaffCapabilityKind::BuiltIn,
            requires_workflow_uuid: false,
            ..custom_capability("built-in-filtered")
        },
        custom_capability("custom-visible"),
    ];
    keep(build_capability_management_view(&app));
}
