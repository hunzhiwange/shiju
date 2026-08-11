//! ACP 新会话历史重放提示词构造工具。
//!
//! ACP 代理会话可能与 VibeWindow 本地会话生命周期不同步。本模块负责在需要创建
//! 新 ACP 会话时，把本地 OpenAI 风格消息转换成可读的恢复上下文，并根据用户配置选择
//! 丢弃、完整、摘要或最近历史策略。

use serde_json::Value;
use std::collections::HashSet;

use super::AcpReplayStrategy;

/// 为一次 ACP 请求构造最终 prompt。
///
/// `force_new_session` 为 `true` 时会按重放策略拼接本地历史；否则只提取最新用户消息。
/// 返回值可能为空，调用方需要把空 prompt 作为请求错误处理。
pub(crate) fn build_request_prompt(
    chat_messages: &Value,
    force_new_session: bool,
    replay_strategy: AcpReplayStrategy,
    replay_recent_count: usize,
) -> String {
    if force_new_session {
        build_replay_prompt(chat_messages, replay_strategy, replay_recent_count)
    } else {
        extract_prompt(chat_messages)
    }
}

fn extract_prompt(chat_messages: &Value) -> String {
    let Some(items) = chat_messages.as_array() else {
        return String::new();
    };
    for item in items.iter().rev() {
        let role = item.get("role").and_then(Value::as_str).unwrap_or_default();
        if role != "user" {
            continue;
        }
        let content = item.get("content").cloned().unwrap_or(Value::Null);
        if let Some(text) = content.as_str() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        if let Some(blocks) = content.as_array() {
            let mut out = String::new();
            for block in blocks {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    out.push_str(text);
                }
            }
            let trimmed = out.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    String::new()
}

/// 将 OpenAI 风格的 content 字段压平成纯文本。
///
/// 支持字符串和文本块数组；无法识别的内容返回空字符串，以免把结构化附件误拼进 prompt。
fn content_to_text(content: &Value) -> String {
    if let Some(text) = content.as_str() {
        return text.trim().to_string();
    }
    let Some(blocks) = content.as_array() else {
        return String::new();
    };
    let mut out = String::new();
    for block in blocks {
        if let Some(text) = block.get("text").and_then(Value::as_str) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text.trim());
        }
    }
    out.trim().to_string()
}

/// ACP accepts a single text prompt, so transient meta-user messages need an
/// explicit serialization boundary. They are not ordinary conversation turns
/// and must not be replayed once per historical request.
fn is_protocol_meta_message(role: &str, content: &str) -> bool {
    role == "user"
        && content.starts_with("<system-reminder>")
        && content.ends_with("</system-reminder>")
}

