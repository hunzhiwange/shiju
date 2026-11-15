//! 网关限流器单元测试模块
//!
//! 本模块提供对网关限流器组件的全面测试覆盖，验证限流功能在各种场景下的正确性。
//!
//! # 测试覆盖范围
//!
//! - **基础限流功能**：验证请求在达到限制后会被正确阻止
//! - **滑动窗口机制**：测试时间窗口过期后的请求恢复
//! - **陈旧条目清理**：验证自动清理机制的触发和执行
//! - **边界基数控制**：测试最大键数量的限制和旧键驱逐
//! - **独立键追踪**：验证不同键的限流计数器相互独立
//! - **配对/Webhook 独立性**：确认两种限流器类型互不干扰
//! - **并发安全性**：多线程环境下的线程安全和无死锁验证
//! - **突发流量处理**：测试短时高频请求后的冷却恢复
//!
//! # 被测组件
//!
//! - [`GatewayRateLimiter`]：网关级别的组合限流器，管理配对和 webhook 两种限流
//! - [`SlidingWindowRateLimiter`]：基于滑动窗口的通用限流器实现

use super::*;
use std::sync::Arc;
use std::time::Instant;

/// 测试网关限流器在达到配对限制后阻止请求
///
/// # 测试场景
///
/// 1. 创建一个配对限制为 2 的网关限流器
/// 2. 对同一 IP 地址连续发起 3 次配对请求
/// 3. 验证前两次请求通过，第三次请求被阻止
///
/// # 预期结果
///
/// - 前两次 `allow_pair()` 调用返回 `true`
/// - 第三次 `allow_pair()` 调用返回 `false`
#[test]
fn gateway_rate_limiter_blocks_after_limit() {
    let limiter = GatewayRateLimiter::new(2, 2, 100);
    assert!(limiter.allow_pair("127.0.0.1"));
    assert!(limiter.allow_pair("127.0.0.1"));
    assert!(!limiter.allow_pair("127.0.0.1"));
}

/// 测试限流器的陈旧条目自动清理机制
///
/// # 测试场景
///
/// 1. 创建限流器并添加 3 个不同 IP 的请求记录
/// 2. 手动将最后清理时间回退到触发清理的阈值之前
/// 3. 模拟 ip-2 和 ip-3 的请求时间戳为空（陈旧状态）
/// 4. 发起新请求触发清理机制
///
/// # 预期结果
///
/// - 清理触发后，只有活跃的 ip-1 条目保留
/// - ip-2 和 ip-3 的陈旧条目被删除
/// - 条目总数从 3 减少到 1
#[test]
fn rate_limiter_sweep_removes_stale_entries() {
    let limiter = SlidingWindowRateLimiter::new(10, Duration::from_secs(60), 100);

    // 为多个 IP 添加请求记录
    assert!(limiter.allow("ip-1"));
    assert!(limiter.allow("ip-2"));
    assert!(limiter.allow("ip-3"));

    {
        let guard = limiter.requests.lock();
        assert_eq!(guard.0.len(), 3);
    }

    // 通过回退 last_sweep 时间戳强制触发清理
    {
        let mut guard = limiter.requests.lock();
        guard.1 = Instant::now()
            .checked_sub(Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS + 1))
            .unwrap();
        // 清空 ip-2 和 ip-3 的时间戳以模拟陈旧条目
        guard.0.get_mut("ip-2").unwrap().clear();
        guard.0.get_mut("ip-3").unwrap().clear();
    }

    // 下一次 allow() 调用应触发清理并移除陈旧条目
    assert!(limiter.allow("ip-1"));

    {
        let guard = limiter.requests.lock();
        assert_eq!(guard.0.len(), 1, "陈旧条目应该已被清理");
        assert!(guard.0.contains_key("ip-1"));
    }
}

/// 测试限流限制为 0 时始终允许请求
///
/// # 测试场景
///
/// 1. 创建限流限制为 0 的限流器（表示无限制）
/// 2. 对同一键连续发起 100 次请求
///
/// # 预期结果
///
/// - 所有 100 次请求都应被允许通过
/// - 限流器不应对请求产生任何阻塞
#[test]
fn rate_limiter_zero_limit_always_allows() {
    let limiter = SlidingWindowRateLimiter::new(0, Duration::from_secs(60), 10);
    for _ in 0..100 {
        assert!(limiter.allow("any-key"));
    }
}

