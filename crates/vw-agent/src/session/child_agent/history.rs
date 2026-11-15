//! 调用边界的可见历史快照；不继承父角色身份或工具执行轨迹。
use serde_json::{Value, json};

pub use session_port::child_agent::ForkTurns;

pub(super) use session_engine::child_agent::history::snapshot;

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
