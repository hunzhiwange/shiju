//! 会话压缩（Compaction）模块
//!
//! 本模块提供了会话历史消息的压缩和修剪功能，用于在会话上下文接近模型限制时
//! 通过独立检查点缩短模型上下文，保留原始聊天记录。
//!
//! # 核心功能
//!
//! - **溢出检测**：判断当前会话是否接近上下文限制，需要执行压缩
//! - **消息修剪**：移除旧的、已完成的工具调用输出，减少上下文占用
//! - **摘要生成**：通过统一模型请求生成单次交接摘要，成功后原子保存检查点
//! - **压缩请求创建**：创建新的压缩请求消息，触发压缩流程
//!
//! # 压缩策略
//!
//! 1. 当会话使用率达到 总上下文的 90%（受输入和输出预算约束）时，触发溢出检测
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
/// 当使用率达到总上下文的 90%（或先达到输入预算） 时返回 true。
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
/// 1. 计算可用输入空间：遵守输入限制，并为摘要预留有界输出；未知窗口使用运行时默认值
/// 2. 计算目标阈值：总窗口的 90%，不超过可用输入空间
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

/// 提取所有待替换消息的文本、工具调用及文件引用；用于验证摘要覆盖范围。
#[cfg(test)]
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

/// Internal request passed to the one-shot summary model.  It deliberately exposes no tool
/// definitions: compaction must never acquire an interactive agent's capabilities.
#[cfg(test)]
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

fn compaction_boundary(input: &ProcessInput) -> Option<usize> {
    let boundary =
        input.messages.iter().position(|message| message.info.id() == input.parent_id)?;
    if !matches!(input.messages[boundary].info, message::Info::User(_)) || boundary == 0 {
        return None;
    }
    let has_content = input.messages[..boundary].iter().flat_map(|message| &message.parts).any(
        |part| match part {
            message::Part::Text(text) => text.ignored != Some(true) && !text.text.trim().is_empty(),
            message::Part::Tool(_)
            | message::Part::File(_)
            | message::Part::Subtask(_)
            | message::Part::Patch(_) => true,
            _ => false,
        },
    );
    has_content.then_some(boundary)
}

/// Stable, tool-free prompt shared by every internal compaction trigger.
#[cfg(test)]
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

fn model_history(messages: &[message::WithParts]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|message| {
            let role = match message.info {
                message::Info::User(_) => "user",
                message::Info::Assistant(_) => "assistant",
            };
            let content = message
                .parts
                .iter()
                .filter_map(|part| match part {
                    message::Part::Text(text) if text.ignored != Some(true) => {
                        Some(text.text.clone())
                    }
                    message::Part::Tool(_)
                    | message::Part::File(_)
                    | message::Part::Subtask(_)
                    | message::Part::Patch(_) => serde_json::to_string(part).ok(),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            json!({"role": role, "content": content})
        })
        .collect()
}

/// Use the same message projection the desktop sends to the model, when a saved
/// snapshot has an exact canonical boundary ID. Tool cards can expand one canonical
/// assistant message into several UI messages, so their text cannot be hashed as one.
fn checkpoint_source(
    input: &ProcessInput,
    boundary: usize,
) -> Result<Vec<serde_json::Value>, crate::session::session::Error> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(project) = instance::project() {
        let snapshot = super::ui_store::load_session_scoped_for_tenant_fallible(
            &super::ui_store::current_tenant_scope(),
            &input.session_id,
            Some(&project.id),
        )
        .map_err(crate::session::session::Error::Runtime)?;
        if let Some(snapshot) = snapshot {
            if let Some(source) = snapshot_checkpoint_source(&snapshot, &input.parent_id) {
                return Ok(source);
            }
        }
    }
    Ok(model_history(&input.messages[..boundary]))
}

fn snapshot_checkpoint_source(
    snapshot: &super::ui_types::ChatSession,
    boundary_id: &str,
) -> Option<Vec<serde_json::Value>> {
    let boundary = snapshot.message_ids.iter().position(|id| id.as_deref() == Some(boundary_id))?;
    if boundary == 0 || snapshot.messages.get(boundary)?.role != super::ui_types::ChatRole::User {
        return None;
    }
    Some(
        snapshot.messages[..boundary]
            .iter()
            .map(|message| {
                let role = match message.role {
                    super::ui_types::ChatRole::User => "user",
                    super::ui_types::ChatRole::Assistant | super::ui_types::ChatRole::Tool => {
                        "assistant"
                    }
                    super::ui_types::ChatRole::System => "system",
                    super::ui_types::ChatRole::Developer => "developer",
                };
                json!({"role": role, "content": message.content})
            })
            .collect(),
    )
}

async fn compact_with_summary(
    input: ProcessInput,
    model_result: Result<String, String>,
    source: &[serde_json::Value],
) -> Result<CompactionOutcome, crate::app::agent::session::session::Error> {
    let Some(boundary) = compaction_boundary(&input) else {
        return Ok(CompactionOutcome::NotNeeded { history: input.messages });
    };
    let summary = match model_result.and_then(valid_summary) {
        Ok(summary) => summary,
        Err(error) => return Ok(CompactionOutcome::Failed { history: input.messages, error }),
    };
    super::compaction_checkpoint::save(&input.session_id, source, &summary, None)
        .await
        .map_err(|error| crate::session::session::Error::Runtime(anyhow::anyhow!(error)))?;
    let history = input.messages.clone();
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

/// Generate a context checkpoint without deleting any persisted chat records.
/// Model errors leave the history untouched and return a failed outcome.
pub async fn process(
    input: ProcessInput,
    model: &provider::Model,
) -> Result<CompactionOutcome, crate::app::agent::session::session::Error> {
    let Some(boundary) = compaction_boundary(&input) else {
        return Ok(CompactionOutcome::NotNeeded { history: input.messages });
    };
    let source = checkpoint_source(&input, boundary)?;
    let context = super::compaction_checkpoint::restore(&input.session_id, &source)
        .await
        .map_err(|error| crate::session::session::Error::Runtime(anyhow::anyhow!(error)))?;
    let cfg = config::get_project_for_tenant(
        &super::ui_store::current_tenant_scope(),
        std::path::Path::new(&crate::project::instance::directory()),
    )
    .await
    .map_err(|error| crate::session::session::Error::Runtime(anyhow::anyhow!(error)))?;
    let model_result = super::compaction_summary::summarize_async(
        input.session_id.clone(),
        context,
        format!("{}/{}", model.provider_id, model.id),
        cfg.reliability,
        model.limit.clone(),
    )
    .await;
    let outcome = compact_with_summary(input, model_result, &source).await?;
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
    _model: &provider::Model,
    mut summarize: impl FnMut(&InternalCompactionRequest) -> Result<String, String>,
) -> Result<CompactionOutcome, crate::app::agent::session::session::Error> {
    let Some(boundary) = compaction_boundary(&input) else {
        return Ok(CompactionOutcome::NotNeeded { history: input.messages });
    };
    let context = extract_compaction_text(&input.messages[..boundary]);
    let result = summarize(&InternalCompactionRequest {
        prompt: internal_summary_prompt(&context),
        tools: Vec::new(),
    });
    let source = model_history(&input.messages[..boundary]);
    compact_with_summary(input, result, &source).await
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
