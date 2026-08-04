//! 计划生成与按批次调度的子会话执行器。

use super::goal::{
    GoalTaskFilesState, build_goal_continuation_prompt, build_goal_evaluation_prompt,
    goal_evaluation_session_id, goal_turn_session_id, inspect_goal_task_files,
    parse_goal_evaluation,
};
use super::prompts::{
    build_execution_prompt, build_goal_planning_prompt, build_planning_prompt, plan_dir_abs,
};
use super::session_bind::{ensure_plan_run_session_info, persist_plan_run_session_history};
use super::{PlanTaskEntry, build_batch_stages, load_tasks_json};
use crate::session::child_agent::coordinator::{
    ChildAgentTaskSnapshot, ChildAgentTaskState, global_child_agent_coordinator,
};
use async_trait::async_trait;
use futures_util::future::join_all;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use vw_api_types::task::{TaskPlanRunMode, TaskPlanRunRequest, TaskPlanRunStreamEventDto};

/// 子任务首次执行失败后允许的额外重试次数。
///
/// 重试只重新运行当前子任务，不重新进入规划阶段，也不会提前启动后续批次。
const SUBTASK_RETRY_MAX_ATTEMPTS: usize = 3;

/// plan-run 本身不为后台后代设置业务超时；用长等待兜底协调器的有限超时接口。
const DESCENDANT_WAIT_TIMEOUT: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// 单次会话启动参数（规划或子任务）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRunRequest {
    /// 会话 ID。
    pub session_id: String,
    /// 提示词 / 查询文本。
    pub query: String,
    /// 工作区根目录。
    pub root: String,
    /// 可选模型。
    pub model: Option<String>,
    /// 可选委托 agent。
    pub agent: Option<String>,
    /// 可选 ACP agent。
    pub acp_agent: Option<String>,
}

/// 可注入的会话启动抽象，便于单测替换真实 LLM。
#[async_trait]
pub trait SessionRunner: Send + Sync {
    /// 启动一次会话并返回完整输出文本。
    async fn run_session(
        &self,
        request: SessionRunRequest,
        delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String>;
}

/// 默认实现：走 gateway/session processor。
#[derive(Debug, Default, Clone, Copy)]
pub struct GatewaySessionRunner;

/// 构建规划/执行子会话的 processor options。
///
/// 对齐桌面任务执行器：始终开启 `full_access`，否则写文件工具会被审批拦住，
/// 规划会话只会回文本、落不了 `tasks.json`。
pub(crate) fn gateway_session_options(request: &SessionRunRequest) -> Map<String, Value> {
    let mut options = Map::new();
    options.insert("full_access".to_string(), json!(true));
    options.insert("cwd".to_string(), json!(request.root.as_str()));

    if let Some(agent) = request.agent.as_ref().filter(|value| !value.trim().is_empty()) {
        options.insert("agent".to_string(), json!(agent));
    }

    let acp_agent = request
        .acp_agent
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let acp_enabled = acp_agent.is_some();
    options.insert("acp_test".to_string(), json!(acp_enabled));
    if let Some(acp_agent) = acp_agent {
        options.insert("acp_agent".to_string(), json!(acp_agent));
        options.insert("acp_permission_mode".to_string(), json!("approve-all"));
        options.insert("acp_force_new_session".to_string(), json!(true));
        options.insert("acp_close_client_after_request".to_string(), json!(true));
        options.insert("acp_history_strategy".to_string(), json!("discard"));
        options.insert("acp_history_recent_count".to_string(), json!(1));
    }

    options
}

#[async_trait]
impl SessionRunner for GatewaySessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        let options = gateway_session_options(&request);

        let processor_request = crate::app::agent::session::processor::Request {
            stream: crate::app::agent::session::session::now_ms(),
            session: request.session_id,
            query: request.query,
            root: Some(request.root),
            model: request.model,
            options: Value::Object(options),
            approval: None,
            channel_name: None,
            non_cli_approval_context: None,
            assistant_message_id: None,
            history: Vec::new(),
            persist_app_session_artifacts: true,
            app_session_scope: None,
            abort_token: None,
            session_tools: crate::session::runtime::SessionToolset::default(),
        };

        crate::channels::session::run_session_processor_for_channel(processor_request, delta_tx)
            .await
            .map_err(|error| error.to_string())
    }
}

