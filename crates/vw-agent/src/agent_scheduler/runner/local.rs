//! 编排本地代理运行的工作区租用、心跳、结果提交、日志记录与资源清理。

use std::path::{Path, PathBuf};
use std::process::Output;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use vw_api_types::ReasoningEffortDto;
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::agent_scheduler::engine::dispatch::ready_queued_finite_task;
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::artifacts::RecordTaskArtifactRequest;
use crate::agent_scheduler::store::events::AppendAgentEventRequest;
use crate::agent_scheduler::store::logs::{
    AgentSchedulerWorkspaceContext, AppendAgentTaskLogRequest,
};
use crate::agent_scheduler::store::runs::{
    AgentRunFinishStatus, ClaimReadyTaskRunRequest, StoredAgentRun,
};
use crate::agent_scheduler::store::tasks::{AgentTaskBoardUpdate, StoredAgentTask};
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};

const DEFAULT_LOCAL_RUN_LEASE_DURATION_MS: i64 = 30 * 60 * 1_000;
const FINAL_REPLY_INSTRUCTION: &str =
    "任务执行完毕后，请输出一句简短最终回复，明确说明已完成；不要在完成后继续等待用户输入。";
const WORKTREE_LOG_PATH_LIMIT: usize = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 启动本地运行所需的任务标识、执行时刻和租约时长。
pub struct LocalAgentRunnerRequest {
    pub task_id: TaskId,
    pub now_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 本地运行启动后返回的运行与工作区信息。
pub struct LocalAgentRunnerOutcome {
    pub task_id: TaskId,
    pub run: StoredAgentRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 执行器申请任务隔离工作区时接收的上下文。
pub struct LocalAgentWorkspaceRequest {
    pub project_path: PathBuf,
    pub task_id: TaskId,
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 已分配工作区的槽位、路径、分支和基线提交。
pub struct LocalAgentWorkspace {
    pub slot_id: String,
    pub path: String,
    pub branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 在已分配工作区内执行代理所需的提示、模型和路由信息。
pub struct LocalAgentExecutionRequest {
    pub project_path: PathBuf,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub title: String,
    pub prompt: String,
    pub model: Option<String>,
    pub agent: Option<String>,
    pub acp_agent: Option<String>,
    pub reasoning_effort: Option<ReasoningEffortDto>,
    pub workspace: LocalAgentWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 执行器返回的最终文本及可选结构化产物。
pub struct LocalAgentExecutionOutput {
    pub raw_output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAgentWorkspaceRecycleRequest {
    pub project_path: PathBuf,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub workspace: LocalAgentWorkspace,
    pub taint_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAgentWorkspaceReleaseRequest {
    pub project_path: PathBuf,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub workspace: LocalAgentWorkspace,
}

/// 本地运行器对工作区生命周期和代理执行后端的抽象。
pub trait LocalAgentExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        bail!(
            "local agent executor did not provide a task worktree lease for task {}",
            request.task_id.as_str()
        )
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput>;

    fn release_workspace(&self, _request: LocalAgentWorkspaceReleaseRequest) -> Result<()> {
        Ok(())
    }

    fn recycle_workspace(&self, _request: LocalAgentWorkspaceRecycleRequest) -> Result<()> {
        Ok(())
    }
}

/// 将运行认领、工作区租约、代理执行和持久化收尾串联起来的运行器。
pub struct LocalAgentRunner<'a, E>
where
    E: LocalAgentExecutor + ?Sized,
{
    store: &'a AgentSchedulerStore,
    executor: &'a E,
    lease_duration_ms: i64,
}

impl<'a, E> LocalAgentRunner<'a, E>
where
    E: LocalAgentExecutor + ?Sized,
{
    pub fn new(store: &'a AgentSchedulerStore, executor: &'a E) -> Self {
        Self { store, executor, lease_duration_ms: DEFAULT_LOCAL_RUN_LEASE_DURATION_MS }
    }

    pub fn run_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        request: LocalAgentRunnerRequest,
    ) -> Result<Option<LocalAgentRunnerOutcome>> {
        let Some(started) = self.start_once(project_path, request.clone())? else {
            return Ok(None);
        };
        let task_id = started.task_id.clone();
        let run_id = started.run.id.clone();
        self.finish_started_run(project_path, &task_id, &run_id, request)
    }

