//! 在原生任务模型与调度存储之间建立看板读写桥梁，并执行用户命令与状态投影。

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use vw_api_types::ReasoningEffortDto;
use vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerLegacyTaskImportDto,
    AgentSchedulerMoveTaskPayloadDto, AgentSchedulerTaskMetadataPatchDto, AgentTaskStatusDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::TaskPoolWorkflowSnapshotDto;

use crate::agent_scheduler::domain::TaskId;
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::lifecycle_bridge::status_role_key;
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::events::{AppendAgentEventRequest, StoredAgentEvent};
use crate::agent_scheduler::store::logs::AppendAgentTaskLogRequest;
use crate::agent_scheduler::store::tasks::{AgentTaskBoardUpdate, NewAgentTask, StoredAgentTask};
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};

const DEFAULT_IMPORT_MODEL: &str = "auto";

/// 将原生任务映射为调度任务并持久化，必要时生成不冲突的新标识。
pub fn create_native_task(
    store: &AgentSchedulerStore,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    mut task: vw_shared::task::Task,
) -> Result<vw_shared::task::Task> {
    task.id = next_native_task_id(store, context, &task.id)?;
    let task_id = TaskId::new(task.id.clone())?;
    let metadata = native_task_metadata(&task)?;
    let created = store.create_task(
        context,
        NewAgentTask {
            id: task_id.clone(),
            title: native_task_title(&task),
            kind: AgentTaskKind::Finite,
            status: agent_status_from_status_key(task.status.to_string_key())
                .unwrap_or(AgentTaskStatus::Queued),
            spec_uri: format!("shared/tasks/{}/spec.md", task.id),
            meta_uri: format!("shared/tasks/{}/meta.json", task.id),
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
            created_at_ms: TimestampMs(i64::try_from(task.created_at_ms)?),
        },
    )?;
    store.update_task_board_task(
        context,
        &task_id,
        AgentTaskBoardUpdate {
            title: None,
            status: None,
            metadata: created.metadata,
            updated_at_ms: TimestampMs(i64::try_from(task.updated_at_ms)?),
        },
    )?;
    for log in &task.logs {
        store.append_task_log(
            context,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: None,
                timestamp_ms: TimestampMs(i64::try_from(log.timestamp_ms)?),
                from_status: log
                    .status_from
                    .and_then(|status| agent_status_from_status_key(status.to_string_key())),
                to_status: log
                    .status_to
                    .and_then(|status| agent_status_from_status_key(status.to_string_key())),
                from_status_key: log.status_from.map(|status| status.to_string_key().to_string()),
                to_status_key: log.status_to.map(|status| status.to_string_key().to_string()),
                message: log.message.clone(),
            },
        )?;
    }
    load_native_task(store, context, &task_id)?.context("created native task disappeared")
}

/// 从调度存储恢复原生任务；不存在时返回 `Ok(None)`。
pub fn load_native_task(
    store: &AgentSchedulerStore,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
) -> Result<Option<vw_shared::task::Task>> {
    store
        .load_task(context, task_id)?
        .map(|stored| stored_native_task(store, context, stored))
        .transpose()
}

/// 列出当前作用域内可投影为原生任务的记录。
pub fn list_native_tasks(
    store: &AgentSchedulerStore,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<Vec<vw_shared::task::Task>> {
    store
        .list_tasks(context, super::store::tasks::AgentTaskFilter::default())?
        .into_iter()
        .map(|stored| stored_native_task(store, context, stored))
        .collect()
}

fn next_native_task_id(
    store: &AgentSchedulerStore,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    suggested: &str,
) -> Result<String> {
    let Some((prefix, sequence)) = suggested.rsplit_once('.') else {
        bail!("native task id must contain a sequence: {suggested}");
    };
    let mut sequence = sequence
        .parse::<u32>()
        .with_context(|| format!("native task id has an invalid sequence: {suggested}"))?;
    for _ in 0..=u16::MAX {
        let candidate = format!("{prefix}.{sequence:04}");
        let candidate_id = TaskId::new(candidate.clone())?;
        if store.load_task(context, &candidate_id)?.is_none() {
            return Ok(candidate);
        }
        sequence = sequence.checked_add(1).context("native task id sequence exhausted")?;
    }
    bail!("native task id sequence exhausted for prefix {prefix}")
}

fn native_task_title(task: &vw_shared::task::Task) -> String {
    [&task.description, &task.prompt, &task.id]
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| task.id.clone())
}

