//! Existing gateway-visible session results.
use message_types as message;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompactionStats {
    pub covered_messages: usize,
    pub covered_parts: usize,
    pub summary_chars: usize,
}

#[derive(Debug, Clone)]
pub enum CompactionOutcome {
    NotNeeded { history: Vec<message::WithParts> },
    Compacted { history: Vec<message::WithParts>, stats: CompactionStats },
    Failed { history: Vec<message::WithParts>, error: String },
}

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
