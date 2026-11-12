//! 会话状态管理模块
//!
//! 本模块提供了会话状态的管理功能，用于跟踪和同步不同会话的运行状态。
//! 支持状态包括空闲（Idle）、重试（Retry）和忙碌（Busy）。
//!
//! # 主要功能
//!
//! - 维护全局会话状态映射表
//! - 通过事件总线发布状态变更通知
//! - 提供线程安全的状态查询和更新接口
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::session::status;
//!
//! // 设置会话状态为忙碌
//! status::set("session-123", status::Info::Busy);
//!
//! // 查询会话状态
//! let info = status::get("session-123");
//!
//! // 获取所有会话状态
//! let all_status = status::list();
//! ```

use crate::app::agent::bus;
use crate::app::agent::project::instance;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;

/// 会话状态相关的事件定义
///
/// 包含用于状态变更通知的事件类型常量。
pub mod event {
    use crate::app::agent::bus;

    /// 会话状态变更事件
    ///
    /// 当会话状态发生任何变更时发布此事件。
    /// 事件载荷包含会话ID和新状态信息。
    pub const STATUS: bus::Definition = bus::Definition { r#type: "session.status" };

    /// 会话进入空闲状态事件
    ///
    /// 当会话从其他状态转为空闲状态时发布此事件。
    /// 事件载荷包含会话ID。
    pub const IDLE: bus::Definition = bus::Definition { r#type: "session.idle" };
}

/// 会话状态信息枚举
///
/// 表示会话当前可能处于的状态。
/// 使用 serde 的 tag 机制进行序列化，确保 JSON 格式的类型安全。
///
/// # 变体说明
///
/// - `Idle` - 会话处于空闲状态，没有正在执行的任务
/// - `Retry` - 会话正在重试某个操作，包含重试次数、错误信息和下次重试时间
/// - `Busy` - 会话正在处理任务，当前忙碌
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Info {
    /// 空闲状态，会话当前无任务执行
    Idle,

    /// 重试状态，会话正在重试某个失败的操作
    ///
    /// # 字段说明
    ///
    /// - `attempt` - 当前重试次数（从1开始）
    /// - `message` - 导致重试的错误信息或描述
    /// - `next` - 下次重试的 Unix 时间戳（秒）
    Retry {
        /// 当前重试次数
        attempt: u64,
        /// 错误描述信息
        message: String,
        /// 下次重试的 Unix 时间戳（秒）
        next: u64,
    },

    /// 忙碌状态，会话正在处理任务
    Busy,
}

/// 全局会话状态存储
///
/// 使用 `LazyLock<Mutex<HashMap>>` 提供线程安全的全局状态管理。
/// 仅存储非空闲状态的会话，空闲会话会被移除。
static STATE: LazyLock<Mutex<HashMap<String, Info>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// 获取当前实例目录的可选值
///
/// 返回实例目录路径，如果目录为空字符串则返回 `None`。
/// 用于事件总线发布时确定作用域。
///
/// # 返回值
///
/// - `Some(String)` - 实例目录路径（非空字符串）
/// - `None` - 实例目录为空
fn instance_directory_opt() -> Option<String> {
    let d = instance::directory();
    // 空字符串视为无实例目录，转换为 None
    if d.is_empty() { None } else { Some(d) }
}

/// 查询指定会话的状态
///
/// 从全局状态存储中获取指定会话的当前状态。
/// 如果会话不存在于状态表中，默认返回空闲状态。
///
/// # 参数
///
/// - `session_id` - 会话唯一标识符
///
/// # 返回值
///
/// 返回该会话的当前状态信息。如果会话未在状态表中记录，返回 `Info::Idle`。
///
/// # 线程安全
///
/// 使用互斥锁保护状态访问，即使锁被污染也能安全恢复。
///
/// # 示例
///
/// ```ignore
/// let status = get("session-123");
/// match status {
///     Info::Idle => println!("会话空闲"),
///     Info::Busy => println!("会话忙碌"),
///     Info::Retry { attempt, message, next } => {
///         println!("会话重试中，第{}次，原因：{}", attempt, message);
///     }
/// }
/// ```
pub fn get(session_id: &str) -> Info {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) // 如果锁被污染，恢复并继续
        .get(session_id)
        .cloned()
        .unwrap_or(Info::Idle) // 未找到时默认返回空闲状态
}

/// 获取所有会话的状态列表
///
/// 返回当前所有非空闲会话的状态映射。
///
/// # 返回值
///
/// 返回一个 `HashMap`，键为会话ID，值为对应的状态信息。
/// 仅包含非空闲状态的会话。
///
/// # 线程安全
///
/// 使用互斥锁保护状态访问，即使锁被污染也能安全恢复。
///
/// # 示例
///
/// ```ignore
/// let all_sessions = list();
/// for (session_id, status) in all_sessions {
///     println!("会话 {} 当前状态: {:?}", session_id, status);
/// }
/// ```
pub fn list() -> HashMap<String, Info> {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Cleanup must not recover a poisoned runtime snapshot as if it were trustworthy.
pub(crate) fn cleanup_snapshot() -> Result<HashMap<String, Info>, &'static str> {
    STATE.lock().map(|state| state.clone()).map_err(|_| "Session runtime state is unavailable")
}

/// 设置会话状态
///
/// 更新指定会话的状态，并通过事件总线发布状态变更通知。
/// 如果新状态为空闲，会额外发布空闲事件并从状态表中移除该会话。
///
/// # 参数
///
/// - `session_id` - 会话唯一标识符
/// - `status` - 新的会话状态
///
/// # 事件发布
///
/// 1. 始终发布 `session.status` 事件，包含会话ID和新状态
/// 2. 如果状态为空闲，额外发布 `session.idle` 事件
///
/// # 状态管理策略
///
/// - 非空闲状态：插入或更新状态表中的记录
/// - 空闲状态：从状态表中移除记录（减少内存占用）
///
/// # 线程安全
///
/// 使用互斥锁保护状态更新，即使锁被污染也能安全恢复。
///
/// # 示例
///
/// ```ignore
/// // 设置会话为忙碌状态
/// set("session-123", Info::Busy);
///
/// // 设置会话为重试状态
/// set("session-456", Info::Retry {
///     attempt: 2,
///     message: "网络超时".to_string(),
///     next: 1234567890,
/// });
///
/// // 设置会话为空闲状态（会从状态表中移除）
/// set("session-123", Info::Idle);
/// ```
pub fn set(session_id: &str, status: Info) {
    // 发布状态变更事件到事件总线
    let _ = bus::publish(
        event::STATUS,
        json!({ "sessionID": session_id, "status": status }),
        instance_directory_opt(),
    );

    // 如果新状态为空闲，发布空闲事件并从状态表中移除
    if matches!(status, Info::Idle) {
        // 发布空闲事件
        let _ =
            bus::publish(event::IDLE, json!({ "sessionID": session_id }), instance_directory_opt());
        // 从全局状态表中移除该会话
        STATE.lock().unwrap_or_else(|e| e.into_inner()).remove(session_id);
        return;
    }

    // 非空闲状态：插入或更新状态表
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) // 如果锁被污染，恢复并继续
        .insert(session_id.to_string(), status);
}
#[cfg(test)]
#[path = "status_tests.rs"]
mod status_tests;
