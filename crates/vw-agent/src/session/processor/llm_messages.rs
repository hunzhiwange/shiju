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

use crate::app::agent::session::session::{Message, Role, Session};
use crate::app::agent::multimodal::context::ImageTurnContext;

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