    /// 原子认领任务并准备工作区；任务不可运行时不产生执行副作用。
    pub fn start_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        request: LocalAgentRunnerRequest,
    ) -> Result<Option<LocalAgentRunnerOutcome>> {
        let Some(task) = self.store.load_task(project_path, &request.task_id)? else {
            return Ok(None);
        };
        if task.kind != AgentTaskKind::Finite {
            bail!("local runner only supports finite tasks: {}", task.id.as_str());
        }
        let Some(task) =
            ready_queued_finite_task(self.store, project_path, Some(&request.task_id))?
        else {
            return Ok(None);
        };

        let Some(run) = self.claim_ready_run(project_path, &task, &request)? else {
            return Ok(None);
        };
        self.append_log(
            project_path,
            &task.id,
            Some(&run.id),
            request.now_ms,
            Some(AgentTaskStatus::Queued),
            Some(AgentTaskStatus::Assigned),
            "local runner assigned task",
        )?;
        self.append_event(
            project_path,
            &task.id,
            Some(&run.id),
            AgentEventKind::WorkerSelected,
            Some(AgentTaskStatus::Queued),
            Some(AgentTaskStatus::Assigned),
            "local runner selected task",
            json!({"dispatch_target": "local"}),
            request.now_ms,
        )?;

        self.advance_to_running(project_path, &task.id, &run.id, &request)?;
        let started_run = self.store.load_run(project_path, &run.id)?.unwrap_or(run);
        Ok(Some(LocalAgentRunnerOutcome { task_id: task.id, run: started_run }))
    }

    /// 提交工作区变化、记录产物并完成运行；失败路径仍负责释放资源。
    pub fn finish_started_run(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: &RunId,
        request: LocalAgentRunnerRequest,
    ) -> Result<Option<LocalAgentRunnerOutcome>> {
        let Some(task) = self.store.load_task(project_path, task_id)? else {
            return Ok(None);
        };
        if task.kind != AgentTaskKind::Finite {
            bail!("local runner only supports finite tasks: {}", task.id.as_str());
        }
        if task.status != AgentTaskStatus::Running {
            return Ok(None);
        }

        let finished_run = self.execute_running_task(project_path, &task, run_id, &request)?;
        Ok(Some(LocalAgentRunnerOutcome { task_id: task.id, run: finished_run }))
    }

    fn claim_ready_run(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        request: &LocalAgentRunnerRequest,
    ) -> Result<Option<StoredAgentRun>> {
        let lease_expires_at_ms = add_ms(request.now_ms, self.lease_duration_ms)
            .context("local run lease expiry timestamp overflow")?;
        let run_id = RunId::new(format!("local-run-{}-{}", task.id.as_str(), request.now_ms.0))?;
        self.store.claim_ready_task_run(
            project_path,
            ClaimReadyTaskRunRequest {
                run_id,
                task_id: task.id.clone(),
                lease_acquired_at_ms: request.now_ms,
                lease_expires_at_ms: Some(lease_expires_at_ms),
            },
        )
    }