/// 使用根取消令牌执行计划；取消会抢占当前规划、批次等待、重试或目标续轮。
pub async fn run_task_plan_with_cancellation<R, F>(
    task_id: &str,
    request: &TaskPlanRunRequest,
    runner: &R,
    cancellation: CancellationToken,
    mut on_event: F,
) -> Result<(), String>
where
    R: SessionRunner + ?Sized,
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            let _ = on_event(TaskPlanRunStreamEventDto::Cancelled {
                task_id: task_id.to_string(),
                session_id: None,
            });
            Ok(())
        }
        result = run_task_plan(task_id, request, runner, &mut on_event) => result,
    }
}

/// 规划 → 读 `tasks.json` → 按批次执行子会话，并通过回调推送事件。
///
/// 回调返回 `false` 时提前停止。子任务失败时先在原子任务会话内进行有限重试；
/// 重试耗尽后发出 `Blocked` 并停止后续批次。该路径仍返回 `Ok(())`
/// （失败已通过事件表达）。规划/调度级错误发出 `Error` 并返回 `Err`。
pub async fn run_task_plan<R, F>(
    task_id: &str,
    request: &TaskPlanRunRequest,
    runner: &R,
    mut on_event: F,
) -> Result<(), String>
where
    R: SessionRunner + ?Sized,
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    let root = request.root.trim();
    if root.is_empty() {
        let message = "task-plan-run 需要非空 root".to_string();
        let _ = on_event(TaskPlanRunStreamEventDto::Error {
            task_id: Some(task_id.to_string()),
            message: message.clone(),
        });
        return Err(message);
    }
    if request.query.trim().is_empty() {
        let message = "task-plan-run 需要非空 query".to_string();
        let _ = on_event(TaskPlanRunStreamEventDto::Error {
            task_id: Some(task_id.to_string()),
            message: message.clone(),
        });
        return Err(message);
    }

    let plan_abs = plan_dir_abs(root, task_id);
    let plan_dir_display = path_display(&plan_abs);
    let planning_session = planning_session_id(task_id);
    let parent_session_id = request
        .parent_session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let planning_title = format!("规划 · {task_id}");

    if let Err(error) = std::fs::create_dir_all(&plan_abs) {
        let message = format!("无法创建计划目录 {}: {error}", plan_dir_display);
        let _ = on_event(TaskPlanRunStreamEventDto::Error {
            task_id: Some(task_id.to_string()),
            message: message.clone(),
        });
        return Err(message);
    }

    // Info 绑定失败不应中断规划：会话目录/历史仍可由 run_session 落盘，
    // 否则桌面侧只看到 PlanningStarted 后流就断，表现为「一发起就挂」。
    if let Err(error) = ensure_plan_run_session_info(
        &planning_session,
        parent_session_id,
        root,
        &planning_title,
    )
    .await
    {
        tracing::warn!(
            target: "vw_agent::task_plan_run",
            task_id = %task_id,
            session_id = %planning_session,
            error = %error,
            "绑定规划会话 Info 失败，继续规划"
        );
    }

    if !emit(
        &mut on_event,
        TaskPlanRunStreamEventDto::PlanningStarted {
            task_id: task_id.to_string(),
            plan_dir: plan_dir_display.clone(),
            session_id: planning_session.clone(),
        },
    ) {
        return Ok(());
    }

    let planning_prompt = match request.mode {
        TaskPlanRunMode::Workflow => build_planning_prompt(task_id, &request.query),
        TaskPlanRunMode::Goal => build_goal_planning_prompt(task_id, &request.query),
    };
    let (planning_delta_tx, mut planning_delta_rx) = tokio::sync::mpsc::channel(64);
    let planning_run = runner.run_session(
        SessionRunRequest {
            session_id: planning_session.clone(),
            query: planning_prompt.clone(),
            root: root.to_string(),
            model: request.model.clone(),
            agent: request.agent.clone(),
            acp_agent: request.acp_agent.clone(),
        },
        Some(planning_delta_tx),
    );
    tokio::pin!(planning_run);
    let planning_result = loop {
        tokio::select! {
            result = &mut planning_run => {
                while let Ok(delta) = planning_delta_rx.try_recv() {
                    if !emit(
                        &mut on_event,
                        TaskPlanRunStreamEventDto::SessionDelta {
                            task_id: task_id.to_string(),
                            session_id: planning_session.clone(),
                            delta,
                        },
                    ) {
                        return Ok(());
                    }
                }
                break result;
            },
            delta = planning_delta_rx.recv() => {
                let Some(delta) = delta else {
                    break planning_run.await;
                };
                if !emit(
                    &mut on_event,
                    TaskPlanRunStreamEventDto::SessionDelta {
                        task_id: task_id.to_string(),
                        session_id: planning_session.clone(),
                        delta,
                    },
                ) {
                    return Ok(());
                }
            }
        }
    };
    let Some(planning_descendants) =
        wait_for_session_descendants(task_id, &planning_session, |event| {
            std::future::ready(emit(&mut on_event, event))
        })
        .await?
    else {
        return Ok(());
    };
    let planning_output = match planning_result {
        Ok(mut output) => {
            append_descendant_evidence(&mut output, &planning_descendants);
            let _ = persist_plan_run_session_history(
                &planning_session,
                root,
                &planning_title,
                &planning_prompt,
                &output,
            )
            .await;
            output
        }
        Err(mut error) => {
            append_descendant_evidence(&mut error, &planning_descendants);
            let _ = persist_plan_run_session_history(
                &planning_session,
                root,
                &planning_title,
                &planning_prompt,
                &format!("规划会话失败: {error}"),
            )
            .await;
            let message = format!("规划会话失败: {error}");
            let _ = on_event(TaskPlanRunStreamEventDto::Error {
                task_id: Some(task_id.to_string()),
                message: message.clone(),
            });
            return Err(message);
        }
    };

    let tasks_json_path = plan_abs.join("tasks.json");
    let document = match load_tasks_json(&tasks_json_path) {
        Ok(document) => document,
        Err(error) => {
            let preview = truncate_for_error(&planning_output, 800);
            let guidance = if tasks_json_path.is_file() {
                "tasks.json 已写出，但内容格式不符合运行时要求；请检查字段类型和必填字段"
            } else {
                "规划会话未在固定目录写出任务文件；请确认模型通过写文件工具落盘，而不是只在回复里描述计划"
            };
            let message = format!(
                "规划完成后无法加载 tasks.json: {error}。{guidance}。规划输出预览:\n{preview}"
            );
            let _ = on_event(TaskPlanRunStreamEventDto::Error {
                task_id: Some(task_id.to_string()),
                message: message.clone(),
            });
            return Err(message);
        }
    };

    if !emit(
        &mut on_event,
        TaskPlanRunStreamEventDto::PlanCreated {
            task_id: task_id.to_string(),
            plan_dir: plan_dir_display.clone(),
            plan_md_path: path_display(&plan_abs.join("plan.md")),
            prompt_md_path: path_display(&plan_abs.join("prompt.md")),
            tasks_json_path: path_display(&tasks_json_path),
            subtask_count: document.tasks.len(),
            session_id: planning_session.clone(),
        },
    ) {
        return Ok(());
    }

    let stages = match build_batch_stages(&document.tasks) {
        Ok(stages) => stages,
        Err(error) => {
            let message = format!("批次调度失败: {error}");
            let _ = on_event(TaskPlanRunStreamEventDto::Error {
                task_id: Some(task_id.to_string()),
                message: message.clone(),
            });
            return Err(message);
        }
    };

    let plan_markdown = std::fs::read_to_string(plan_abs.join("plan.md")).unwrap_or_default();
    let mut combined_output = String::new();

    for stage in stages {
        let batch = document.tasks[stage[0]].batch;
        let subtask_ids: Vec<String> = stage
            .iter()
            .map(|&index| document.tasks[index].id.clone())
            .collect();

        if !emit(
            &mut on_event,
            TaskPlanRunStreamEventDto::BatchStarted {
                task_id: task_id.to_string(),
                batch,
                subtask_ids: subtask_ids.clone(),
            },
        ) {
            return Ok(());
        }

        // 先串行绑定会话并登记 SubtaskStarted、准备提示词，再并发启动子会话。
        let mut launches = Vec::with_capacity(stage.len());
        for &index in &stage {
            let task = &document.tasks[index];
            let task_file_abs = resolve_task_file(&plan_abs, task);
            let task_file_display = path_display(&task_file_abs);
            let session_id = subtask_session_id(task_id, &task.id);
            let subtask_title = task.title.clone();
            if let Err(error) = ensure_plan_run_session_info(
                &session_id,
                parent_session_id,
                root,
                &subtask_title,
            )
            .await
            {
                tracing::warn!(
                    target: "vw_agent::task_plan_run",
                    task_id = %task_id,
                    session_id = %session_id,
                    error = %error,
                    "绑定子任务会话 Info 失败，继续执行"
                );
            }

            if !emit(
                &mut on_event,
                TaskPlanRunStreamEventDto::SubtaskStarted {
                    task_id: task_id.to_string(),
                    batch,
                    subtask_id: task.id.clone(),
                    title: task.title.clone(),
                    task_file: task_file_display.clone(),
                    session_id: session_id.clone(),
                },
            ) {
                return Ok(());
            }

            let subtask_markdown = std::fs::read_to_string(&task_file_abs).unwrap_or_default();
            let prompt = build_execution_prompt(
                &request.query,
                task.batch,
                &task.depends_on_batches,
                &plan_markdown,
                &subtask_markdown,
            );
            launches.push(PreparedLaunch {
                index,
                task: task.clone(),
                task_file_display,
                session_title: subtask_title,
                session_request: SessionRunRequest {
                    session_id,
                    query: prompt,
                    root: root.to_string(),
                    model: request.model.clone(),
                    agent: request.agent.clone(),
                    acp_agent: request.acp_agent.clone(),
                },
            });
        }

        let (stage_event_tx, mut stage_event_rx) = tokio::sync::mpsc::channel(64);
        let stage_run = join_all(launches.into_iter().map(|launch| {
            run_prepared_launch_with_retry(task_id, runner, launch, stage_event_tx.clone())
        }));
        drop(stage_event_tx);
        tokio::pin!(stage_run);
        let results = loop {
            tokio::select! {
                results = &mut stage_run => {
                    while let Ok(event) = stage_event_rx.try_recv() {
                        if !emit(&mut on_event, event) {
                            return Ok(());
                        }
                    }
                    break results;
                },
                event = stage_event_rx.recv() => {
                    let Some(event) = event else {
                        break stage_run.await;
                    };
                    if !emit(&mut on_event, event) {
                        return Ok(());
                    }
                }
            }
        };

        let mut ordered = results;
        ordered.sort_by_key(|(index, _, _, _)| *index);

        let mut stage_failed: Option<(String, String)> = None;
        for (_index, task, task_file_display, result) in ordered {
            let session_id = subtask_session_id(task_id, &task.id);
            match result {
                Ok(output) => {
                    if !combined_output.is_empty() {
                        combined_output.push_str("\n\n");
                    }
                    combined_output.push_str(&output);
                    if !emit(
                        &mut on_event,
                        TaskPlanRunStreamEventDto::SubtaskFinished {
                            task_id: task_id.to_string(),
                            batch,
                            subtask_id: task.id.clone(),
                            title: task.title.clone(),
                            task_file: task_file_display,
                            success: true,
                            output: Some(output),
                            error: None,
                            session_id,
                        },
                    ) {
                        return Ok(());
                    }
                }
                Err(error) => {
                    if !emit(
                        &mut on_event,
                        TaskPlanRunStreamEventDto::SubtaskFinished {
                            task_id: task_id.to_string(),
                            batch,
                            subtask_id: task.id.clone(),
                            title: task.title.clone(),
                            task_file: task_file_display,
                            success: false,
                            output: None,
                            error: Some(error.clone()),
                            session_id,
                        },
                    ) {
                        return Ok(());
                    }
                    stage_failed.get_or_insert((task.id.clone(), error));
                }
            }
        }

        if !emit(
            &mut on_event,
            TaskPlanRunStreamEventDto::BatchFinished {
                task_id: task_id.to_string(),
                batch,
                subtask_ids,
                success: stage_failed.is_none(),
            },
        ) {
            return Ok(());
        }

        if let Some((subtask_id, reason)) = stage_failed {
            if request.mode == TaskPlanRunMode::Goal {
                if !combined_output.is_empty() {
                    combined_output.push_str("\n\n");
                }
                combined_output.push_str(&format!("子任务 {subtask_id} 执行失败：{reason}"));
                continue;
            }
            let session_id = subtask_session_id(task_id, &subtask_id);
            let _ = on_event(TaskPlanRunStreamEventDto::Blocked {
                task_id: task_id.to_string(),
                batch: Some(batch),
                subtask_id: Some(subtask_id),
                reason,
                session_id,
            });
            return Ok(());
        }
    }

    if request.mode == TaskPlanRunMode::Goal {
        return run_goal_loop(
            task_id,
            request,
            runner,
            &mut on_event,
            &plan_abs,
            &plan_dir_display,
            &plan_markdown,
            &document.tasks,
            combined_output,
        )
        .await;
    }

    let _ = on_event(TaskPlanRunStreamEventDto::Finished {
        task_id: task_id.to_string(),
        plan_dir: plan_dir_display,
        status: "completed".to_string(),
        output: combined_output,
        session_id: Some(planning_session),
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_goal_loop<R, F>(
    task_id: &str,
    request: &TaskPlanRunRequest,
    runner: &R,
    on_event: &mut F,
    plan_abs: &Path,
    plan_dir_display: &str,
    plan_markdown: &str,
    tasks: &[PlanTaskEntry],
    mut execution_evidence: String,
) -> Result<(), String>
where
    R: SessionRunner + ?Sized,
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    let root = request.root.trim();
    let parent_session_id = request
        .parent_session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut turn = 1_u32;
    let mut last_infrastructure_error: Option<String> = None;
    let mut repeated_infrastructure_errors = 0_u8;

    loop {
        let task_files_state = inspect_goal_task_files(plan_abs, tasks)?;
        if let GoalTaskFilesState::Blocked(reason) = &task_files_state {
            let _ = on_event(TaskPlanRunStreamEventDto::Blocked {
                task_id: task_id.to_string(),
                batch: None,
                subtask_id: None,
                reason: reason.clone(),
                session_id: goal_turn_session_id(task_id, turn),
            });
            return Ok(());
        }

        let evaluation_session_id = goal_evaluation_session_id(task_id, turn);
        let evaluation_title = format!("目标验收 · 第 {turn} 轮");
        let evaluation_prompt = build_goal_evaluation_prompt(
            &request.query,
            plan_markdown,
            &execution_evidence,
            &task_files_state,
        );
        if let Err(error) = ensure_plan_run_session_info(
            &evaluation_session_id,
            parent_session_id,
            root,
            &evaluation_title,
        )
        .await
        {
            tracing::warn!(
                target: "vw_agent::task_plan_run",
                task_id = %task_id,
                session_id = %evaluation_session_id,
                error = %error,
                "绑定目标验收会话 Info 失败，继续验收"
            );
        }

        let evaluation_result = runner
            .run_session(
                SessionRunRequest {
                    session_id: evaluation_session_id.clone(),
                    query: evaluation_prompt.clone(),
                    root: root.to_string(),
                    model: request.model.clone(),
                    agent: request.agent.clone(),
                    acp_agent: request.acp_agent.clone(),
                },
                None,
            )
            .await;
        let Some(evaluation_descendants) =
            wait_for_session_descendants(task_id, &evaluation_session_id, |event| {
                std::future::ready(emit(on_event, event))
            })
            .await?
        else {
            return Ok(());
        };
        append_descendant_evidence(&mut execution_evidence, &evaluation_descendants);

        let (achieved, reason) = match evaluation_result {
            Ok(output) => {
                let _ = persist_plan_run_session_history(
                    &evaluation_session_id,
                    root,
                    &evaluation_title,
                    &evaluation_prompt,
                    &output,
                )
                .await;
                match parse_goal_evaluation(&output) {
                    Ok(evaluation) => {
                        let achieved = evaluation.achieved
                            && matches!(&task_files_state, GoalTaskFilesState::Complete);
                        let reason = if evaluation.achieved && !achieved {
                            "编号任务文件仍存在，缺少验证证据".to_string()
                        } else {
                            evaluation.reason
                        };
                        (achieved, reason)
                    }
                    Err(error) => {
                        let reason = format!("目标验收基础设施错误：{error}");
                        if record_repeated_infrastructure_error(
                            &reason,
                            &mut last_infrastructure_error,
                            &mut repeated_infrastructure_errors,
                        ) {
                            emit_repeated_infrastructure_block(
                                task_id,
                                turn,
                                &reason,
                                on_event,
                            );
                            return Ok(());
                        }
                        (false, reason)
                    }
                }
            }
            Err(error) => {
                let _ = persist_plan_run_session_history(
                    &evaluation_session_id,
                    root,
                    &evaluation_title,
                    &evaluation_prompt,
                    &format!("目标验收失败: {error}"),
                )
                .await;
                let reason = format!("目标验收基础设施错误：{error}");
                if record_repeated_infrastructure_error(
                    &reason,
                    &mut last_infrastructure_error,
                    &mut repeated_infrastructure_errors,
                ) {
                    emit_repeated_infrastructure_block(task_id, turn, &reason, on_event);
                    return Ok(());
                }
                (false, reason)
            }
        };

        if !emit(
            on_event,
            TaskPlanRunStreamEventDto::GoalEvaluated {
                task_id: task_id.to_string(),
                turn,
                achieved,
                reason: reason.clone(),
                session_id: evaluation_session_id,
            },
        ) {
            return Ok(());
        }

        if achieved {
            let _ = on_event(TaskPlanRunStreamEventDto::Finished {
                task_id: task_id.to_string(),
                plan_dir: plan_dir_display.to_string(),
                status: "completed".to_string(),
                output: execution_evidence,
                session_id: Some(goal_turn_session_id(task_id, turn)),
            });
            return Ok(());
        }

        turn = turn.saturating_add(1);
        let continuation_session_id = goal_turn_session_id(task_id, turn);
        if !emit(
            on_event,
            TaskPlanRunStreamEventDto::GoalTurnStarted {
                task_id: task_id.to_string(),
                turn,
                session_id: continuation_session_id.clone(),
                reason: reason.clone(),
            },
        ) {
            return Ok(());
        }

        let continuation_title = format!("目标续轮 · 第 {turn} 轮");
        let continuation_prompt = build_goal_continuation_prompt(
            &request.query,
            plan_markdown,
            &reason,
            &task_files_state,
        );
        if let Err(error) = ensure_plan_run_session_info(
            &continuation_session_id,
            parent_session_id,
            root,
            &continuation_title,
        )
        .await
        {
            tracing::warn!(
                target: "vw_agent::task_plan_run",
                task_id = %task_id,
                session_id = %continuation_session_id,
                error = %error,
                "绑定目标续轮会话 Info 失败，继续执行"
            );
        }

        let continuation_result = runner
            .run_session(
                SessionRunRequest {
                    session_id: continuation_session_id.clone(),
                    query: continuation_prompt.clone(),
                    root: root.to_string(),
                    model: request.model.clone(),
                    agent: request.agent.clone(),
                    acp_agent: request.acp_agent.clone(),
                },
                None,
            )
            .await;
        let Some(continuation_descendants) =
            wait_for_session_descendants(task_id, &continuation_session_id, |event| {
                std::future::ready(emit(on_event, event))
            })
            .await?
        else {
            return Ok(());
        };

        match continuation_result {
            Ok(mut output) => {
                append_descendant_evidence(&mut output, &continuation_descendants);
                let _ = persist_plan_run_session_history(
                    &continuation_session_id,
                    root,
                    &continuation_title,
                    &continuation_prompt,
                    &output,
                )
                .await;
                if !execution_evidence.is_empty() {
                    execution_evidence.push_str("\n\n");
                }
                execution_evidence.push_str(&output);
            }
            Err(mut error) => {
                append_descendant_evidence(&mut error, &continuation_descendants);
                let _ = persist_plan_run_session_history(
                    &continuation_session_id,
                    root,
                    &continuation_title,
                    &continuation_prompt,
                    &format!("目标续轮失败: {error}"),
                )
                .await;
                let infrastructure_error = format!("目标续轮基础设施错误：{error}");
                if record_repeated_infrastructure_error(
                    &infrastructure_error,
                    &mut last_infrastructure_error,
                    &mut repeated_infrastructure_errors,
                ) {
                    emit_repeated_infrastructure_block(
                        task_id,
                        turn,
                        &infrastructure_error,
                        on_event,
                    );
                    return Ok(());
                }
                if !execution_evidence.is_empty() {
                    execution_evidence.push_str("\n\n");
                }
                execution_evidence.push_str(&infrastructure_error);
            }
        }
    }
}

fn record_repeated_infrastructure_error(
    error: &str,
    last_error: &mut Option<String>,
    repeated_errors: &mut u8,
) -> bool {
    if last_error.as_deref() == Some(error) {
        *repeated_errors = repeated_errors.saturating_add(1);
    } else {
        *last_error = Some(error.to_string());
        *repeated_errors = 1;
    }
    *repeated_errors >= 3
}

fn emit_repeated_infrastructure_block<F>(
    task_id: &str,
    turn: u32,
    error: &str,
    on_event: &mut F,
) where
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    let _ = on_event(TaskPlanRunStreamEventDto::Blocked {
        task_id: task_id.to_string(),
        batch: None,
        subtask_id: None,
        reason: format!("连续三轮出现相同基础设施错误，目标模式已停止：{error}"),
        session_id: goal_turn_session_id(task_id, turn),
    });
}

