//! 测试任务板元数据、状态移动、归档恢复与旧任务导入。
//! 元数据补丁与旧格式导入同时覆盖，非法输入不得部分移动或覆盖任务。

use anyhow::Result;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use vw_api_types::ReasoningEffortDto;
use vw_api_types::agent_scheduler::{
    AgentSchedulerLegacyTaskImportDto, AgentSchedulerMoveTaskPayloadDto,
    AgentSchedulerStaffCommandPayloadDto, AgentSchedulerStageActionCommandPayloadDto,
    AgentSchedulerStageActionInputsDto, AgentSchedulerStageActionOutcomeDto,
    AgentSchedulerTaskMetadataPatchDto, AgentTaskStatusDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::TaskPoolWorkflowStatusRoleDto;

use super::domain::TaskId;
use super::lifecycle_bridge::LifecycleStatusProjection;
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::logs::AppendAgentTaskLogRequest;
use super::store::tasks::NewAgentTask;
use super::store::tasks::StoredAgentTask;
use super::task_board::{
    AgentTaskBoardService, apply_metadata_patch, new_task_from_legacy_import,
    reasoning_effort_from_metadata, task_board_status_key,
};

// 旧任务夹具固定非关注字段，让导入测试专注元数据兼容与状态推导。
fn legacy_import(metadata: Value) -> AgentSchedulerLegacyTaskImportDto {
    AgentSchedulerLegacyTaskImportDto {
        task_id: "task-import".into(),
        title: "Imported task".to_string(),
        status_key: None,
        status: None,
        priority: 3,
        order_no: 4,
        model: String::new(),
        agent: None,
        acp_agent: None,
        reasoning_effort: None,
        prompt: String::new(),
        workflow_id: None,
        metadata,
        created_at_ms: TimestampMs(10),
        updated_at_ms: None,
    }
}

fn stored_task(status: AgentTaskStatus, metadata: Value) -> StoredAgentTask {
    StoredAgentTask {
        id: TaskId::new("task-status").unwrap(),
        title: "Status task".to_string(),
        kind: AgentTaskKind::Finite,
        status,
        spec_uri: "spec.md".to_string(),
        meta_uri: "meta.json".to_string(),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata,
        created_at_ms: TimestampMs(1),
        updated_at_ms: TimestampMs(2),
    }
}

fn new_board_task(id: &str, status_key: &str, status: AgentTaskStatus) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new(id)?,
        title: format!("Task {id}"),
        kind: AgentTaskKind::Finite,
        status,
        spec_uri: format!("shared/tasks/{id}/spec.md"),
        meta_uri: format!("shared/tasks/{id}/meta.json"),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "task_board.status_key": status_key,
            "selected_worktree_path": "/workspace/task",
            "selected_worktree_slot_id": "slot-1",
            "selected_worktree_branch": "feature/task"
        }),
        created_at_ms: TimestampMs(1),
    })
}

#[test]
fn task_reasoning_effort_roundtrips_metadata() -> Result<()> {
    let legacy = AgentSchedulerLegacyTaskImportDto {
        task_id: "task-reasoning".into(),
        title: "Reasoning task".to_string(),
        status_key: None,
        status: None,
        priority: 1,
        order_no: 0,
        model: "auto".to_string(),
        agent: None,
        acp_agent: Some("codex".to_string()),
        reasoning_effort: Some(ReasoningEffortDto::High),
        prompt: "reason carefully".to_string(),
        workflow_id: None,
        metadata: json!({}),
        created_at_ms: TimestampMs(1),
        updated_at_ms: None,
    };
    let imported = new_task_from_legacy_import(legacy, None)?;
    assert_eq!(imported.metadata["reasoning_effort"], "high");
    assert_eq!(reasoning_effort_from_metadata(&imported.metadata)?, Some(ReasoningEffortDto::High));

    let Value::Object(mut metadata) = imported.metadata else {
        panic!("task metadata must be an object");
    };
    apply_metadata_patch(
        &mut metadata,
        AgentSchedulerTaskMetadataPatchDto {
            reasoning_effort: Some(ReasoningEffortDto::Max),
            ..Default::default()
        },
    )?;
    assert_eq!(metadata["reasoning_effort"], "max");
    assert_eq!(
        reasoning_effort_from_metadata(&Value::Object(metadata))?,
        Some(ReasoningEffortDto::Max)
    );
    assert_eq!(reasoning_effort_from_metadata(&json!({}))?, None);
    assert_eq!(reasoning_effort_from_metadata(&json!({"reasoning_effort": null}))?, None);
    Ok(())
}