    fn advance_to_running(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: &RunId,
        request: &LocalAgentRunnerRequest,
    ) -> Result<()> {
        let dispatched_at_ms = add_ms(request.now_ms, 1)?;
        self.transition_and_log(
            project_path,
            task_id,
            run_id,
            AgentTaskStatus::Assigned,
            AgentTaskStatus::Dispatched,
            dispatched_at_ms,
            "local dispatch prepared",
        )?;

        let acknowledged_at_ms = add_ms(request.now_ms, 2)?;
        self.transition_and_log(
            project_path,
            task_id,
            run_id,
            AgentTaskStatus::Dispatched,
            AgentTaskStatus::Acknowledged,
            acknowledged_at_ms,
            "local worker acknowledged dispatch",
        )?;

        let running_at_ms = add_ms(request.now_ms, 3)?;
        self.store
            .transition_task_status(project_path, task_id, AgentTaskStatus::Running, running_at_ms)?
            .with_context(|| {
                format!("task disappeared before local start: {}", task_id.as_str())
            })?;
        self.store
            .heartbeat_run(
                project_path,
                run_id,
                running_at_ms,
                add_ms(running_at_ms, self.lease_duration_ms)?,
            )?
            .with_context(|| format!("run disappeared before local start: {}", run_id.as_str()))?;
        self.append_log(
            project_path,
            task_id,
            Some(run_id),
            running_at_ms,
            Some(AgentTaskStatus::Acknowledged),
            Some(AgentTaskStatus::Running),
            "local execution started",
        )?;
        self.append_event(
            project_path,
            task_id,
            Some(run_id),
            AgentEventKind::RunStarted,
            Some(AgentTaskStatus::Acknowledged),
            Some(AgentTaskStatus::Running),
            "local execution started",
            json!({"dispatch_target": "local"}),
            running_at_ms,
        )?;
        Ok(())
    }

    fn execute_running_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        run_id: &RunId,
        request: &LocalAgentRunnerRequest,
    ) -> Result<StoredAgentRun> {
        let workspace_request = LocalAgentWorkspaceRequest {
            project_path: project_path.storage_project_path().to_path_buf(),
            task_id: task.id.clone(),
            run_id: run_id.clone(),
        };

        let workspace = match self.executor.lease_workspace(workspace_request) {
            Ok(workspace) => workspace,
            Err(error) => {
                return self.fail_running_task(
                    project_path,
                    task,
                    run_id,
                    request,
                    None,
                    error.to_string(),
                );
            }
        };
        self.record_workspace(
            project_path,
            &task.id,
            run_id,
            &workspace,
            add_ms(request.now_ms, 4)?,
        )?;

        let acp_agent = execution_acp_agent(task);
        let reasoning_effort =
            crate::agent_scheduler::task_board::reasoning_effort_from_metadata(&task.metadata)?;
        let execution = self.executor.execute(LocalAgentExecutionRequest {
            project_path: project_path.storage_project_path().to_path_buf(),
            task_id: task.id.clone(),
            run_id: run_id.clone(),
            title: task.title.clone(),
            prompt: execution_prompt(task),
            model: execution_model(task),
            agent: execution_agent(task, acp_agent.as_deref()),
            acp_agent,
            reasoning_effort,
            workspace: workspace.clone(),
        });

        match execution {
            Ok(output) => match commit_workspace_changes(&workspace, task) {
                Ok(Some(commit_sha)) => {
                    self.append_log(
                        project_path,
                        &task.id,
                        Some(run_id),
                        add_ms(request.now_ms, 5)?,
                        None,
                        None,
                        format!("local execution committed workspace changes: {commit_sha}"),
                    )?;
                    self.complete_running_task(
                        project_path,
                        task,
                        run_id,
                        request,
                        &workspace,
                        output.raw_output,
                    )
                }
                Ok(None) => {
                    self.append_log(
                        project_path,
                        &task.id,
                        Some(run_id),
                        add_ms(request.now_ms, 5)?,
                        None,
                        None,
                        "local execution produced no committable workspace changes",
                    )?;
                    self.complete_running_task(
                        project_path,
                        task,
                        run_id,
                        request,
                        &workspace,
                        output.raw_output,
                    )
                }
                Err(error) => {
                    let message = format!("local execution workspace commit failed: {error}");
                    let failed = self.fail_running_task(
                        project_path,
                        task,
                        run_id,
                        request,
                        Some(&workspace),
                        message.clone(),
                    )?;
                    self.executor.recycle_workspace(LocalAgentWorkspaceRecycleRequest {
                        project_path: project_path.storage_project_path().to_path_buf(),
                        task_id: task.id.clone(),
                        run_id: run_id.clone(),
                        workspace,
                        taint_reason: message,
                    })?;
                    Ok(failed)
                }
            },
            Err(error) => {
                let message = error.to_string();
                let failed = self.fail_running_task(
                    project_path,
                    task,
                    run_id,
                    request,
                    Some(&workspace),
                    message.clone(),
                )?;
                self.executor.recycle_workspace(LocalAgentWorkspaceRecycleRequest {
                    project_path: project_path.storage_project_path().to_path_buf(),
                    task_id: task.id.clone(),
                    run_id: run_id.clone(),
                    workspace,
                    taint_reason: message,
                })?;
                Ok(failed)
            }
        }
    }