async fn wait_for_session_descendants<F, Fut>(
    task_id: &str,
    session_id: &str,
    on_waiting: F,
) -> Result<Option<Vec<ChildAgentTaskSnapshot>>, String>
where
    F: FnOnce(TaskPlanRunStreamEventDto) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let coordinator = global_child_agent_coordinator();
    let initial = coordinator
        .descendant_snapshots(session_id, &[])
        .map_err(|error| format!("读取会话 {session_id} 的 AgentTool 后代失败: {error}"))?;
    let active_descendant_session_ids = initial
        .iter()
        .filter(|task| !task.state.is_terminal())
        .map(|task| task.child_session_id.clone())
        .collect::<Vec<_>>();

    if !active_descendant_session_ids.is_empty()
        && !on_waiting(TaskPlanRunStreamEventDto::SessionWaiting {
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            active_descendant_session_ids,
        })
        .await
    {
        return Ok(None);
    }

    let outcome = coordinator
        .wait_for_descendants(session_id, &[], DESCENDANT_WAIT_TIMEOUT)
        .await
        .map_err(|error| format!("等待会话 {session_id} 的 AgentTool 后代失败: {error}"))?;
    if outcome.timed_out {
        return Err(format!(
            "等待会话 {session_id} 的 AgentTool 后代终态超时"
        ));
    }
    Ok(Some(outcome.tasks))
}