#[test]
fn metadata_patch_trims_fields_merges_metadata_and_removes_keys() -> Result<()> {
    let Value::Object(mut metadata) = json!({"remove_me": true, "preserved": 1}) else {
        unreachable!();
    };

    apply_metadata_patch(
        &mut metadata,
        AgentSchedulerTaskMetadataPatchDto {
            status_key: Some("  completed  ".to_string()),
            priority: Some(8),
            order_no: Some(-2),
            model: Some("  model-a  ".to_string()),
            metadata_merge: Some(json!({"merged": "value"})),
            metadata_remove_keys: vec!["remove_me".to_string()],
            ..Default::default()
        },
    )?;

    assert_eq!(metadata["task_board.status_key"], "completed");
    assert_eq!(metadata["priority"], 8);
    assert_eq!(metadata["order_no"], -2);
    assert_eq!(metadata["model"], "model-a");
    assert_eq!(metadata["merged"], "value");
    assert_eq!(metadata["preserved"], 1);
    assert!(!metadata.contains_key("remove_me"));
    Ok(())
}

#[test]
fn metadata_patch_rejects_blank_text_and_non_object_merge() {
    let mut metadata = serde_json::Map::new();
    let blank = apply_metadata_patch(
        &mut metadata,
        AgentSchedulerTaskMetadataPatchDto { model: Some("   ".to_string()), ..Default::default() },
    )
    .expect_err("blank model should be rejected");
    assert!(blank.to_string().contains("model must not be empty"));

    let non_object = apply_metadata_patch(
        &mut metadata,
        AgentSchedulerTaskMetadataPatchDto {
            metadata_merge: Some(json!(["not", "an", "object"])),
            ..Default::default()
        },
    )
    .expect_err("non-object merge should be rejected");
    assert!(non_object.to_string().contains("metadata_merge must be a JSON object"));
}

#[test]
fn legacy_import_applies_defaults_and_explicit_status() -> Result<()> {
    let mut legacy = legacy_import(json!({"custom": true}));
    legacy.status = Some(AgentTaskStatusDto::Paused);
    legacy.updated_at_ms = Some(TimestampMs(20));

    let imported = new_task_from_legacy_import(legacy, None)?;

    assert_eq!(imported.status, AgentTaskStatus::Paused);
    assert_eq!(imported.metadata["model"], "auto");
    assert_eq!(imported.metadata["priority"], 3);
    assert_eq!(imported.metadata["order_no"], 4);
    assert_eq!(imported.metadata["custom"], true);
    assert_eq!(imported.metadata["task_board.legacy_updated_at_ms"], 20);
    assert!(imported.metadata.get("prompt").is_none());
    Ok(())
}

#[test]
fn legacy_import_rejects_invalid_metadata_and_blank_title() {
    let invalid_metadata = new_task_from_legacy_import(legacy_import(json!("invalid")), None)
        .expect_err("scalar metadata should be rejected");
    assert!(invalid_metadata.to_string().contains("task metadata must be a JSON object"));

    let mut blank_title = legacy_import(Value::Null);
    blank_title.title = " \n ".to_string();
    let error =
        new_task_from_legacy_import(blank_title, None).expect_err("blank title should be rejected");
    assert!(error.to_string().contains("task title must not be empty"));
}

#[test]
fn task_board_status_key_uses_control_map_and_legacy_fallbacks() {
    let control = stored_task(
        AgentTaskStatus::Assigned,
        json!({
            "task_board.status_key": "planning",
            "task_board.status_key_map": {"queued": "ready_for_agent"}
        }),
    );
    assert_eq!(task_board_status_key(&control), "ready_for_agent");

    let submitted = stored_task(AgentTaskStatus::Submitted, json!({}));
    assert_eq!(task_board_status_key(&submitted), "code_complete");

    let blank_explicit =
        stored_task(AgentTaskStatus::Completed, json!({"task_board.status_key": "   "}));
    assert_eq!(task_board_status_key(&blank_explicit), "completed");
}

#[test]
fn reasoning_effort_metadata_rejects_unknown_value() {
    let error = reasoning_effort_from_metadata(&json!({"reasoning_effort": "turbo"}))
        .expect_err("unknown effort should be rejected");

    assert!(error.to_string().contains("invalid task metadata field reasoning_effort"));
}

