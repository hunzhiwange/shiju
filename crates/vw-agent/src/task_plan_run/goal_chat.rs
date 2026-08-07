//! 无工作流时的轻量目标续轮：以用户查询为完成条件，不要求编号任务文件。

use super::goal::{
    GoalEvaluation, goal_evaluation_session_id, goal_turn_session_id, parse_goal_evaluation,
};
use super::runner::{SessionRunRequest, SessionRunner};
use crate::agent::prompt::multi_agent::{
    descendant_acceptance_policy, recursive_orchestration_policy,
};
use vw_api_types::task::TaskPlanRunStreamEventDto;

const MAX_EVIDENCE_CHARS: usize = 24_000;

/// 轻量目标下一动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LightweightGoalAction {
    /// 未达成，带原因续轮。
    Continue { reason: String },
    /// 已达成，结束。
    Complete { reason: String },
    /// 明确阻塞，停止续轮。
    Blocked { reason: String },
}

/// 构建无任务文件门闸的轻量目标验收提示。
pub fn build_lightweight_goal_evaluation_prompt(
    original_goal: &str,
    execution_evidence: &str,
) -> String {
    format!(
        "你是独立目标验收器，只判断已有证据，不执行工具或命令。\
\n请根据用户给出的完成条件与执行会话展示的确切命令/退出码/结果，判断目标是否真正完成。\
\n完成条件即本轮用户原始查询；请以可验证证据证明完成。没有新鲜验证证据、验证失败或约束未满足时，achieved 必须为 false。\
\n本路径是普通聊天上的独立验收，不依赖工作流计划目录或任务清单文件是否删除。\
\n\n{}\
\n只返回一个严格 JSON 对象，不要 Markdown、代码围栏或额外文字：\
\n{{\"achieved\":true或false,\"reason\":\"简洁且可执行的中文原因\"}}\
\n\n# 完成条件（用户原始查询）\n{}\
\n\n# 执行证据\n{}",
        descendant_acceptance_policy(),
        original_goal.trim(),
        truncate_chars(execution_evidence, MAX_EVIDENCE_CHARS),
    )
}

/// 构建无任务文件门闸的轻量目标续轮提示。
pub fn build_lightweight_goal_continuation_prompt(original_goal: &str, reason: &str) -> String {
    format!(
        "这是目标续轮。目标尚未通过独立验收，请继续在当前仓库自主排障、实现并验证，直到满足完成条件。\
\n不要询问用户；不要重复规划。必须运行相关验证并在回复中展示确切命令、退出码和结果。\
\n请以可验证证据证明完成。外部凭据、权限或安全风险确实无法自行解决时，在回复中精确写出 `状态：已阻塞` 并记录阻塞原因。\
\n本路径是普通聊天续轮，不创建或依赖工作流计划目录与任务清单文件。\
\n\n{}\
\n\n# 本轮未通过原因\n{}\
\n\n# 完成条件（用户原始查询）\n{}",
        recursive_orchestration_policy(),
        reason.trim(),
        original_goal.trim(),
    )
}

/// 从执行文本中识别明确阻塞（需要用户输入/外部凭据等）。
pub fn detect_explicit_lightweight_blocker(text: &str) -> Option<String> {
    let has_blocked_status = text.lines().any(|line| {
        let line = line.trim();
        line == "状态：已阻塞" || line == "状态: 已阻塞"
    });
    if !has_blocked_status {
        return None;
    }
    let reason = text.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("阻塞原因：")
            .or_else(|| line.strip_prefix("阻塞原因:"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    });
    Some(reason.unwrap_or_else(|| "目标执行明确标记为已阻塞".to_string()))
}

/// 根据验收结果与显式阻塞决定下一动作。
pub fn decide_lightweight_goal_action(
    evaluation: &GoalEvaluation,
    explicit_blocker: Option<&str>,
) -> LightweightGoalAction {
    if let Some(reason) = explicit_blocker.map(str::trim).filter(|value| !value.is_empty()) {
        return LightweightGoalAction::Blocked {
            reason: reason.to_string(),
        };
    }
    if evaluation.achieved {
        LightweightGoalAction::Complete {
            reason: evaluation.reason.clone(),
        }
    } else {
        LightweightGoalAction::Continue {
            reason: evaluation.reason.clone(),
        }
    }
}