fn native_task_metadata(task: &vw_shared::task::Task) -> Result<Value> {
    Ok(json!({
        "task_board.status_key": task.status.to_string_key(),
        "priority": task.priority,
        "order_no": task.order,
        "assignee": task.assignee,
        "model": task.model,
        "executor_id": task.executor.id(),
        "description": task.description,
        "prompt": task.prompt,
        "deleted": task.deleted,
        "archived": task.archived,
        "subtasks": serde_json::to_value(&task.subtasks)?,
        "task_split_mode": task.task_split_mode.to_string_key(),
        "auto_promote_delay_ms": task.auto_promote_delay_ms,
        "last_error": task.last_error,
        "pause_reason": task.pause_reason,
        "retry_count": task.retry_count,
        "last_active_at_ms": task.last_active_at_ms,
        "execution_started_at_ms": task.execution_started_at_ms,
        "last_execution_duration_ms": task.last_execution_duration_ms,
        "merge_source_branch": task.merge_source_branch,
        "merge_target_branch": task.merge_target_branch,
        "selected_worktree_path": task.selected_worktree_path,
    }))
}

fn stored_native_task(
    store: &AgentSchedulerStore,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    stored: StoredAgentTask,
) -> Result<vw_shared::task::Task> {
    let fields = stored.metadata.pointer("/legacy_task_pool/fields").unwrap_or(&stored.metadata);
    let status_key = stored
        .metadata
        .get("task_board.status_key")
        .and_then(Value::as_str)
        .unwrap_or_else(|| legacy_task_board_status_key(stored.status));
    let logs = store
        .list_task_logs(context, &stored.id)?
        .into_iter()
        .map(|log| vw_shared::task::TaskLogEntry {
            timestamp_ms: u64::try_from(log.timestamp_ms.0).unwrap_or_default(),
            status_from: log
                .from_status_key
                .as_deref()
                .and_then(vw_shared::task::TaskStatus::parse_key),
            status_to: log
                .to_status_key
                .as_deref()
                .and_then(vw_shared::task::TaskStatus::parse_key),
            message: log.message,
        })
        .collect();
    Ok(vw_shared::task::Task {
        id: stored.id.as_str().to_string(),
        priority: metadata_u32(fields, "priority"),
        assignee: metadata_string_or(fields, "assignee", "agent"),
        model: metadata_string_or(fields, "model", DEFAULT_IMPORT_MODEL),
        executor: metadata_string(fields, "executor_id")
            .or_else(|| metadata_string(fields, "acp_agent"))
            .and_then(vw_shared::task::TaskExecutorBackend::from_id)
            .unwrap_or(vw_shared::task::TaskExecutorBackend::Internal),
        description: metadata_string(fields, "description")
            .unwrap_or(stored.title.as_str())
            .to_string(),
        prompt: metadata_string_or(fields, "prompt", ""),
        status: vw_shared::task::TaskStatus::parse_key(status_key)
            .unwrap_or(vw_shared::task::TaskStatus::Pool),
        created_at_ms: u64::try_from(stored.created_at_ms.0).unwrap_or_default(),
        updated_at_ms: u64::try_from(stored.updated_at_ms.0).unwrap_or_default(),
        logs,
        order: metadata_u32(fields, "order_no"),
        deleted: metadata_bool(fields, "deleted"),
        archived: metadata_bool(fields, "archived"),
        subtasks: fields
            .get("subtasks")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .context("parse native task subtasks from scheduler metadata")?
            .unwrap_or_default(),
        task_split_mode: metadata_string(fields, "task_split_mode")
            .and_then(vw_shared::task::TaskSplitMode::parse_key)
            .unwrap_or_default(),
        auto_promote_delay_ms: metadata_u64_optional(fields, "auto_promote_delay_ms"),
        last_error: metadata_owned_string(fields, "last_error"),
        pause_reason: metadata_owned_string(fields, "pause_reason"),
        retry_count: metadata_u32(fields, "retry_count"),
        last_active_at_ms: metadata_u64(fields, "last_active_at_ms"),
        execution_started_at_ms: metadata_u64_optional(fields, "execution_started_at_ms"),
        last_execution_duration_ms: metadata_u64_optional(fields, "last_execution_duration_ms"),
        merge_source_branch: metadata_owned_string(fields, "merge_source_branch"),
        merge_target_branch: metadata_owned_string(fields, "merge_target_branch"),
        selected_worktree_path: metadata_owned_string(fields, "selected_worktree_path"),
    })
}

