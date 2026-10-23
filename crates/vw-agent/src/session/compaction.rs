//! 会话压缩（Compaction）模块
//!
//! 本模块提供了会话历史消息的压缩和修剪功能，用于在会话上下文接近模型限制时
//! 自动清理旧消息，释放上下文空间，确保对话能够持续进行。
//!
//! # 核心功能
//!
//! - **溢出检测**：判断当前会话是否接近上下文限制，需要执行压缩
//! - **消息修剪**：移除旧的、已完成的工具调用输出，减少上下文占用
//! - **摘要生成**：分块覆盖全部待替换的文本与工具记录，生成会话摘要
//! - **压缩请求创建**：创建新的压缩请求消息，触发压缩流程
//!
//! # 压缩策略
//!
//! 1. 当会话使用率达到 70% 的可用上下文时，触发溢出检测
//! 2. 修剪时会保护最近 2 轮对话和所有 skill 工具调用
//! 3. 只有当可修剪的 token 数量超过最小阈值时才执行实际修剪
//! 4. 压缩后会发布 `session.compacted` 事件通知相关组件

use crate::app::agent::bus;
use crate::app::agent::config;
use crate::app::agent::id;
use crate::app::agent::project::instance;
use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use crate::app::agent::skills::SkillListingSessionState;
use crate::app::agent::util::token;
use crate::session::processor::context_governance;
use serde_json::json;
use std::collections::HashMap;

/// 压缩相关事件定义
pub mod event {
    use crate::app::agent::bus;