/// 测试限流器的边界基数限制和最旧键驱逐策略
///
/// # 测试场景
///
/// 1. 创建最大键数量限制为 2 的限流器
/// 2. 依次为 ip-1、ip-2、ip-3 发起请求
///
/// # 预期结果
///
/// - 当键数量达到上限（2个）时，新请求会驱逐最旧的键
/// - ip-1 被驱逐，保留 ip-2 和 ip-3
/// - 最终只保留最近的 2 个键
#[test]
fn rate_limiter_bounded_cardinality_evicts_oldest_key() {
    let limiter = SlidingWindowRateLimiter::new(5, Duration::from_secs(60), 2);
    assert!(limiter.allow("ip-1"));
    assert!(limiter.allow("ip-2"));
    assert!(limiter.allow("ip-3"));

    let guard = limiter.requests.lock();
    assert_eq!(guard.0.len(), 2);
    assert!(guard.0.contains_key("ip-2"));
    assert!(guard.0.contains_key("ip-3"));
}

/// 测试滑动窗口过期后的请求恢复机制
///
/// # 测试场景
///
/// 1. 创建窗口大小为 50ms、限制为 2 的限流器
/// 2. 对同一 IP 快速发起 3 次请求（前 2 次通过，第 3 次被阻止）
/// 3. 等待窗口过期（60ms）
/// 4. 再次发起请求
///
/// # 预期结果
///
/// - 窗口内的前 2 次请求被允许
/// - 第 3 次请求因超出限制被阻止
/// - 窗口过期后，新请求重新被允许
#[test]
fn rate_limiter_allows_after_window_expires() {
    let window = Duration::from_millis(50);
    let limiter = SlidingWindowRateLimiter::new(2, window, 100);
    assert!(limiter.allow("ip-1"));
    assert!(limiter.allow("ip-1"));
    assert!(!limiter.allow("ip-1")); // 被阻止

    // 等待窗口过期
    std::thread::sleep(Duration::from_millis(60));

    // 应该再次被允许
    assert!(limiter.allow("ip-1"));
}

/// 测试不同键的限流计数器相互独立
///
/// # 测试场景
///
/// 1. 创建限流器，对 ip-1 发起请求直至达到限制
/// 2. 验证 ip-1 被阻止后，ip-2 的限流计数器仍从 0 开始
/// 3. 对 ip-2 同样发起请求直至达到限制
///
/// # 预期结果
///
/// - ip-1 达到限制后被阻止
/// - ip-2 的请求计数与 ip-1 完全独立
/// - 每个键各自拥有独立的限流窗口和计数器
#[test]
fn rate_limiter_independent_keys_tracked_separately() {
    let limiter = SlidingWindowRateLimiter::new(2, Duration::from_secs(60), 100);
    assert!(limiter.allow("ip-1"));
    assert!(limiter.allow("ip-1"));
    assert!(!limiter.allow("ip-1")); // ip-1 被阻止

    // ip-2 应该仍然可用
    assert!(limiter.allow("ip-2"));
    assert!(limiter.allow("ip-2"));
    assert!(!limiter.allow("ip-2")); // ip-2 现在也被阻止
}

/// 测试边界基数恰好等于最大键数量的场景
///
/// # 测试场景
///
/// 1. 创建最大键数量为 3 的限流器
/// 2. 依次添加 ip-1、ip-2、ip-3 达到容量上限
/// 3. 添加 ip-4 触发驱逐
///
/// # 预期结果
///
/// - ip-4 添加时，最旧的 ip-1 被驱逐
/// - 最终保留 ip-2、ip-3、ip-4 共 3 个键
/// - 严格遵循先进先出（FIFO）的驱逐策略
#[test]
fn rate_limiter_exact_boundary_at_max_keys() {
    let limiter = SlidingWindowRateLimiter::new(10, Duration::from_secs(60), 3);
    assert!(limiter.allow("ip-1"));
    assert!(limiter.allow("ip-2"));
    assert!(limiter.allow("ip-3"));
    // 已达到容量上限
    assert!(limiter.allow("ip-4")); // 应该驱逐 ip-1

    let guard = limiter.requests.lock();
    assert_eq!(guard.0.len(), 3);
    assert!(!guard.0.contains_key("ip-1"), "ip-1 应该已被驱逐");
    assert!(guard.0.contains_key("ip-2"));
    assert!(guard.0.contains_key("ip-3"));
    assert!(guard.0.contains_key("ip-4"));
}