    fn complete_running_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        run_id: &RunId,
        request: &LocalAgentRunnerRequest,
        workspace: &LocalAgentWorkspace,
        raw_output: String,
    ) -> Result<StoredAgentRun> {
        let submitted_at_ms = add_ms(request.now_ms, 5)?;
        self.sync_task_workspace_metadata(project_path, task, workspace, submitted_at_ms)?;
        self.transition_and_log(
            project_path,
            &task.id,
            run_id,
            AgentTaskStatus::Running,
            AgentTaskStatus::Submitted,
            submitted_at_ms,
            "local execution submitted result",
        )?;
        self.append_event(
            project_path,
            &task.id,
            Some(run_id),
            AgentEventKind::RunSubmitted,
            Some(AgentTaskStatus::Running),
            Some(AgentTaskStatus::Submitted),
            "local execution submitted result",
            json!({"dispatch_target": "local"}),
            submitted_at_ms,
        )?;
        self.record_execution_artifact(
            project_path,
            &task.id,
            run_id,
            Some(workspace),
            true,
            Some(raw_output.as_str()),
            None,
            submitted_at_ms,
        )?;

        let finished = self
            .store
            .finish_run(
                project_path,
                run_id,
                AgentRunFinishStatus::Succeeded,
                submitted_at_ms,
                None,
            )?
            .with_context(|| {
                format!("run disappeared before local submission: {}", run_id.as_str())
            })?;
        Ok(finished)
    }

    fn sync_task_workspace_metadata(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        workspace: &LocalAgentWorkspace,
        updated_at_ms: TimestampMs,
    ) -> Result<()> {
        let mut metadata = object_metadata(task.metadata.clone())?;
        metadata.insert("selected_worktree_slot_id".to_string(), json!(workspace.slot_id.clone()));
        metadata.insert("selected_worktree_path".to_string(), json!(workspace.path.clone()));
        metadata.insert("selected_worktree_branch".to_string(), json!(workspace.branch.clone()));
        metadata.insert("merge_source_branch".to_string(), json!(workspace.branch.clone()));
        self.store
            .update_task_board_task(
                project_path,
                &task.id,
                AgentTaskBoardUpdate {
                    title: None,
                    status: None,
                    metadata: Value::Object(metadata),
                    updated_at_ms,
                },
            )?
            .with_context(|| {
                format!("task disappeared before workspace metadata sync: {}", task.id.as_str())
            })?;
        Ok(())
    }

    fn fail_running_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        run_id: &RunId,
        request: &LocalAgentRunnerRequest,
        workspace: Option<&LocalAgentWorkspace>,
        error_message: String,
    ) -> Result<StoredAgentRun> {
        let failed_at_ms = add_ms(request.now_ms, 5)?;
        self.record_execution_artifact(
            project_path,
            &task.id,
            run_id,
            workspace,
            false,
            None,
            Some(error_message.as_str()),
            failed_at_ms,
        )?;
        let finished = self
            .store
            .finish_run(
                project_path,
                run_id,
                AgentRunFinishStatus::Failed,
                failed_at_ms,
                Some(error_message.clone()),
            )?
            .with_context(|| {
                format!("run disappeared before local failure: {}", run_id.as_str())
            })?;
        self.transition_and_log(
            project_path,
            &task.id,
            run_id,
            AgentTaskStatus::Running,
            AgentTaskStatus::Failed,
            failed_at_ms,
            format!("local execution failed: {error_message}"),
        )?;
        self.append_event(
            project_path,
            &task.id,
            Some(run_id),
            AgentEventKind::RunFailed,
            Some(AgentTaskStatus::Running),
            Some(AgentTaskStatus::Failed),
            format!("local execution failed: {error_message}"),
            json!({"dispatch_target": "local", "error": error_message}),
            failed_at_ms,
        )?;
        Ok(finished)
    }

