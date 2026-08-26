//! 任务执行器的 runner.rs 子模块。
//!
//! 该模块聚焦任务运行过程中的一个局部职责，供执行器入口组合调用。注释说明边界、错误传播和平台差异，避免调用方需要阅读完整执行链才能理解行为。

#[cfg(not(target_arch = "wasm32"))]
use super::backend_output::{TASK_AUTOMATION_TOOL_GUARDRAIL, build_model_prompt};
#[cfg(not(target_arch = "wasm32"))]
use super::gateway_output::GatewayVisibleOutputFilter;
#[cfg(not(target_arch = "wasm32"))]
use super::process_utils::{emit_stderr_log, emit_stdout_log};
use super::state::TaskLogStream;
#[cfg(not(target_arch = "wasm32"))]
use super::state::{
    GIT_SOURCE_BRANCH_TAG, GIT_SUMMARY_TAG, GIT_TARGET_BRANCH_TAG, GIT_WORKTREE_PATH_TAG,
    SelectedExecutionWorkspace,
};
#[cfg(not(target_arch = "wasm32"))]
use super::worktree_admin::resolve_task_execution_workspace;
use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::task::{SubTask, TASK_AGENT_MAIN, normalize_task_acp_agent_input};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Map, Value, json};

#[cfg(not(target_arch = "wasm32"))]
const TASK_GATEWAY_CONTINUE_PROMPT: &str =
    "检查一下任务是否完成，完成了就请直接结束，否则继续执行。";
#[cfg(not(target_arch = "wasm32"))]
const TASK_GATEWAY_IDLE_TIMEOUT_SECS: u64 = 10 * 60;
/// 空窗超时后，在同一会话发送 continue 提示重试的最大次数。
///
/// 任务池里 agent 经常自行运行分钟级命令（如 cargo llvm-cov、长测试），单次重试不足以
/// 让长任务跑完；这里允许多次接续，每次都沿用同一会话，避免把仍在执行的任务误判为失败。
#[cfg(not(target_arch = "wasm32"))]
const TASK_GATEWAY_CONTINUE_RETRY_MAX_ATTEMPTS: usize = 3;
/// ACP 子进程启动失败（如二进制找不到、进程未就绪）时，用全新会话重试的最大次数。
///
/// 这类错误发生在会话建立之前，进程根本没起来，沿用同一会话发 continue 无意义；
/// 必须丢弃失败会话、重新拉起进程。启动失败常是临时竞态（二进制刚装好、路径未刷新、
/// 进程被并发清理），短暂退避后重试通常即可恢复，避免把可恢复的启动抖动误判为任务失败。
#[cfg(not(target_arch = "wasm32"))]
const TASK_GATEWAY_SPAWN_RETRY_MAX_ATTEMPTS: usize = 3;
/// 启动失败重试前的退避秒数，给进程清理与文件系统刷新留出时间。
#[cfg(not(target_arch = "wasm32"))]
const TASK_GATEWAY_SPAWN_RETRY_DELAY_SECS: u64 = 2;
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_GIT_MERGE_MAX_ATTEMPTS: usize = 10;
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_GIT_MERGE_RETRY_BASE_DELAY_SECS: u64 = 1;
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_GIT_MERGE_RETRY_MAX_DELAY_SECS: u64 = 30;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GatewayPromptSessionMode {
    Fresh,
    ContinueExisting,
}