fn metadata_string<'a>(metadata: &'a Value, key: &str) -> Option<&'a str> {
    metadata.get(key).and_then(Value::as_str)
}

fn metadata_owned_string(metadata: &Value, key: &str) -> Option<String> {
    metadata_string(metadata, key).map(str::to_string)
}

fn metadata_string_or(metadata: &Value, key: &str, fallback: &str) -> String {
    metadata_string(metadata, key).unwrap_or(fallback).to_string()
}

fn metadata_u64(metadata: &Value, key: &str) -> u64 {
    metadata.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn metadata_u64_optional(metadata: &Value, key: &str) -> Option<u64> {
    metadata.get(key).and_then(Value::as_u64)
}

fn metadata_u32(metadata: &Value, key: &str) -> u32 {
    metadata_u64(metadata, key).try_into().unwrap_or_default()
}

fn metadata_bool(metadata: &Value, key: &str) -> bool {
    metadata.get(key).and_then(Value::as_bool).unwrap_or_default()
}

/// 执行看板移动、归档、人员分派和阶段动作的应用服务。
pub struct AgentTaskBoardService<'a> {
    store: &'a AgentSchedulerStore,
}

/// 看板写入后的任务快照及同步追加的审计事件。
pub struct AgentTaskBoardWriteOutcome {
    pub task: StoredAgentTask,
    pub event: StoredAgentEvent,
}

/// 一批旧任务导入后的成功记录与拒绝原因。
pub struct AgentTaskBoardLegacyImportOutcome {
    pub imported: u32,
    pub skipped: u32,
    pub tasks: Vec<StoredAgentTask>,
    pub events: Vec<StoredAgentEvent>,
}

impl<'a> AgentTaskBoardService<'a> {
    pub fn new(store: &'a AgentSchedulerStore) -> Self {
        Self { store }
    }