    fn transition_and_log(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: &RunId,
        from_status: AgentTaskStatus,
        to_status: AgentTaskStatus,
        timestamp_ms: TimestampMs,
        message: impl Into<String>,
    ) -> Result<()> {
        self.store
            .transition_task_status(project_path, task_id, to_status, timestamp_ms)?
            .with_context(|| {
                format!(
                    "task disappeared during local transition {} -> {}: {}",
                    from_status.as_str(),
                    to_status.as_str(),
                    task_id.as_str()
                )
            })?;
        self.append_log(
            project_path,
            task_id,
            Some(run_id),
            timestamp_ms,
            Some(from_status),
            Some(to_status),
            message,
        )
    }

    fn append_log(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: Option<&RunId>,
        timestamp_ms: TimestampMs,
        from_status: Option<AgentTaskStatus>,
        to_status: Option<AgentTaskStatus>,
        message: impl Into<String>,
    ) -> Result<()> {
        self.store.append_task_log(
            project_path,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: run_id.cloned(),
                timestamp_ms,
                from_status,
                to_status,
                from_status_key: None,
                to_status_key: None,
                message: message.into(),
            },
        )?;
        Ok(())
    }

    fn append_event(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: Option<&RunId>,
        kind: AgentEventKind,
        from_status: Option<AgentTaskStatus>,
        to_status: Option<AgentTaskStatus>,
        message: impl Into<String>,
        payload: Value,
        created_at_ms: TimestampMs,
    ) -> Result<()> {
        self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task_id.clone()),
                run_id: run_id.cloned(),
                worker_id: None,
                team_id: None,
                kind,
                from_status,
                to_status,
                from_status_key: None,
                to_status_key: None,
                message: message.into(),
                payload,
                created_at_ms,
            },
        )?;
        Ok(())
    }

    fn record_workspace(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: &RunId,
        workspace: &LocalAgentWorkspace,
        timestamp_ms: TimestampMs,
    ) -> Result<()> {
        self.store
            .record_run_workspace_context(
                project_path,
                task_id,
                run_id,
                AgentSchedulerWorkspaceContext {
                    slot_id: workspace.slot_id.clone(),
                    path: workspace.path.clone(),
                    branch: workspace.branch.clone(),
                },
                timestamp_ms,
            )?
            .with_context(|| {
                format!("run disappeared before workspace record: {}", run_id.as_str())
            })?;
        self.append_log(
            project_path,
            task_id,
            Some(run_id),
            timestamp_ms,
            None,
            None,
            format!(
                "local worktree lease recorded: slot={} branch={} path={}",
                workspace.slot_id,
                workspace.branch,
                truncate_log_text(&workspace.path, WORKTREE_LOG_PATH_LIMIT)
            ),
        )?;
        Ok(())
    }

    fn record_execution_artifact(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: &RunId,
        workspace: Option<&LocalAgentWorkspace>,
        success: bool,
        output: Option<&str>,
        error: Option<&str>,
        timestamp_ms: TimestampMs,
    ) -> Result<()> {
        self.store.record_task_artifact(
            project_path,
            RecordTaskArtifactRequest {
                task_id: task_id.clone(),
                run_id: Some(run_id.clone()),
                kind: "execution_result".to_string(),
                name: "execution-result.json".to_string(),
                path: Some(format!(
                    "shared/tasks/{}/artifacts/execution-result.json",
                    task_id.as_str()
                )),
                payload: execution_artifact_payload(success, output, error, workspace),
                created_at_ms: timestamp_ms,
            },
        )?;
        Ok(())
    }
}

