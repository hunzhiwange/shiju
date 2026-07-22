//! 会话回退与撤销回退模块
//!
//! 本模块提供会话状态的回退（revert）和撤销回退（unrevert）功能。
//! 主要用于在对话过程中将系统状态恢复到之前的某个时间点，
//! 支持基于消息 ID 和部分 ID 的精确回退控制。
//!
//! # 核心功能
//!
//! - **回退（revert）**：将会话状态恢复到指定消息或消息部分之前的状态
//! - **撤销回退（unrevert）**：取消之前的回退操作，恢复到回退前的状态
//! - **清理（cleanup）**：在确认回退后，删除回退点之后的所有消息
//!
//! # 工作原理
//!
//! 1. 回退时，系统会：
//!    - 收集目标消息之后的所有补丁（patch）
//!    - 反向应用这些补丁以恢复文件状态
//!    - 记录回退前的快照，以便后续撤销
//!
//! 2. 撤销回退时，系统会：
//!    - 使用之前保存的快照恢复文件状态
//!    - 清除回退信息标记

use crate::app::agent::bus;
use crate::app::agent::project::instance;
use crate::app::agent::snapshot;
use crate::app::agent::storage;
use crate::app::agent::util::log;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::fmt;
use std::path::PathBuf;
use std::sync::LazyLock;

/// 模块专用日志记录器
///
/// 使用 `session.revert` 作为服务标识，用于记录回退操作相关的日志信息。
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("session.revert".to_string()));
    log::create(Some(tags))
});