    /// 按工作流状态键移动任务，并在同一事务语义下记录状态事件。
    pub fn move_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        payload: AgentSchedulerMoveTaskPayloadDto,
        reason: Option<&str>,
        now_ms: TimestampMs,
    ) -> Result<Option<AgentTaskBoardWriteOutcome>> {
        let Some(current) = self.store.load_task(project_path, task_id)? else {
            return Ok(None);
        };
        let from_status_key = task_board_status_key(&current);
        if let Some(expected) = payload.from_status_key.as_deref().and_then(non_empty_str)
            && expected != from_status_key
        {
            bail!(
                "task {} is in status_key {}, not {}",
                task_id.as_str(),
                from_status_key,
                expected
            );
        }

        let to_status_key = required_text(payload.to_status_key, "to_status_key")?;
        let mut metadata = object_metadata(current.metadata.clone())?;
        set_metadata_string(&mut metadata, "task_board.status_key", Some(to_status_key.clone()))?;
        if let Some(order_no) = payload.order_no {
            metadata.insert("order_no".to_string(), json!(order_no));
        }
        let status = payload.to_status.map(task_status_from_dto);
        let updated = self.update_task(
            project_path,
            &current,
            None,
            status,
            Value::Object(metadata),
            now_ms,
        )?;
        Ok(Some(self.audit_task_board_write(
            project_path,
            &current,
            updated,
            AgentSchedulerCommandKindDto::Move,
            AgentEventKind::HumanIntervened,
            Some(from_status_key),
            Some(to_status_key),
            reason,
            now_ms,
        )?))
    }

    /// 应用受限元数据补丁，不允许借此绕过专用状态转换入口。
    pub fn update_metadata(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        patch: AgentSchedulerTaskMetadataPatchDto,
        reason: Option<&str>,
        now_ms: TimestampMs,
    ) -> Result<Option<AgentTaskBoardWriteOutcome>> {
        let Some(current) = self.store.load_task(project_path, task_id)? else {
            return Ok(None);
        };
        let from_status_key = task_board_status_key(&current);
        let title = optional_required_text(patch.title.clone(), "task title")?;
        let status = patch.status.clone().map(task_status_from_dto);
        let mut metadata = object_metadata(current.metadata.clone())?;
        apply_metadata_patch(&mut metadata, patch)?;
        let updated = self.update_task(
            project_path,
            &current,
            title,
            status,
            Value::Object(metadata),
            now_ms,
        )?;
        let to_status_key = task_board_status_key(&updated);
        Ok(Some(self.audit_task_board_write(
            project_path,
            &current,
            updated,
            AgentSchedulerCommandKindDto::UpdateMetadata,
            AgentEventKind::HumanIntervened,
            Some(from_status_key),
            Some(to_status_key),
            reason,
            now_ms,
        )?))
    }

    pub fn archive_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        reason: Option<&str>,
        now_ms: TimestampMs,
    ) -> Result<Option<AgentTaskBoardWriteOutcome>> {
        let Some(current) = self.store.load_task(project_path, task_id)? else {
            return Ok(None);
        };
        let from_status_key = task_board_status_key(&current);
        let mut metadata = object_metadata(current.metadata.clone())?;
        metadata
            .insert("task_board.previous_status_key".to_string(), json!(from_status_key.clone()));
        metadata
            .insert("task_board.previous_agent_status".to_string(), json!(current.status.as_str()));
        metadata.insert("task_board.status_key".to_string(), json!("archived"));
        metadata.insert("task_board.archived".to_string(), json!(true));
        metadata.insert("task_board.archived_at_ms".to_string(), json!(now_ms.0));
        clear_selected_worktree_metadata(&mut metadata);
        let updated = self.update_task(
            project_path,
            &current,
            None,
            Some(AgentTaskStatus::Archived),
            Value::Object(metadata),
            now_ms,
        )?;
        Ok(Some(self.audit_task_board_write(
            project_path,
            &current,
            updated,
            AgentSchedulerCommandKindDto::Archive,
            AgentEventKind::TaskArchived,
            Some(from_status_key),
            Some("archived".to_string()),
            reason,
            now_ms,
        )?))
    }

    /// 将任务移动到归档状态以实现可恢复的软删除。
    pub fn soft_delete_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        reason: Option<&str>,
        now_ms: TimestampMs,
    ) -> Result<Option<AgentTaskBoardWriteOutcome>> {
        let Some(current) = self.store.load_task(project_path, task_id)? else {
            return Ok(None);
        };
        let from_status_key = task_board_status_key(&current);
        let mut metadata = object_metadata(current.metadata.clone())?;
        metadata
            .insert("task_board.previous_status_key".to_string(), json!(from_status_key.clone()));
        metadata.insert("task_board.deleted".to_string(), json!(true));
        metadata.insert("task_board.deleted_at_ms".to_string(), json!(now_ms.0));
        clear_selected_worktree_metadata(&mut metadata);
        let updated =
            self.update_task(project_path, &current, None, None, Value::Object(metadata), now_ms)?;
        Ok(Some(self.audit_task_board_write(
            project_path,
            &current,
            updated,
            AgentSchedulerCommandKindDto::SoftDelete,
            AgentEventKind::HumanIntervened,
            Some(from_status_key.clone()),
            Some(from_status_key),
            reason,
            now_ms,
        )?))
    }

    /// 将已归档任务恢复到工作流允许的目标状态。
    pub fn restore_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        reason: Option<&str>,
        now_ms: TimestampMs,
    ) -> Result<Option<AgentTaskBoardWriteOutcome>> {
        let Some(current) = self.store.load_task(project_path, task_id)? else {
            return Ok(None);
        };
        let from_status_key = task_board_status_key(&current);
        let mut metadata = object_metadata(current.metadata.clone())?;
        metadata.remove("task_board.deleted");
        metadata.remove("task_board.deleted_at_ms");
        metadata.remove("task_board.archived");
        metadata.remove("task_board.archived_at_ms");
        let restored_status_key = metadata
            .get("task_board.previous_status_key")
            .and_then(Value::as_str)
            .and_then(non_empty_str)
            .map(str::to_string);
        if let Some(status_key) = restored_status_key.as_deref() {
            metadata.insert("task_board.status_key".to_string(), json!(status_key));
        }
        let restored_status = metadata
            .get("task_board.previous_agent_status")
            .and_then(Value::as_str)
            .and_then(|status| AgentTaskStatus::parse(status).ok());
        metadata.remove("task_board.previous_status_key");
        metadata.remove("task_board.previous_agent_status");
        let status = (current.status == AgentTaskStatus::Archived)
            .then_some(restored_status.unwrap_or(AgentTaskStatus::Queued));
        let updated = self.update_task(
            project_path,
            &current,
            None,
            status,
            Value::Object(metadata),
            now_ms,
        )?;
        let to_status_key = task_board_status_key(&updated);
        Ok(Some(self.audit_task_board_write(
            project_path,
            &current,
            updated,
            AgentSchedulerCommandKindDto::Restore,
            AgentEventKind::TaskResumed,
            Some(from_status_key),
            Some(to_status_key),
            reason,
            now_ms,
        )?))
    }

    /// 批量导入旧任务；单项失败会记录原因而不丢弃其它成功项。
    pub fn import_legacy_tasks(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        workflow: Option<TaskPoolWorkflowSnapshotDto>,
        tasks: Vec<AgentSchedulerLegacyTaskImportDto>,
        dry_run: bool,
        now_ms: TimestampMs,
    ) -> Result<AgentTaskBoardLegacyImportOutcome> {
        if let Some(workflow) = workflow.as_ref() {
            workflow.validate_for_schedule().map_err(anyhow::Error::msg)?;
            if !dry_run {
                self.store.save_task_board_workflow(project_path, workflow)?;
            }
        }

        let mut imported = 0;
        let mut skipped = 0;
        let mut imported_tasks = Vec::new();
        let mut events = Vec::new();
        for legacy in tasks {
            let task_id = TaskId::new(legacy.task_id.as_ref().to_string())?;
            if let Some(existing) = self.store.load_task(project_path, &task_id)? {
                skipped += 1;
                imported_tasks.push(existing);
                continue;
            }
            if dry_run {
                skipped += 1;
                continue;
            }

            let task = new_task_from_legacy_import(legacy, workflow.as_ref())?;
            let created = self.store.create_task(project_path, task)?;
            let updated_at_ms =
                timestamp_field(&created.metadata, "task_board.legacy_updated_at_ms")
                    .unwrap_or(created.updated_at_ms);
            let stored = if updated_at_ms != created.updated_at_ms {
                self.store
                    .update_task_board_task(
                        project_path,
                        &created.id,
                        AgentTaskBoardUpdate {
                            title: None,
                            status: None,
                            metadata: created.metadata.clone(),
                            updated_at_ms,
                        },
                    )?
                    .with_context(|| {
                        format!(
                            "legacy imported task disappeared after timestamp update: {}",
                            created.id.as_str()
                        )
                    })?
            } else {
                created
            };
            let event = self.append_audit_event(
                project_path,
                &stored,
                AgentEventKind::ImportedLegacyTask,
                None,
                Some(task_board_status_key(&stored)),
                "imported legacy desktop task",
                AgentSchedulerCommandKindDto::ImportLegacyTask,
                now_ms,
            )?;
            self.append_audit_log(
                project_path,
                &stored,
                None,
                Some(task_board_status_key(&stored)),
                "imported legacy desktop task",
                now_ms,
            )?;
            imported += 1;
            imported_tasks.push(stored);
            events.push(event);
        }

        Ok(AgentTaskBoardLegacyImportOutcome { imported, skipped, tasks: imported_tasks, events })
    }

    fn update_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        current: &StoredAgentTask,
        title: Option<String>,
        status: Option<AgentTaskStatus>,
        metadata: Value,
        now_ms: TimestampMs,
    ) -> Result<StoredAgentTask> {
        self.store
            .update_task_board_task(
                project_path,
                &current.id,
                AgentTaskBoardUpdate { title, status, metadata, updated_at_ms: now_ms },
            )?
            .with_context(|| {
                format!("task disappeared during task board update: {}", current.id.as_str())
            })
    }

    fn audit_task_board_write(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        current: &StoredAgentTask,
        updated: StoredAgentTask,
        command: AgentSchedulerCommandKindDto,
        event_kind: AgentEventKind,
        from_status_key: Option<String>,
        to_status_key: Option<String>,
        reason: Option<&str>,
        now_ms: TimestampMs,
    ) -> Result<AgentTaskBoardWriteOutcome> {
        let message = reason.and_then(non_empty_str).map(str::to_string).unwrap_or_else(|| {
            format!("accepted agent scheduler task board command {}", command_key(command))
        });
        let event = self.append_audit_event(
            project_path,
            &updated,
            event_kind,
            from_status_key.clone(),
            to_status_key.clone(),
            &message,
            command,
            now_ms,
        )?;
        self.append_audit_log(
            project_path,
            &updated,
            from_status_key.or_else(|| Some(task_board_status_key(current))),
            to_status_key.or_else(|| Some(task_board_status_key(&updated))),
            message,
            now_ms,
        )?;
        Ok(AgentTaskBoardWriteOutcome { task: updated, event })
    }

    fn append_audit_event(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        kind: AgentEventKind,
        from_status_key: Option<String>,
        to_status_key: Option<String>,
        message: impl Into<String>,
        command: AgentSchedulerCommandKindDto,
        now_ms: TimestampMs,
    ) -> Result<StoredAgentEvent> {
        let message = message.into();
        self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: None,
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind,
                from_status: None,
                to_status: Some(task.status),
                from_status_key,
                to_status_key,
                message,
                payload: json!({
                    "command": command_key(command),
                    "task_id": task.id.as_str(),
                    "status_key": task_board_status_key(task),
                }),
                created_at_ms: now_ms,
            },
        )
    }

    fn append_audit_event_with_payload(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        kind: AgentEventKind,
        from_status_key: Option<String>,
        to_status_key: Option<String>,
        message: impl Into<String>,
        command: AgentSchedulerCommandKindDto,
        extra_payload: Value,
        now_ms: TimestampMs,
    ) -> Result<StoredAgentEvent> {
        let message = message.into();
        let mut payload = json!({
            "command": command_key(command),
            "task_id": task.id.as_str(),
            "status_key": task_board_status_key(task),
        });
        if let (Value::Object(payload), Value::Object(extra)) = (&mut payload, extra_payload) {
            for (key, value) in extra {
                payload.insert(key, value);
            }
        }
        self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: None,
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind,
                from_status: None,
                to_status: Some(task.status),
                from_status_key,
                to_status_key,
                message,
                payload,
                created_at_ms: now_ms,
            },
        )
    }

    fn append_audit_log(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        from_status_key: Option<String>,
        to_status_key: Option<String>,
        message: impl Into<String>,
        now_ms: TimestampMs,
    ) -> Result<()> {
        self.store.append_task_log(
            project_path,
            AppendAgentTaskLogRequest {
                task_id: task.id.clone(),
                run_id: None,
                timestamp_ms: now_ms,
                from_status: None,
                to_status: Some(task.status),
                from_status_key,
                to_status_key,
                message: message.into(),
            },
        )?;
        Ok(())
    }
}