fn commit_workspace_changes(
    workspace: &LocalAgentWorkspace,
    task: &StoredAgentTask,
) -> Result<Option<String>> {
    let workspace_path = Path::new(&workspace.path);
    if !workspace_path.is_dir() {
        bail!("workspace path does not exist: {}", workspace.path);
    }

    run_git_ok(workspace_path, &["rev-parse", "--is-inside-work-tree"])?;
    run_git_ok(workspace_path, &["add", "-A"])?;
    if !git_has_staged_changes(workspace_path)? {
        return Ok(None);
    }

    let message = format!("task({}): submit local execution result", task.id.as_str());
    run_git_ok(workspace_path, &["commit", "--no-verify", "-m", &message])?;
    let output = run_git_ok(workspace_path, &["rev-parse", "--short", "HEAD"])?;
    Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
}

pub(crate) fn git_has_staged_changes(workspace_path: &Path) -> Result<bool> {
    let output = run_git(workspace_path, &["diff", "--cached", "--quiet"])?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => bail!("git diff --cached failed: {}", git_output_detail(&output)),
    }
}

pub(crate) fn run_git_ok(workspace_path: &Path, args: &[&str]) -> Result<Output> {
    let output = run_git(workspace_path, args)?;
    if output.status.success() {
        return Ok(output);
    }
    bail!("git {} failed: {}", args.join(" "), git_output_detail(&output))
}

fn run_git(workspace_path: &Path, args: &[&str]) -> Result<Output> {
    crate::app::agent::shell::git_std_command()
        .current_dir(workspace_path)
        .args(args)
        .output()
        .with_context(|| format!("run git {} in {}", args.join(" "), workspace_path.display()))
}

pub(crate) fn git_output_detail(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("exit status {}", output.status),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
    }
}

pub(crate) fn add_ms(timestamp: TimestampMs, delta_ms: i64) -> Result<TimestampMs> {
    Ok(TimestampMs(timestamp.0.checked_add(delta_ms).context("timestamp overflow")?))
}

pub(crate) fn truncate_log_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let mut truncated: String = trimmed.chars().take(keep).collect();
    truncated.push_str("...");
    truncated
}

pub(crate) fn execution_prompt(task: &StoredAgentTask) -> String {
    let prompt = task
        .metadata
        .get("prompt")
        .and_then(Value::as_str)
        .filter(|prompt| !prompt.trim().is_empty())
        .unwrap_or(&task.title)
        .trim()
        .to_string();
    format!("{prompt}\n\n{FINAL_REPLY_INSTRUCTION}")
}

pub(crate) fn execution_model(task: &StoredAgentTask) -> Option<String> {
    execution_metadata_string(task, "model").filter(|model| !model.eq_ignore_ascii_case("auto"))
}

pub(crate) fn execution_agent(task: &StoredAgentTask, acp_agent: Option<&str>) -> Option<String> {
    let agent = execution_metadata_string(task, "agent")?;
    if acp_agent.is_some() && agent.eq_ignore_ascii_case("main") {
        return None;
    }
    Some(agent)
}

pub(crate) fn execution_acp_agent(task: &StoredAgentTask) -> Option<String> {
    execution_metadata_string(task, "acp_agent").and_then(normalize_acp_agent)
}

pub(crate) fn normalize_acp_agent(value: String) -> Option<String> {
    match value.to_ascii_lowercase().as_str() {
        "default" | "acp" | "internal" => None,
        _ => Some(value),
    }
}

pub(crate) fn execution_metadata_string(task: &StoredAgentTask, key: &str) -> Option<String> {
    task.metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn object_metadata(value: Value) -> Result<Map<String, Value>> {
    match value {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Map::new()),
        _ => bail!("task metadata must be a JSON object"),
    }
}

pub(crate) fn execution_artifact_payload(
    success: bool,
    output: Option<&str>,
    error: Option<&str>,
    workspace: Option<&LocalAgentWorkspace>,
) -> Value {
    json!({
        "success": success,
        "output": output,
        "error": error,
        "workspace": workspace.map(|workspace| {
            json!({
                "slot_id": workspace.slot_id,
                "path": workspace.path,
                "branch": workspace.branch,
            })
        }),
    })
}