fn append_descendant_evidence(output: &mut String, tasks: &[ChildAgentTaskSnapshot]) {
    if tasks.is_empty() {
        return;
    }
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str("[AgentTool 后代终态]\n");
    for task in tasks {
        output.push_str("- session=");
        output.push_str(&task.child_session_id);
        output.push_str(" state=");
        output.push_str(descendant_state_label(task.state));
        if let Some(task_output) = task.output.as_deref().filter(|value| !value.trim().is_empty()) {
            output.push_str(" output=");
            output.push_str(&descendant_evidence_preview(task_output));
        }
        if let Some(error) = task.error.as_deref().filter(|value| !value.trim().is_empty()) {
            output.push_str(" error=");
            output.push_str(&descendant_evidence_preview(error));
        }
        output.push('\n');
    }
}

fn descendant_state_label(state: ChildAgentTaskState) -> &'static str {
    match state {
        ChildAgentTaskState::Queued => "queued",
        ChildAgentTaskState::Running => "running",
        ChildAgentTaskState::Succeeded => "succeeded",
        ChildAgentTaskState::Failed => "failed",
        ChildAgentTaskState::Cancelled => "cancelled",
    }
}

fn descendant_evidence_preview(value: &str) -> String {
    truncate_for_error(&value.split_whitespace().collect::<Vec<_>>().join(" "), 400)
}