#[test]
fn task_board_service_moves_archives_soft_deletes_and_restores_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(temp.path())?;
    let service = AgentTaskBoardService::new(&store);
    let task_id = TaskId::new("task-service-flow")?;
    store.create_task(
        temp.path(),
        new_board_task(task_id.as_str(), "pending", AgentTaskStatus::Queued)?,
    )?;

    let moved = service
        .move_task(
            temp.path(),
            &task_id,
            AgentSchedulerMoveTaskPayloadDto {
                from_status_key: Some("pending".to_string()),
                to_status_key: "running".to_string(),
                to_status: Some(AgentTaskStatusDto::Running),
                order_no: Some(9),
            },
            Some("move to execution"),
            TimestampMs(20),
        )?
        .expect("task should move");
    assert_eq!(moved.task.status, AgentTaskStatus::Running);
    assert_eq!(moved.task.metadata["task_board.status_key"], "running");
    assert_eq!(moved.task.metadata["order_no"], 9);
    assert_eq!(moved.event.payload["command"], "move");
    assert_eq!(moved.event.from_status_key.as_deref(), Some("pending"));
    assert_eq!(moved.event.to_status_key.as_deref(), Some("running"));

    let mismatch = match service.move_task(
        temp.path(),
        &task_id,
        AgentSchedulerMoveTaskPayloadDto {
            from_status_key: Some("pending".to_string()),
            to_status_key: "completed".to_string(),
            to_status: Some(AgentTaskStatusDto::Completed),
            order_no: None,
        },
        None,
        TimestampMs(21),
    ) {
        Ok(_) => panic!("stale source status should be rejected"),
        Err(error) => error,
    };
    assert!(mismatch.to_string().contains("not pending"));

    let archived = service
        .archive_task(temp.path(), &task_id, Some("done for now"), TimestampMs(30))?
        .expect("task should archive");
    assert_eq!(archived.task.status, AgentTaskStatus::Archived);
    assert_eq!(archived.task.metadata["task_board.status_key"], "archived");
    assert_eq!(archived.task.metadata["task_board.previous_status_key"], "running");
    assert!(archived.task.metadata.get("selected_worktree_path").is_none());

    let restored = service
        .restore_task(temp.path(), &task_id, Some("restore"), TimestampMs(40))?
        .expect("archived task should restore");
    assert_eq!(restored.task.status, AgentTaskStatus::Running);
    assert_eq!(restored.task.metadata["task_board.status_key"], "running");
    assert!(restored.task.metadata.get("task_board.archived").is_none());

    let deleted = service
        .soft_delete_task(temp.path(), &task_id, None, TimestampMs(50))?
        .expect("task should soft delete");
    assert_eq!(deleted.task.metadata["task_board.deleted"], true);

    let restored = service
        .restore_task(temp.path(), &task_id, None, TimestampMs(60))?
        .expect("deleted task should restore");
    assert!(restored.task.metadata.get("task_board.deleted").is_none());

    let logs = store.list_task_logs(temp.path(), &task_id)?;
    assert!(logs.iter().any(|log| log.message == "move to execution"));
    assert!(
        logs.iter()
            .any(|log| log.message == "accepted agent scheduler task board command soft_delete")
    );
    assert!(
        service
            .archive_task(temp.path(), &TaskId::new("missing")?, None, TimestampMs(70))?
            .is_none()
    );
    Ok(())
}