fn emit<F>(on_event: &mut F, event: TaskPlanRunStreamEventDto) -> bool
where
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    on_event(event)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}…")
    } else if value.is_empty() {
        "(空)".to_string()
    } else {
        truncated
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

/// 无工作流目标续轮主循环：执行 → 独立验收 → 未达成则续轮；不创建计划目录。
#[allow(clippy::too_many_arguments)]
pub async fn run_lightweight_goal_chat<R, F>(
    session_key: &str,
    original_goal: &str,
    root: &str,
    model: Option<String>,
    agent: Option<String>,
    acp_agent: Option<String>,
    runner: &R,
    mut on_event: F,
) -> Result<(), String>
where
    R: SessionRunner + ?Sized,
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    let task_id = session_key.trim();
    if task_id.is_empty() {
        return Err("轻量目标续轮需要非空会话标识".to_string());
    }
    let goal = original_goal.trim();
    if goal.is_empty() {
        return Err("轻量目标续轮需要非空完成条件".to_string());
    }

    let mut turn = 1_u32;
    let mut execution_evidence;
    let mut last_infrastructure_error: Option<String> = None;
    let mut repeated_infrastructure_errors = 0_u8;

    // 首轮：以用户原始查询执行。
    let first_session_id = goal_turn_session_id(task_id, turn);
    let first_output = runner
        .run_session(
            SessionRunRequest {
                session_id: first_session_id.clone(),
                query: goal.to_string(),
                root: root.to_string(),
                model: model.clone(),
                agent: agent.clone(),
                acp_agent: acp_agent.clone(),
            },
            None,
        )
        .await;
    match first_output {
        Ok(output) => {
            execution_evidence = output;
        }
        Err(error) => {
            let infrastructure_error = format!("目标执行基础设施错误：{error}");
            if record_repeated_infrastructure_error(
                &infrastructure_error,
                &mut last_infrastructure_error,
                &mut repeated_infrastructure_errors,
            ) {
                let _ = emit(
                    &mut on_event,
                    TaskPlanRunStreamEventDto::Blocked {
                        task_id: task_id.to_string(),
                        batch: None,
                        subtask_id: None,
                        reason: format!(
                            "连续三轮出现相同基础设施错误，目标模式已停止：{infrastructure_error}"
                        ),
                        session_id: first_session_id,
                    },
                );
                return Ok(());
            }
            execution_evidence = infrastructure_error;
        }
    }

    loop {
        if let Some(blocker) = detect_explicit_lightweight_blocker(&execution_evidence) {
            let _ = emit(
                &mut on_event,
                TaskPlanRunStreamEventDto::Blocked {
                    task_id: task_id.to_string(),
                    batch: None,
                    subtask_id: None,
                    reason: blocker,
                    session_id: goal_turn_session_id(task_id, turn),
                },
            );
            return Ok(());
        }

        let evaluation_session_id = goal_evaluation_session_id(task_id, turn);
        let evaluation_prompt =
            build_lightweight_goal_evaluation_prompt(goal, &execution_evidence);
        let evaluation_result = runner
            .run_session(
                SessionRunRequest {
                    session_id: evaluation_session_id.clone(),
                    query: evaluation_prompt,
                    root: root.to_string(),
                    model: model.clone(),
                    agent: agent.clone(),
                    acp_agent: acp_agent.clone(),
                },
                None,
            )
            .await;

        let (achieved, reason) = match evaluation_result {
            Ok(output) => match parse_goal_evaluation(&output) {
                Ok(evaluation) => (evaluation.achieved, evaluation.reason),
                Err(error) => {
                    let reason = format!("目标验收基础设施错误：{error}");
                    if record_repeated_infrastructure_error(
                        &reason,
                        &mut last_infrastructure_error,
                        &mut repeated_infrastructure_errors,
                    ) {
                        let _ = emit(
                            &mut on_event,
                            TaskPlanRunStreamEventDto::Blocked {
                                task_id: task_id.to_string(),
                                batch: None,
                                subtask_id: None,
                                reason: format!(
                                    "连续三轮出现相同基础设施错误，目标模式已停止：{reason}"
                                ),
                                session_id: evaluation_session_id,
                            },
                        );
                        return Ok(());
                    }
                    (false, reason)
                }
            },
            Err(error) => {
                let reason = format!("目标验收基础设施错误：{error}");
                if record_repeated_infrastructure_error(
                    &reason,
                    &mut last_infrastructure_error,
                    &mut repeated_infrastructure_errors,
                ) {
                    let _ = emit(
                        &mut on_event,
                        TaskPlanRunStreamEventDto::Blocked {
                            task_id: task_id.to_string(),
                            batch: None,
                            subtask_id: None,
                            reason: format!(
                                "连续三轮出现相同基础设施错误，目标模式已停止：{reason}"
                            ),
                            session_id: evaluation_session_id,
                        },
                    );
                    return Ok(());
                }
                (false, reason)
            }
        };

        if !emit(
            &mut on_event,
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

        let action = decide_lightweight_goal_action(
            &GoalEvaluation {
                achieved,
                reason: reason.clone(),
            },
            None,
        );
        match action {
            LightweightGoalAction::Complete { .. } => {
                let _ = on_event(TaskPlanRunStreamEventDto::Finished {
                    task_id: task_id.to_string(),
                    // 轻量路径不创建严格工作流计划目录。
                    plan_dir: String::new(),
                    status: "completed".to_string(),
                    output: execution_evidence,
                    session_id: Some(goal_turn_session_id(task_id, turn)),
                });
                return Ok(());
            }
            LightweightGoalAction::Blocked { reason } => {
                let _ = on_event(TaskPlanRunStreamEventDto::Blocked {
                    task_id: task_id.to_string(),
                    batch: None,
                    subtask_id: None,
                    reason,
                    session_id: goal_turn_session_id(task_id, turn),
                });
                return Ok(());
            }
            LightweightGoalAction::Continue { reason } => {
                turn = turn.saturating_add(1);
                let continuation_session_id = goal_turn_session_id(task_id, turn);
                if !emit(
                    &mut on_event,
                    TaskPlanRunStreamEventDto::GoalTurnStarted {
                        task_id: task_id.to_string(),
                        turn,
                        session_id: continuation_session_id.clone(),
                        reason: reason.clone(),
                    },
                ) {
                    return Ok(());
                }

                let continuation_prompt =
                    build_lightweight_goal_continuation_prompt(goal, &reason);
                let continuation_result = runner
                    .run_session(
                        SessionRunRequest {
                            session_id: continuation_session_id,
                            query: continuation_prompt,
                            root: root.to_string(),
                            model: model.clone(),
                            agent: agent.clone(),
                            acp_agent: acp_agent.clone(),
                        },
                        None,
                    )
                    .await;
                match continuation_result {
                    Ok(output) => {
                        if !execution_evidence.is_empty() {
                            execution_evidence.push_str("\n\n");
                        }
                        execution_evidence.push_str(&output);
                    }
                    Err(error) => {
                        let infrastructure_error = format!("目标续轮基础设施错误：{error}");
                        if record_repeated_infrastructure_error(
                            &infrastructure_error,
                            &mut last_infrastructure_error,
                            &mut repeated_infrastructure_errors,
                        ) {
                            let _ = emit(
                                &mut on_event,
                                TaskPlanRunStreamEventDto::Blocked {
                                    task_id: task_id.to_string(),
                                    batch: None,
                                    subtask_id: None,
                                    reason: format!(
                                        "连续三轮出现相同基础设施错误，目标模式已停止：{infrastructure_error}"
                                    ),
                                    session_id: goal_turn_session_id(task_id, turn),
                                },
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
    }
}
