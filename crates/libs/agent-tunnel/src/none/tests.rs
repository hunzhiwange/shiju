//! NoneTunnel 单元测试模块
//!
//! 本模块包含对 [`NoneTunnel`] 的全面测试用例，验证空隧道实现的行为。
//!
//! # 测试范围
//!
//! - 名称返回值验证
//! - 启动行为验证（返回本地 URL）
//! - 停止行为验证（空操作）
//! - 健康检查验证（始终返回 true）
//! - 公共 URL 验证（始终返回 None）
//!
//! # 设计理念
//!
//! NoneTunnel 是一个空实现的隧道，不实际创建任何外部隧道服务。
//! 它主要用于本地开发、测试环境或不暴露公共访问的场景。

use super::*;

/// 验证隧道名称正确返回 "none"
///
/// # 测试目标
///
/// 确认 [`NoneTunnel::name`] 方法返回预期的标识字符串。
/// 此名称用于在工厂注册和日志中标识隧道类型。
#[test]
fn name_is_none() {
    let tunnel = NoneTunnel;
    assert_eq!(tunnel.name(), "none");
}

/// 验证启动隧道返回本地 URL
///
/// # 测试目标
///
/// 确认 [`NoneTunnel::start`] 方法：
/// - 接受本地主机地址和端口号
/// - 返回格式正确的本地 HTTP URL
/// - 不执行任何实际的网络操作
///
/// # 示例
///
/// 输入 `"127.0.0.1"` 和 `7788` 端口时，
/// 应返回 `"http://127.0.0.1:7788"`。
#[tokio::test]
async fn start_returns_local_url() {
    let tunnel = NoneTunnel;
    let url = tunnel.start("127.0.0.1", 7788).await.unwrap();
    assert_eq!(url, "http://127.0.0.1:7788");
}

/// 验证停止隧道为空操作且始终成功
///
/// # 测试目标
///
/// 确认 [`NoneTunnel::stop`] 方法：
/// - 不执行任何实际操作
/// - 始终返回 `Ok(())`
///
/// 由于 NoneTunnel 不建立任何外部连接，
/// 停止操作无需执行任何清理工作。
#[tokio::test]
async fn stop_is_noop_success() {
    let tunnel = NoneTunnel;
    assert!(tunnel.stop().await.is_ok());
}

/// 验证健康检查始终返回 true
///
/// # 测试目标
///
/// 确认 [`NoneTunnel::health_check`] 方法始终返回 `true`。
///
/// # 设计理由
///
/// 由于 NoneTunnel 不依赖任何外部服务，
/// 它始终处于"健康"状态。这简化了本地开发
/// 和测试环境中的健康检查逻辑。
#[tokio::test]
async fn health_check_is_always_true() {
    let tunnel = NoneTunnel;
    assert!(tunnel.health_check().await);
}

/// 验证公共 URL 始终返回 None
///
/// # 测试目标
///
/// 确认 [`NoneTunnel::public_url`] 方法返回 `None`。
///
/// # 设计理由
///
/// NoneTunnel 不创建任何公共隧道，因此不存在
/// 可从外部访问的公共 URL。调用者应正确处理
/// `None` 情况，或选择其他隧道实现以获取公共访问能力。
#[test]
fn public_url_is_always_none() {
    let tunnel = NoneTunnel;
    assert!(tunnel.public_url().is_none());
}