/// 生成单行预览文本。
///
/// 按字符截断而不是按字节截断，避免破坏 UTF-8；用于摘要策略中的人工可读 digest。
fn preview_line(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = compact.chars().take(max_chars).collect::<String>();
    if compact.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

/// 将若干摘要行渲染为 markdown bullet 列表。
fn render_bullets(lines: &[String]) -> String {
    lines.iter().map(|line| format!("- {line}")).collect::<Vec<_>>().join("\n")
}

/// 渲染最近若干轮对话的紧凑 digest。
fn render_turn_digest(messages: &[(String, String)], limit: usize) -> String {
    messages
        .iter()
        .rev()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .enumerate()
        .map(|(idx, (role, content))| {
            format!("Turn {} | {} | {}", idx + 1, role, preview_line(&content, 220))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从合并后的模型选项中解析 ACP 历史重放策略。
///
/// 未配置或未知值都会回退到 `Discard`，因为这是最保守的默认行为：不会意外扩大
/// 发送给外部 ACP 代理的历史上下文。
pub(crate) fn parse_replay_strategy(options: &Value) -> AcpReplayStrategy {
    match options
        .get("acp_history_strategy")
        .and_then(Value::as_str)
        .unwrap_or("discard")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "discard" => AcpReplayStrategy::Discard,
        "full" => AcpReplayStrategy::Full,
        "recent" => AcpReplayStrategy::Recent,
        "summary" => AcpReplayStrategy::Summary,
        _ => AcpReplayStrategy::Discard,
    }
}

/// 解析 recent 策略的历史轮数。
///
/// 返回值会限制在 1 到 20 之间，避免配置错误导致 prompt 过大。
pub(crate) fn parse_recent_count(options: &Value) -> usize {
    options
        .get("acp_history_recent_count")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(1, 20) as usize)
        .unwrap_or(3)
}

/// 构造新 ACP 会话的历史恢复 prompt。
///
/// `chat_messages` 应为 OpenAI 风格消息数组；当输入不是数组或没有当前用户请求时，
/// 会回退为最新用户消息提取。函数只做本地字符串构造，不执行 IO，也不返回错误。
pub(crate) fn build_replay_prompt(
    chat_messages: &Value,
    strategy: AcpReplayStrategy,
    recent_count: usize,
) -> String {
    let Some(items) = chat_messages.as_array() else {
        return extract_prompt(chat_messages);
    };

    let mut system_messages = Vec::new();
    let mut protocol_messages = Vec::new();
    let mut seen_protocol_messages = HashSet::new();
    let mut non_system_messages = Vec::<(String, String)>::new();
    for item in items {
        let role = item.get("role").and_then(Value::as_str).unwrap_or_default();
        let content = content_to_text(&item.get("content").cloned().unwrap_or(Value::Null));
        if content.is_empty() {
            continue;
        }
        if is_protocol_meta_message(role, &content) {
            if seen_protocol_messages.insert(content.clone()) {
                protocol_messages.push(content);
            }
            continue;
        }
        if role == "system" {
            system_messages.push(content);
        } else {
            non_system_messages.push((role.to_string(), content));
        }
    }

    let latest_user = non_system_messages
        .iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, content)| content.clone())
        .unwrap_or_default();
    if latest_user.is_empty() {
        return extract_prompt(chat_messages);
    }

    // 当前用户请求会单独放入 `<current_user_request>`，旧历史只取它之前的内容，
    // 避免同一请求在重放 prompt 中出现两次。
    let older_messages = if non_system_messages.is_empty() {
        Vec::new()
    } else {
        non_system_messages[..non_system_messages.len().saturating_sub(1)].to_vec()
    };

    let mut sections = Vec::new();
    sections.push(
        "You are continuing a local conversation in a newly created ACP session. Use the reconstructed context below and continue naturally."
            .to_string(),
    );

    if !system_messages.is_empty() {
        sections.push(format!("<system>\n{}\n</system>", system_messages.join("\n\n")));
    }
    if !protocol_messages.is_empty() {
        sections.push(format!(
            "<protocol_context>\n{}\n</protocol_context>",
            protocol_messages.join("\n\n")
        ));
    }

    match strategy {
        AcpReplayStrategy::Discard => {}
        AcpReplayStrategy::Full => {
            if !older_messages.is_empty() {
                let transcript = older_messages
                    .iter()
                    .map(|(role, content)| format!("[{role}]\n{content}"))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                sections
                    .push(format!("<conversation_history>\n{transcript}\n</conversation_history>"));
            }
        }
        AcpReplayStrategy::Recent => {
            let recent = older_messages
                .iter()
                .rev()
                .take(recent_count)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|(role, content)| format!("[{role}]\n{content}"))
                .collect::<Vec<_>>()
                .join("\n\n");
            if !recent.is_empty() {
                sections.push(format!("<recent_messages>\n{recent}\n</recent_messages>"));
            }
        }
        AcpReplayStrategy::Summary => {
            if !older_messages.is_empty() {
                // 摘要策略故意使用本地确定性摘录，而不是再次调用模型摘要。
                // 这样新建会话恢复路径不会引入额外网络请求或不可预测输出。
                let system_constraints = system_messages
                    .iter()
                    .take(4)
                    .map(|text| preview_line(text, 180))
                    .collect::<Vec<_>>();
                let recent_user_goals = older_messages
                    .iter()
                    .filter(|(role, _)| role == "user")
                    .rev()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(|(_, content)| preview_line(&content, 180))
                    .collect::<Vec<_>>();
                let assistant_progress = older_messages
                    .iter()
                    .filter(|(role, _)| role == "assistant")
                    .rev()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(|(_, content)| preview_line(&content, 180))
                    .collect::<Vec<_>>();
                let digest = render_turn_digest(&older_messages, 8);

                let mut summary_sections = Vec::new();
                summary_sections.push("Session Snapshot v2".to_string());
                summary_sections
                    .push(format!("Messages in local history: {}", non_system_messages.len()));
                if !system_constraints.is_empty() {
                    summary_sections.push(format!(
                        "<system_constraints>\n{}\n</system_constraints>",
                        render_bullets(&system_constraints)
                    ));
                }
                if !recent_user_goals.is_empty() {
                    summary_sections.push(format!(
                        "<recent_user_goals>\n{}\n</recent_user_goals>",
                        render_bullets(&recent_user_goals)
                    ));
                }
                if !assistant_progress.is_empty() {
                    summary_sections.push(format!(
                        "<assistant_progress>\n{}\n</assistant_progress>",
                        render_bullets(&assistant_progress)
                    ));
                }
                if !digest.is_empty() {
                    summary_sections.push(format!("<turn_digest>\n{digest}\n</turn_digest>"));
                }
                let summary = summary_sections.join("\n\n");
                if !summary.trim().is_empty() {
                    sections.push(format!(
                        "<conversation_summary>\n{summary}\n</conversation_summary>"
                    ));
                }
                let recent = older_messages
                    .iter()
                    .rev()
                    .take(recent_count.max(2))
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(|(role, content)| format!("[{role}]\n{content}"))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if !recent.is_empty() {
                    sections.push(format!("<recent_messages>\n{recent}\n</recent_messages>"));
                }
            }
        }
    }

    sections.push(format!("<current_user_request>\n{latest_user}\n</current_user_request>"));
    sections.join("\n\n")
}
#[cfg(test)]
#[path = "replay_tests.rs"]
mod replay_tests;