    /// 会话已压缩事件
    ///
    /// 当会话完成压缩操作后发布此事件，携带会话 ID 信息
    pub const COMPACTED: bus::Definition = bus::Definition { r#type: "session.compacted" };
}

/// 检测会话是否溢出上下文限制
///
/// 根据当前的 token 使用情况和模型的上下文限制，判断是否需要执行压缩。
/// 当使用率达到可用上下文的 90% 时返回 true。
///
/// # 参数
///
/// - `tokens`: 当前的 token 使用信息，包含输入、输出、缓存读写的 token 数
/// - `model`: 当前使用的模型配置，包含上下文和输出限制
///
/// # 返回值
///
/// - `true`: 会话已接近上下文限制，需要执行压缩
/// - `false`: 会话仍在安全范围内，无需压缩
///
/// # 算法说明
///
/// 1. 计算可用的上下文空间：优先使用输入限制，否则从上下文限制扣除输出限制
/// 2. 计算目标阈值：可用空间的 90%
/// 3. 比较当前使用量与目标阈值
///
/// # 示例
///
/// ```ignore
/// let tokens = message::TokenInfo {
///     total: None,
///     input: 50000,
///     output: 10000,
///     reasoning: 0,
///     cache: message::TokenCacheInfo { read: 0, write: 0 },
/// };
/// let model = provider::Model { /* ... */ };
///
/// if is_overflow(&tokens, &model).await {
///     // 执行压缩操作
/// }
/// ```
pub async fn is_overflow(tokens: &message::TokenInfo, model: &provider::Model) -> bool {
    let _cfg = config::get().await;
    if model.limit.context == 0
        || context_governance::effective_input_window(&model.limit) == 0
        || model
            .limit
            .input
            .is_some_and(|input| input > model.limit.context.saturating_sub(model.limit.output))
    {
        return false;
    }
    let count = tokens.total.unwrap_or_else(|| {
        tokens
            .input
            .saturating_add(tokens.output)
            .saturating_add(tokens.cache.read)
            .saturating_add(tokens.cache.write)
    });
    context_governance::should_compact(count.max(0) as u64, &model.limit)
}

/// 修剪最小阈值
///
/// 只有当可修剪的 token 数量超过此值时才执行实际修剪操作
pub const PRUNE_MINIMUM: u64 = 20_000;

/// 修剪保护阈值
///
/// 当累计的可修剪 token 数量超过此值时才开始记录待修剪项
pub const PRUNE_PROTECT: u64 = 40_000;

/// 在压缩完成后请求 AGENTS 重载，并重新建立所有已调用技能的临时上下文。
///
/// 实际运行时负责把返回的上下文更新应用到该轮模型请求；此接口保持重放顺序可测试。
pub(crate) fn restore_invoked_skills_after_compaction(
    llm_messages: &mut Vec<serde_json::Value>,
    skills: &SkillListingSessionState,
    agent_id: &str,
    reload_agents: impl FnOnce(),
) -> Result<Vec<context_governance::InvokedSkillContextUpdate>, String> {
    reload_agents();
    let replays = skills.restore_invoked(agent_id)?;
    Ok(context_governance::restore_invoked_skill_context(llm_messages, &replays))
}

/// 受保护的工具列表
///
/// 这些工具的调用结果不会被修剪，保留完整的历史记录
const PRUNE_PROTECTED_TOOLS: [&str; 1] = ["skill"];

/// 获取当前时间的毫秒时间戳
///
/// # 返回值
///
/// 返回自 Unix 纪元以来的毫秒数，如果获取失败则返回 0
fn now_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 获取实例目录的可选值
///
/// 将空字符串转换为 None，避免传递无效的目录路径
///
/// # 返回值
///
/// - `Some(String)`: 实例目录非空时返回目录路径
/// - `None`: 实例目录为空时返回 None
fn instance_directory_opt() -> Option<String> {
    let d = instance::directory();
    if d.is_empty() { None } else { Some(d) }
}

/// 修剪会话中的旧消息
///
/// 移除旧的、已完成的工具调用输出，释放上下文空间。
/// 修剪操作会保护最近的对话轮次和特定的工具类型。
///
/// # 参数
///
/// - `session_id`: 要修剪的会话 ID
///
/// # 返回值
///
/// - `Ok(())`: 修剪成功完成
/// - `Err(Error)`: 修剪过程中发生错误
///
/// # 修剪规则
///
/// 1. 保护最近 2 轮对话（user 消息）不被修剪
/// 2. 遇到摘要消息（summary=true）时停止修剪
/// 3. 遇到已压缩的工具调用时停止修剪
/// 4. skill 工具的调用结果受保护，不会被修剪
/// 5. 只有当可修剪的 token 数量超过 `PRUNE_MINIMUM` 时才执行实际修剪
///
/// # 示例
///
/// ```ignore
/// // 在会话溢出时调用修剪
/// if let Err(e) = prune(&session_id).await {
///     eprintln!("修剪失败: {:?}", e);
/// }
/// ```
pub async fn prune(session_id: &str) -> Result<(), crate::app::agent::session::session::Error> {
    let _cfg = config::get().await;

    // 加载会话的所有消息
    let mut msgs = message::messages(session_id, None).await?;
    // 按消息 ID 排序，确保处理顺序正确
    msgs.sort_by(|a, b| a.info.id().cmp(b.info.id()));

    // 统计变量
    let mut total: u64 = 0; // 总的 token 估计量
    let mut pruned: u64 = 0; // 可修剪的 token 量
    let mut to_prune: Vec<message::Part> = Vec::new(); // 待修剪的消息部分
    let mut turns: u64 = 0; // 对话轮次计数

    // 从后向前遍历消息，寻找可修剪的工具调用输出
    'outer: for msg in msgs.iter().rev() {
        // 统计对话轮次（以用户消息为界）
        if matches!(msg.info, message::Info::User(_)) {
            turns += 1;
        }

        // 保护最近 2 轮对话
        if turns < 2 {
            continue;
        }

        // 遇到摘要消息时停止修剪
        if let message::Info::Assistant(a) = &msg.info {
            if a.summary == Some(true) {
                break 'outer;
            }
        }

        // 遍历消息中的各个部分，寻找可修剪的工具调用
        for part in msg.parts.iter().rev() {
            // 只处理工具调用部分
            let message::Part::Tool(tp) = part else { continue };

            // 只处理已完成的工具调用
            let message::ToolState::Completed(st) = &tp.state else { continue };

            // 保护特定类型的工具调用（如 skill）
            if PRUNE_PROTECTED_TOOLS.contains(&tp.tool.as_str()) {
                continue;
            }

            // 遇到已压缩的工具调用时停止
            if st.time.compacted.is_some() {
                break 'outer;
            }

            // 估计工具输出的 token 数量
            let estimate = token::estimate(&st.output);
            total = total.saturating_add(estimate);

            // 超过保护阈值的部分才纳入修剪范围
            if total > PRUNE_PROTECT {
                pruned = pruned.saturating_add(estimate);
                to_prune.push(part.clone());
            }
        }
    }

