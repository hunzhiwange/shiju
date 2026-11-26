//! 任务执行器的 runner.rs 子模块。
//!
//! 该模块聚焦任务运行过程中的一个局部职责，供执行器入口组合调用。注释说明边界、错误传播和平台差异，避免调用方需要阅读完整执行链才能理解行为。

#[cfg(not(target_arch = "wasm32"))]
use super::backend_output::{TASK_AUTOMATION_TOOL_GUARDRAIL, build_model_prompt};
#[cfg(not(target_arch = "wasm32"))]
use super::gateway_output::GatewayVisibleOutputFilter;
#[cfg(not(target_arch = "wasm32"))]
use super::process_utils::{emit_stderr_log, emit_stdout_log};
use super::TaskLogStream;
use super::*;
use desktop_task_model::Task;

#[cfg(not(target_arch = "wasm32"))]
fn task_session_id(task_id: &str) -> String { format!("task-board-{}", task_id) }
#[cfg(not(target_arch = "wasm32"))]
use desktop_task_model::{TASK_AGENT_MAIN, normalize_task_acp_agent_input};
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
pub enum GatewayPromptSessionMode {
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
    options.insert("cwd".to_string(), json!(execution_path));
    // A named agent may resolve to ACP at the gateway even without an explicit backend.
    options.insert("acp_background".to_string(), json!(true));
    if acp_enabled {
        options.insert("acp_permission_mode".to_string(), json!("approve-reads"));
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
    event: gateway_chat_stream::GatewayChatStreamEvent,
    visible_filter: &mut GatewayVisibleOutputFilter,
    output: &mut String,
    pending: &mut String,
    stream_error: &mut Option<String>,
    done_seen: &mut bool,
    sender: Option<&Sender<TaskLogStream>>,
) -> bool {
    match event {
        gateway_chat_stream::GatewayChatStreamEvent::Delta(delta) => {
            let visible_delta = visible_filter.push(&delta);
            output.push_str(&visible_delta);
            pending.push_str(&visible_delta);
            flush_gateway_output_lines(pending, sender, false);
            true
        }
        gateway_chat_stream::GatewayChatStreamEvent::Done { finish_reason, .. } => {
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
        gateway_chat_stream::GatewayChatStreamEvent::Error(error) => {
            let visible_tail = visible_filter.finish();
            output.push_str(&visible_tail);
            pending.push_str(&visible_tail);
            flush_gateway_output_lines(pending, sender, true);
            emit_stderr_log(sender, format!("[GATEWAY] error {}", error));
            *stream_error = Some(error);
            false
        }
        gateway_chat_stream::GatewayChatStreamEvent::Other(payload) => {
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
pub fn execute_gateway_prompt_with_streaming(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    agent: Option<String>,
    acp_agent: Option<String>,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<String, String> {
    execute_gateway_prompt_with_streaming_in_mode(endpoint,
        session_id,
        execution_path,
        model,
        prompt,
        GatewayPromptRoute {
            agent,
            acp_agent,
            reasoning_effort: None,
            session_mode: GatewayPromptSessionMode::Fresh,
        },
        sender,
        None,
    )
}

#[cfg(not(target_arch = "wasm32"))]
struct GatewayPromptRoute {
    agent: Option<String>,
    acp_agent: Option<String>,
    reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    session_mode: GatewayPromptSessionMode,
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_gateway_prompt_with_streaming_in_mode(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    route: GatewayPromptRoute,
    sender: Option<&Sender<TaskLogStream>>,
    idle_timeout: Option<Duration>,
) -> Result<String, String> {
    let GatewayPromptRoute { agent, acp_agent, reasoning_effort, session_mode } = route;
    let endpoint = endpoint();
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
        gateway_chat_stream::stream_chat_blocking_with_idle_timeout(
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
        gateway_chat_stream::stream_chat_blocking(
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
    reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    session_mode: GatewayPromptSessionMode,
) -> gateway_chat_stream::GatewayChatStreamRequest {
    use vw_api_acp_model::acp_model_source::AcpModelSource;

    let acp_enabled = acp_agent.is_some();
    let native_model = model == desktop_task_model::TASK_MODEL_ACP_NATIVE;
    let cli_local = native_model && acp_enabled;
    // A CLI-local selection explicitly targets the selected ACP backend.
    let agent = if cli_local { None } else { agent };
    let mut options = gateway_prompt_options_for_mode(execution_path, acp_enabled, session_mode);
    if agent.is_some() {
        options.insert("agent".to_string(), json!(agent));
    }
    gateway_chat_stream::GatewayChatStreamRequest {
        acp_model_source: if cli_local {
            AcpModelSource::CliLocal
        } else {
            AcpModelSource::SessionModel
        },
        session_id: Some(session_id.into()),
        messages: vec![json!({ "role": "user", "content": prompt })],
        system: None,
        model: (!native_model && model != "auto").then(|| model.to_string()),
        reasoning_effort,
        agent: agent.or_else(|| acp_agent.map(|backend| format!("acp-{backend}"))),
        allowed_tools: None,
        acp_allowed_tools: None,
        options: Some(serde_json::Value::Object(options)),
    }
}

/// 公开的 execute_gateway_prompt_with_streaming 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
#[cfg(target_arch = "wasm32")]
pub fn execute_gateway_prompt_with_streaming(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
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
fn execute_task_with_selected_backend_in_session(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    task: &Task,
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    sender: Option<&Sender<TaskLogStream>>,
    session_mode: GatewayPromptSessionMode,
) -> Result<String, String> {
    execute_gateway_prompt_with_streaming_in_mode(endpoint,
        session_id,
        execution_path,
        model,
        prompt,
        GatewayPromptRoute {
            agent: resolve_task_execution_delegate_agent(task),
            acp_agent: resolve_task_execution_acp_agent(task),
            reasoning_effort: task.reasoning_effort,
            session_mode,
        },
        sender,
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

#[cfg(all(not(target_arch = "wasm32"), test))]
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

#[cfg(not(target_arch = "wasm32"))]
struct GatewayGitContext {
    project_id: vw_api_transport::ProjectId,
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_gateway_path(value: &str) -> String {
    value.replace('\\', "/").trim_end_matches('/').to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_gateway_git_context(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    project_path: &str,
    git_directory: &str,
) -> Result<GatewayGitContext, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint())?;
    let project = super::block_on_gateway(gateway_project::GatewayProjectClient::new(client.transport().clone()).project_resolve(
        &endpoint().tenant_scope().clone(),
        &vw_api_project::ResolveProjectRequest {
            directory: project_path.to_string(),
            create_if_missing: true,
        },
    ))?
    .project;

    let target_directory = normalize_gateway_path(git_directory);
    if target_directory == normalize_gateway_path(&project.directory) {
        return Ok(GatewayGitContext { project_id: project.id });
    }

    let project_scope = vw_api_transport::ProjectScope::new(endpoint().tenant_scope().clone(), project.id.clone()).map_err(|error| error.to_string())?;
    let worktree_id = super::block_on_gateway(gateway_project::GatewayProjectClient::new(client.transport().clone()).project_worktrees(&project_scope))?
        .items
        .into_iter()
        .find(|worktree| normalize_gateway_path(&worktree.directory) == target_directory)
        .map(|worktree| worktree.id);

    let Some(_) = worktree_id else {
        return Err(format!("网关未登记任务 worktree，拒绝绕过网关执行 git: {}", git_directory));
    };

    Ok(GatewayGitContext { project_id: project.id })
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
    request: &vw_api_git::GitMergeRequest,
    source_branch: &str,
    target_branch: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<vw_api_git::GitMergeResponse, String> {
    let mut first_error: Option<String> = None;
    for attempt in 1..=GATEWAY_GIT_MERGE_MAX_ATTEMPTS {
        match super::block_on_gateway(gateway_project::GatewayProjectClient::new(client.transport().clone()).git_merge(request)) {
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
pub fn execute_task_blocking_in_mode(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    execution_path: String,
    task: Task,
    _project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
    initial_session_mode: GatewayPromptSessionMode,
) -> (String, Result<String, String>) {
    let prompt = build_model_prompt(&task, &task.prompt);
    let session_id = task_session_id(&task.id);
    let sender = log_sender.as_ref();
    let result = execute_prompt_with_continue_retry_on_timeout_in_mode(
        &session_id,
        "task",
        &prompt,
        initial_session_mode,
        resolve_task_execution_acp_agent(&task).is_some(),
        sender,
        |retry_prompt, session_mode| {
            execute_task_with_selected_backend_in_session(
                endpoint, &task, &session_id, &execution_path, &task.model,
                retry_prompt, sender, session_mode,
            )
        },
    );
    (task.id, result)
}

#[cfg(not(target_arch = "wasm32"))]
struct GatewayMergeTargetLockGuard {
    client: vw_gateway_client::GatewayClient,
    project_id: vw_api_transport::ProjectId,
    task_id: String,
    target_branch: String,
    sender: Option<Sender<TaskLogStream>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for GatewayMergeTargetLockGuard {
    fn drop(&mut self) {
        let result = super::block_on_gateway(gateway_task_pool::GatewayTaskPoolClient::new(self.client.transport().clone()).task_pool_worktree_merge_unlock(
            &vw_api_task::TaskPoolWorktreeMergeUnlockRequest {
                project_id: self.project_id.clone(),
                task_id: vw_api_transport::TaskId(self.task_id.clone()),
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
fn lock_merge_target_via_gateway(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    project_path: &str,
    task: &Task,
    target_branch: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<GatewayMergeTargetLockGuard, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint())?;
    let (project, response) = super::block_on_gateway({
        let client = client.clone();
        let project_path = project_path.to_string();
        let task_id = task.id.clone();
        let target_branch = target_branch.to_string();
        async move {
            let project = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_resolve(
                    &endpoint().tenant_scope().clone(),
                    &vw_api_project::ResolveProjectRequest {
                        directory: project_path,
                        create_if_missing: false,
                    },
                )
                .await?
                .project;
            let response = gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
                .task_pool_worktree_merge_lock(
                    &vw_api_task::TaskPoolWorktreeMergeLockRequest {
                        project_id: project.id.clone(),
                        task_id: vw_api_transport::TaskId(task_id),
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

#[cfg(not(target_arch = "wasm32"))]
fn execute_task_merge_blocking(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    let sender_ref = log_sender.as_ref();
    let task_id = task.id.clone();
    let source_branch = task.merge_source_branch.clone().unwrap_or_default();
    let target_branch = task.merge_target_branch.clone().unwrap_or_default();
    let _merge_lock_guard =
        match lock_merge_target_via_gateway(endpoint, &project_path, &task, &target_branch, sender_ref) {
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
        let client = match vw_gateway_client::GatewayClient::new(endpoint()) {
            Ok(client) => client,
            Err(error) => return (task_id, Err(error)),
        };
        let context = match resolve_gateway_git_context(endpoint, &project_path, &project_path) {
            Ok(context) => context,
            Err(error) => return (task_id, Err(error)),
        };
        let request = vw_api_git::GitMergeRequest {
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
pub async fn execute_task_merge_async(endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_merge_blocking(endpoint, task, project_path, log_sender)
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
