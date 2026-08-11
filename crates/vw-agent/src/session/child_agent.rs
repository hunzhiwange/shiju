//! 持久化子会话执行入口。
//!
//! 每次执行都会先创建带父会话 ID 的真实 Session，再同步等待现有 Gateway stream
//! 收集路径结束。流执行失败属于子会话终态，不会抹掉已经创建的子 Session。

use std::collections::HashSet;
use std::future::Future;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use vw_api_types::chat::GatewayChatStreamRequest;
use vw_api_types::id::SessionId;

use crate::app::agent::project;
use crate::gateway::ApiError;
use crate::gateway::api::handlers::session::stream::{
    GatewayChatStreamCollected, collect_gateway_chat_stream, persist_child_agent_turn,
};

use super::{message, session};

pub(crate) mod coordinator;

const CHILD_AGENT_HANDOFF_PROMPT: &str = "\
This prompt is already a delegated task. Do not pass the full task, or an equivalent restatement, \
to another agent. Delegate only a clearly narrower, non-overlapping subtask with a distinct deliverable. \
Your final response is a handoff to the parent agent. Adapt its detail to the task. \
For research, search, or diagnosis, include conclusions, key evidence, source URLs or file paths, \
and uncertainties. For implementation or execution, include the outcome, changed artifacts, \
validation, and blockers. Give the parent enough evidence to continue without replaying the full \
transcript, and do not dump raw tool logs or hidden reasoning.";

#[cfg(test)]
#[path = "child_agent/coordinator_tests.rs"]
mod coordinator_tests;

/// 启动一次持久化子会话所需的输入。
#[derive(Debug, Clone)]
pub struct ChildAgentRunInput {
    /// 父会话 ID。
    pub parent_session_id: String,
    /// 子会话运行工作区。
    pub directory: String,
    /// 要使用的委托 agent key。
    pub agent_key: String,
    /// 父会话当前模型的完整 `provider/model` 引用。
    pub model: Option<String>,
    /// 本轮子任务提示词。
    pub prompt: String,
    /// 提示词之前的 Gateway 格式上下文消息。
    pub context: Vec<Value>,
}

/// 子会话执行结束后的终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildAgentTerminalState {
    /// Agent stream 正常完成。
    Succeeded,
    /// Agent stream 返回错误。
    Failed,
    /// 父智能体取消了仍在运行的子任务。
    Cancelled,
}

/// 一次持久化子会话执行的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildAgentRunResult {
    /// 新创建且已持久化的子会话 ID。
    pub child_session_id: String,
    /// 子会话终态。
    pub terminal_state: ChildAgentTerminalState,
    /// 成功时供父会话消费的最终交接报告。
    pub output: Option<String>,
    /// 失败时的错误文本。
    pub error: Option<String>,
}

/// 已创建、尚未执行首轮模型请求的持久化子会话。
#[derive(Debug, Clone)]
pub(crate) struct CreatedChildAgentRun {
    input: ChildAgentRunInput,
    /// 新创建且已持久化的子会话 ID。
    pub child_session_id: String,
}

/// 返回当前会话相对根会话的委托深度。
///
/// 根会话深度为 0；每经过一个 `parent_id` 增加 1。循环父链会被显式拒绝，
/// 避免主智能体允许自委托后因损坏的会话关系绕过深度限制。
pub(crate) async fn session_depth(directory: String, session_id: String) -> Result<usize> {
    let project_directory = directory.clone();
    project::instance::provide(project_directory, None, move || {
        Box::pin(async move {
            let mut current_session_id = session_id;
            let mut visited = HashSet::new();
            let mut depth = 0usize;

            loop {
                if !visited.insert(current_session_id.clone()) {
                    bail!("child agent session parent chain contains a cycle");
                }
                let current = session::get(&current_session_id)
                    .await
                    .with_context(|| format!("load child agent session '{current_session_id}'"))?;
                let Some(parent_id) = current.parent_id.filter(|value| !value.trim().is_empty())
                else {
                    return Ok(depth);
                };
                depth = depth.checked_add(1).context("child agent session depth overflow")?;
                current_session_id = parent_id;
            }
        })
    })
    .await
    .context("initialize child agent project context")?
}

fn validate_input(input: &ChildAgentRunInput) -> Result<()> {
    if input.parent_session_id.trim().is_empty() {
        bail!("child agent parent session id must not be empty");
    }
    if input.directory.trim().is_empty() {
        bail!("child agent directory must not be empty");
    }
    if input.agent_key.trim().is_empty() {
        bail!("child agent key must not be empty");
    }
    if input.prompt.trim().is_empty() {
        bail!("child agent prompt must not be empty");
    }
    Ok(())
}