    // 只有当可修剪的 token 数量超过最小阈值时才执行实际修剪
    if pruned > PRUNE_MINIMUM {
        let ts = now_ms();
        // 标记所有待修剪的部分为已压缩
        for mut part in to_prune {
            let message::Part::Tool(tp) = &mut part else { continue };
            let message::ToolState::Completed(st) = &mut tp.state else { continue };
            st.time.compacted = Some(ts);
            message::update_part(&part).await?;
        }
    }

    Ok(())
}

/// 压缩处理输入参数
///
/// 包含执行会话压缩所需的所有输入信息
#[derive(Debug, Clone)]
pub struct ProcessInput {
    /// 父消息 ID，压缩消息将作为此消息的回复
    pub parent_id: String,
    /// 会话中的所有消息列表，用于生成摘要
    pub messages: Vec<message::WithParts>,
    /// 目标会话 ID
    pub session_id: String,
    /// 是否为自动压缩模式
    ///
    /// - `true`: 自动压缩，会生成继续提示消息
    /// - `false`: 手动压缩，不生成继续提示
    pub auto: bool,
}

/// 提取所有待替换消息的文本、工具调用及文件引用；模型请求阶段再分块。
fn extract_compaction_text(messages: &[message::WithParts]) -> String {
    let mut out = String::new();
    for message in messages {
        let role = match &message.info {
            message::Info::User(_) => "user",
            message::Info::Assistant(_) => "assistant",
        };
        for part in &message.parts {
            let text = match part {
                message::Part::Text(part) => part.text.trim().to_string(),
                message::Part::Tool(_)
                | message::Part::File(_)
                | message::Part::Subtask(_)
                | message::Part::Patch(_) => serde_json::to_string(part)
                    .expect("message parts contain serializable JSON values"),
                _ => continue,
            };
            if text.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(role);
            out.push_str(": ");
            out.push_str(&text);
        }
    }
    out
}

const INTERNAL_COMPACTION_MODE: &str = "chat";