#[cfg(not(target_arch = "wasm32"))]
fn flush_gateway_output_lines(
    pending: &mut String,
    sender: Option<&Sender<TaskLogStream>>,
    force: bool,
) {
    while let Some(pos) = pending.find('\n') {
        let line = pending[..pos].trim_end_matches('\r').to_string();
        pending.drain(..pos + 1);
        if !line.trim().is_empty() {
            emit_stdout_log(sender, line);
        }
    }

    if force {
        let line = pending.trim();
        if !line.is_empty() {
            emit_stdout_log(sender, line.to_string());
        }
        pending.clear();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_prompt_options_for_mode(
    execution_path: &str,
    acp_enabled: bool,
    session_mode: GatewayPromptSessionMode,
) -> Map<String, Value> {
    let mut options = Map::new();
    options.insert("acp_test".to_string(), json!(acp_enabled));
    options.insert("cwd".to_string(), json!(execution_path));
    options.insert("full_access".to_string(), json!(true));
    if acp_enabled {
        options.insert("acp_permission_mode".to_string(), json!("approve-all"));
        options.insert(
            "acp_force_new_session".to_string(),
            json!(session_mode == GatewayPromptSessionMode::Fresh),
        );
        options.insert("acp_close_client_after_request".to_string(), json!(true));
        if session_mode == GatewayPromptSessionMode::Fresh {
            options.insert("acp_history_strategy".to_string(), json!("discard"));
            options.insert("acp_history_recent_count".to_string(), json!(1));
        }
    }
    options
}

#[cfg(not(target_arch = "wasm32"))]
fn task_gateway_idle_timeout() -> Duration {
    Duration::from_secs(TASK_GATEWAY_IDLE_TIMEOUT_SECS)
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_gateway_stream_event(
    event: vw_gateway_client::GatewayChatStreamEvent,
    visible_filter: &mut GatewayVisibleOutputFilter,
    output: &mut String,
    pending: &mut String,
    stream_error: &mut Option<String>,
    done_seen: &mut bool,
    sender: Option<&Sender<TaskLogStream>>,
) -> bool {
    match event {
        vw_gateway_client::GatewayChatStreamEvent::Delta(delta) => {
            let visible_delta = visible_filter.push(&delta);
            output.push_str(&visible_delta);
            pending.push_str(&visible_delta);
            flush_gateway_output_lines(pending, sender, false);
            true
        }
        vw_gateway_client::GatewayChatStreamEvent::Done { finish_reason, .. } => {
            *done_seen = true;
            let visible_tail = visible_filter.finish();
            output.push_str(&visible_tail);
            pending.push_str(&visible_tail);
            flush_gateway_output_lines(pending, sender, true);
            emit_stdout_log(
                sender,
                format!(
                    "[GATEWAY] done finish_reason={}",
                    finish_reason.unwrap_or_else(|| "unknown".to_string())
                ),
            );
            true
        }
        vw_gateway_client::GatewayChatStreamEvent::Error(error) => {
            let visible_tail = visible_filter.finish();
            output.push_str(&visible_tail);
            pending.push_str(&visible_tail);
            flush_gateway_output_lines(pending, sender, true);
            emit_stderr_log(sender, format!("[GATEWAY] error {}", error));
            *stream_error = Some(error);
            false
        }
        vw_gateway_client::GatewayChatStreamEvent::Other(payload) => {
            match payload.get("type").and_then(serde_json::Value::as_str) {
                Some("chat.step_start") => {
                    let step_index = payload
                        .get("step_index")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or_default();
                    emit_stdout_log(sender, format!("[GATEWAY] step_start {}", step_index));
                }
                Some("chat.step_finish") => {
                    let step_index = payload
                        .get("step_index")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or_default();
                    emit_stdout_log(sender, format!("[GATEWAY] step_finish {}", step_index));
                }
                _ => {}
            }
            true
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn send_gateway_stream_exit(
    sender: Option<&Sender<TaskLogStream>>,
    success: bool,
    code: Option<i32>,
) {
    let _ =
        sender.map(|stream| stream.send(TaskLogStream::ExitStatus { success, code, signal: None }));
}

#[cfg(not(target_arch = "wasm32"))]
fn fail_gateway_stream(
    pending: &mut String,
    sender: Option<&Sender<TaskLogStream>>,
    error: String,
) -> Result<String, String> {
    flush_gateway_output_lines(pending, sender, true);
    send_gateway_stream_exit(sender, false, None);
    Err(error)
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_gateway_stream_done(
    done_seen: bool,
    pending: &mut String,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<(), String> {
    if done_seen {
        return Ok(());
    }
    flush_gateway_output_lines(pending, sender, true);
    send_gateway_stream_exit(sender, false, None);
    Err("gateway 流式响应未完成：连接在 chat.done 前关闭".to_string())
}

/// 公开的 execute_gateway_prompt_with_streaming 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
#[cfg(not(target_arch = "wasm32"))]
pub fn execute_gateway_prompt_with_streaming(
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    agent: Option<String>,
    acp_agent: Option<String>,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<String, String> {
    execute_gateway_prompt_with_streaming_in_mode(
        session_id,
        execution_path,
        model,
        prompt,
        agent,
        acp_agent,
        None,
        sender,
        GatewayPromptSessionMode::Fresh,
        None,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_gateway_prompt_with_streaming_in_mode(
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    agent: Option<String>,
    acp_agent: Option<String>,
    reasoning_effort: Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    sender: Option<&Sender<TaskLogStream>>,
    session_mode: GatewayPromptSessionMode,
    idle_timeout: Option<Duration>,
) -> Result<String, String> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    let acp_enabled = acp_agent.is_some();
    let agent_enabled = agent.is_some();
    let route_label = if acp_enabled {
        "acp"
    } else if agent_enabled {
        "agent"
    } else {
        "model"
    };
    let session_mode_label = match session_mode {
        GatewayPromptSessionMode::Fresh => "fresh",
        GatewayPromptSessionMode::ContinueExisting => "continue",
    };
    let selected_agent = acp_agent.clone().unwrap_or_else(|| "disabled".to_string());
    let selected_delegate_agent = agent.clone().unwrap_or_else(|| "disabled".to_string());

    emit_stdout_log(
        sender,
        format!(
            "[GATEWAY] endpoint={} session={} session_mode={} route={} agent={} acp_agent={} model={} cwd={}",
            endpoint.describe(),
            session_id,
            session_mode_label,
            route_label,
            selected_delegate_agent,
            selected_agent,
            model,
            execution_path
        ),
    );

    let mut output = String::new();
    let mut pending = String::new();
    let mut visible_filter = GatewayVisibleOutputFilter::default();
    let mut stream_error: Option<String> = None;
    let mut done_seen = false;
    let request = gateway_task_stream_request(
        session_id,
        execution_path,
        model,
        prompt,
        agent,
        acp_agent,
        reasoning_effort,
        session_mode,
    );

    let result = if let Some(idle_timeout) = idle_timeout {
        vw_gateway_client::GatewayClient::stream_chat_blocking_with_idle_timeout(
            &endpoint,
            Some(execution_path),
            &request,
            idle_timeout,
            |event| {
                handle_gateway_stream_event(
                    event,
                    &mut visible_filter,
                    &mut output,
                    &mut pending,
                    &mut stream_error,
                    &mut done_seen,
                    sender,
                )
            },
        )
    } else {
        vw_gateway_client::GatewayClient::stream_chat_blocking(
            &endpoint,
            Some(execution_path),
            &request,
            |event| {
                handle_gateway_stream_event(
                    event,
                    &mut visible_filter,
                    &mut output,
                    &mut pending,
                    &mut stream_error,
                    &mut done_seen,
                    sender,
                )
            },
        )
    };

    let visible_tail = visible_filter.finish();
    output.push_str(&visible_tail);
    pending.push_str(&visible_tail);

    if let Err(error) = result {
        return fail_gateway_stream(&mut pending, sender, format!("gateway 调度失败: {}", error));
    }

    if let Some(error) = stream_error {
        return fail_gateway_stream(&mut pending, sender, error);
    }

    ensure_gateway_stream_done(done_seen, &mut pending, sender)?;
    flush_gateway_output_lines(&mut pending, sender, true);
    send_gateway_stream_exit(sender, true, Some(0));
    Ok(output)
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_task_stream_request(
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    agent: Option<String>,
    acp_agent: Option<String>,
    reasoning_effort: Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    session_mode: GatewayPromptSessionMode,
) -> vw_gateway_client::GatewayChatStreamRequest {
    let acp_enabled = acp_agent.is_some();
    let mut options = gateway_prompt_options_for_mode(execution_path, acp_enabled, session_mode);
    if agent.is_some() {
        options.insert("agent".to_string(), json!(agent));
    }
    if acp_enabled {
        options.insert("acp_agent".to_string(), json!(acp_agent));
    }
    vw_gateway_client::GatewayChatStreamRequest {
        session_id: Some(session_id.into()),
        messages: vec![json!({ "role": "user", "content": prompt })],
        system: None,
        model: (model != "auto").then(|| model.to_string()),
        reasoning_effort: acp_enabled.then_some(reasoning_effort).flatten(),
        agent,
        allowed_tools: None,
        acp_agent: acp_enabled.then_some(acp_agent).flatten(),
        acp_allowed_tools: None,
        options: Some(serde_json::Value::Object(options)),
    }
}

/// 公开的 execute_gateway_prompt_with_streaming 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
#[cfg(target_arch = "wasm32")]
pub fn execute_gateway_prompt_with_streaming(
    _session_id: &str,
    _execution_path: &str,
    _model: &str,
    _prompt: &str,
    _agent: Option<String>,
    _acp_agent: Option<String>,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<String, String> {
    let _ = sender.map(|stream| {
        stream.send(TaskLogStream::ExitStatus { success: false, code: None, signal: None })
    });
    Err("WASM 平台暂不支持通过网关执行流式任务".to_string())
}

/// 模块内部可见的 resolve_task_execution_acp_agent 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_task_execution_acp_agent(task: &Task) -> Option<String> {
    task.acp_agent.as_deref().and_then(normalize_task_acp_agent_input)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_task_execution_delegate_agent(task: &Task) -> Option<String> {
    let acp_agent = resolve_task_execution_acp_agent(task);
    let agent = task.agent.as_deref().map(str::trim).filter(|value| !value.is_empty());
    match agent {
        Some(agent) if agent.eq_ignore_ascii_case(TASK_AGENT_MAIN) && acp_agent.is_some() => None,
        Some(agent) => Some(agent.to_string()),
        None if acp_agent.is_some() => None,
        None => Some(TASK_AGENT_MAIN.to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_task_with_selected_backend(
    task: &Task,
    execution_path: &str,
    model: &str,
    prompt: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<String, String> {
    let session_id = task_session_id(&task.id);
    execute_prompt_with_continue_retry_on_timeout(
        &session_id,
        "task",
        prompt,
        resolve_task_execution_acp_agent(task).is_some(),
        sender,
        |retry_prompt, session_mode| {
            execute_task_with_selected_backend_in_session(
                task,
                &session_id,
                execution_path,
                model,
                retry_prompt,
                sender,
                session_mode,
            )
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_task_with_selected_backend_in_session(
    task: &Task,
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    sender: Option<&Sender<TaskLogStream>>,
    session_mode: GatewayPromptSessionMode,
) -> Result<String, String> {
    execute_gateway_prompt_with_streaming_in_mode(
        session_id,
        execution_path,
        model,
        prompt,
        resolve_task_execution_delegate_agent(task),
        resolve_task_execution_acp_agent(task),
        task.reasoning_effort,
        sender,
        session_mode,
        Some(task_gateway_idle_timeout()),
    )
}

/// 判断错误是否为 ACP 子进程启动失败，需要丢弃会话、用全新会话重新拉起进程。
///
/// 典型表现是 spawn 阶段的 `No such file or directory (os error 2)`、`os error 2`、
/// `failed to spawn`、`acp command is empty` 等——进程根本没起来，会话尚未建立。
/// 注意排除 ACP 运行起来之后才可能出现的运行期错误（例如代理工具自身报告找不到某个
/// 项目文件），那类错误不应触发整会话重启。
#[cfg(not(target_arch = "wasm32"))]
fn is_gateway_spawn_retry_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("failed to spawn")
        || normalized.contains("acp command is empty")
        || normalized.contains("no such file or directory (os error 2)")
        || normalized.contains("os error 2")
}

#[cfg(not(target_arch = "wasm32"))]
fn is_gateway_continue_retry_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("模型响应超时")
        || normalized.contains("model response timeout")
        || normalized.contains("llm response timeout")
        || normalized.contains("response timeout")
        || normalized.contains("idle timeout")
        || normalized.contains("without response")
        || normalized.contains("no response")
        || normalized.contains("无响应")
        || normalized.contains("没有任何响应")
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_prompt_with_continue_retry_on_timeout(
    session_id: &str,
    retry_scope: &str,
    initial_prompt: &str,
    provider_retains_session_history: bool,
    sender: Option<&Sender<TaskLogStream>>,
    execute: impl FnMut(&str, GatewayPromptSessionMode) -> Result<String, String>,
) -> Result<String, String> {
    execute_prompt_with_continue_retry_on_timeout_in_mode(
        session_id,
        retry_scope,
        initial_prompt,
        GatewayPromptSessionMode::Fresh,
        provider_retains_session_history,
        sender,
        execute,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_prompt_with_continue_retry_on_timeout_in_mode(
    session_id: &str,
    retry_scope: &str,
    initial_prompt: &str,
    initial_session_mode: GatewayPromptSessionMode,
    provider_retains_session_history: bool,
    sender: Option<&Sender<TaskLogStream>>,
    mut execute: impl FnMut(&str, GatewayPromptSessionMode) -> Result<String, String>,
) -> Result<String, String> {
    let mut result = execute(initial_prompt, initial_session_mode);

    // ACP 子进程启动失败时，会话尚未建立，只能丢弃失败会话、用全新会话重新拉起进程。
    // 这里用初始提示与初始会话模式重试，避免把可恢复的启动抖动误判为任务失败。
    let mut spawn_attempts = 0usize;
    while spawn_attempts < TASK_GATEWAY_SPAWN_RETRY_MAX_ATTEMPTS {
        let spawn_error = match &result {
            Err(error) if is_gateway_spawn_retry_error(error) => error.clone(),
            _ => break,
        };
        spawn_attempts += 1;
        emit_stderr_log(
            sender,
            format!(
                "[TASK] ACP 进程启动失败，丢弃会话 {} 重新拉起重试 scope={} attempt={}/{} err={}",
                session_id,
                retry_scope,
                spawn_attempts,
                TASK_GATEWAY_SPAWN_RETRY_MAX_ATTEMPTS,
                spawn_error
            ),
        );
        std::thread::sleep(std::time::Duration::from_secs(TASK_GATEWAY_SPAWN_RETRY_DELAY_SECS));
        result = execute(initial_prompt, initial_session_mode);
    }

    let mut continue_attempts = 0usize;
    let continue_prompt = if provider_retains_session_history {
        TASK_GATEWAY_CONTINUE_PROMPT.to_string()
    } else {
        format!("{}\n\n## 原始任务上下文\n{}", TASK_GATEWAY_CONTINUE_PROMPT, initial_prompt.trim())
    };
    let mut last_error = match &result {
        Ok(_) => return result,
        Err(error) if is_gateway_continue_retry_error(error) => error.clone(),
        Err(_) => return result,
    };

    while continue_attempts < TASK_GATEWAY_CONTINUE_RETRY_MAX_ATTEMPTS {
        continue_attempts += 1;
        emit_stderr_log(
            sender,
            format!(
                "[TASK] gateway 响应超时，沿用会话 {} 发送“{}”重试 scope={} attempt={}/{}",
                session_id,
                TASK_GATEWAY_CONTINUE_PROMPT,
                retry_scope,
                continue_attempts,
                TASK_GATEWAY_CONTINUE_RETRY_MAX_ATTEMPTS
            ),
        );
        result = execute(&continue_prompt, GatewayPromptSessionMode::ContinueExisting);
        match &result {
            Ok(_) => return result,
            Err(error) if is_gateway_continue_retry_error(error) => last_error = error.clone(),
            Err(error) => return Err(error.clone()),
        }
    }

    Err(format!(
        "gateway 响应超时；已在同一会话发送“{}”重试 {} 次但仍失败: {}",
        TASK_GATEWAY_CONTINUE_PROMPT, continue_attempts, last_error
    ))
}

#[derive(Debug, Clone)]
pub struct TaskPlanGenerationOutcome {
    pub subtasks: Vec<TaskPlanSubTask>,
    pub raw_output: String,
    /// 确定性计划目录的绝对路径。
    pub plan_dir: String,
    /// 已解析的 `prompt.md` 绝对路径。
    pub prompt_md_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPlanSubTask {
    pub title: String,
    pub boundary: String,
    pub acceptance_criteria: Vec<String>,
    pub target_files: Vec<String>,
    /// 任务所属批次，来自 `prompt.md` 标题中的 `【批次 N】`。
    pub batch: u32,
    /// 依赖批次列表，已去重并升序排序；无依赖时为空。
    pub depends_on_batches: Vec<u32>,
    /// 来自 `prompt.md` 中紧随标题的 ```text 代码块的完整可复制执行提示。
    pub execution_prompt: String,
}

#[cfg(not(target_arch = "wasm32"))]
fn task_plan_root_dir(home: &std::path::Path) -> std::path::PathBuf {
    vw_config_types::paths::task_plan_storage_dir(home)
}

#[cfg(not(target_arch = "wasm32"))]
fn task_plan_dir(home: &std::path::Path, task_id: &str) -> std::path::PathBuf {
    let mut path = task_plan_root_dir(home);
    path.push(format!("task-{}", sanitize_task_plan_file_part(task_id)));
    path
}

#[cfg(not(target_arch = "wasm32"))]
fn task_plan_file_path(home: &std::path::Path, task_id: &str) -> std::path::PathBuf {
    let mut path = task_plan_dir(home, task_id);
    path.push("plan.md");
    path
}

#[cfg(not(target_arch = "wasm32"))]
fn sanitize_task_plan_file_part(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            output.push(ch);
        } else if ch == '.' {
            output.push('-');
        }
    }
    if output.is_empty() { "subtask".to_string() } else { output }
}

#[cfg(not(target_arch = "wasm32"))]
fn subtask_plan_file_name(index: usize, subtask: &SubTask) -> String {
    format!("{:03}-{}.md", index + 1, sanitize_task_plan_file_part(&subtask.id))
}

#[cfg(not(target_arch = "wasm32"))]
fn subtask_plan_file_path(
    home: &std::path::Path,
    task_id: &str,
    index: usize,
    subtask: &SubTask,
) -> std::path::PathBuf {
    let mut path = task_plan_dir(home, task_id);
    path.push(subtask_plan_file_name(index, subtask));
    path
}

#[cfg(not(target_arch = "wasm32"))]
fn format_task_plan_duration(ms: Option<u64>) -> String {
    let Some(ms) = ms else {
        return "-".to_string();
    };
    let total_secs = ms / 1000;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    if minutes > 0 { format!("{minutes}m{seconds}s") } else { format!("{seconds}s") }
}

#[cfg(not(target_arch = "wasm32"))]
fn format_task_plan_started_at(ms: Option<u64>) -> String {
    static FMT: once_cell::sync::Lazy<Vec<time::format_description::FormatItem<'static>>> =
        once_cell::sync::Lazy::new(|| {
            time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                .unwrap_or_default()
        });

    let Some(ms) = ms else {
        return "-".to_string();
    };
    let nanos = (ms as i128).saturating_mul(1_000_000);
    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|dt| dt.format(&FMT).ok())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn subtask_status_label(status: crate::app::task::SubTaskStatus) -> &'static str {
    match status {
        crate::app::task::SubTaskStatus::Pending => "pending",
        crate::app::task::SubTaskStatus::Running => "running",
        crate::app::task::SubTaskStatus::Completed => "completed",
        crate::app::task::SubTaskStatus::Failed => "failed",
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn format_markdown_list(items: &[String], empty: &str) -> String {
    if items.is_empty() {
        format!("- {empty}\n")
    } else {
        let mut output = String::new();
        for item in items {
            output.push_str("- ");
            output.push_str(item.trim());
            output.push('\n');
        }
        output
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_task_plan_files_in_home(home: &std::path::Path, task: &Task) -> Result<(), String> {
    let dir = task_plan_dir(home, &task.id);
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("无法创建任务计划目录 {}: {err}", dir.display()))?;

    let expected_files = task
        .subtasks
        .iter()
        .enumerate()
        .map(|(index, subtask)| subtask_plan_file_name(index, subtask))
        .collect::<std::collections::HashSet<_>>();

    let plan_path = task_plan_file_path(home, &task.id);
    std::fs::write(&plan_path, build_task_plan_markdown(task))
        .map_err(|err| format!("无法写入任务计划 {}: {err}", plan_path.display()))?;
    for (index, subtask) in task.subtasks.iter().enumerate() {
        let subtask_path = subtask_plan_file_path(home, &task.id, index, subtask);
        std::fs::write(
            &subtask_path,
            build_subtask_plan_markdown(task, subtask, index, task.subtasks.len()),
        )
        .map_err(|err| format!("无法写入子任务计划 {}: {err}", subtask_path.display()))?;
    }

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if file_name != "plan.md" && !expected_files.contains(file_name) {
                std::fs::remove_file(path).map_err(|err| err.to_string())?;
            }
        }
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_task_plan_files(_project_path: &str, task: &Task) -> Result<(), String> {
    let home = resolve_task_plan_home()?;
    write_task_plan_files_in_home(&home, task)
}

#[cfg(target_arch = "wasm32")]
pub fn write_task_plan_files(_project_path: &str, _task: &Task) -> Result<(), String> {
    Ok(())
}

/// 将子任务依赖批次格式化为计划文案，空依赖显示“无”。
#[cfg(not(target_arch = "wasm32"))]
fn format_subtask_depends_on_batches(depends_on_batches: &[u32]) -> String {
    if depends_on_batches.is_empty() {
        "无".to_string()
    } else {
        depends_on_batches.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_task_plan_markdown(task: &Task) -> String {
    let mut output = String::new();
    output.push_str(&format!("# {}\n\n", task.id));
    output.push_str(&format!("- 状态: {}\n", task.status.label()));
    output.push_str(&format!("- 描述: {}\n\n", task.description.trim()));
    output.push_str("## 原始需求\n\n");
    output.push_str(task.prompt.trim());
    output.push_str("\n\n## 子任务\n\n");
    for (index, subtask) in task.subtasks.iter().enumerate() {
        let file_name = subtask_plan_file_name(index, subtask);
        output.push_str(&format!(
            "{}. [{}] [{}]({}) {}\n",
            index + 1,
            subtask_status_label(subtask.status),
            subtask.content.trim(),
            file_name,
            subtask.boundary.trim()
        ));
        output.push_str(&format!(
            "   - 批次: {}\n   - 依赖批次: {}\n   - 目标文件: {}\n   - 开始时间: {}\n   - 执行耗时: {}\n",
            subtask.batch,
            format_subtask_depends_on_batches(&subtask.depends_on_batches),
            if subtask.target_files.is_empty() {
                "未指定".to_string()
            } else {
                subtask.target_files.join(", ")
            },
            format_task_plan_started_at(subtask.execution_started_at_ms),
            format_task_plan_duration(
                subtask.display_execution_duration_ms(crate::app::time::now_ms())
            )
        ));
    }
    output
}

#[cfg(not(target_arch = "wasm32"))]
fn build_subtask_plan_markdown(
    task: &Task,
    subtask: &SubTask,
    index: usize,
    total: usize,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("# 子任务 {:03}: {}\n\n", index + 1, subtask.content.trim()));
    output.push_str(&format!("- 任务ID: {}\n", task.id));
    output.push_str(&format!("- 子任务ID: {}\n", subtask.id));
    output.push_str(&format!("- 状态: {}\n", subtask_status_label(subtask.status)));
    output.push_str(&format!("- 顺序: {}/{}\n", index + 1, total));
    output.push_str(&format!("- 批次: {}\n", subtask.batch));
    output.push_str(&format!(
        "- 依赖批次: {}\n",
        format_subtask_depends_on_batches(&subtask.depends_on_batches)
    ));
    output.push_str(&format!(
        "- 开始时间: {}\n",
        format_task_plan_started_at(subtask.execution_started_at_ms)
    ));
    output.push_str(&format!(
        "- 执行耗时: {}\n\n",
        format_task_plan_duration(
            subtask.display_execution_duration_ms(crate::app::time::now_ms())
        )
    ));

    output.push_str("## 边界\n\n");
    let boundary = subtask.boundary.trim();
    if boundary.is_empty() {
        output.push_str("完成本子任务标题所描述的范围，不处理后续子任务。\n\n");
    } else {
        output.push_str(boundary);
        output.push_str("\n\n");
    }

    output.push_str("## 需要修改的文件\n\n");
    output.push_str(&format_markdown_list(&subtask.target_files, "未指定，由执行者按边界判断"));
    output.push_str("\n## 验收条件\n\n");
    output.push_str(&format_markdown_list(
        &subtask.acceptance_criteria,
        "完成后能用本子任务边界中的行为或检查方式验证",
    ));
    output.push_str("\n## 执行约束\n\n");
    output.push_str("- 只执行当前子任务，不开始后续子任务。\n");
    output.push_str("- 如发现必须扩大范围，先在结果中说明阻塞原因和建议边界。\n");
    output
}

/// 框架运行时 create-task-plans 规划规范的来源。
///
/// 拆分计划不依赖用户项目安装技能，直接注入打包的运行时规范。
#[cfg(not(target_arch = "wasm32"))]
const CREATE_TASK_PLANS_SKILL_SOURCE: &str =
    "crates/vw-agent/assets/runtime-skills/create-task-plans-runtime/SKILL.md";
#[cfg(not(target_arch = "wasm32"))]
const CREATE_TASK_PLANS_SKILL: &str = include_str!(
    "../../../../../vw-agent/assets/runtime-skills/create-task-plans-runtime/SKILL.md"
);

/// 框架运行时 do-task-plans 执行规范的来源。
///
/// 子任务执行不依赖用户项目安装技能，直接注入打包的运行时规范，
/// 让执行者按统一流程执行单个子任务并保留审查检查点。
#[cfg(not(target_arch = "wasm32"))]
const DO_TASK_PLANS_SKILL_SOURCE: &str =
    "crates/vw-agent/assets/runtime-skills/do-task-plans-runtime/SKILL.md";
#[cfg(not(target_arch = "wasm32"))]
const DO_TASK_PLANS_SKILL: &str =
    include_str!("../../../../../vw-agent/assets/runtime-skills/do-task-plans-runtime/SKILL.md");

/// 任务拆分使用的确定性计划目录名。
///
/// 与 create-task-plans 的 `YYYY-MM-DD-<功能名称>` 不同，这里按任务 ID 生成稳定目录名，
/// 使规划完成后执行器可以从固定位置读取 `prompt.md`，不依赖日期或功能命名。
#[cfg(not(target_arch = "wasm32"))]
fn task_split_plan_dir_name(task: &Task) -> String {
    format!("task-{}", sanitize_task_plan_file_part(&task.id))
}

/// 任务拆分确定性计划目录的绝对路径。
#[cfg(not(target_arch = "wasm32"))]
fn task_split_plan_abs_dir(home: &std::path::Path, task: &Task) -> std::path::PathBuf {
    task_plan_root_dir(home).join(task_split_plan_dir_name(task))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_task_plan_home() -> Result<std::path::PathBuf, String> {
    super::programs::user_home_dir().ok_or_else(|| {
        "无法解析用户 home：HOME 与 USERPROFILE 均未设置，无法访问任务计划存储".to_string()
    })
}

/// 读取确定性计划目录下由 create-task-plans 生成的 `prompt.md`。
///
/// 规划完成后执行器必须从此固定位置读取 `prompt.md`，缺失时返回包含路径的明确错误，
/// 不静默回退旧 JSON 解析。
#[cfg(not(target_arch = "wasm32"))]
fn read_task_plan_prompt_md(home: &std::path::Path, task: &Task) -> Result<String, String> {
    let prompt_md_path = task_split_plan_abs_dir(home, task).join("prompt.md");
    std::fs::read_to_string(&prompt_md_path)
        .map_err(|err| format!("无法读取任务拆分结果 {}: {err}", prompt_md_path.display()))
}

/// 解析 `## 任务 N: <标题> 【批次 N】 依赖批次 X,Y` 标题行，返回标题、批次与依赖批次。
///
/// 缺失批次或依赖批次非法（自依赖）时返回明确错误，不伪造默认值。
#[cfg(not(target_arch = "wasm32"))]
fn parse_task_heading(heading: &str) -> Result<(String, u32, Vec<u32>), String> {
    // 去掉 `## 任务 N:` 前缀，保留标题与批次/依赖批次文本。
    let after_marker = heading
        .trim_start()
        .trim_start_matches('#')
        .trim_start()
        .strip_prefix("任务")
        .ok_or_else(|| format!("任务标题缺少“任务”前缀: {heading}"))?;
    // 去掉任务编号与冒号：`3: 标题 ...` -> `标题 ...`。
    let rest = match after_marker.split_once(':').or_else(|| after_marker.split_once('：')) {
        Some((_, rest)) => rest.trim(),
        None => after_marker.trim(),
    };

    let batch_start =
        rest.find("【批次").ok_or_else(|| format!("任务标题缺少“【批次 N】”: {heading}"))?;
    let title = rest[..batch_start].trim().to_string();
    if title.is_empty() {
        return Err(format!("任务标题为空: {heading}"));
    }

    let batch_section = &rest[batch_start..];
    let batch_inner = batch_section
        .strip_prefix("【批次")
        .and_then(|value| value.split_once('】'))
        .map(|(inner, _)| inner.trim())
        .ok_or_else(|| format!("任务标题“【批次 N】”格式不正确: {heading}"))?;
    let batch: u32 =
        batch_inner.parse().map_err(|_| format!("无法解析批次编号“{batch_inner}”: {heading}"))?;

    let depends_text = batch_section.split_once('】').map(|(_, tail)| tail.trim()).unwrap_or("");
    let depends_on_batches = parse_depends_on_batches(depends_text, batch, heading)?;

    Ok((title, batch, depends_on_batches))
}

/// 解析 `依赖批次 X,Y` 文本：去重、升序排序，并校验不自依赖。
#[cfg(not(target_arch = "wasm32"))]
fn parse_depends_on_batches(
    depends_text: &str,
    batch: u32,
    heading: &str,
) -> Result<Vec<u32>, String> {
    let body = depends_text.trim().strip_prefix("依赖批次").map(str::trim).unwrap_or("");
    if body.is_empty() || body == "无" {
        return Ok(Vec::new());
    }

    let mut batches = Vec::new();
    for part in body.split([',', '，', ' ', '、']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let value: u32 =
            part.parse().map_err(|_| format!("无法解析依赖批次“{part}”: {heading}"))?;
        if value == batch {
            return Err(format!("依赖批次不能包含当前批次 {batch}: {heading}"));
        }
        if value > batch {
            return Err(format!("依赖批次 {value} 不能大于当前批次 {batch}: {heading}"));
        }
        if !batches.contains(&value) {
            batches.push(value);
        }
    }
    batches.sort_unstable();
    Ok(batches)
}

/// 解析 create-task-plans 生成的 `prompt.md`，把每个 `## 任务 N` 转换为带批次依赖的子任务。
///
/// 子任务顺序与 `prompt.md` 顺序一致；完整执行提示来自紧随标题的 ```text 代码块。
/// 没有任务、缺失代码块或批次/依赖非法时返回明确错误，不伪造成功子任务。
#[cfg(not(target_arch = "wasm32"))]
fn parse_create_task_plans_prompt(prompt_md: &str) -> Result<Vec<TaskPlanSubTask>, String> {
    let lines: Vec<&str> = prompt_md.lines().collect();
    let mut subtasks = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if !is_task_heading(line) {
            index += 1;
            continue;
        }

        let (title, batch, depends_on_batches) = parse_task_heading(line)?;

        // 在下一个任务标题之前寻找紧随其后的 ```text 代码块。
        let mut cursor = index + 1;
        let mut execution_prompt: Option<String> = None;
        while cursor < lines.len() {
            let candidate = lines[cursor];
            if is_task_heading(candidate) {
                break;
            }
            if candidate.trim_start().starts_with("```") {
                let mut body = String::new();
                cursor += 1;
                let mut closed = false;
                while cursor < lines.len() {
                    if lines[cursor].trim_start().starts_with("```") {
                        closed = true;
                        cursor += 1;
                        break;
                    }
                    body.push_str(lines[cursor]);
                    body.push('\n');
                    cursor += 1;
                }
                if !closed {
                    return Err(format!("任务“{title}”的代码块未闭合"));
                }
                let trimmed = body.trim();
                if trimmed.is_empty() {
                    return Err(format!("任务“{title}”的执行提示代码块为空"));
                }
                execution_prompt = Some(trimmed.to_string());
                break;
            }
            cursor += 1;
        }

        let execution_prompt =
            execution_prompt.ok_or_else(|| format!("任务“{title}”缺少执行提示代码块"))?;

        subtasks.push(TaskPlanSubTask {
            title,
            boundary: String::new(),
            acceptance_criteria: Vec::new(),
            target_files: Vec::new(),
            batch,
            depends_on_batches,
            execution_prompt,
        });

        index = cursor;
    }

    if subtasks.is_empty() {
        return Err("prompt.md 中没有任何“## 任务 N”任务".to_string());
    }

    Ok(subtasks)
}

/// 判断一行是否为 `## 任务 N` 编号任务标题（“任务”后必须紧跟数字编号）。
#[cfg(not(target_arch = "wasm32"))]
fn is_task_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return false;
    }
    let body = trimmed.trim_start_matches('#').trim_start();
    let Some(after) = body.strip_prefix("任务") else {
        return false;
    };
    after.trim_start().starts_with(|ch: char| ch.is_ascii_digit())
}

#[cfg(not(target_arch = "wasm32"))]
fn build_task_split_prompt(task: &Task, plan_dir: &std::path::Path) -> String {
    let plan_dir = plan_dir.display();
    format!(
        "请使用下面由框架运行时注入的 create-task-plans 规划规范，把这个开发需求拆分成可独立执行并跟踪的实施计划，不要直接写代码，只生成计划目录与文件。\n\n## 框架运行时规范来源\n\n- create-task-plans-runtime: {CREATE_TASK_PLANS_SKILL_SOURCE}\n- do-task-plans-runtime: {DO_TASK_PLANS_SKILL_SOURCE}\n\n## 输出要求\n\n- 计划目录固定使用绝对路径 `{plan_dir}/`，不要按日期或功能名另起目录；目录不存在时创建它。\n- 在该目录下生成 `plan.md`（整体架构指南，生成后只读）、`prompt.md`（所有任务的执行提示列表）、`tasks.json`（运行时调度事实来源）以及 `1-<任务名>.md`、`2-<任务名>.md` 等编号任务文件。\n- `tasks.json` 必须与 Markdown 文件一一对应；每个任务条目必须包含 `id`、`title`、`file`、`batch`、`depends_on_batches`、`status`。\n- 所有计划内容使用中文，并严格遵循下方技能模板的 plan.md / prompt.md / tasks.json / 任务文件结构。\n- prompt.md 必须覆盖每个编号任务，并引用对应任务文件、当前批次和依赖批次。\n\n## 批次与依赖批次\n\n- 为每个编号任务在任务文件、prompt.md 和 tasks.json 中写明`当前批次`和可为多个的`依赖批次`（无依赖时写“无”或空数组）。\n- 同一批次只允许放入彼此没有任务依赖、不会争用同一文件、且验证资源不冲突的任务；不能证明可并发时必须升高批次数，不要把不能并行的任务塞进同一批次。\n- 用批次表达线性顺序与可并行集合：后批次依赖前批次完成，同批次内的任务可并发执行。\n\n## 自动化排障约束\n\n- 这是自动化执行，用户无法实时干预；不要询问用户、不要等待澄清。先用仓库上下文、现有模块、相邻测试和搜索自行收敛假设。\n- 只有遇到外部依赖缺失、权限不足或安全风险等无法自行解决的情况，才在对应任务文件中记录阻塞原因；其余情况自行查证并继续。\n- {TASK_AUTOMATION_TOOL_GUARDRAIL}\n\n## 框架运行时 create-task-plans 规划规范\n\n{CREATE_TASK_PLANS_SKILL}\n\n## 本次需求\n\n任务ID: {}\n标题: {}\n需求:\n{}",
        task.id,
        task.description.trim(),
        task.prompt.trim()
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_task_plan_blocking(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<TaskPlanGenerationOutcome, String>) {
    let sender_ref = log_sender.as_ref();
    let home = match resolve_task_plan_home() {
        Ok(home) => home,
        Err(error) => return (task.id, Err(error)),
    };
    let plan_dir = task_split_plan_abs_dir(&home, &task);
    if let Err(error) = std::fs::create_dir_all(&plan_dir) {
        return (task.id, Err(format!("无法创建任务拆分目录 {}: {error}", plan_dir.display())));
    }
    let model = if task.model == "auto" { "auto".to_string() } else { task.model.clone() };
    let prompt = build_task_split_prompt(&task, &plan_dir);
    let session_id = format!("{}-plan", task_session_id(&task.id));
    let result = execute_prompt_with_continue_retry_on_timeout(
        &session_id,
        "plan",
        &prompt,
        resolve_task_execution_acp_agent(&task).is_some(),
        sender_ref,
        |retry_prompt, session_mode| {
            execute_gateway_prompt_with_streaming_in_mode(
                &session_id,
                &project_path,
                &model,
                retry_prompt,
                resolve_task_execution_delegate_agent(&task),
                resolve_task_execution_acp_agent(&task),
                task.reasoning_effort,
                sender_ref,
                session_mode,
                Some(task_gateway_idle_timeout()),
            )
        },
    )
    .and_then(|raw_output| {
        // 模型完成后从确定性计划目录读取 prompt.md，解析失败直接报错，不回退旧 JSON。
        let prompt_md = read_task_plan_prompt_md(&home, &task)?;
        let subtasks = parse_create_task_plans_prompt(&prompt_md)?;
        Ok(TaskPlanGenerationOutcome {
            subtasks,
            raw_output,
            plan_dir: plan_dir.to_string_lossy().to_string(),
            prompt_md_path: plan_dir.join("prompt.md").to_string_lossy().to_string(),
        })
    });
    (task.id, result)
}

pub async fn execute_task_plan_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<TaskPlanGenerationOutcome, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_plan_blocking(task, project_path, log_sender)
        })
        .await
        .unwrap_or_else(|error| (task_id, Err(format!("任务拆分线程异常: {}", error))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_path, log_sender);
        (task.id, Err("Task planning not supported on Web".to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
struct GitPrepareOutcome {
    summary: String,
    source_branch: Option<String>,
    target_branch: Option<String>,
    worktree_path: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
struct GatewayGitContext {
    project_id: vw_gateway_client::vw_api_types::id::ProjectId,
    worktree_id: Option<vw_gateway_client::vw_api_types::id::WorktreeId>,
}

#[cfg(not(target_arch = "wasm32"))]
fn format_git_summary(add: &str, commit: &str, merge: &str) -> String {
    format!("Git动作摘要: add={}; commit={}; merge={}", add, commit, merge)
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_gateway_path(value: &str) -> String {
    value.replace('\\', "/").trim_end_matches('/').to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_gateway_git_context(
    project_path: &str,
    git_directory: &str,
) -> Result<GatewayGitContext, String> {
    let client = crate::app::config::gateway_client()?;
    let project = super::block_on_gateway(client.project_resolve(
        &crate::app::session_gateway::active_tenant_scope(),
        &vw_gateway_client::vw_api_types::project::ResolveProjectRequest {
            directory: project_path.to_string(),
            create_if_missing: true,
        },
    ))?
    .project;

    let target_directory = normalize_gateway_path(git_directory);
    if target_directory == normalize_gateway_path(&project.directory) {
        return Ok(GatewayGitContext { project_id: project.id, worktree_id: None });
    }

    let project_scope = crate::app::session_gateway::active_project_scope(project.id.clone())?;
    let worktree_id = super::block_on_gateway(client.project_worktrees(&project_scope))?
        .items
        .into_iter()
        .find(|worktree| normalize_gateway_path(&worktree.directory) == target_directory)
        .map(|worktree| worktree.id);

    let Some(worktree_id) = worktree_id else {
        return Err(format!("网关未登记任务 worktree，拒绝绕过网关执行 git: {}", git_directory));
    };

    Ok(GatewayGitContext { project_id: project.id, worktree_id: Some(worktree_id) })
}

#[cfg(not(target_arch = "wasm32"))]
fn is_retryable_gateway_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    if is_gateway_merge_conflict_error(&normalized) {
        return false;
    }
    [
        "timeout",
        "timed out",
        "connection",
        "connect",
        "refused",
        "reset",
        "closed",
        "unavailable",
        "deadline",
        "temporarily",
        "index.lock",
        "unable to write index",
        "another git process",
        "超时",
        "连接",
        "断开",
        "重置",
        "不可用",
        "暂时",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

#[cfg(not(target_arch = "wasm32"))]
fn is_gateway_merge_conflict_error(normalized_error: &str) -> bool {
    [
        "automatic merge failed",
        "fix conflicts",
        "merge conflict",
        "content conflict",
        "conflict (",
        "冲突",
    ]
    .iter()
    .any(|needle| normalized_error.contains(needle))
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_git_merge_retry_delay(attempt: usize) -> std::time::Duration {
    let exponent = attempt.saturating_sub(1).min(30) as u32;
    let delay_secs = GATEWAY_GIT_MERGE_RETRY_BASE_DELAY_SECS
        .saturating_mul(2_u64.saturating_pow(exponent))
        .min(GATEWAY_GIT_MERGE_RETRY_MAX_DELAY_SECS);
    std::time::Duration::from_secs(delay_secs)
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_gateway_git_merge_with_retry(
    client: &vw_gateway_client::GatewayClient,
    request: &vw_gateway_client::vw_api_types::git::GitMergeRequest,
    source_branch: &str,
    target_branch: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<vw_gateway_client::vw_api_types::git::GitMergeResponse, String> {
    let mut first_error: Option<String> = None;
    for attempt in 1..=GATEWAY_GIT_MERGE_MAX_ATTEMPTS {
        match super::block_on_gateway(client.git_merge(request)) {
            Ok(response) => return Ok(response),
            Err(error)
                if attempt < GATEWAY_GIT_MERGE_MAX_ATTEMPTS
                    && is_retryable_gateway_error(&error) =>
            {
                if first_error.is_none() {
                    first_error = Some(error.clone());
                }
                emit_stderr_log(
                    sender,
                    format!(
                        "[GATEWAY GIT MERGE] transient error, retrying attempt={}/{} delay_secs={} source={} target={} err={}",
                        attempt,
                        GATEWAY_GIT_MERGE_MAX_ATTEMPTS,
                        gateway_git_merge_retry_delay(attempt).as_secs(),
                        source_branch,
                        target_branch,
                        error
                    ),
                );
                std::thread::sleep(gateway_git_merge_retry_delay(attempt));
            }
            Err(error) => {
                if let Some(first_error) = first_error {
                    return Err(format!(
                        "首次失败: {}; 第{}次失败: {}",
                        first_error, attempt, error
                    ));
                }
                return Err(error);
            }
        }
    }
    Err("网关合并重试耗尽".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_git_prepare_actions(
    task: &Task,
    project_path: &str,
    workspace: &SelectedExecutionWorkspace,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<GitPrepareOutcome, String> {
    emit_stdout_log(sender, "[GATEWAY GIT] 请求网关执行 add/commit 自动流程".to_string());
    let context = resolve_gateway_git_context(project_path, &workspace.execution_path)?;
    let client = crate::app::config::gateway_client()?;
    let commit_message = build_task_commit_message(task, &format!("task({})", task.id));
    let response = super::block_on_gateway(client.git_commit(
        &vw_gateway_client::vw_api_types::git::GitCommitRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            message: commit_message.clone(),
            stage_all: true,
            selected_files: Vec::new(),
            selected_hunks: Vec::new(),
            selected_lines: Vec::new(),
            selected_old_lines: Vec::new(),
        },
    ))?;
    if !response.ok {
        return Err("网关未确认 git commit 成功".to_string());
    }
    if response.created {
        emit_stdout_log(
            sender,
            format!(
                "[GATEWAY GIT] commit 成功 sha={} message={}",
                response.commit.sha, response.commit.message
            ),
        );
    } else {
        emit_stdout_log(sender, "[GATEWAY GIT] 无变更，跳过 commit".to_string());
    }
    let add_status = "成功(网关)".to_string();
    let commit_status = if response.created {
        format!("成功({})", commit_message)
    } else {
        "跳过(无变更)".to_string()
    };
    let merge_status = "跳过(等待审核后再合并)".to_string();

    Ok(GitPrepareOutcome {
        summary: format_git_summary(&add_status, &commit_status, &merge_status),
        source_branch: workspace.selected_worktree_branch.clone(),
        target_branch: workspace.merge_target_branch.clone(),
        worktree_path: workspace.selected_worktree_path.clone(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn read_plan_context(
    home: &std::path::Path,
    task: &Task,
    subtask: &SubTask,
    index: usize,
    total: usize,
) -> (String, String) {
    let plan = std::fs::read_to_string(task_plan_file_path(home, &task.id))
        .unwrap_or_else(|_| build_task_plan_markdown(task));
    let subtask_plan =
        std::fs::read_to_string(subtask_plan_file_path(home, &task.id, index, subtask))
            .unwrap_or_else(|_| build_subtask_plan_markdown(task, subtask, index, total));
    (plan, subtask_plan)
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_subtask_status(
    home: &std::path::Path,
    task: &mut Task,
    subtask_id: &str,
    update: impl FnOnce(&mut SubTask),
    sender: Option<&Sender<TaskLogStream>>,
) {
    if let Some(subtask) = task.subtasks.iter_mut().find(|item| item.id == subtask_id) {
        update(subtask);
        if let Err(error) = write_task_plan_files_in_home(home, task) {
            emit_stderr_log(sender, format!("[PLAN] 子任务状态写入失败: {}", error));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_subtask_execution_prompt(
    task: &Task,
    subtask: &SubTask,
    plan_markdown: &str,
    subtask_markdown: &str,
) -> String {
    let depends_on = format_subtask_depends_on_batches(&subtask.depends_on_batches);
    format!(
        "请按下面由框架运行时注入的 do-task-plans 执行流程，单独执行当前子任务.md 中定义的子任务。不要开始后续子任务，也不要重新拆分任务。\
\n\n## 任务池自动化执行规则\
\n\n- 这是自动化执行，用户无法实时干预；不要询问用户、不要等待澄清。先用仓库上下文、现有模块、相邻测试和搜索自行查证、自行排障并继续。\
\n- 能自行解决的小问题就直接解决，不要因为可以自行排障的小障碍立即停下来询问用户。\
\n- 只有遇到外部凭据缺失、权限不足、安全风险、破坏性操作或连续验证始终无法推进时，才在对应编号任务文件中标记 `已阻塞` 并记录具体原因和已运行的检查；不要把阻塞原因写进 plan.md。\
\n- `plan.md` 为只读整体架构指南，任何情况下都不要修改、清理或往其中记录进度。\
\n- 子任务获得所需验证证据后，在最终回复中报告完成证据；被阻塞时只在对应编号任务文件中更新为 `已阻塞`。\
\n- 本流程由框架运行时直接注入，无需安装 do-task-plans 技能（不依赖用户项目安装该技能）即可执行。\
\n- {}\
\n\n## 当前批次上下文\
\n\n- 当前批次: {}\
\n- 依赖批次: {}\
\n- 同批次内的子任务可并发执行；依赖批次必须全部完成后，当前批次才会开始。\
\n\n## 框架运行时 do-task-plans 执行规范\
\n\n来源: {}\
\n\n{}\
\n\n# plan.md\n{}\
\n\n# 子任务.md\n{}\
\n\n# 原始需求\n{}",
        TASK_AUTOMATION_TOOL_GUARDRAIL,
        subtask.batch,
        depends_on,
        DO_TASK_PLANS_SKILL_SOURCE,
        DO_TASK_PLANS_SKILL.trim(),
        plan_markdown.trim(),
        subtask_markdown.trim(),
        task.prompt.trim()
    )
}

/// 子任务执行会话 ID 规则：基于任务 ID 与子任务 ID 生成确定性会话。
///
/// 同批子任务并发执行时各自需要独立会话，复用任务级会话会导致并发请求互相污染
/// 历史与状态。这里以任务级会话作前缀，再追加经过净化的子任务 ID，保证：同任务的不
/// 同子任务得到不同会话；同一子任务始终得到同一会话；ID 中的不安全字符不会破坏会话名。
#[cfg(not(target_arch = "wasm32"))]
fn subtask_session_id(task_id: &str, subtask_id: &str) -> String {
    format!("{}-sub-{}", task_session_id(task_id), sanitize_task_plan_file_part(subtask_id))
}

/// 按批次构造并发执行阶段，返回每个阶段包含的子任务下标列表（按批次号升序）。
///
/// 调度语义：
/// - 同一批次内的子任务彼此无依赖、可并发执行，归入同一个阶段。
/// - 后批次依赖前批次完成；依赖批次必须严格小于当前批次（升序推进天然无环）。
///
/// 拒绝无法证明可安全并发的拓扑并返回明确错误：
/// - 空子任务集合；
/// - 自依赖或前向依赖（依赖的批次号 >= 当前批次号）；
/// - 依赖了不存在的批次；
/// - 同批次内显式声明了同一个目标文件（会争用文件，不能并发）。
#[cfg(not(target_arch = "wasm32"))]
fn build_batch_execution_stages(subtasks: &[SubTask]) -> Result<Vec<Vec<usize>>, String> {
    if subtasks.is_empty() {
        return Err("批次调度失败: 子任务集合为空".to_string());
    }

    let existing_batches: std::collections::BTreeSet<u32> =
        subtasks.iter().map(|subtask| subtask.batch).collect();

    let mut stages: std::collections::BTreeMap<u32, Vec<usize>> = std::collections::BTreeMap::new();
    for (index, subtask) in subtasks.iter().enumerate() {
        let batch = subtask.batch;
        for dependency in &subtask.depends_on_batches {
            if *dependency >= batch {
                return Err(format!(
                    "批次调度失败: 子任务 {} 的批次 {} 依赖了非更早批次 {}（不允许自依赖或前向依赖）",
                    subtask.id, batch, dependency
                ));
            }
            if !existing_batches.contains(dependency) {
                return Err(format!(
                    "批次调度失败: 子任务 {} 依赖了不存在的批次 {}",
                    subtask.id, dependency
                ));
            }
        }
        stages.entry(batch).or_default().push(index);
    }

    for indices in stages.values() {
        let mut seen_files: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for &index in indices {
            for file in &subtasks[index].target_files {
                let normalized = file.trim();
                if normalized.is_empty() {
                    continue;
                }
                if !seen_files.insert(normalized) {
                    return Err(format!(
                        "批次调度失败: 批次 {} 内多个子任务争用同一目标文件 {}，无法并发",
                        subtasks[index].batch, normalized
                    ));
                }
            }
        }
    }

    Ok(stages.into_values().collect())
}

/// 在独立子任务会话中执行单个子任务的网关调用，仅做执行与重试，不触碰共享计划文件。
///
/// 该函数被同批并发线程各自调用：每个子任务拥有独立会话（见 [`subtask_session_id`]），
/// 因此可以安全并发，不会复用同一个网关会话。计划文件持久化与日志事件由调度方在主线程
/// 串行处理，避免并发写入同一份 `plan.md`。
#[cfg(not(target_arch = "wasm32"))]
fn execute_single_subtask(
    task: &Task,
    subtask: &SubTask,
    execution_path: &str,
    model: &str,
    plan_markdown: &str,
    subtask_markdown: &str,
    initial_session_mode: GatewayPromptSessionMode,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<String, String> {
    let subtask_session = subtask_session_id(&task.id, &subtask.id);
    let subtask_prompt =
        build_subtask_execution_prompt(task, subtask, plan_markdown, subtask_markdown);
    let model_prompt = build_model_prompt(task, &subtask_prompt);
    let retry_scope = format!("subtask={}", subtask.id);
    execute_prompt_with_continue_retry_on_timeout_in_mode(
        &subtask_session,
        &retry_scope,
        &model_prompt,
        initial_session_mode,
        resolve_task_execution_acp_agent(task).is_some(),
        sender,
        |prompt, session_mode| {
            execute_task_with_selected_backend_in_session(
                task,
                &subtask_session,
                execution_path,
                model,
                prompt,
                sender,
                session_mode,
            )
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_task_blocking_in_mode(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
    initial_session_mode: GatewayPromptSessionMode,
) -> (String, Result<String, String>) {
    let sender_ref = log_sender.as_ref();
    let home = match resolve_task_plan_home() {
        Ok(home) => home,
        Err(error) => return (task.id, Err(error)),
    };
    let (workspace, _claim_guard) =
        match resolve_task_execution_workspace(&task, &project_path, sender_ref) {
            Ok(result) => result,
            Err(error) => return (task.id, Err(error)),
        };

    let model = if task.model == "auto" { "auto".to_string() } else { task.model.clone() };
    let subtasks = if task.subtasks.is_empty() {
        vec![SubTask::new(task.prompt.clone())]
    } else {
        task.subtasks.clone()
    };
    let mut task_snapshot = task.clone();
    if task_snapshot.subtasks.is_empty() {
        task_snapshot.subtasks = subtasks.clone();
    }

    let stages = match build_batch_execution_stages(&subtasks) {
        Ok(stages) => stages,
        Err(error) => return (task.id, Err(error)),
    };

    let total = subtasks.len();
    let mut execution_output = String::new();
    let mut execution_result: Result<String, String> = Ok(String::new());

    'stages: for stage in stages {
        // 阶段启动：先在主线程串行登记开始状态、发送 SubTaskStarted，并读取计划上下文，
        // 避免并发线程争用同一份 plan.md。
        let mut launches: Vec<(usize, String, String)> = Vec::with_capacity(stage.len());
        for &index in &stage {
            let subtask = &subtasks[index];
            let started_at_ms = crate::app::time::now_ms();
            persist_subtask_status(
                &home,
                &mut task_snapshot,
                &subtask.id,
                |subtask| subtask.start_execution_at(started_at_ms),
                sender_ref,
            );
            if let Some(sender) = sender_ref {
                let _ = sender.send(TaskLogStream::SubTaskStarted {
                    subtask_id: subtask.id.clone(),
                    content: subtask.content.clone(),
                    timestamp_ms: started_at_ms,
                });
            }
            let (plan_markdown, subtask_markdown) =
                read_plan_context(&home, &task_snapshot, subtask, index, total);
            launches.push((index, plan_markdown, subtask_markdown));
        }

        // 同阶段（同批次）的子任务并发执行：每个子任务用独立会话，互不复用。
        // 线程只做网关调用，不写计划文件；结果回收后再在主线程串行持久化。
        let results: Vec<(usize, Result<String, String>)> = std::thread::scope(|scope| {
            let handles = launches
                .into_iter()
                .map(|(index, plan_markdown, subtask_markdown)| {
                    let task = &task;
                    let subtask = &subtasks[index];
                    let execution_path = workspace.execution_path.as_str();
                    let model = model.as_str();
                    let thread_sender = log_sender.clone();
                    scope.spawn(move || {
                        let result = execute_single_subtask(
                            task,
                            subtask,
                            execution_path,
                            model,
                            &plan_markdown,
                            &subtask_markdown,
                            initial_session_mode,
                            thread_sender.as_ref(),
                        );
                        (index, result)
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| {
                    handle.join().unwrap_or((usize::MAX, Err("子任务线程异常".to_string())))
                })
                .collect()
        });

        // 主线程串行处理结果：保留阶段内启动顺序，便于日志稳定。
        let mut ordered = results;
        ordered.sort_by_key(|(index, _)| *index);
        let mut stage_failed: Option<String> = None;
        for (index, result) in ordered {
            // 线程 join 失败时 index 为 usize::MAX，没有对应子任务，直接记为阶段失败。
            let Some(subtask) = subtasks.get(index) else {
                stage_failed.get_or_insert_with(|| "子任务线程异常".to_string());
                continue;
            };
            match result {
                Ok(output) => {
                    let completed_at_ms = crate::app::time::now_ms();
                    persist_subtask_status(
                        &home,
                        &mut task_snapshot,
                        &subtask.id,
                        |subtask| subtask.mark_completed_at(completed_at_ms),
                        sender_ref,
                    );
                    if let Some(sender) = sender_ref {
                        let _ = sender.send(TaskLogStream::SubTaskCompleted {
                            subtask_id: subtask.id.clone(),
                            timestamp_ms: completed_at_ms,
                        });
                    }
                    if !execution_output.is_empty() {
                        execution_output.push_str("\n\n");
                    }
                    execution_output.push_str(&output);
                }
                Err(error) => {
                    let failed_at_ms = crate::app::time::now_ms();
                    persist_subtask_status(
                        &home,
                        &mut task_snapshot,
                        &subtask.id,
                        |subtask| subtask.mark_failed_at(failed_at_ms),
                        sender_ref,
                    );
                    if let Some(sender) = sender_ref {
                        let _ = sender.send(TaskLogStream::SubTaskFailed {
                            subtask_id: subtask.id.clone(),
                            error: error.clone(),
                            timestamp_ms: failed_at_ms,
                        });
                    }
                    stage_failed.get_or_insert(error);
                }
            }
        }

        // 阶段内任一子任务失败：保留已完成子任务状态，停止启动依赖它的后续批次。
        if let Some(error) = stage_failed {
            execution_result = Err(error);
            break 'stages;
        }
    }
    if execution_result.is_ok() {
        execution_result = Ok(execution_output);
    }

    let final_result = match execution_result {
        Ok(exec) => match execute_git_prepare_actions(&task, &project_path, &workspace, sender_ref)
        {
            Ok(git) => {
                let mut output = format!("{}\n{}:{}", exec, GIT_SUMMARY_TAG, git.summary);
                if let Some(source) = git.source_branch {
                    output.push_str(&format!("\n{}:{}", GIT_SOURCE_BRANCH_TAG, source));
                }
                if let Some(target) = git.target_branch {
                    output.push_str(&format!("\n{}:{}", GIT_TARGET_BRANCH_TAG, target));
                }
                if let Some(worktree_path) = git.worktree_path {
                    output.push_str(&format!("\n{}:{}", GIT_WORKTREE_PATH_TAG, worktree_path));
                }
                Ok(output)
            }
            Err(git_err) => Err(format!("任务执行成功，但 Git 流程失败: {}", git_err)),
        },
        Err(exec_err) => {
            emit_stdout_log(
                sender_ref,
                format!("[GATEWAY GIT] 跳过 add/commit/merge，原因=执行失败: {}", exec_err),
            );
            Err(exec_err)
        }
    };

    (task.id, final_result)
}

/// 公开的 execute_task_async 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub async fn execute_task_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_blocking_in_mode(
                task,
                project_path,
                log_sender,
                GatewayPromptSessionMode::Fresh,
            )
        })
        .await
        .unwrap_or_else(|error| (task_id, Err(format!("任务执行线程异常: {}", error))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_path, log_sender);
        (task.id, Err("Task execution not supported on Web".to_string()))
    }
}

/// 公开的 execute_task_retry_async 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub async fn execute_task_retry_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_blocking_in_mode(
                task,
                project_path,
                log_sender,
                GatewayPromptSessionMode::ContinueExisting,
            )
        })
        .await
        .unwrap_or_else(|error| (task_id, Err(format!("任务执行线程异常: {}", error))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_path, log_sender);
        (task.id, Err("Task execution not supported on Web".to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_task_review_blocking(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    let sender_ref = log_sender.as_ref();
    let model = if task.model == "auto" { "auto".to_string() } else { task.model.clone() };
    let model_prompt = build_model_prompt(&task, &task.prompt);
    let execution_result =
        execute_task_with_selected_backend(&task, &project_path, &model, &model_prompt, sender_ref);
    (task.id, execution_result)
}

#[cfg(not(target_arch = "wasm32"))]
struct GatewayMergeTargetLockGuard {
    client: vw_gateway_client::GatewayClient,
    project_id: vw_gateway_client::vw_api_types::id::ProjectId,
    task_id: String,
    target_branch: String,
    sender: Option<Sender<TaskLogStream>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for GatewayMergeTargetLockGuard {
    fn drop(&mut self) {
        let result = super::block_on_gateway(self.client.task_pool_worktree_merge_unlock(
            &vw_gateway_client::vw_api_types::task::TaskPoolWorktreeMergeUnlockRequest {
                project_id: self.project_id.clone(),
                task_id: vw_gateway_client::vw_api_types::id::TaskId(self.task_id.clone()),
                target_branch: self.target_branch.clone(),
            },
        ));
        match result {
            Ok(response) if response.released => emit_stdout_log(
                self.sender.as_ref(),
                format!(
                    "[GATEWAY GIT MERGE] lock released task_id={} target={}",
                    self.task_id, self.target_branch
                ),
            ),
            Ok(_) => emit_stderr_log(
                self.sender.as_ref(),
                format!(
                    "[GATEWAY GIT MERGE] lock release not confirmed task_id={} target={}",
                    self.task_id, self.target_branch
                ),
            ),
            Err(error) => emit_stderr_log(
                self.sender.as_ref(),
                format!(
                    "[GATEWAY GIT MERGE] lock release failed task_id={} target={} err={}",
                    self.task_id, self.target_branch, error
                ),
            ),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn lock_merge_target_via_gateway(
    project_path: &str,
    task: &Task,
    target_branch: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<GatewayMergeTargetLockGuard, String> {
    let client = crate::app::config::gateway_client()?;
    let (project, response) = super::block_on_gateway({
        let client = client.clone();
        let project_path = project_path.to_string();
        let task_id = task.id.clone();
        let target_branch = target_branch.to_string();
        async move {
            let project = client
                .project_resolve(
                    &crate::app::session_gateway::active_tenant_scope(),
                    &vw_gateway_client::vw_api_types::project::ResolveProjectRequest {
                        directory: project_path,
                        create_if_missing: false,
                    },
                )
                .await?
                .project;
            let response = client
                .task_pool_worktree_merge_lock(
                    &vw_gateway_client::vw_api_types::task::TaskPoolWorktreeMergeLockRequest {
                        project_id: project.id.clone(),
                        task_id: vw_gateway_client::vw_api_types::id::TaskId(task_id),
                        target_branch,
                    },
                )
                .await?;
            Ok::<_, String>((project, response))
        }
    })?;
    if !response.locked {
        let holder =
            response.holder_task_id.map(|holder| holder.0).unwrap_or_else(|| "unknown".to_string());
        return Err(format!("合并锁被占用: target={} holder={}", target_branch, holder));
    }
    emit_stdout_log(
        sender,
        format!("[GATEWAY GIT MERGE] lock acquired task_id={} target={}", task.id, target_branch),
    );
    Ok(GatewayMergeTargetLockGuard {
        client,
        project_id: project.id,
        task_id: task.id.clone(),
        target_branch: target_branch.to_string(),
        sender: sender.cloned(),
    })
}

/// 公开的 execute_task_review_async 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub async fn execute_task_review_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_review_blocking(task, project_path, log_sender)
        })
        .await
        .unwrap_or_else(|error| (task_id, Err(format!("代码审核线程异常: {}", error))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_path, log_sender);
        (task.id, Err("Code review not supported on Web".to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_task_merge_blocking(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    let sender_ref = log_sender.as_ref();
    let task_id = task.id.clone();
    let source_branch = task.merge_source_branch.clone().unwrap_or_default();
    let target_branch = task.merge_target_branch.clone().unwrap_or_default();
    let _merge_lock_guard =
        match lock_merge_target_via_gateway(&project_path, &task, &target_branch, sender_ref) {
            Ok(guard) => guard,
            Err(error) => return (task_id, Err(error)),
        };
    emit_stdout_log(
        sender_ref,
        format!(
            "[GATEWAY GIT MERGE] start task={} {} -> {}",
            task.id, source_branch, target_branch
        ),
    );

    let merge_result = if source_branch.trim().is_empty() || target_branch.trim().is_empty() {
        emit_stdout_log(sender_ref, "[GATEWAY GIT MERGE] 未发现可合并分支，跳过".to_string());
        Ok("合并跳过: 无可合并分支".to_string())
    } else if source_branch == target_branch || source_branch == "HEAD" {
        emit_stdout_log(
            sender_ref,
            format!(
                "[GATEWAY GIT MERGE] 合并分支无效，跳过 source={} target={}",
                source_branch, target_branch
            ),
        );
        Ok(format!("合并跳过: source={} target={}", source_branch, target_branch))
    } else {
        let client = match crate::app::config::gateway_client() {
            Ok(client) => client,
            Err(error) => return (task_id, Err(error)),
        };
        let context = match resolve_gateway_git_context(&project_path, &project_path) {
            Ok(context) => context,
            Err(error) => return (task_id, Err(error)),
        };
        let request = vw_gateway_client::vw_api_types::git::GitMergeRequest {
            project_id: context.project_id,
            source_branch: source_branch.clone(),
            target_branch: target_branch.clone(),
        };
        let response = execute_gateway_git_merge_with_retry(
            &client,
            &request,
            &source_branch,
            &target_branch,
            sender_ref,
        );
        match response {
            Ok(response) if response.ok && response.already_merged => {
                emit_stdout_log(
                    sender_ref,
                    format!(
                        "[GATEWAY GIT MERGE] source 已在 target 中，跳过 source={} target={} workspace={}",
                        response.source_branch, response.target_branch, response.workspace
                    ),
                );
                Ok(format!(
                    "合并跳过: source={} 已包含于 target={} workspace={}",
                    response.source_branch, response.target_branch, response.workspace
                ))
            }
            Ok(response) if response.ok => {
                emit_stdout_log(
                    sender_ref,
                    format!(
                        "[GATEWAY GIT MERGE] merge 成功 workspace={} source={} target={}",
                        response.workspace, response.source_branch, response.target_branch
                    ),
                );
                Ok(format!(
                    "Git动作摘要: add=跳过; commit=跳过; merge=成功({}->{}) workspace={}",
                    response.source_branch, response.target_branch, response.workspace
                ))
            }
            Ok(response) => Err(format!(
                "网关未确认 merge 成功 source={} target={} workspace={} already_merged={} message={}",
                response.source_branch,
                response.target_branch,
                response.workspace,
                response.already_merged,
                response.message
            )),
            Err(error) => Err(format!(
                "网关合并失败 source={} target={} worktree={} err={}",
                source_branch,
                target_branch,
                task.selected_worktree_path.as_deref().unwrap_or("none"),
                error
            )),
        }
    };

    if let Err(error) = &merge_result {
        emit_stderr_log(
            sender_ref,
            format!(
                "[GATEWAY GIT MERGE] completed task_id={} source={} target={} result=error err={}",
                task_id, source_branch, target_branch, error
            ),
        );
    }

    (task_id, merge_result)
}

/// 公开的 execute_task_merge_async 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub async fn execute_task_merge_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_merge_blocking(task, project_path, log_sender)
        })
        .await
        .unwrap_or_else(|error| (task_id, Err(format!("代码合并线程异常: {}", error))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_path, log_sender);
        (task.id, Err("Task merge not supported on Web".to_string()))
    }
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;
