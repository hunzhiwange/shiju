//! 调用边界的可见历史快照；不继承父角色身份或工具执行轨迹。
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ForkTurns {
    #[default]
    All,
    None,
}

pub(super) fn snapshot(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .filter_map(|message| {
            let role = message.get("role")?.as_str()?;
            let text = message.get("content")?.as_str()?;
            let content = match role {
                "user" => text.to_string(),
                "assistant" => super::visible_answer_from_execution_output(text),
                _ => return None,
            };
            if content.trim().is_empty() {
                return None;
            }
            Some(json!({"role":role,"content":content}))
        })
        .collect()
}

pub(super) async fn stored_snapshot(parent: &str) -> anyhow::Result<Vec<Value>> {
    let mut messages = super::message::messages(parent, None).await?;
    messages.reverse();
    let values = messages
        .into_iter()
        .map(|message| {
            let role = match message.info {
                super::message::Info::User(_) => "user",
                super::message::Info::Assistant(_) => "assistant",
            };
            let content = message
                .parts
                .into_iter()
                .filter_map(|part| match part {
                    super::message::Part::Text(text) if text.ignored != Some(true) => {
                        Some(text.text)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            json!({"role":role,"content":content})
        })
        .collect::<Vec<_>>();
    Ok(snapshot(&values))
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
