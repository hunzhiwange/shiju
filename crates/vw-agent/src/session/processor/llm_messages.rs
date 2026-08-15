//! LLM 消息格式转换模块
//!
//! 本模块提供将内部会话消息（Session Message）转换为 LLM（大语言模型）API 所需的标准消息格式的功能。
//!
//! # 主要功能
//!
//! - 将 `Session` 中的消息列表转换为 LLM 可识别的 JSON 格式
//! - 处理不同角色（User、Assistant、System、Tool）的映射
//!
//! # 消息格式
//!
//! 转换后的每条消息都是包含 `role` 和 `content` 字段的 JSON 对象，符合主流 LLM API 的标准格式。

use crate::app::agent::multimodal::context::ImageTurnContext;
use crate::app::agent::session::session::{Message, Role, Session};
use crate::session::context_injection;
use crate::session::instruction::Resolved;
use serde_json::Value;

pub(crate) const INTERNAL_INPUT_SOURCE_OPTION: &str = "_vw_input_source";
const INTERNAL_MESSAGE_SOURCE_FIELD: &str = "_vw_source";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssembledProtocolMessages {
    pub(crate) messages: Vec<Value>,
    pub(crate) message_sources: Vec<Value>,
}

impl std::ops::Deref for AssembledProtocolMessages {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

pub(crate) fn context_message_source(form: &str, entries: &[String]) -> Value {
    serde_json::json!({
        "kind": "context",
        "form": form,
        "entries": entries,
    })
}

pub(crate) fn sourced_protocol_message(mut message: Value, source: Value) -> Value {
    if let Some(object) = message.as_object_mut() {
        object.insert(INTERNAL_MESSAGE_SOURCE_FIELD.to_string(), source);
    }
    message
}

pub(crate) fn annotate_current_user_source(messages: &mut [Value], source: Value) {
    let Some(message) = messages
        .iter_mut()
        .rfind(|message| message.get("role").and_then(Value::as_str) == Some("user"))
    else {
        return;
    };
    if let Some(object) = message.as_object_mut() {
        object.insert(INTERNAL_MESSAGE_SOURCE_FIELD.to_string(), source);
    }
}

pub(crate) fn input_source_from_options(options: &Value) -> Option<Value> {
    options.get(INTERNAL_INPUT_SOURCE_OPTION).filter(|source| source.is_object()).cloned()
}

fn split_message_sources(mut messages: Vec<Value>) -> AssembledProtocolMessages {
    let message_sources = messages
        .iter_mut()
        .map(|message| {
            let Some(object) = message.as_object_mut() else { return Value::Null };
            object.remove("source");
            object.remove(INTERNAL_MESSAGE_SOURCE_FIELD).unwrap_or(Value::Null)
        })
        .collect();
    AssembledProtocolMessages { messages, message_sources }
}

pub(crate) fn session_message_to_llm_message(message: &Message) -> Option<serde_json::Value> {
    match message.role {
        Role::User => Some(serde_json::json!({ "role": "user", "content": message.content })),
        Role::Assistant => {
            Some(serde_json::json!({ "role": "assistant", "content": message.content }))
        }
        Role::System => Some(serde_json::json!({ "role": "system", "content": message.content })),
        Role::Tool => Some(serde_json::json!({ "role": "assistant", "content": message.content })),
    }
}

pub(crate) fn extend_llm_messages_from_session_range(
    llm_messages: &mut Vec<serde_json::Value>,
    session: &Session,
    start_index: usize,
) {
    llm_messages.extend(
        session.messages.iter().skip(start_index).filter_map(session_message_to_llm_message),
    )
}

/// 将会话消息转换为 LLM API 格式的消息列表
///
/// 该函数遍历会话中的所有消息，并根据消息的角色将其转换为 LLM API 兼容的 JSON 格式。
/// 不同角色的消息会被映射到标准的 LLM 消息角色。
///
/// # 参数
///
/// * `session` - 会话对象的引用，包含待转换的消息列表
///
/// # 返回值
///
/// 返回一个 `serde_json::Value` 向量，每个元素都是一条 LLM 格式的消息对象。
/// 每个消息对象包含：
/// - `role`: 消息角色字符串（"user"、"assistant" 或 "system"）
/// - `content`: 消息内容字符串
///
/// # 角色映射规则
///
/// | Session 角色 | LLM 角色 | 说明 |
/// |--------------|----------|------|
/// | User | user | 用户输入的消息 |
/// | Assistant | assistant | 助手/模型生成的回复 |
/// | System | system | 系统提示/指令 |
/// | Tool | assistant | 工具执行结果（作为助手消息传递） |
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::session::session::{Session, Role, Message};
///
/// let session = Session {
///     messages: vec![
///         Message { role: Role::User, content: "你好".to_string() },
///         Message { role: Role::Assistant, content: "你好！有什么可以帮助你的？".to_string() },
///     ],
///     // ... 其他字段
/// };
///
/// let llm_messages = session_messages_to_llm_messages(&session);
/// // 结果:
/// // [
/// //   {"role": "user", "content": "你好"},
/// //   {"role": "assistant", "content": "你好！有什么可以帮助你的？"}
/// // ]
/// ```
///
/// # 注意事项
///
/// - `Tool` 角色的消息会被映射为 `assistant` 角色，这是因为工具执行结果通常作为助手回复的一部分传递给 LLM
/// - 该函数使用 `filter_map`，如果未来需要过滤某些消息类型，可以通过返回 `None` 来实现
pub(crate) fn session_messages_to_llm_messages(session: &Session) -> Vec<serde_json::Value> {
    // 遍历会话中的所有消息，根据角色进行转换
    session.messages.iter().filter_map(session_message_to_llm_message).collect()
}

/// 组装桌面会话单步请求的临时协议上下文。
///
/// 这些提醒只发给 provider，绝不回写到 `Session`，从而保持桌面 UI 历史可见内容不变。
/// 顺序固定为已调用技能、技能目录、AGENTS 和真实历史。
pub(crate) fn assemble_session_protocol_messages(
    history: &[Value],
    instructions: &[Resolved],
    invoked_skill_messages: &[Value],
    skill_listing: Option<Value>,
    default_tool_reminder: Option<Value>,
) -> Vec<Value> {
    assemble_session_protocol_messages_with_sources(
        history,
        instructions,
        invoked_skill_messages,
        skill_listing,
        default_tool_reminder,
    )
    .messages
}

pub(crate) fn assemble_session_protocol_messages_with_sources(
    history: &[Value],
    instructions: &[Resolved],
    invoked_skill_messages: &[Value],
    skill_listing: Option<Value>,
    default_tool_reminder: Option<Value>,
) -> AssembledProtocolMessages {
    let injection = context_injection::assemble_agents_context(instructions, history);
    let instruction_entries = instructions
        .iter()
        .map(|instruction| instruction.filepath.clone())
        .collect::<Vec<_>>();
    let instruction_source = context_message_source("instructions", &instruction_entries);
    let agents_messages = injection
        .transient_messages
        .iter()
        .map(|message| {
            sourced_protocol_message(
                serde_json::json!({
                    "role": message.role.as_str(),
                    "content": message.content,
                }),
                instruction_source.clone(),
            )
        })
        .collect::<Vec<_>>();
    let mut protocol_messages = invoked_skill_messages.to_vec();
    if let Some(skill_listing) = skill_listing {
        protocol_messages.push(skill_listing);
    }
    if let Some(default_tool_reminder) = default_tool_reminder {
        protocol_messages.push(default_tool_reminder);
    }
    // AGENTS 必须是当前真实用户指令前的最后一条协议消息，避免后续技能目录或默认工具
    // 提醒稀释仓库约束的近因权重。
    protocol_messages.extend(agents_messages);

    // 协议提醒必须紧邻本轮用户请求。若始终放在整段历史最前面，长会话中的模型会把它
    // 当成陈旧上下文；用户手工再次要求读取 AGENTS 时反而能生效。工具轮次仍以最后一条
    // 真实用户消息作为本轮边界，保持后续 assistant/tool 配对顺序不变。
    let insertion_index = injection
        .history
        .iter()
        .rposition(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        .unwrap_or(0);
    let mut messages = Vec::with_capacity(injection.history.len() + protocol_messages.len());
    messages.extend(injection.history[..insertion_index].iter().cloned());
    messages.extend(protocol_messages);
    messages.extend(injection.history[insertion_index..].iter().cloned());
    split_message_sources(messages)
}

/// 构建临近当前用户请求的默认工具提醒；它不构成可见性白名单。
pub(crate) fn default_tool_reminder(default_tool_ids: &[String]) -> Option<Value> {
    (!default_tool_ids.is_empty()).then(|| {
        sourced_protocol_message(
            serde_json::json!({
                "role": "user",
                "content": format!(
                    "<system-reminder>Default tools for this delegated agent: {}. This is a default selection, not a restriction on other available tools.</system-reminder>",
                    default_tool_ids.join(", ")
                ),
            }),
            context_message_source("default_tools", default_tool_ids),
        )
    })
}

/// 按图片上下文状态改写 LLM JSON 消息。
pub(crate) fn apply_image_turn_context(
    llm_messages: &mut [serde_json::Value],
    context: &ImageTurnContext,
) {
    context.rewrite_json_messages(llm_messages);
}

#[cfg(test)]
#[path = "llm_messages_tests.rs"]
mod llm_messages_tests;