pub(super) fn apply_metadata_patch(
    metadata: &mut Map<String, Value>,
    patch: AgentSchedulerTaskMetadataPatchDto,
) -> Result<()> {
    if let Some(status_key) = patch.status_key {
        set_metadata_string(metadata, "task_board.status_key", Some(status_key))?;
    }
    set_metadata_u32(metadata, "priority", patch.priority);
    set_metadata_i64(metadata, "order_no", patch.order_no);
    set_metadata_string(metadata, "model", patch.model)?;
    set_metadata_string(metadata, "agent", patch.agent)?;
    set_metadata_string(metadata, "acp_agent", patch.acp_agent)?;
    set_reasoning_effort(metadata, patch.reasoning_effort)?;
    set_metadata_string(metadata, "prompt", patch.prompt)?;
    set_metadata_string(metadata, "last_error", patch.last_error)?;
    set_metadata_string(metadata, "pause_reason", patch.pause_reason)?;
    set_metadata_string(metadata, "merge_source_branch", patch.merge_source_branch)?;
    set_metadata_string(metadata, "merge_target_branch", patch.merge_target_branch)?;
    set_metadata_string(metadata, "selected_worktree_path", patch.selected_worktree_path)?;
    set_metadata_string(metadata, "selected_worktree_slot_id", patch.selected_worktree_slot_id)?;
    set_metadata_string(metadata, "selected_worktree_branch", patch.selected_worktree_branch)?;
    set_metadata_string(metadata, "task_board.workflow_id", patch.workflow_id)?;
    if let Some(role) = patch.status_role {
        metadata.insert("task_board.status_role".to_string(), json!(status_role_key(role)));
    }
    if let Some(merge) = patch.metadata_merge {
        let Value::Object(merge) = merge else {
            bail!("metadata_merge must be a JSON object");
        };
        for (key, value) in merge {
            metadata.insert(key, value);
        }
    }
    for key in patch.metadata_remove_keys {
        metadata.remove(&key);
    }
    Ok(())
}