#[test]
fn task_board_service_records_staff_assignment_and_stage_action_metadata() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(temp.path())?;
    let service = AgentTaskBoardService::new(&store);
    let task_id = TaskId::new("task-staff-flow")?;
    store.create_task(
        temp.path(),
        new_board_task(task_id.as_str(), "pending", AgentTaskStatus::Queued)?,
    )?;

    let assigned = service
        .assign_staff(
            temp.path(),
            &task_id,
            AgentSchedulerStaffCommandPayloadDto {
                command_id: "cmd-1".to_string(),
                member_id: "member-1".to_string(),
                stage_id: Some("stage-1".to_string()),
                role: Some("reviewer".to_string()),
                action: Some("inspect".to_string()),
                metadata: Some(json!({"ticket": 7})),
            },
            None,
            TimestampMs(100),
        )?
        .expect("staff assignment should update task");
    assert_eq!(assigned.task.metadata["task_board.staff.command_id"], "cmd-1");
    assert_eq!(assigned.task.metadata["task_board.staff.member_id"], "member-1");
    assert_eq!(assigned.event.payload["staff"]["stage_id"], "stage-1");

    let mut parameters = BTreeMap::new();
    parameters.insert("unsafe_secret".to_string(), json!("do-not-store"));
    let staged = service
        .run_stage_action(
            temp.path(),
            &task_id,
            AgentSchedulerStageActionCommandPayloadDto {
                stage_id: "stage-2".to_string(),
                capability_id: "capability-1".to_string(),
                workflow_uuid: Some("workflow-1".to_string()),
                inputs: AgentSchedulerStageActionInputsDto {
                    workflow_uuid: Some("input-workflow".to_string()),
                    parameters,
                },
                outcome: Some(AgentSchedulerStageActionOutcomeDto::Success),
                member_id: Some("member-2".to_string()),
                metadata: Some(json!({"note": "ok"})),
            },
            Some(LifecycleStatusProjection {
                status_key: "code_complete".to_string(),
                status_role: TaskPoolWorkflowStatusRoleDto::ReviewReady,
                runtime_status: AgentTaskStatus::Submitted,
            }),
            Some("stage succeeded"),
            TimestampMs(110),
        )?
        .expect("stage action should update task");
    assert_eq!(staged.task.status, AgentTaskStatus::Submitted);
    assert_eq!(staged.task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(staged.task.metadata["task_board.status_role"], "review_ready");
    assert_eq!(staged.task.metadata["task_board.stage_action.result"], "success");
    assert_eq!(staged.task.metadata["task_board.stage_action.inputs"]["parameter_count"], 1);
    assert_eq!(
        staged.task.metadata["task_board.stage_action.inputs"]["parameter_keys"][0],
        "unsafe_secret"
    );
    assert!(staged.task.metadata["task_board.stage_action.inputs"].get("unsafe_secret").is_none());
    assert_eq!(staged.event.payload["stage_action"]["target_status_key"], "code_complete");

    let missing_target = match service.run_stage_action(
        temp.path(),
        &task_id,
        AgentSchedulerStageActionCommandPayloadDto {
            stage_id: "stage-3".to_string(),
            capability_id: "capability-2".to_string(),
            workflow_uuid: None,
            inputs: AgentSchedulerStageActionInputsDto::default(),
            outcome: Some(AgentSchedulerStageActionOutcomeDto::Failure),
            member_id: None,
            metadata: None,
        },
        None,
        None,
        TimestampMs(120),
    ) {
        Ok(_) => panic!("outcome without target should fail"),
        Err(error) => error,
    };
    assert!(missing_target.to_string().contains("requires resolved workflow status"));
    Ok(())
}

#[test]
fn task_board_service_imports_legacy_tasks_and_tracks_skipped_records() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(temp.path())?;
    let service = AgentTaskBoardService::new(&store);
    store.create_task(
        temp.path(),
        new_board_task("existing-task", "pending", AgentTaskStatus::Queued)?,
    )?;

    let mut existing = legacy_import(json!({}));
    existing.task_id = "existing-task".into();
    let mut created = legacy_import(json!({"source": "legacy"}));
    created.task_id = "created-task".into();
    created.title = "Created legacy".to_string();
    created.status_key = Some("code_complete".to_string());
    created.prompt = "ship it".to_string();

    let dry_run = service.import_legacy_tasks(
        temp.path(),
        None,
        vec![existing.clone(), created.clone()],
        true,
        TimestampMs(200),
    )?;
    assert_eq!(dry_run.imported, 0);
    assert_eq!(dry_run.skipped, 2);
    assert!(dry_run.events.is_empty());
    assert!(store.load_task(temp.path(), &TaskId::new("created-task")?)?.is_none());

    let imported = service.import_legacy_tasks(
        temp.path(),
        None,
        vec![existing, created],
        false,
        TimestampMs(210),
    )?;
    assert_eq!(imported.imported, 1);
    assert_eq!(imported.skipped, 1);
    assert_eq!(imported.events.len(), 1);
    let created = store
        .load_task(temp.path(), &TaskId::new("created-task")?)?
        .expect("legacy task should be imported");
    assert_eq!(created.title, "Created legacy");
    assert_eq!(created.status, AgentTaskStatus::Submitted);
    assert_eq!(created.metadata["source"], "legacy");
    assert_eq!(created.metadata["prompt"], "ship it");
    Ok(())
}