/// Internal request passed to the one-shot summary model.  It deliberately exposes no tool
/// definitions: compaction must never acquire an interactive agent's capabilities.
#[derive(Debug, Clone)]
pub(crate) struct InternalCompactionRequest {
    pub(crate) prompt: String,
    pub(crate) tools: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompactionStats {
    pub covered_messages: usize,
    pub covered_parts: usize,
    pub summary_chars: usize,
}

#[derive(Debug, Clone)]
pub enum CompactionOutcome {
    NotNeeded { history: Vec<message::WithParts> },
    Compacted { history: Vec<message::WithParts>, stats: CompactionStats },
    Failed { history: Vec<message::WithParts>, error: String },
}

/// Private runtime profile for compaction. This is not an entry in the user Agent directory.
pub(crate) fn internal_compaction_profile() -> crate::app::agent::session::llm::AgentInfo {
    crate::app::agent::session::llm::AgentInfo {
        name: "session-compaction".to_string(),
        mode: INTERNAL_COMPACTION_MODE.to_string(),
        prompt: None,
        temperature: None,
        top_p: None,
        options: HashMap::new(),
        permission: Default::default(),
    }
}

fn build_internal_request(input: &ProcessInput) -> Option<(usize, String)> {
    let boundary =
        input.messages.iter().position(|message| message.info.id() == input.parent_id)?;
    if !matches!(input.messages[boundary].info, message::Info::User(_)) || boundary == 0 {
        return None;
    }
    let context = extract_compaction_text(&input.messages[..boundary]);
    if context.trim().is_empty() {
        return None;
    }
    Some((boundary, context))
}

/// Stable, tool-free prompt shared by every internal compaction trigger.
pub(crate) fn internal_summary_prompt(context: &str) -> String {
    format!(
        "Summarize the conversation below for a future continuation. Preserve decisions,\nopen work, constraints, and important technical details. Return only the summary.\n\n---\n{context}\n---"
    )
}

fn valid_summary(summary: String) -> Result<String, String> {
    let summary = summary.trim().to_string();
    if summary.is_empty() {
        return Err("internal compaction generated an empty summary".to_string());
    }
    Ok(summary)
}

fn summary_message(
    input: &ProcessInput,
    model: &provider::Model,
    user: &message::UserInfo,
    summary: String,
) -> Result<message::WithParts, crate::app::agent::session::session::Error> {
    let assistant_id = id::ascending(id::Prefix::Message, None)?;
    let now = now_ms();
    let info = message::Info::Assistant(Box::new(message::AssistantInfo {
        id: assistant_id.clone(),
        session_id: input.session_id.clone(),
        time: message::AssistantTime {
            created: now,
            completed: Some(now),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        },
        error: None,
        parent_id: input.parent_id.clone(),
        model_id: model.id.clone(),
        provider_id: model.provider_id.clone(),
        mode: "compaction".to_string(),
        agent: user.agent.clone(),
        path: message::PathInfo { cwd: instance::directory(), root: instance::worktree() },
        summary: Some(true),
        cost: 0.0,
        tokens: message::TokenInfo {
            total: None,
            input: 0,
            output: 0,
            reasoning: 0,
            cache: message::TokenCacheInfo { read: 0, write: 0 },
        },
        variant: user.variant.clone(),
        finish: Some("stop".to_string()),
    }));
    let boundary = message::Part::Compaction(message::CompactionPart {
        base: message::PartBase {
            id: id::ascending(id::Prefix::Part, None)?,
            session_id: input.session_id.clone(),
            message_id: assistant_id.clone(),
        },
        auto: input.auto,
    });
    let text = message::Part::Text(message::TextPart {
        base: message::PartBase {
            id: id::ascending(id::Prefix::Part, None)?,
            session_id: input.session_id.clone(),
            message_id: assistant_id,
        },
        text: summary,
        synthetic: Some(true),
        ignored: None,
        time: Some(message::PartTime { start: now, end: Some(now) }),
        metadata: None,
    });
    Ok(message::WithParts { info, parts: vec![boundary, text] })
}

async fn persist_compaction(
    input: &ProcessInput,
    boundary: usize,
    summary: &message::WithParts,
) -> Result<(), crate::app::agent::session::session::Error> {
    // The summary is fully generated and validated before any history mutation. Thus provider
    // failures cannot leave a partial replacement behind.
    message::update_message(&summary.info).await?;
    for part in &summary.parts {
        message::update_part(part).await?;
    }
    for covered in &input.messages[..boundary] {
        for part in &covered.parts {
            message::remove_part(&input.session_id, covered.info.id(), part.id()).await?;
        }
        message::remove_message(&input.session_id, covered.info.id()).await?;
    }
    Ok(())
}

async fn compact_with_summary(
    input: ProcessInput,
    model: &provider::Model,
    model_result: Result<String, String>,
) -> Result<CompactionOutcome, crate::app::agent::session::session::Error> {
    let Some((boundary, _)) = build_internal_request(&input) else {
        return Ok(CompactionOutcome::NotNeeded { history: input.messages });
    };
    let summary = match model_result.and_then(valid_summary) {
        Ok(summary) => summary,
        Err(error) => return Ok(CompactionOutcome::Failed { history: input.messages, error }),
    };
    let message::Info::User(user) = &input.messages[boundary].info else { unreachable!() };
    let summary_message = summary_message(&input, model, user, summary.clone())?;
    persist_compaction(&input, boundary, &summary_message).await?;

    let mut history = input.messages[boundary..].to_vec();
    history.push(summary_message);
    Ok(CompactionOutcome::Compacted {
        history,
        stats: CompactionStats {
            covered_messages: boundary,
            covered_parts: input.messages[..boundary]
                .iter()
                .map(|message| message.parts.len())
                .sum(),
            summary_chars: summary.chars().count(),
        },
    })
}

async fn stream_internal_summary(
    input: &ProcessInput,
    model: &provider::Model,
    request: &InternalCompactionRequest,
) -> Result<String, String> {
    use std::sync::{Arc, Mutex};

    let Some(user) = input
        .messages
        .iter()
        .find_map(|message| {
            (message.info.id() == input.parent_id).then(|| match &message.info {
                message::Info::User(user) => Some(user.as_ref().clone()),
                _ => None,
            })
        })
        .flatten()
    else {
        return Err("compaction boundary is not a user message".to_string());
    };
    let output = Arc::new(Mutex::new(String::new()));
    let output_for_events = Arc::clone(&output);
    let saw_tool_call = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let saw_tool_call_for_events = Arc::clone(&saw_tool_call);
    crate::app::agent::session::llm::stream(
        crate::app::agent::session::llm::StreamInput {
            agent: internal_compaction_profile(),
            user,
            session_id: input.session_id.clone(),
            model: model.clone(),
            system: Vec::new(),
            abort: None,
            messages: vec![serde_json::json!({ "role": "user", "content": request.prompt })],
            small: false,
            tools: Default::default(),
            retries: 0,
        },
        move |event| match event {
            crate::app::agent::session::llm::StreamEvent::Delta(delta) => {
                if let Ok(mut output) = output_for_events.lock() {
                    output.push_str(&delta);
                }
            }
            crate::app::agent::session::llm::StreamEvent::ToolCalls(_) => {
                saw_tool_call_for_events.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        },
    )
    .await
    .map_err(|error| format!("internal compaction model request failed: {error}"))?;
    if saw_tool_call.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("internal compaction rejected a model tool call".to_string());
    }
    Ok(output.lock().unwrap_or_else(|error| error.into_inner()).clone())
}

/// Generate a bounded rolling summary of all covered history before replacing persisted records.
/// Model errors leave the history untouched and return a failed outcome.
pub async fn process(
    input: ProcessInput,
    model: &provider::Model,
) -> Result<CompactionOutcome, crate::app::agent::session::session::Error> {
    let Some((_, request)) = build_internal_request(&input) else {
        return Ok(CompactionOutcome::NotNeeded { history: input.messages });
    };
    let model_result = async {
        let mut chunks =
            super::compaction_summary::SummaryChunks::new(&request, Some(&model.limit))?;
        while let Some(prompt) = chunks.next_prompt() {
            let request = InternalCompactionRequest { prompt, tools: Vec::new() };
            chunks.accept(stream_internal_summary(&input, model, &request).await?)?;
        }
        chunks.finish()
    }
    .await;
    let outcome = compact_with_summary(input, model, model_result).await?;
    if matches!(outcome, CompactionOutcome::Compacted { .. }) {
        let session_id = match &outcome {
            CompactionOutcome::Compacted { history, .. } => history
                .first()
                .map(|message| match &message.info {
                    message::Info::User(user) => user.session_id.clone(),
                    message::Info::Assistant(assistant) => assistant.session_id.clone(),
                })
                .unwrap_or_default(),
            _ => String::new(),
        };
        let _ = bus::publish(
            event::COMPACTED,
            json!({ "sessionID": session_id }),
            instance_directory_opt(),
        );
    }
    Ok(outcome)
}

/// Run the internal compaction service for a persisted session.
///
/// Manual callers only know a session and the user-message boundary.  Resolve the model from
/// that boundary so the summary is generated with the same model family as the conversation,
/// then delegate to [`process`].  A failed or empty summary is represented by
/// [`CompactionOutcome::Failed`] and deliberately leaves persisted history untouched.
pub async fn process_session(
    session_id: &str,
    parent_id: &str,
    auto: bool,
) -> Result<CompactionOutcome, crate::app::agent::session::session::Error> {
    let mut messages = message::messages(session_id, None).await?;
    messages.sort_by(|left, right| left.info.id().cmp(right.info.id()));

    let model_ref = messages
        .iter()
        .find_map(|message| {
            (message.info.id() == parent_id).then(|| match &message.info {
                message::Info::User(user) => Some(user.model.clone()),
                message::Info::Assistant(_) => None,
            })
        })
        .flatten()
        .ok_or_else(|| {
            crate::app::agent::session::session::Error::Runtime(anyhow::anyhow!(
                "compaction boundary must be an existing user message"
            ))
        })?;
    if model_ref.provider_id.trim().is_empty() || model_ref.model_id.trim().is_empty() {
        return Err(crate::app::agent::session::session::Error::Runtime(anyhow::anyhow!(
            "compaction boundary has no model reference"
        )));
    }
    let model = provider::get_model(&model_ref.provider_id, &model_ref.model_id).await.map_err(
        |error| {
            crate::app::agent::session::session::Error::Runtime(anyhow::anyhow!(error.to_string()))
        },
    )?;
    process(
        ProcessInput {
            parent_id: parent_id.to_string(),
            messages,
            session_id: session_id.to_string(),
            auto,
        },
        &model,
    )
    .await
}

#[cfg(test)]
async fn process_with_summarizer(
    input: ProcessInput,
    model: &provider::Model,
    mut summarize: impl FnMut(&InternalCompactionRequest) -> Result<String, String>,
) -> Result<CompactionOutcome, crate::app::agent::session::session::Error> {
    let Some((_, context)) = build_internal_request(&input) else {
        return Ok(CompactionOutcome::NotNeeded { history: input.messages });
    };
    let result = (|| {
        let mut chunks =
            super::compaction_summary::SummaryChunks::new(&context, Some(&model.limit))?;
        while let Some(prompt) = chunks.next_prompt() {
            chunks.accept(summarize(&InternalCompactionRequest { prompt, tools: Vec::new() })?)?;
        }
        chunks.finish()
    })();
    compact_with_summary(input, model, result).await
}

/// 创建压缩请求输入参数
///
/// 包含创建新的压缩请求消息所需的所有信息
#[derive(Debug, Clone)]
pub struct CreateInput {
    /// 目标会话 ID
    pub session_id: String,
    /// 代理标识，指定处理此压缩请求的代理
    pub agent: String,
    /// 模型引用，指定用于压缩的模型
    pub model: message::ModelRef,
    /// 是否为自动压缩模式
    pub auto: bool,
}

/// 创建压缩请求
///
/// 创建一个新的压缩请求消息，触发会话压缩流程。
/// 该消息包含一个特殊的压缩部分，标记压缩类型。
///
/// # 参数
///
/// - `input`: 创建压缩请求的输入参数
///
/// # 返回值
///
/// - `Ok(())`: 压缩请求创建成功
/// - `Err(Error)`: 创建过程中发生错误
///
/// # 处理流程
///
/// 1. 生成新的用户消息 ID
/// 2. 创建用户消息信息，携带代理和模型配置
/// 3. 创建压缩部分，标记为自动或手动压缩
/// 4. 保存消息和部分到数据库
///
/// # 示例
///
/// ```ignore
/// let input = CreateInput {
///     session_id: "session_123".to_string(),
///     agent: "default".to_string(),
///     model: message::ModelRef {
///         provider: "openai".to_string(),
///         model: "gpt-4".to_string(),
///     },
///     auto: true,
/// };
///
/// create(input).await?;
/// ```
pub async fn create(input: CreateInput) -> Result<(), crate::app::agent::session::session::Error> {
    // 生成压缩请求消息的 ID
    let msg_id = id::ascending(id::Prefix::Message, None)?;

    // 构建压缩请求用户消息信息
    let info = message::Info::User(Box::new(message::UserInfo {
        id: msg_id.clone(),
        session_id: input.session_id.clone(),
        time: message::UserTime { created: now_ms() },
        summary: None,
        agent: input.agent,
        model: input.model,
        system: None,
        tools: None,
        variant: None,
    }));
    message::update_message(&info).await?;

    // 创建压缩部分，标记压缩类型
    let part_id = id::ascending(id::Prefix::Part, None)?;
    let part = message::Part::Compaction(message::CompactionPart {
        base: message::PartBase { id: part_id, session_id: input.session_id, message_id: msg_id },
        auto: input.auto, // 标记是否为自动压缩
    });
    message::update_part(&part).await?;

    Ok(())
}
#[cfg(test)]
#[path = "compaction_tests.rs"]
mod compaction_tests;