/// 回退操作可能产生的错误类型
///
/// 封装了会话操作和快照操作可能产生的错误。
#[derive(Debug)]
pub enum Error {
    /// 会话相关错误
    Session(super::session::Error),
    /// 快照相关错误
    Snapshot(snapshot::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Session(e) => write!(f, "{}", e),
            Error::Snapshot(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<super::session::Error> for Error {
    fn from(value: super::session::Error) -> Self {
        Error::Session(value)
    }
}

impl From<snapshot::Error> for Error {
    fn from(value: snapshot::Error) -> Self {
        Error::Snapshot(value)
    }
}

/// 回退操作的输入参数
///
/// 指定要回退到的会话位置，可以精确到消息级别或消息部分级别。
///
/// # 字段说明
///
/// - `session_id`: 要操作的会话唯一标识符
/// - `message_id`: 要回退到的消息 ID
/// - `part_id`: 可选的消息部分 ID，用于更精确的回退控制
///
/// # 示例
///
/// ```ignore
/// let input = RevertInput {
///     session_id: "session-123".to_string(),
///     message_id: "msg-456".to_string(),
///     part_id: Some("part-789".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertInput {
    /// 会话 ID，指定要回退的会话
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// 消息 ID，指定要回退到的消息位置
    #[serde(rename = "messageID")]
    pub message_id: String,
    /// 消息部分 ID，可选，用于更细粒度的回退控制
    #[serde(rename = "partID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
}

/// 获取实例目录的可选值
///
/// 如果实例目录为空字符串，返回 `None`；否则返回 `Some(目录路径)`。
fn instance_directory_opt() -> Option<String> {
    let d = instance::directory();
    if d.is_empty() { None } else { Some(d) }
}

/// 获取工作树路径
///
/// 如果工作树配置为空，返回当前目录（`"."`）；否则返回配置的工作树路径。
fn worktree_path() -> PathBuf {
    let wt = instance::worktree();
    if wt.is_empty() { PathBuf::from(".") } else { PathBuf::from(wt) }
}

fn collect_revert_plan(
    all: &[super::message::WithParts],
    input: &RevertInput,
) -> Option<(super::session::RevertInfo, Vec<snapshot::Patch>)> {
    let mut last_user_id: Option<String> = None;
    let mut revert_info: Option<super::session::RevertInfo> = None;
    let mut patches: Vec<snapshot::Patch> = Vec::new();

    for msg in all {
        if matches!(msg.info, super::message::Info::User(_)) {
            last_user_id = Some(msg.info.id().to_string());
        }

        let mut remaining_useful = false;
        for part in &msg.parts {
            if revert_info.is_some() {
                if let super::message::Part::Patch(p) = part {
                    patches.push(snapshot::Patch { hash: p.hash.clone(), files: p.files.clone() });
                }
                continue;
            }

            let matches_target = (msg.info.id() == input.message_id && input.part_id.is_none())
                || input.part_id.as_deref() == Some(part.id());

            if matches_target {
                let part_id = if remaining_useful { input.part_id.clone() } else { None };
                let message_id = if part_id.is_none() {
                    last_user_id.clone().unwrap_or_else(|| msg.info.id().to_string())
                } else {
                    msg.info.id().to_string()
                };
                revert_info = Some(super::session::RevertInfo {
                    message_id,
                    part_id,
                    snapshot: None,
                    diff: None,
                });
            }

            if matches!(part, super::message::Part::Text(_) | super::message::Part::Tool(_)) {
                remaining_useful = true;
            }
        }
    }

    revert_info.map(|info| (info, patches))
}

/// 执行会话回退操作
///
/// 将会话状态回退到指定的消息或消息部分之前的状态。此函数会：
///
/// 1. 获取会话中的所有消息并按 ID 排序
/// 2. 找到目标消息/部分，收集其后的所有补丁
/// 3. 反向应用补丁以恢复文件状态
/// 4. 记录回退信息和差异统计
/// 5. 发布差异事件通知
///
/// # 参数
///
/// - `input`: 回退输入参数，包含会话 ID、消息 ID 和可选的部分 ID
///
/// # 返回值
///
/// 返回更新后的会话信息，包含回退元数据和差异统计。
///
/// # 错误
///
/// 可能返回以下错误：
/// - `Error::Session`: 会话操作失败
/// - `Error::Snapshot`: 快照操作失败
///
/// # 示例
///
/// ```ignore
/// let input = RevertInput {
///     session_id: "session-123".to_string(),
///     message_id: "msg-456".to_string(),
///     part_id: None,
/// };
/// let session_info = revert(input).await?;
/// ```
pub async fn revert(input: RevertInput) -> Result<super::session::Info, Error> {
    // 获取会话中的所有消息
    let mut all = super::message::messages(&input.session_id, None).await?;
    // 按消息 ID 排序，确保时间顺序
    all.sort_by(|a, b| a.info.id().cmp(b.info.id()));

    // 如果没有找到回退点，直接返回当前会话信息
    let Some((mut revert_info, patches)) = collect_revert_plan(&all, &input) else {
        return Ok(super::session::get(&input.session_id).await?);
    };

    // 获取当前会话信息
    let session = super::session::get(&input.session_id).await?;
    let worktree = worktree_path();

    // 获取回退前的快照
    // 如果会话已有快照，复用；否则创建新快照
    let snapshot_before =
        if let Some(existing) = session.revert.as_ref().and_then(|r| r.snapshot.clone()) {
            Some(existing)
        } else {
            snapshot::track(&worktree)?
        };

    // 保存回退前的快照
    revert_info.snapshot = snapshot_before.clone();
    // 反向应用补丁，恢复文件状态
    snapshot::revert(&worktree, &patches)?;

    // 计算回退后的差异
    if let Some(snap) = snapshot_before.clone() {
        let d = snapshot::diff(&worktree, &snap)?;
        if !d.is_empty() {
            revert_info.diff = Some(d);
        }
    }

    // 计算完整的文件差异统计
    let diffs: Vec<snapshot::FileDiff> = if let Some(from) = snapshot_before {
        let to = snapshot::track(&worktree)?;
        if let Some(to) = to { snapshot::diff_full(&worktree, &from, &to)? } else { Vec::new() }
    } else {
        Vec::new()
    };

    // 持久化差异信息到存储
    storage::write(&["session_diff", &input.session_id], &diffs)
        .await
        .map_err(|e| Error::Session(super::session::Error::Storage(e)))?;

    // 发布差异事件，通知其他组件
    let _ = bus::publish(
        super::session::event::DIFF,
        json!({ "sessionID": input.session_id, "diff": diffs }),
        instance_directory_opt(),
    );

    // 更新会话信息，添加回退元数据和差异统计
    Ok(super::session::update(&input.session_id, |draft| {
        draft.revert = Some(revert_info);
        draft.summary = Some(super::session::Summary {
            additions: diffs.iter().map(|x| x.additions).sum(),
            deletions: diffs.iter().map(|x| x.deletions).sum(),
            files: diffs.len() as i64,
            diffs: None,
        });
    })
    .await?)
}

/// 撤销回退操作
///
/// 取消之前的回退操作，将会话状态恢复到回退前的状态。
/// 如果存在回退前保存的快照，将使用该快照恢复文件状态。
///
/// # 参数
///
/// - `session_id`: 要撤销回退的会话 ID
///
/// # 返回值
///
/// 返回更新后的会话信息，回退信息将被清除。
///
/// # 错误
///
/// 可能返回以下错误：
/// - `Error::Session`: 会话操作失败
/// - `Error::Snapshot`: 快照恢复失败
///
/// # 示例
///
/// ```ignore
/// let session_info = unrevert("session-123").await?;
/// ```
pub async fn unrevert(session_id: &str) -> Result<super::session::Info, Error> {
    // 记录撤销回退操作日志
    LOGGER.info("unreverting", Some(extra([("sessionID", Value::String(session_id.to_string()))])));

    // 获取当前会话信息
    let session = super::session::get(session_id).await?;

    // 如果没有回退信息，直接返回当前会话
    let Some(revert) = session.revert.clone() else {
        return Ok(session);
    };

    // 如果存在快照，使用快照恢复文件状态
    if let Some(snapshot) = revert.snapshot {
        let worktree = worktree_path();
        snapshot::restore(&worktree, &snapshot)?;
    }

    // 更新会话，清除回退信息
    Ok(super::session::update(session_id, |draft| {
        draft.revert = None;
    })
    .await?)
}

/// 清理回退后的会话消息
///
/// 在确认回退操作后，删除回退点之后的所有消息。
/// 如果回退到消息的某个部分，还会删除该部分之后的内容。
///
/// # 参数
///
/// - `session`: 要清理的会话信息
///
/// # 返回值
///
/// 成功时返回 `Ok(())`。
///
/// # 错误
///
/// 可能返回以下错误：
/// - `Error::Session`: 消息操作失败
///
/// # 示例
///
/// ```ignore
/// let session = session::get("session-123").await?;
/// cleanup(&session).await?;
/// ```
pub async fn cleanup(session: &super::session::Info) -> Result<(), Error> {
    let Some(revert) = session.revert.as_ref() else {
        return Ok(());
    };

    cleanup_after_message(session.id.as_str(), &revert.message_id).await?;

    if let Some(part_id) = revert.part_id.as_deref() {
        let session_id = session.id.as_str();
        let mut msgs = super::message::messages(session_id, None).await?;
        msgs.sort_by(|a, b| a.info.id().cmp(b.info.id()));
        if let Some(last) = msgs.last() {
            let msg_id = last.info.id();
            let parts = super::message::parts(session_id, msg_id).await?;
            let mut hit = false;

            for part in parts {
                if !hit && part.id() == part_id {
                    hit = true;
                }
                if hit {
                    super::message::remove_part(session_id, msg_id, part.id()).await?;
                }
            }
        }
    }

    let _ = super::session::update(session.id.as_str(), |draft| {
        draft.revert = None;
    })
    .await?;

    Ok(())
}

pub async fn cleanup_from_message(session_id: &str, message_id: &str) -> Result<(), Error> {
    cleanup_message_range(session_id, message_id, true).await
}

pub async fn cleanup_after_message(session_id: &str, message_id: &str) -> Result<(), Error> {
    cleanup_message_range(session_id, message_id, false).await
}

fn collect_cleanup_messages(
    msgs: Vec<super::message::WithParts>,
    message_id: &str,
    include_target: bool,
) -> Vec<super::message::WithParts> {
    let mut remove = Vec::new();
    let mut hit = false;

    for msg in msgs {
        if !hit && msg.info.id() == message_id {
            hit = true;
            if !include_target {
                continue;
            }
        }
        if hit {
            remove.push(msg);
        }
    }

    remove
}

async fn cleanup_message_range(
    session_id: &str,
    message_id: &str,
    include_target: bool,
) -> Result<(), Error> {
    let mut msgs = super::message::messages(session_id, None).await?;
    msgs.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    let remove = collect_cleanup_messages(msgs, message_id, include_target);

    for msg in remove {
        let message_id = msg.info.id().to_string();
        for part in msg.parts {
            super::message::remove_part(session_id, &message_id, part.id()).await?;
        }
        super::message::remove_message(session_id, &message_id).await?;
    }

    Ok(())
}

/// 创建日志额外的字段映射
///
/// 将键值对数组转换为 JSON 对象，用于日志记录。
///
/// # 参数
///
/// - `pairs`: 键值对数组
///
/// # 返回值
///
/// 返回包含所有键值对的 JSON Map。
fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}
#[cfg(test)]
#[path = "revert_tests.rs"]
mod revert_tests;