/// 测试网关限流器的配对和 webhook 限流相互独立
///
/// # 测试场景
///
/// 1. 创建配对限制为 2、webhook 限制为 3 的网关限流器
/// 2. 对同一 IP 的配对请求发起至限制
/// 3. 验证同一 IP 的 webhook 请求仍可用并继续使用
///
/// # 预期结果
///
/// - 配对请求达到限制后被阻止
/// - webhook 请求使用独立的限流计数器，不受配对限制影响
/// - 两种限流类型完全隔离，互不干扰
#[test]
fn gateway_rate_limiter_pair_and_webhook_are_independent() {
    let limiter = GatewayRateLimiter::new(2, 3, 100);

    // 耗尽配对限制
    assert!(limiter.allow_pair("ip-1"));
    assert!(limiter.allow_pair("ip-1"));
    assert!(!limiter.allow_pair("ip-1")); // 配对被阻止

    // Webhook 应该仍然可用
    assert!(limiter.allow_webhook("ip-1"));
    assert!(limiter.allow_webhook("ip-1"));
    assert!(limiter.allow_webhook("ip-1"));
    assert!(!limiter.allow_webhook("ip-1")); // webhook 现在也被阻止
}

/// 测试最大键数量为 1 时的单一键限流行为
///
/// # 测试场景
///
/// 1. 创建最大键数量为 1 的限流器
/// 2. 对 ip-1 发起请求，然后对 ip-2 发起请求
///
/// # 预期结果
///
/// - ip-2 的请求会驱逐 ip-1
/// - 最终只保留最新的一个键（ip-2）
/// - 验证极端基数限制（max_keys=1）下的驱逐逻辑
#[test]
fn rate_limiter_single_key_max_allows_one_request() {
    let limiter = SlidingWindowRateLimiter::new(5, Duration::from_secs(60), 1);
    assert!(limiter.allow("ip-1"));
    assert!(limiter.allow("ip-2")); // 驱逐 ip-1

    let guard = limiter.requests.lock();
    assert_eq!(guard.0.len(), 1);
    assert!(guard.0.contains_key("ip-2"));
    assert!(!guard.0.contains_key("ip-1"));
}

/// 测试限流器在多线程并发访问下的线程安全性
///
/// # 测试场景
///
/// 1. 创建 Arc 包装的限流器实例
/// 2. 启动 10 个线程，每个线程发起 100 次不同键的请求
/// 3. 等待所有线程完成
///
/// # 预期结果
///
/// - 所有线程都能正常完成，不会发生 panic 或死锁
/// - 最终键数量不超过 max_keys 限制（1000）
/// - 验证 Mutex 保护的共享状态在并发下的正确性
#[test]
fn rate_limiter_concurrent_access_safe() {
    let limiter = Arc::new(SlidingWindowRateLimiter::new(1000, Duration::from_secs(60), 1000));
    let mut handles = Vec::new();

    for i in 0..10 {
        let limiter = limiter.clone();
        handles.push(std::thread::spawn(move || {
            for j in 0..100 {
                limiter.allow(&format!("thread-{i}-req-{j}"));
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 不应出现 panic 或死锁
    let guard = limiter.requests.lock();
    assert!(guard.0.len() <= 1000, "应遵守 max_keys 限制");
}

/// 测试突发流量后的冷却恢复机制
///
/// # 测试场景
///
/// 1. 创建窗口大小为 50ms、限制为 5 的限流器
/// 2. 快速发起 5 次请求耗尽配额（突发流量）
/// 3. 验证第 6 次请求被阻止
/// 4. 等待窗口过期（60ms）进入冷却期
/// 5. 再次发起请求
///
/// # 预期结果
///
/// - 突发阶段的 5 次请求全部通过
/// - 第 6 次请求因超出限制被阻止
/// - 冷却期结束后，请求恢复正常
#[test]
fn rate_limiter_rapid_burst_then_cooldown() {
    let limiter = SlidingWindowRateLimiter::new(5, Duration::from_millis(50), 100);

    // 突发：用尽 5 次请求
    for _ in 0..5 {
        assert!(limiter.allow("burst-ip"));
    }
    assert!(!limiter.allow("burst-ip")); // 第 6 次应失败

    // 冷却
    std::thread::sleep(Duration::from_millis(60));

    // 应该再次被允许
    assert!(limiter.allow("burst-ip"));
}
