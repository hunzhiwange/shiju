//! 幂等性存储模块的单元测试
//!
//! 本模块测试 `IdempotencyStore` 的核心功能，包括：
//! - 重复请求的拒绝机制
//! - 有界容量下的键淘汰策略（LRU）
//! - TTL 过期后的键重用
//! - 并发访问安全性
//!
//! 幂等性是分布式系统中的重要特性，确保同一请求多次执行产生相同结果。

use super::*;
use std::sync::Arc;

/// 测试幂等性存储拒绝重复键
///
/// 验证 `IdempotencyStore` 能够正确识别并拒绝重复的请求键。
/// 当一个键首次记录时应当返回 `true`，再次使用相同键时应当返回 `false`。
///
/// # 测试场景
/// - 首次记录 "req-1" 应该成功
/// - 再次记录 "req-1" 应该被拒绝
/// - 记录不同的键 "req-2" 应该成功
#[test]
fn idempotency_store_rejects_duplicate_key() {
    // 创建容量为 10、TTL 为 30 秒的幂等性存储
    let store = IdempotencyStore::new(Duration::from_secs(30), 10);
    // 首次记录 "req-1" 应成功
    assert!(store.record_if_new("req-1"));
    // 重复记录 "req-1" 应被拒绝
    assert!(!store.record_if_new("req-1"));
    // 记录新键 "req-2" 应成功
    assert!(store.record_if_new("req-2"));
}

/// 测试有界容量下的键淘汰策略（驱逐最旧的键）
///
/// 验证当存储达到最大容量限制时，会按照 LRU（最近最少使用）策略
/// 淘汰最早插入的键，为新的键腾出空间。
///
/// # 测试场景
/// - 创建容量为 2 的存储
/// - 依次插入 k1, k2, k3 三个键（加入短暂延时确保时间戳不同）
/// - 验证最终存储中只保留最新的 2 个键（k2 和 k3）
/// - 验证最早的键 k1 已被淘汰
///
/// # 实现细节
/// 使用 `std::thread::sleep` 确保各键插入时间有微小差异，
/// 以便验证基于时间的淘汰顺序。
#[test]
fn idempotency_store_bounded_cardinality_evicts_oldest_key() {
    // 创建容量为 2、TTL 为 300 秒的存储
    let store = IdempotencyStore::new(Duration::from_secs(300), 2);
    // 插入第一个键
    assert!(store.record_if_new("k1"));
    // 短暂延时确保时间戳不同
    std::thread::sleep(Duration::from_millis(2));
    // 插入第二个键，此时容量已满
    assert!(store.record_if_new("k2"));
    std::thread::sleep(Duration::from_millis(2));
    // 插入第三个键，应触发淘汰 k1
    assert!(store.record_if_new("k3"));

    // 验证内部状态：只保留最新的 2 个键
    let keys = store.keys.lock();
    assert_eq!(keys.len(), 2);
    // k1 应该已被淘汰
    assert!(!keys.contains_key("k1"));
    // k2 和 k3 应该保留
    assert!(keys.contains_key("k2"));
    assert!(keys.contains_key("k3"));
}

/// 测试幂等性存储接受不同的键
///
/// 验证存储能够正确处理多个不同的键，每个唯一的键都应被成功记录。
///
/// # 测试场景
/// - 创建容量为 100 的存储
/// - 依次记录 4 个不同的键（key-a 到 key-d）
/// - 验证所有键都被成功接受
#[test]
fn idempotency_store_allows_different_keys() {
    // 创建容量为 100、TTL 为 60 秒的存储
    let store = IdempotencyStore::new(Duration::from_secs(60), 100);
    // 记录不同的键，每个都应该成功
    assert!(store.record_if_new("key-a"));
    assert!(store.record_if_new("key-b"));
    assert!(store.record_if_new("key-c"));
    assert!(store.record_if_new("key-d"));
}

/// 测试最大键数参数被下限约束为 1
///
/// 验证当 `max_keys` 参数设置为 0 时，存储会将其调整为至少 1，
/// 确保存储能够正常工作而不会出现零容量的边界情况。
///
/// # 测试场景
/// - 创建 max_keys 为 0 的存储（内部应调整为 1）
/// - 记录一个键应成功
/// - 重复记录同一键应被拒绝
#[test]
fn idempotency_store_max_keys_clamped_to_one() {
    // max_keys 为 0 时，内部应钳制为 1
    let store = IdempotencyStore::new(Duration::from_secs(60), 0);
    // 唯一的键应能被记录
    assert!(store.record_if_new("only-key"));
    // 重复键应被拒绝
    assert!(!store.record_if_new("only-key"));
}