fn stream_request(
    child_session_id: &str,
    agent_key: &str,
    model: Option<String>,
    prompt: &str,
    mut context: Vec<Value>,
) -> GatewayChatStreamRequest {
    context.push(serde_json::json!({
        "role": "system",
        "content": CHILD_AGENT_HANDOFF_PROMPT,
    }));
    context.push(serde_json::json!({
        "role": "user",
        "content": prompt,
    }));
    GatewayChatStreamRequest {
        session_id: Some(SessionId::from(child_session_id)),
        messages: context,
        agent: Some(agent_key.to_string()),
        model,
        ..GatewayChatStreamRequest::default()
    }
}

fn is_think_tag_remainder(remainder: &str) -> bool {
    if remainder.is_empty() || remainder.chars().next().is_some_and(char::is_whitespace) {
        return true;
    }
    remainder
        .strip_prefix("ing")
        .is_some_and(|after| after.is_empty() || after.chars().next().is_some_and(char::is_whitespace))
}

fn find_think_open_tag(s: &str) -> Option<(usize, usize)> {
    let mut search_from = 0usize;
    while let Some(pos_rel) = s[search_from..].find("<think") {
        let pos = search_from + pos_rel;
        if s[pos..].starts_with("</think") {
            search_from = pos + 1;
            continue;
        }
        let Some(tag_end_rel) = s[pos..].find('>') else {
            return None;
        };
        let tag = &s[pos + "<think".len()..pos + tag_end_rel];
        if is_think_tag_remainder(tag) {
            return Some((pos, tag_end_rel + 1));
        }
        search_from = pos + 1;
    }
    None
}

fn find_think_close_tag(s: &str) -> Option<(usize, usize)> {
    let mut search_from = 0usize;
    while let Some(pos_rel) = s[search_from..].find("</think") {
        let pos = search_from + pos_rel;
        let Some(tag_end_rel) = s[pos..].find('>') else {
            return None;
        };
        let tag = &s[pos + "</think".len()..pos + tag_end_rel];
        if is_think_tag_remainder(tag) {
            return Some((pos, tag_end_rel + 1));
        }
        search_from = pos + 1;
    }
    None
}

fn strip_completed_think_blocks(output: &str) -> String {
    let mut visible = String::with_capacity(output.len());
    let mut cursor = 0;
    while let Some((rel_start, open_len)) = find_think_open_tag(&output[cursor..]) {
        let start = cursor + rel_start;
        let open_end = start + open_len;
        let Some((rel_close, close_len)) = find_think_close_tag(&output[open_end..]) else {
            break;
        };
        visible.push_str(&output[cursor..start]);
        cursor = open_end + rel_close + close_len;
    }
    visible.push_str(&output[cursor..]);
    visible
}

fn tool_block_end(output: &str, start: usize) -> Option<usize> {
    if start != 0 && !output[..start].ends_with('\n') {
        return None;
    }
    let tail = output.get(start..)?;
    let line_end = tail.find('\n')?;
    let tool_name = tail[..line_end].strip_prefix("tool ")?.trim();
    if tool_name.is_empty() {
        return None;
    }
    let json_start = start + line_end + 1;
    let mut values =
        serde_json::Deserializer::from_str(output.get(json_start..)?).into_iter::<Value>();
    values.next()?.ok()?;
    Some(json_start + values.byte_offset())
}

pub(crate) fn child_agent_handoff_from_output(output: &str) -> String {
    let visible = strip_completed_think_blocks(output);
    let mut segments = Vec::new();
    let mut cursor = 0;
    let mut search_from = 0;
    while let Some(relative_start) = visible[search_from..].find("tool ") {
        let start = search_from + relative_start;
        let Some(end) = tool_block_end(&visible, start) else {
            search_from = start + "tool ".len();
            continue;
        };
        let segment = visible[cursor..start].trim();
        if !segment.is_empty() {
            segments.push(segment);
        }
        cursor = end;
        search_from = end;
    }
    let final_segment = visible[cursor..].trim();
    if !final_segment.is_empty() {
        segments.push(final_segment);
    }
    segments
        .last()
        .map(|segment| (*segment).to_string())
        .unwrap_or_else(|| visible.trim().to_string())
}

async fn ensure_turn_history(
    child_session_id: &str,
    prompt: &str,
    agent_key: &str,
    output: Option<&str>,
    error: Option<&str>,
) -> Result<()> {
    let stored = message::messages(child_session_id, None)
        .await
        .context("load child agent session history")?;
    match stored.len() {
        0 => {
            let assistant_text = output.or(error).unwrap_or_default();
            persist_child_agent_turn(child_session_id, prompt, assistant_text, agent_key, error)
                .await
                .map_err(|persist_error| anyhow::anyhow!(persist_error.to_string()))?;
            Ok(())
        }
        1 => bail!("child agent session history is incomplete after stream execution"),
        _ => Ok(()),
    }
}

/// 创建并同步执行持久化子会话。
pub async fn run(input: ChildAgentRunInput) -> Result<ChildAgentRunResult> {
    let created = create(input).await?;
    run_created(created).await
}

