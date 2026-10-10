//! 会话消息与消息片段的持久化入口。
//!
//! 本模块复用 `vw_shared` 中的消息类型，并负责把消息/part 写入本地 storage 后发布 bus
//! 事件。写入和事件发布保持分离：存储失败会返回错误，事件发布失败则不阻断主路径。

/// 共享消息类型的本地再导出，供 session 子系统沿用既有路径引用。
pub use vw_shared::message::types::*;

use crate::app::agent::bus;
use crate::app::agent::project::instance;
use crate::app::agent::storage;
use serde_json::json;

pub mod event {
    //! 消息存储变更事件定义。
    //!
    //! 事件名保持稳定字符串，供桌面端和其他订阅者监听消息、part 的新增、更新和删除。

    use crate::app::agent::bus;

    /// 消息信息已更新。
    pub const UPDATED: bus::Definition = bus::Definition { r#type: "message.updated" };
    /// 消息信息已删除。
    pub const REMOVED: bus::Definition = bus::Definition { r#type: "message.removed" };
    /// 消息片段已更新。
    pub const PART_UPDATED: bus::Definition = bus::Definition { r#type: "message.part.updated" };
    /// 消息片段已删除。
    pub const PART_REMOVED: bus::Definition = bus::Definition { r#type: "message.part.removed" };
}

/// 获取当前实例目录。
///
/// 空目录会转为 `None`，让 bus 发布逻辑按无目录作用域处理。
fn instance_directory_opt() -> Option<String> {
    let d = instance::directory();
    if d.is_empty() { None } else { Some(d) }
}

/// 写入一条消息并发布更新事件。
///
/// 存储写入失败会返回 session 错误；事件发布失败会被忽略，因为消息持久化才是主操作。
pub async fn update_message(info: &Info) -> Result<(), super::session::Error> {
    storage::write(
        &[
            "message",
            match info {
                Info::User(u) => &u.session_id,
                Info::Assistant(a) => &a.session_id,
            },
            info.id(),
        ],
        info,
    )
    .await?;
    let _ = bus::publish(event::UPDATED, json!({ "info": info }), instance_directory_opt());
    Ok(())
}

/// 删除一条消息并发布删除事件。
///
/// 参数分别是会话 id 和消息 id。存储删除失败会返回错误；事件发布失败不会阻断返回。
pub async fn remove_message(
    session_id: &str,
    message_id: &str,
) -> Result<(), super::session::Error> {
    storage::remove(&["message", session_id, message_id]).await?;
    let _ = bus::publish(
        event::REMOVED,
        json!({ "sessionID": session_id, "messageID": message_id }),
        instance_directory_opt(),
    );
    Ok(())
}

/// 写入一条消息片段并发布更新事件。
///
/// part 的 message id 和 part id 会从类型自身读取，避免调用方传错索引键。
pub async fn update_part(part: &Part) -> Result<(), super::session::Error> {
    let message_id = part.message_id().to_string();
    let part_id = part.id().to_string();
    storage::write(&["part", &message_id, &part_id], part).await?;
    let _ = bus::publish(event::PART_UPDATED, json!({ "part": part }), instance_directory_opt());
    Ok(())
}

/// 删除一条消息片段并发布删除事件。
///
/// 会同时尝试删除新旧两种存储路径，兼容历史数据布局；旧路径删除失败不会覆盖主删除结果。
pub async fn remove_part(
    session_id: &str,
    message_id: &str,
    part_id: &str,
) -> Result<(), super::session::Error> {
    storage::remove(&["part", message_id, part_id]).await?;
    // 旧版本曾在 part 键中包含 session_id；这里保留清理动作，避免升级后留下孤立数据。
    let _ = storage::remove(&["part", session_id, message_id, part_id]).await;
    let _ = bus::publish(
        event::PART_REMOVED,
        json!({ "sessionID": session_id, "messageID": message_id, "partID": part_id }),
        instance_directory_opt(),
    );
    Ok(())
}

/// 按 storage 前缀读取所有可解析的消息片段。
///
/// 单个 part 解析失败会被跳过，避免一条损坏记录阻止整个消息渲染。
async fn read_parts_from_prefix(prefix: &[&str]) -> Result<Vec<Part>, super::session::Error> {
    let mut out = Vec::new();
    for key in storage::list(prefix).await? {
        let key_refs = key.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        if let Ok(p) = storage::read::<Part>(&key_refs).await {
            out.push(p);
        }
    }
    Ok(out)
}

/// 读取指定消息的所有片段。
///
/// 会优先读取当前布局，读不到时回退到旧布局；返回结果按 part id 排序。
pub async fn parts(session_id: &str, message_id: &str) -> Result<Vec<Part>, super::session::Error> {
    let mut out = read_parts_from_prefix(&["part", message_id]).await?;

    if out.is_empty() {
        out = read_parts_from_prefix(&["part", session_id, message_id]).await?;
    }

    out.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(out)
}

/// 读取一条消息及其片段。
///
/// 任一必要存储读取失败都会返回 session 错误。
pub async fn get(session_id: &str, message_id: &str) -> Result<WithParts, super::session::Error> {
    let info = storage::read::<Info>(&["message", session_id, message_id]).await?;
    let parts = parts(session_id, message_id).await?;
    Ok(WithParts { info, parts })
}

/// 读取指定会话的消息列表。
///
/// `limit` 为 `Some` 时最多返回对应数量，并跳过损坏或缺失的单条消息。
/// `limit` 为 `None` 时表示完整历史读取，任何单条读取失败都会向上传播。
pub async fn messages(
    session_id: &str,
    limit: Option<usize>,
) -> Result<Vec<WithParts>, super::session::Error> {
    let mut keys = storage::list(&["message", session_id]).await?;
    keys.sort();
    let mut out = Vec::new();
    for key in keys.into_iter().rev() {
        if let Some(limit) = limit {
            if out.len() >= limit {
                break;
            }
        }
        if key.len() != 3 {
            continue;
        }
        let msg_id = key[2].clone();
        match get(session_id, &msg_id).await {
            Ok(msg) => out.push(msg),
            Err(error) if limit.is_none() => return Err(error),
            Err(_) => {}
        }
    }
    Ok(out)
}
#[cfg(test)]
#[path = "message_tests.rs"]
mod message_tests;