/// 测试快速连续的重复请求被拒绝
///
/// 验证在极短时间内重复提交相同的键会被立即拒绝，
/// 这是幂等性保护的核心场景。
///
/// # 测试场景
/// - 创建容量为 100 的存储
/// - 首次记录 "rapid" 应成功
/// - 立即再次记录 "rapid" 应被拒绝
#[test]
fn idempotency_store_rapid_duplicate_rejected() {
    // 创建容量为 100、TTL 为 300 秒的存储
    let store = IdempotencyStore::new(Duration::from_secs(300), 100);
    // 首次记录应成功
    assert!(store.record_if_new("rapid"));
    // 快速重复记录应被拒绝
    assert!(!store.record_if_new("rapid"));
}

/// 测试 TTL 过期后键可被重新记录
///
/// 验证当键的 TTL（生存时间）过期后，相同的键可以被重新记录。
/// 这是幂等性窗口的重要特性——幂等性保证只在特定时间窗口内有效。
///
/// # 测试场景
/// - 创建 TTL 为 1 毫秒的存储
/// - 记录 "ttl-key" 应成功
/// - 等待 10 毫秒（超过 TTL）
/// - 再次记录 "ttl-key" 应成功（因原键已过期）
#[test]
fn idempotency_store_accepts_after_ttl_expires() {
    // 创建容量为 100、TTL 仅为 1 毫秒的存储
    let store = IdempotencyStore::new(Duration::from_millis(1), 100);
    // 首次记录
    assert!(store.record_if_new("ttl-key"));
    // 等待超过 TTL 时间
    std::thread::sleep(Duration::from_millis(10));
    // TTL 过期后，相同键可被重新记录
    assert!(store.record_if_new("ttl-key"));
}

/// 测试淘汰策略保留最新的键
///
/// 验证当容量达到上限时，淘汰机制会移除最旧的键而非最新的键。
/// 这与 `bounded_cardinality_evicts_oldest_key` 测试类似，但使用更极端的容量限制。
///
/// # 测试场景
/// - 创建容量仅为 1 的存储
/// - 记录 "old-key" 应成功
/// - 等待短暂时间
/// - 记录 "new-key" 应成功（"old-key" 被淘汰）
/// - 验证只保留 "new-key"
#[test]
fn idempotency_store_eviction_preserves_newest() {
    // 创建容量仅为 1 的存储
    let store = IdempotencyStore::new(Duration::from_secs(300), 1);
    // 插入第一个键
    assert!(store.record_if_new("old-key"));
    // 确保时间戳差异
    std::thread::sleep(Duration::from_millis(2));
    // 插入第二个键，应淘汰 old-key
    assert!(store.record_if_new("new-key"));

    // 验证只保留了最新的键
    let keys = store.keys.lock();
    assert_eq!(keys.len(), 1);
    // old-key 应被淘汰
    assert!(!keys.contains_key("old-key"));
    // new-key 应保留
    assert!(keys.contains_key("new-key"));
}

/// 测试并发访问的安全性
///
/// 验证 `IdempotencyStore` 在多线程并发访问时的线程安全性。
/// 存储使用内部互斥锁保护，应能安全处理来自多个线程的并发写入。
///
/// # 测试场景
/// - 创建容量为 1000 的共享存储（使用 `Arc` 包装）
/// - 启动 10 个线程，每个线程写入 100 个不同的键
/// - 等待所有线程完成
/// - 验证最终键数量不超过 max_keys 限制
///
/// # 并发模型
/// - 使用 `Arc<IdempotencyStore>` 实现跨线程共享
/// - 每个线程操作不同前缀的键，避免竞争同一键
/// - 验证内部锁机制在压力下不会导致数据竞争或崩溃
#[test]
fn idempotency_store_concurrent_access_safe() {
    // 创建共享的幂等性存储，容量 1000
    let store = Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000));
    let mut handles = Vec::new();

    // 启动 10 个并发线程
    for i in 0..10 {
        let store = store.clone();
        handles.push(std::thread::spawn(move || {
            // 每个线程写入 100 个不同前缀的键
            for j in 0..100 {
                store.record_if_new(&format!("thread-{i}-key-{j}"));
            }
        }));
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证最终键数量不超过容量限制
    let keys = store.keys.lock();
    assert!(keys.len() <= 1000, "should respect max_keys");
}