/// 创建持久化子会话，但不启动模型执行。
pub(crate) async fn create(input: ChildAgentRunInput) -> Result<CreatedChildAgentRun> {
    validate_input(&input)?;
    let directory = input.directory.clone();
    let parent_session_id = input.parent_session_id.clone();
    let project_directory = directory.clone();
    let child = project::instance::provide(project_directory, None, move || {
        Box::pin(async move {
            session::create_next(session::CreateInput {
                parent_id: Some(parent_session_id),
                title: None,
                directory,
                permission: None,
            })
            .await
            .context("create persistent child agent session")
        })
    })
    .await
    .context("initialize child agent project context")??;

    Ok(CreatedChildAgentRun { input, child_session_id: child.id })
}

/// 同步执行一个已经创建的持久化子会话。
pub(crate) async fn run_created(created: CreatedChildAgentRun) -> Result<ChildAgentRunResult> {
    let directory = created.input.directory.clone();
    run_created_with_stream_executor(created, move |request| {
        collect_gateway_chat_stream(directory, request)
    })
    .await
}

/// 将已经创建的子会话持久化为取消终态。
pub(crate) async fn cancel_created(
    created: CreatedChildAgentRun,
    reason: impl Into<String>,
) -> Result<ChildAgentRunResult> {
    let CreatedChildAgentRun {
        input: ChildAgentRunInput {
            parent_session_id: _,
            directory,
            agent_key,
            model: _,
            prompt,
            context: _,
        },
        child_session_id,
    } = created;
    let reason = reason.into();
    let result_child_session_id = child_session_id.clone();
    let project_directory = directory.clone();
    project::instance::provide(project_directory, None, move || {
        Box::pin(async move {
            ensure_turn_history(&child_session_id, &prompt, &agent_key, None, Some(&reason))
                .await?;
            Ok::<_, anyhow::Error>(ChildAgentRunResult {
                child_session_id,
                terminal_state: ChildAgentTerminalState::Cancelled,
                output: None,
                error: Some(reason),
            })
        })
    })
    .await
    .context("initialize child agent project context")?
    .with_context(|| format!("persist cancelled child agent session '{result_child_session_id}'"))
}

/// 使用可注入的 stream 收集器创建并执行持久化子会话。
///
/// 注入点只替代非确定的 LLM stream；真实 Session 创建与消息持久化仍由本函数负责。
pub(crate) async fn run_with_stream_executor<F, Fut>(
    input: ChildAgentRunInput,
    execute_stream: F,
) -> Result<ChildAgentRunResult>
where
    F: FnOnce(GatewayChatStreamRequest) -> Fut + Send + 'static,
    Fut:
        Future<Output = std::result::Result<GatewayChatStreamCollected, ApiError>> + Send + 'static,
{
    let created = create(input).await?;
    run_created_with_stream_executor(created, execute_stream).await
}

/// 使用可注入的 stream 收集器执行一个已经创建的持久化子会话。
pub(crate) async fn run_created_with_stream_executor<F, Fut>(
    created: CreatedChildAgentRun,
    execute_stream: F,
) -> Result<ChildAgentRunResult>
where
    F: FnOnce(GatewayChatStreamRequest) -> Fut + Send + 'static,
    Fut:
        Future<Output = std::result::Result<GatewayChatStreamCollected, ApiError>> + Send + 'static,
{
    let CreatedChildAgentRun {
        input:
            ChildAgentRunInput {
                parent_session_id: _,
                directory,
                agent_key,
                model,
                prompt,
                context,
            },
        child_session_id,
    } = created;
    let project_directory = directory.clone();
    let result = project::instance::provide(project_directory, None, move || {
        Box::pin(async move {
            let request = stream_request(&child_session_id, &agent_key, model, &prompt, context);

            let result = match execute_stream(request).await {
                Ok(collected) => {
                    ensure_turn_history(
                        &child_session_id,
                        &prompt,
                        &agent_key,
                        Some(&collected.output),
                        None,
                    )
                    .await?;
                    let handoff = child_agent_handoff_from_output(&collected.output);
                    ChildAgentRunResult {
                        child_session_id,
                        terminal_state: ChildAgentTerminalState::Succeeded,
                        output: Some(handoff),
                        error: None,
                    }
                }
                Err(stream_error) => {
                    let error = stream_error.to_string();
                    ensure_turn_history(&child_session_id, &prompt, &agent_key, None, Some(&error))
                        .await?;
                    ChildAgentRunResult {
                        child_session_id,
                        terminal_state: ChildAgentTerminalState::Failed,
                        output: None,
                        error: Some(error),
                    }
                }
            };
            Ok::<_, anyhow::Error>(result)
        })
    })
    .await
    .context("initialize child agent project context")?;

    result
}