pub(super) fn new_task_from_legacy_import(
    legacy: AgentSchedulerLegacyTaskImportDto,
    workflow: Option<&TaskPoolWorkflowSnapshotDto>,
) -> Result<NewAgentTask> {
    let task_id = TaskId::new(legacy.task_id.as_ref().to_string())?;
    let mut metadata = object_metadata(legacy.metadata)?;
    metadata.insert("priority".to_string(), json!(legacy.priority));
    metadata.insert("order_no".to_string(), json!(legacy.order_no));
    metadata.insert(
        "model".to_string(),
        json!(non_empty_str(&legacy.model).unwrap_or(DEFAULT_IMPORT_MODEL)),
    );
    if let Some(agent) = legacy.agent {
        set_metadata_string(&mut metadata, "agent", Some(agent))?;
    }
    if let Some(acp_agent) = legacy.acp_agent {
        set_metadata_string(&mut metadata, "acp_agent", Some(acp_agent))?;
    }
    set_reasoning_effort(&mut metadata, legacy.reasoning_effort)?;
    if let Some(workflow_id) =
        legacy.workflow_id.or_else(|| workflow.map(|workflow| workflow.id.clone()))
    {
        set_metadata_string(&mut metadata, "task_board.workflow_id", Some(workflow_id))?;
    }
    if let Some(status_key) = legacy.status_key {
        set_metadata_string(&mut metadata, "task_board.status_key", Some(status_key))?;
    }
    if !legacy.prompt.trim().is_empty() {
        metadata.insert("prompt".to_string(), json!(legacy.prompt));
    }
    if let Some(updated_at_ms) = legacy.updated_at_ms {
        metadata.insert("task_board.legacy_updated_at_ms".to_string(), json!(updated_at_ms.0));
    }
    let status = legacy
        .status
        .map(task_status_from_dto)
        .or_else(|| {
            metadata
                .get("task_board.status_key")
                .and_then(Value::as_str)
                .and_then(agent_status_from_status_key)
        })
        .unwrap_or(AgentTaskStatus::Queued);
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: required_text(legacy.title, "task title")?,
        kind: AgentTaskKind::Finite,
        status,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
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
        metadata: Value::Object(metadata),
        created_at_ms: legacy.created_at_ms,
    })
}

