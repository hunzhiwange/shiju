//! 将会话外部上下文临时装配为模型消息。

use serde_json::Value;
use serde_json::json;

use super::instruction::Resolved;

/// 临时加入 provider 上下文、但不持久化到可见聊天记录的消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporaryContextMessage {
    /// Provider 请求使用的消息角色。
    pub role: ContextRole,
    /// 临时消息正文。
    pub content: String,
    /// 此消息不会写入可见聊天记录。
    pub visible: bool,
}

/// 临时上下文消息在 provider 请求中的角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextRole {
    /// 以 system 角色传递仓库约束，确保其优先级高于普通用户文本。
    System,
}

impl ContextRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
        }
    }
}

/// 一次请求的临时上下文及未修改的历史消息。
#[derive(Debug, Clone, PartialEq)]
pub struct ContextInjection {
    /// 仅本次 provider 请求存在的消息。
    pub transient_messages: Vec<TemporaryContextMessage>,
    /// 从持久化会话复制而来、保持字节不变的历史消息。
    pub history: Vec<Value>,
}

impl ContextInjection {
    /// 返回仅供本次 provider 请求使用的消息序列。
    pub fn provider_messages(&self) -> Vec<Value> {
        let mut messages = self
            .transient_messages
            .iter()
            .map(|message| json!({ "role": message.role.as_str(), "content": message.content }))
            .collect::<Vec<_>>();
        messages.extend(self.history.iter().cloned());
        messages
    }
}

/// 在历史消息前临时装配 AGENTS 指令。
pub fn assemble_agents_context(instructions: &[Resolved], history: &[Value]) -> ContextInjection {
    let content = instructions
        .iter()
        .map(|instruction| instruction.content.trim())
        .filter(|content| !content.is_empty())
        .collect::<Vec<_>>();
    let transient_messages = if content.is_empty() {
        Vec::new()
    } else {
        vec![TemporaryContextMessage {
            role: ContextRole::System,
            content: format!(
                "<system-reminder>\n\
                 AGENTS.md loaded for this request.\n\
                 The following repository instructions are active, override default instructions, and must be followed exactly.\n\n\
                 {}\n\
                 </system-reminder>",
                content.join("\n\n")
            ),
            visible: false,
        }]
    };
    ContextInjection { transient_messages, history: history.to_vec() }
}

#[cfg(test)]
#[path = "context_injection_tests.rs"]
mod context_injection_tests;
