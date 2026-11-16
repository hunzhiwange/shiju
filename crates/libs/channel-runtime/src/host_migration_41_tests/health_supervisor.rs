//! # 健康监督器测试模块
//!
//! 本模块提供了对通道健康检查结果分类功能的单元测试。
//!
//! ## 测试目标
//!
//! 本模块测试 [`classify_health_result`] 函数的正确性，该函数负责将健康检查
//! 的原始结果分类为 [`ChannelHealthState`] 枚举的三种状态之一：
//!
//! - **健康（Healthy）**：通道健康检查成功返回 `true`
//! - **不健康（Unhealthy）**：通道健康检查返回 `false`
//! - **超时（Timeout）**：健康检查操作超时未响应
//!
//! ## 测试覆盖
//!
//! - [`classify_health_ok_true`]：验证成功返回的健康状态
//! - [`classify_health_ok_false`]：验证失败返回的不健康状态
//! - [`classify_health_timeout`]：验证超时场景的状态分类
//!
//! [`classify_health_result`]: crate::manager::health::classify_health_result
//! [`ChannelHealthState`]: crate::ChannelHealthState

use super::*;

/// 测试健康检查返回 Ok(true) 的分类
///
/// 验证当健康检查成功完成并返回 `true` 时，`classify_health_result`
/// 函数应该正确地将其分类为 [`ChannelHealthState::Healthy`] 状态。
///
/// # 测试场景
///
/// - 输入：`Ok(true)` - 表示通道健康检查成功且通道状态良好
/// - 期望输出：`ChannelHealthState::Healthy`
///
/// # 示例
///
/// ```ignore
/// let result: Result<bool, tokio::time::error::Elapsed> = Ok(true);
/// let state = classify_health_result(&result);
/// assert_eq!(state, ChannelHealthState::Healthy);
/// ```
///
/// [`ChannelHealthState::Healthy`]: crate::ChannelHealthState::Healthy
#[test]
fn classify_health_ok_true() {
    // 准备测试数据：模拟健康检查成功返回 true
    let state = classify_health_result(&Ok(true));
    // 验证分类结果为健康状态
    assert_eq!(state, ChannelHealthState::Healthy);
}

/// 测试健康检查返回 Ok(false) 的分类
///
/// 验证当健康检查成功完成但返回 `false` 时，`classify_health_result`
/// 函数应该正确地将其分类为 [`ChannelHealthState::Unhealthy`] 状态。
///
/// # 测试场景
///
/// - 输入：`Ok(false)` - 表示通道健康检查完成但检测到问题
/// - 期望输出：`ChannelHealthState::Unhealthy`
///
/// # 说明
///
/// 健康检查返回 `false` 可能表示：
/// - 认证失败
/// - 配置错误
/// - 网络连接问题
/// - 通道服务不可用
///
/// [`ChannelHealthState::Unhealthy`]: crate::ChannelHealthState::Unhealthy
#[test]
fn classify_health_ok_false() {
    // 准备测试数据：模拟健康检查返回 false（检测到问题）
    let state = classify_health_result(&Ok(false));
    // 验证分类结果为不健康状态
    assert_eq!(state, ChannelHealthState::Unhealthy);
}

/// 测试健康检查超时的分类
///
/// 验证当健康检查操作超时（返回 `Err(Elapsed)`）时，
/// `classify_health_result` 函数应该正确地将其分类为
/// [`ChannelHealthState::Timeout`] 状态。
///
/// # 测试场景
///
/// - 输入：`Err(Elapsed)` - 表示健康检查未在指定时间内完成
/// - 期望输出：`ChannelHealthState::Timeout`
///
/// # 测试方法
///
/// 本测试使用异步超时机制来模拟超时场景：
/// 1. 设置一个极短的超时时间（1毫秒）
/// 2. 在异步块中执行一个更长的延迟（20毫秒）
/// 3. 这会导致 `tokio::time::timeout` 返回 `Err(Elapsed)`
/// 4. 验证分类函数正确识别为超时状态
///
/// # 示例
///
/// ```ignore
/// use tokio::time::{timeout, Duration};
///
/// let result = timeout(Duration::from_millis(1), async {
///     tokio::time::sleep(Duration::from_millis(20)).await;
///     true
/// }).await;
///
/// let state = classify_health_result(&result);
/// assert_eq!(state, ChannelHealthState::Timeout);
/// ```
///
/// [`ChannelHealthState::Timeout`]: crate::ChannelHealthState::Timeout
#[tokio::test]
async fn classify_health_timeout() {
    // 模拟超时场景：
    // 设置 1 毫秒超时，但异步操作需要 20 毫秒才能完成
    let result = tokio::time::timeout(Duration::from_millis(1), async {
        // 模拟一个耗时的健康检查操作
        tokio::time::sleep(Duration::from_millis(20)).await;
        true
    })
    .await;

    // 将超时结果传递给分类函数
    let state = classify_health_result(&result);
    // 验证分类结果为超时状态
    assert_eq!(state, ChannelHealthState::Timeout);
}