pub fn task_board_status_key(task: &StoredAgentTask) -> String {
    if agent_scheduler_control_status(task.status) {
        return task_board_status_key_from_map(AgentTaskStatus::Queued, &task.metadata)
            .unwrap_or_else(|| legacy_task_board_status_key(AgentTaskStatus::Queued).to_string());
    }
    task.metadata
        .get("task_board.status_key")
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .map(str::to_string)
        .or_else(|| task_board_status_key_from_map(task.status, &task.metadata))
        .unwrap_or_else(|| legacy_task_board_status_key(task.status).to_string())
}

fn task_board_status_key_from_map(status: AgentTaskStatus, metadata: &Value) -> Option<String> {
    let status_key_map = metadata.get("task_board.status_key_map").and_then(Value::as_object)?;
    status_key_map
        .get(status.as_str())
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .map(str::to_string)
}

fn legacy_task_board_status_key(status: AgentTaskStatus) -> &'static str {
    match status {
        AgentTaskStatus::Submitted => "code_complete",
        _ => status.as_str(),
    }
}

fn agent_scheduler_control_status(status: AgentTaskStatus) -> bool {
    matches!(
        status,
        AgentTaskStatus::Assigned | AgentTaskStatus::Dispatched | AgentTaskStatus::Acknowledged
    )
}

fn agent_status_from_status_key(status_key: &str) -> Option<AgentTaskStatus> {
    match status_key {
        "pool" | "draft" => Some(AgentTaskStatus::Draft),
        "pending" | "queued" => Some(AgentTaskStatus::Queued),
        "planning" | "assigned" => Some(AgentTaskStatus::Assigned),
        "running" | "active" => Some(AgentTaskStatus::Running),
        "failed" => Some(AgentTaskStatus::Failed),
        "paused" => Some(AgentTaskStatus::Paused),
        "code_complete" | "submitted" | "review_ready" => Some(AgentTaskStatus::Submitted),
        "completed" | "done" => Some(AgentTaskStatus::Completed),
        "archived" => Some(AgentTaskStatus::Archived),
        _ => None,
    }
}