struct PreparedLaunch {
    index: usize,
    task: PlanTaskEntry,
    task_file_display: String,
    session_title: String,
    session_request: SessionRunRequest,
}

async fn run_prepared_launch_with_retry<R>(
    task_id: &str,
    runner: &R,
    launch: PreparedLaunch,
    stage_event_tx: tokio::sync::mpsc::Sender<TaskPlanRunStreamEventDto>,
) -> (usize, PlanTaskEntry, String, Result<String, String>)
where
    R: SessionRunner + ?Sized,
{
    let session_id = launch.session_request.session_id.clone();
    let query = launch.session_request.query.clone();
    let root = launch.session_request.root.clone();
    let title = launch.session_title.clone();
    let mut retry_attempt = 0usize;

    loop {
        let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel(64);
        let session_run = runner.run_session(launch.session_request.clone(), Some(delta_tx));
        tokio::pin!(session_run);
        let result = loop {
            tokio::select! {
                result = &mut session_run => {
                    while let Ok(delta) = delta_rx.try_recv() {
                        let _ = stage_event_tx
                            .send(TaskPlanRunStreamEventDto::SessionDelta {
                                task_id: task_id.to_string(),
                                session_id: session_id.clone(),
                                delta,
                            })
                            .await;
                    }
                    break result;
                },
                delta = delta_rx.recv() => {
                    let Some(delta) = delta else {
                        break session_run.await;
                    };
                    let _ = stage_event_tx
                        .send(TaskPlanRunStreamEventDto::SessionDelta {
                            task_id: task_id.to_string(),
                            session_id: session_id.clone(),
                            delta,
                        })
                        .await;
                }
            }
        };
        let descendants =
            match wait_for_session_descendants(task_id, &session_id, |event| {
                let stage_event_tx = stage_event_tx.clone();
                async move { stage_event_tx.send(event).await.is_ok() }
            })
            .await
            {
                Ok(Some(descendants)) => descendants,
                Ok(None) => Vec::new(),
                Err(error) => {
                    return (
                        launch.index,
                        launch.task,
                        launch.task_file_display,
                        Err(error),
                    );
                }
            };
        let result = result
            .map(|mut output| {
                append_descendant_evidence(&mut output, &descendants);
                output
            })
            .map_err(|mut error| {
                append_descendant_evidence(&mut error, &descendants);
                error
            });

        match &result {
            Ok(output) => {
                let _ =
                    persist_plan_run_session_history(&session_id, &root, &title, &query, output)
                        .await;
            }
            Err(error) => {
                let _ = persist_plan_run_session_history(
                    &session_id,
                    &root,
                    &title,
                    &query,
                    &format!("子任务失败: {error}"),
                )
                .await;
            }
        }

        match result {
            Ok(output) => {
                return (
                    launch.index,
                    launch.task,
                    launch.task_file_display,
                    Ok(output),
                );
            }
            Err(error) if retry_attempt < SUBTASK_RETRY_MAX_ATTEMPTS => {
                retry_attempt += 1;
                tracing::warn!(
                    target: "vw_agent::task_plan_run",
                    task_id = %launch.task.id,
                    session_id = %session_id,
                    retry_attempt,
                    max_retries = SUBTASK_RETRY_MAX_ATTEMPTS,
                    error = %error,
                    "子任务会话失败，重新执行当前子任务"
                );
                let retry_notice = format!(
                    "\n\n[系统] 当前子任务执行失败，正在重试（{retry_attempt}/{SUBTASK_RETRY_MAX_ATTEMPTS}）：{}\n",
                    truncate_for_error(&error, 280)
                );
                let _ = stage_event_tx
                    .send(TaskPlanRunStreamEventDto::SessionDelta {
                        task_id: task_id.to_string(),
                        session_id: session_id.clone(),
                        delta: retry_notice,
                    })
                    .await;
            }
            Err(error) => {
                return (
                    launch.index,
                    launch.task,
                    launch.task_file_display,
                    Err(format!(
                        "当前子任务已额外重试 {SUBTASK_RETRY_MAX_ATTEMPTS} 次仍失败: {error}"
                    )),
                );
            }
        }
    }
}

fn emit<F>(on_event: &mut F, event: TaskPlanRunStreamEventDto) -> bool
where
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    on_event(event)
}

fn resolve_task_file(plan_abs: &Path, task: &PlanTaskEntry) -> PathBuf {
    let file = Path::new(&task.file);
    if file.is_absolute() {
        file.to_path_buf()
    } else {
        plan_abs.join(file)
    }
}

fn path_display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sanitize_session_part(value: &str) -> String {
    super::prompts::sanitize_task_plan_id_part(value)
}

fn planning_session_id(task_id: &str) -> String {
    format!("{}-plan", sanitize_session_part(task_id))
}

fn subtask_session_id(task_id: &str, subtask_id: &str) -> String {
    format!(
        "{}-{}",
        sanitize_session_part(task_id),
        sanitize_session_part(subtask_id)
    )
}

fn truncate_for_error(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "(空)".to_string();
    }
    let truncated: String = trimmed.chars().take(max_chars).collect();
    if trimmed.chars().count() > max_chars {
        format!("{truncated}…")
    } else {
        truncated
    }
}
