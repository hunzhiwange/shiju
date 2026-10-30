//! 会话处理器的提示词组装逻辑，负责把当前会话历史整理成模型可消费的输入。

use crate::app::agent::session::session::{Role, Session};
use crate::session::prompt;

/// 执行 build_prompt 操作，并返回调用方需要的结果。
pub(crate) fn build_prompt(
    session: &Session,
    model: Option<&str>,
    root: Option<&str>,
    extra_assistant: Option<&str>,
) -> String {
    const MAX_PROMPT_BYTES: usize = 120 * 1024;
    const MAX_TOOL_MESSAGE_BYTES: usize = 3 * 1024;

    let mut out = String::new();
    out.push_str(&prompt::system(model, root));
    out.push_str("\n\n");

    let mut chunks: Vec<String> = Vec::new();
    let mut used = out.as_bytes().len();
    for msg in session.messages.iter().rev() {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Developer => "developer",
            Role::Tool => "tool",
        };
        let mut content = msg.content.as_str();
        if matches!(msg.role, Role::Tool) && content.as_bytes().len() > MAX_TOOL_MESSAGE_BYTES {
            let mut cut = MAX_TOOL_MESSAGE_BYTES;
            while cut > 0 && !content.is_char_boundary(cut) {
                cut -= 1;
            }
            content = &content[..cut];
        }
        let mut chunk = String::new();
        chunk.push_str(role);
        chunk.push_str(": ");
        chunk.push_str(content);
        if !chunk.ends_with('\n') {
            chunk.push('\n');
        }
        if used + chunk.as_bytes().len() > MAX_PROMPT_BYTES {
            break;
        }
        used += chunk.as_bytes().len();
        chunks.push(chunk);
    }
    chunks.reverse();
    for c in chunks {
        out.push_str(&c);
    }
    if let Some(extra) = extra_assistant {
        if !extra.trim().is_empty() {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("assistant: ");
            out.push_str(extra.trim());
            out.push('\n');
        }
    }
    out
}
#[cfg(test)]
#[path = "prompting_tests.rs"]
mod prompting_tests;