fn task_status_from_dto(status: AgentTaskStatusDto) -> AgentTaskStatus {
    match status {
        AgentTaskStatusDto::Draft => AgentTaskStatus::Draft,
        AgentTaskStatusDto::Queued => AgentTaskStatus::Queued,
        AgentTaskStatusDto::Assigned => AgentTaskStatus::Assigned,
        AgentTaskStatusDto::Dispatched => AgentTaskStatus::Dispatched,
        AgentTaskStatusDto::Acknowledged => AgentTaskStatus::Acknowledged,
        AgentTaskStatusDto::Active => AgentTaskStatus::Active,
        AgentTaskStatusDto::Running => AgentTaskStatus::Running,
        AgentTaskStatusDto::Submitted => AgentTaskStatus::Submitted,
        AgentTaskStatusDto::Revision => AgentTaskStatus::Revision,
        AgentTaskStatusDto::Blocked => AgentTaskStatus::Blocked,
        AgentTaskStatusDto::Paused => AgentTaskStatus::Paused,
        AgentTaskStatusDto::Completed => AgentTaskStatus::Completed,
        AgentTaskStatusDto::Failed => AgentTaskStatus::Failed,
        AgentTaskStatusDto::Cancelled => AgentTaskStatus::Cancelled,
        AgentTaskStatusDto::Archived => AgentTaskStatus::Archived,
    }
}

fn command_key(command: AgentSchedulerCommandKindDto) -> &'static str {
    match command {
        AgentSchedulerCommandKindDto::Start => "start",
        AgentSchedulerCommandKindDto::Retry => "retry",
        AgentSchedulerCommandKindDto::Pause => "pause",
        AgentSchedulerCommandKindDto::Resume => "resume",
        AgentSchedulerCommandKindDto::Cancel => "cancel",
        AgentSchedulerCommandKindDto::Archive => "archive",
        AgentSchedulerCommandKindDto::Promote => "promote",
        AgentSchedulerCommandKindDto::Merge => "merge",
        AgentSchedulerCommandKindDto::Move => "move",
        AgentSchedulerCommandKindDto::UpdateMetadata => "update_metadata",
        AgentSchedulerCommandKindDto::SoftDelete => "soft_delete",
        AgentSchedulerCommandKindDto::Restore => "restore",
        AgentSchedulerCommandKindDto::ImportLegacyTask => "import_legacy_task",
    }
}

fn object_metadata(value: Value) -> Result<Map<String, Value>> {
    match value {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Map::new()),
        _ => bail!("task metadata must be a JSON object"),
    }
}

fn set_metadata_string(
    metadata: &mut Map<String, Value>,
    key: &str,
    value: Option<String>,
) -> Result<()> {
    if let Some(value) = optional_required_text(value, key)? {
        metadata.insert(key.to_string(), json!(value));
    }
    Ok(())
}

fn set_reasoning_effort(
    metadata: &mut Map<String, Value>,
    reasoning_effort: Option<ReasoningEffortDto>,
) -> Result<()> {
    if let Some(reasoning_effort) = reasoning_effort {
        metadata.insert(
            "reasoning_effort".to_string(),
            serde_json::to_value(reasoning_effort).context("serialize reasoning_effort")?,
        );
    }
    Ok(())
}

pub(super) fn reasoning_effort_from_metadata(
    metadata: &Value,
) -> Result<Option<ReasoningEffortDto>> {
    metadata
        .get("reasoning_effort")
        .cloned()
        .filter(|value| !value.is_null())
        .map(|value| {
            serde_json::from_value(value).context("invalid task metadata field reasoning_effort")
        })
        .transpose()
}

fn set_metadata_u32(metadata: &mut Map<String, Value>, key: &str, value: Option<u32>) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), json!(value));
    }
}

fn set_metadata_i64(metadata: &mut Map<String, Value>, key: &str, value: Option<i64>) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), json!(value));
    }
}

fn clear_selected_worktree_metadata(metadata: &mut Map<String, Value>) {
    metadata.remove("selected_worktree_path");
    metadata.remove("selected_worktree_slot_id");
    metadata.remove("selected_worktree_branch");
}

fn timestamp_field(value: &Value, key: &str) -> Option<TimestampMs> {
    value.get(key).and_then(Value::as_i64).map(TimestampMs)
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).and_then(non_empty_str).map(str::to_string)
}

fn required_text(value: String, field_name: &str) -> Result<String> {
    optional_required_text(Some(value), field_name)?
        .with_context(|| format!("{field_name} must not be empty"))
}

fn optional_required_text(value: Option<String>, field_name: &str) -> Result<Option<String>> {
    value
        .map(|value| {
            non_empty_str(&value)
                .map(str::to_string)
                .with_context(|| format!("{field_name} must not be empty"))
        })
        .transpose()
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}
