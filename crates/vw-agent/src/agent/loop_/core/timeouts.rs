//! 超时计算工具模块
//!
//! 本模块提供通道消息超时时间的相关计算功能，用于确保代理在执行任务时具有合理的超时限制。
//!
//! # 主要功能
//!
//! - **有效超时计算**：确保消息超时时间不低于最小阈值，防止过短的超时导致操作失败
//! - **超时预算缩放**：根据工具迭代次数动态调整超时预算，为复杂任务提供足够的执行时间
//!
//! # 设计原则
//!
//! 1. **下限保护**：所有超时值都有一个最小阈值（30秒），保证网络延迟等不可控因素不会导致误超时
//! 2. **动态缩放**：超时预算根据任务复杂度（工具迭代次数）进行缩放，但设有上限防止无限增长
//! 3. **溢出安全**：使用饱和运算（saturating operations）避免数值溢出
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::agent::agent::loop_::core::timeouts::*;
//!
//! // 计算有效的消息超时时间
//! let timeout = effective_message_timeout_secs(60);
//! assert_eq!(timeout, 60);
//!
//! // 如果配置值小于最小值，则使用最小值
//! let min_timeout = effective_message_timeout_secs(10);
//! assert_eq!(min_timeout, 30);
//!
//! // 计算超时预算
//! let budget = message_timeout_budget_secs(30, 10);
//! // budget = 30 * min(10, 4) = 30 * 4 = 120
//! ```

use super::constants::{CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP, MIN_CHANNEL_MESSAGE_TIMEOUT_SECS};

#[cfg(test)]
#[path = "timeouts_tests.rs"]
mod timeouts_tests;

/// 计算有效的消息超时时间（秒）
///
/// 此函数确保配置的超时时间不低于最小阈值，防止因超时设置过短而导致的操作失败。
/// 这是超时时间计算的"下限保护"机制。
///
/// # 参数
///
/// - `configured`: 用户配置或默认的消息超时时间（秒）
///
/// # 返回值
///
/// 返回有效的超时时间（秒），保证至少为 `MIN_CHANNEL_MESSAGE_TIMEOUT_SECS`（30秒）
///
/// # 计算逻辑
///
/// ```
/// effective = max(configured, MIN_CHANNEL_MESSAGE_TIMEOUT_SECS)
/// ```
///
/// # 示例
///
/// ```ignore
/// // 配置值大于最小值，直接返回配置值
/// assert_eq!(effective_message_timeout_secs(60), 60);
///
/// // 配置值小于最小值，返回最小值
/// assert_eq!(effective_message_timeout_secs(10), 30);
///
/// // 配置值等于最小值，返回最小值
/// assert_eq!(effective_message_timeout_secs(30), 30);
/// ```
///
/// # 设计考量
///
/// 设置最小阈值的原因为：
/// - **网络延迟**：考虑到网络请求的不确定性，需要足够的时间缓冲
/// - **服务响应**：某些外部服务可能需要较长的响应时间
/// - **错误恢复**：为重试和错误恢复预留时间
pub fn effective_message_timeout_secs(configured: u64) -> u64 {
    configured.max(MIN_CHANNEL_MESSAGE_TIMEOUT_SECS)
}

/// 计算消息超时预算（秒）
///
/// 此函数根据工具迭代次数动态计算超时预算，为复杂的多步骤任务提供充足的执行时间。
/// 这是超时时间计算的"动态缩放"机制。
///
/// # 参数
///
/// - `message_timeout_secs`: 基础消息超时时间（秒），通常来自 `effective_message_timeout_secs` 的结果
/// - `max_iterations`: 最大工具迭代次数，表示任务的复杂度
///
/// # 返回值
///
/// 返回缩放后的超时预算（秒）
///
/// # 计算逻辑
///
/// ```text
/// iterations = max(max_iterations, 1)  // 确保至少为1
/// scale = min(iterations, CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP)  // 缩放因子上限为4
/// budget = message_timeout_secs * scale  // 饱和乘法，防止溢出
/// ```
///
/// # 示例
///
/// ```ignore
/// // 迭代次数小于缩放上限，按实际迭代次数缩放
/// let budget = message_timeout_budget_secs(30, 3);
/// assert_eq!(budget, 30 * 3);  // 90秒
///
/// // 迭代次数大于缩放上限，使用缩放上限
/// let budget = message_timeout_budget_secs(30, 10);
/// assert_eq!(budget, 30 * 4);  // 120秒，而不是300秒
///
/// // 迭代次数为0，按1次计算
/// let budget = message_timeout_budget_secs(30, 0);
/// assert_eq!(budget, 30 * 1);  // 30秒
/// ```
///
/// # 设计考量
///
/// 1. **至少一次迭代**：使用 `max(1)` 确保即使配置为0次迭代也至少有基础超时时间
/// 2. **缩放上限制**：使用 `min(..., 4)` 防止超时预算无限增长，避免长时间阻塞
///    - 最大缩放因子为4，即最大超时预算 = 基础超时 * 4
///    - 对于30秒基础超时，最大预算为120秒
/// 3. **溢出安全**：使用 `saturating_mul` 进行饱和乘法，防止数值溢出
///
/// # 使用场景
///
/// 此函数通常用于：
/// - 设置整个消息处理循环的超时上限
/// - 为工具链式调用预留足够的总时间
/// - 平衡响应速度和任务完成率
pub fn message_timeout_budget_secs(message_timeout_secs: u64, max_iterations: usize) -> u64 {
    // 确保迭代次数至少为1，避免0次迭代导致预算为0
    let iterations = max_iterations.max(1) as u64;

    // 限制缩放因子不超过上限（4），防止超时预算过大
    // 这样即使迭代次数很多，超时预算也不会无限增长
    let scale = iterations.min(CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP);

    // 使用饱和乘法计算最终预算，避免数值溢出
    // saturating_mul 在溢出时返回 u64::MAX 而不是 panic
    message_timeout_secs.saturating_mul(scale)
}
