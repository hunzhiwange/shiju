//! 网关限流与幂等性控制模块
//!
//! 本模块提供了网关层的流量控制和请求幂等性管理功能，确保系统在高并发场景下的稳定性和可靠性。
//!
//! ## 主要功能
//!
//! - **滑动窗口限流器**：基于滑动时间窗口的速率限制，支持多客户端并发访问控制
//! - **网关统一限流**：针对配对请求和 Webhook 请求的独立限流管理
//! - **幂等性存储**：防止重复请求导致的重复操作，保证接口调用的幂等性
//!
//! ## 设计特点
//!
//! - 线程安全：使用 `Mutex` 保护共享状态，支持多线程并发访问
//! - 内存高效：自动清理过期条目，防止内存无限增长
//! - 配置灵活：支持自定义时间窗口、限流阈值和最大跟踪键数
//!
//! ## 使用场景
//!
//! - API 网关流量控制
//! - Webhook 调用频率限制
//! - 防止重复请求处理

use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 网关限流的滑动时间窗口大小（秒）
///
/// 定义了限流统计的时间窗口长度，默认为 60 秒（1分钟）。
/// 在此时间窗口内，每个客户端的请求计数将被累计。
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// 网关限流器中跟踪的最大客户端键数的默认值
///
/// 当达到此限制时，系统会根据 LRU（最近最少使用）策略淘汰旧的条目。
/// 默认值为 10,000，适用于中等规模的并发访问场景。
pub const RATE_LIMIT_MAX_KEYS_DEFAULT: usize = 10_000;

/// 网关内存中保留的最大幂等性键数的默认值
///
/// 用于防止内存无限增长，当达到此限制时，会淘汰最旧的幂等性记录。
/// 默认值为 10,000，可根据实际业务需求调整。
pub const IDEMPOTENCY_MAX_KEYS_DEFAULT: usize = 10_000;

/// 限流器清理过期 IP 条目的扫描间隔（秒）
///
/// 为了避免频繁的全量扫描影响性能，系统会定期执行过期条目清理。
/// 默认为 300 秒（5 分钟），在内存使用和性能之间取得平衡。
pub const RATE_LIMITER_SWEEP_INTERVAL_SECS: u64 = 300;

/// 滑动窗口限流器
///
/// 实现基于滑动时间窗口的速率限制算法，为每个唯一的客户端键（如 IP 地址、用户 ID 等）
/// 维护独立的请求计数和时间戳记录。
///
/// # 工作原理
///
/// 1. 为每个客户端键维护一个时间戳列表，记录最近的请求时间
/// 2. 每次请求时，移除窗口外的时间戳，只保留窗口内的有效请求
/// 3. 如果窗口内的请求数未达到限制，则允许请求并记录时间戳
/// 4. 定期执行清理，移除长时间无活动的客户端记录
///
/// # 线程安全
///
/// 内部使用 `Mutex` 保护请求记录，支持多线程并发访问。
///
/// # 示例
///
/// ```rust,ignore
/// use std::time::Duration;
///
/// // 创建限流器：每分钟最多 100 次请求，最多跟踪 1000 个客户端
/// let limiter = SlidingWindowRateLimiter::new(100, Duration::from_secs(60), 1000);
///
/// // 检查是否允许请求
/// if limiter.allow("client_192.168.1.1") {
///     // 处理请求
/// } else {
///     // 拒绝请求
/// }
/// ```
#[derive(Debug)]
pub struct SlidingWindowRateLimiter {
    /// 时间窗口内允许的最大请求数
    limit_per_window: u32,
    /// 滑动时间窗口的持续时间
    window: Duration,
    /// 最大跟踪的客户端键数量
    max_keys: usize,
    /// 请求记录：每个客户端键对应的时间戳列表，以及上次清理时间
    pub(crate) requests: Mutex<(HashMap<String, Vec<Instant>>, Instant)>,
}

impl SlidingWindowRateLimiter {
    /// 创建新的滑动窗口限流器实例
    ///
    /// # 参数
    ///
    /// - `limit_per_window`: 在时间窗口内允许的最大请求数，设为 0 表示无限制
    /// - `window`: 滑动时间窗口的持续时间
    /// - `max_keys`: 最大跟踪的客户端键数量，至少为 1（小于 1 时自动调整为 1）
    ///
    /// # 返回值
    ///
    /// 返回初始化后的限流器实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let limiter = SlidingWindowRateLimiter::new(
    ///     100,                              // 每分钟最多 100 次
    ///     Duration::from_secs(60),          // 1 分钟窗口
    ///     5000                              // 最多跟踪 5000 个客户端
    /// );
    /// ```
    pub fn new(limit_per_window: u32, window: Duration, max_keys: usize) -> Self {
        Self {
            limit_per_window,
            window,
            max_keys: max_keys.max(1),
            requests: Mutex::new((HashMap::new(), Instant::now())),
        }
    }

    /// 清理过期的请求记录
    ///
    /// 移除所有时间戳早于截止时间的记录，以及没有任何时间戳的客户端键。
    /// 这有助于控制内存使用，避免存储大量过期的历史数据。
    ///
    /// # 参数
    ///
    /// - `requests`: 请求记录的可变引用
    /// - `cutoff`: 截止时间点，早于此时间的记录将被移除
    fn prune_stale(requests: &mut HashMap<String, Vec<Instant>>, cutoff: Instant) {
        // 保留每个键下晚于截止时间的时间戳，并移除空列表对应的键
        requests.retain(|_, timestamps| {
            timestamps.retain(|t| *t > cutoff);
            !timestamps.is_empty()
        });
    }

    /// 检查是否允许指定客户端的请求
    ///
    /// 这是限流器的核心方法，实现了完整的滑动窗口限流逻辑：
    /// 1. 如果限制为 0，直接允许（无限制模式）
    /// 2. 定期执行过期条目清理
    /// 3. 当达到最大键数时，执行淘汰策略
    /// 4. 检查当前客户端在窗口内的请求数是否超限
    ///
    /// # 参数
    ///
    /// - `key`: 客户端标识符，通常是 IP 地址、用户 ID 或其他唯一标识
    ///
    /// # 返回值
    ///
    /// - `true`: 允许此次请求，已记录请求时间戳
    /// - `false`: 拒绝此次请求，已达到速率限制
    ///
    /// # 淘汰策略
    ///
    /// 当跟踪的客户端数量达到 `max_keys` 时，会淘汰最近请求时间最早（最不活跃）的客户端，
    /// 为新客户端腾出空间。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let limiter = SlidingWindowRateLimiter::new(10, Duration::from_secs(60), 100);
    ///
    /// // 连续发送 10 次请求
    /// for _ in 0..10 {
    ///     assert!(limiter.allow("user_123"));
    /// }
    ///
    /// // 第 11 次请求应该被拒绝
    /// assert!(!limiter.allow("user_123"));
    /// ```
    pub fn allow(&self, key: &str) -> bool {
        // 如果限制为 0，表示无限制，直接允许
        if self.limit_per_window == 0 {
            return true;
        }

        let now = Instant::now();
        // 计算滑动窗口的截止时间（当前时间 - 窗口大小）
        let cutoff = now.checked_sub(self.window).unwrap_or_else(Instant::now);

        let mut guard = self.requests.lock();
        let (requests, last_sweep) = &mut *guard;

        // 周期性清理：定期移除长时间无活动的客户端键
        // 这样可以避免内存无限增长，同时不影响热点客户端的性能
        if last_sweep.elapsed() >= Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS) {
            Self::prune_stale(requests, cutoff);
            *last_sweep = now;
        }

        // 当需要添加新键且已达到最大键数限制时
        if !requests.contains_key(key) && requests.len() >= self.max_keys {
            // 在淘汰之前，先尝试清理过期的条目（机会性清理）
            // 这可能会释放出一些空间，避免不必要的淘汰
            Self::prune_stale(requests, cutoff);
            *last_sweep = now;

            // 如果清理后仍然达到限制，则执行淘汰策略
            // 淘汰最不活跃的客户端（最近请求时间最早的那个）
            if requests.len() >= self.max_keys {
                // 找到最近请求时间最早的键进行淘汰
                let evict_key = requests
                    .iter()
                    .min_by_key(|(_, timestamps)| timestamps.last().copied().unwrap_or(cutoff))
                    .map(|(k, _)| k.clone());
                if let Some(evict_key) = evict_key {
                    requests.remove(&evict_key);
                }
            }
        }

        // 获取或创建当前键的请求时间戳列表
        let entry = requests.entry(key.to_owned()).or_default();
        // 移除窗口外的时间戳，只保留有效窗口内的请求记录
        entry.retain(|instant| *instant > cutoff);

        // 检查是否已达到限流阈值
        if entry.len() >= self.limit_per_window as usize {
            return false;
        }

        // 记录此次请求的时间戳
        entry.push(now);
        true
    }
}

/// 网关统一限流器
///
/// 为网关提供统一的限流管理，针对不同类型的请求（配对请求和 Webhook 请求）
/// 维护独立的限流策略。
///
/// # 设计理由
///
/// 配对请求和 Webhook 请求通常具有不同的流量特征和重要性：
/// - 配对请求：通常是用户主动发起，需要更严格的限流保护
/// - Webhook 请求：来自外部系统的回调，可能需要不同的限流策略
///
/// # 线程安全
///
/// 内部使用两个独立的 `SlidingWindowRateLimiter`，每个都是线程安全的。
///
/// # 示例
///
/// ```rust,ignore
/// // 创建网关限流器：配对请求每分钟 60 次，Webhook 每分钟 120 次
/// let limiter = GatewayRateLimiter::new(60, 120, 10_000);
///
/// // 检查配对请求
/// if limiter.allow_pair("192.168.1.1") {
///     // 处理配对请求
/// }
///
/// // 检查 Webhook 请求
/// if limiter.allow_webhook("service_a") {
///     // 处理 Webhook 请求
/// }
/// ```
#[derive(Debug)]
pub struct GatewayRateLimiter {
    /// 配对请求的限流器
    pair: SlidingWindowRateLimiter,
    /// Webhook 请求的限流器
    webhook: SlidingWindowRateLimiter,
}

impl GatewayRateLimiter {
    /// 创建新的网关限流器实例
    ///
    /// # 参数
    ///
    /// - `pair_per_minute`: 配对请求每分钟允许的最大次数
    /// - `webhook_per_minute`: Webhook 请求每分钟允许的最大次数
    /// - `max_keys`: 每个限流器最大跟踪的客户端键数量
    ///
    /// # 返回值
    ///
    /// 返回初始化后的网关限流器实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let limiter = GatewayRateLimiter::new(
    ///     60,      // 配对请求：每分钟 60 次
    ///     120,     // Webhook 请求：每分钟 120 次
    ///     10_000   // 最多跟踪 10,000 个客户端
    /// );
    /// ```
    pub fn new(pair_per_minute: u32, webhook_per_minute: u32, max_keys: usize) -> Self {
        let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
        Self {
            pair: SlidingWindowRateLimiter::new(pair_per_minute, window, max_keys),
            webhook: SlidingWindowRateLimiter::new(webhook_per_minute, window, max_keys),
        }
    }

    /// 检查是否允许指定的配对请求
    ///
    /// # 参数
    ///
    /// - `key`: 客户端标识符（通常是 IP 地址或会话 ID）
    ///
    /// # 返回值
    ///
    /// - `true`: 允许此次配对请求
    /// - `false`: 拒绝此次配对请求（已达到速率限制）
    pub fn allow_pair(&self, key: &str) -> bool {
        self.pair.allow(key)
    }

    /// 检查是否允许指定的 Webhook 请求
    ///
    /// # 参数
    ///
    /// - `key`: 客户端或服务标识符（通常是来源服务名称或 IP 地址）
    ///
    /// # 返回值
    ///
    /// - `true`: 允许此次 Webhook 请求
    /// - `false`: 拒绝此次 Webhook 请求（已达到速率限制）
    pub fn allow_webhook(&self, key: &str) -> bool {
        self.webhook.allow(key)
    }
}

/// 幂等性存储
///
/// 用于跟踪和防止重复请求的处理，确保相同的请求不会被重复执行。
/// 这对于保证系统的一致性和可靠性至关重要，特别是在网络不稳定或客户端重试的场景下。
///
/// # 工作原理
///
/// 1. 每个请求携带唯一的幂等性键（Idempotency Key）
/// 2. 首次见到该键时，记录下来并返回 true（表示是新请求）
/// 3. 在 TTL 有效期内再次见到相同键时，返回 false（表示是重复请求）
/// 4. 超过 TTL 后，键会被自动清理，允许重新使用
///
/// # 内存管理
///
/// - 使用 TTL（生存时间）自动过期旧的幂等性记录
/// - 当达到最大键数限制时，淘汰最旧的记录
/// - 每次记录新键时都会执行过期清理
///
/// # 线程安全
///
/// 内部使用 `Mutex` 保护键存储，支持多线程并发访问。
///
/// # 示例
///
/// ```rust,ignore
/// use std::time::Duration;
///
/// // 创建幂等性存储：TTL 为 1 小时，最多存储 10,000 个键
/// let store = IdempotencyStore::new(Duration::from_secs(3600), 10_000);
///
/// // 首次请求 - 应该被允许
/// assert!(store.record_if_new("req_12345"));
///
/// // 重复请求 - 应该被拒绝
/// assert!(!store.record_if_new("req_12345"));
/// ```
#[derive(Debug)]
pub struct IdempotencyStore {
    /// 幂等性键的生存时间（TTL）
    ttl: Duration,
    /// 最大存储的幂等性键数量
    max_keys: usize,
    /// 幂等性键存储：键 -> 首次见到的时间
    pub(crate) keys: Mutex<HashMap<String, Instant>>,
}

impl IdempotencyStore {
    /// 创建新的幂等性存储实例
    ///
    /// # 参数
    ///
    /// - `ttl`: 幂等性键的生存时间，超过此时间的记录将被自动清理
    /// - `max_keys`: 最大存储的幂等性键数量，至少为 1（小于 1 时自动调整为 1）
    ///
    /// # 返回值
    ///
    /// 返回初始化后的幂等性存储实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let store = IdempotencyStore::new(
    ///     Duration::from_secs(3600),  // 1 小时 TTL
    ///     5000                         // 最多存储 5000 个键
    /// );
    /// ```
    pub fn new(ttl: Duration, max_keys: usize) -> Self {
        Self { ttl, max_keys: max_keys.max(1), keys: Mutex::new(HashMap::new()) }
    }

    /// 记录幂等性键，如果是新键则返回 true
    ///
    /// 这是幂等性检查的核心方法，实现了完整的防重逻辑：
    /// 1. 清理所有过期的幂等性记录
    /// 2. 检查键是否已存在（重复请求）
    /// 3. 如需要，淘汰最旧的键为新键腾出空间
    /// 4. 记录新键并返回 true
    ///
    /// # 参数
    ///
    /// - `key`: 幂等性键，通常是请求的唯一标识符（如 UUID）
    ///
    /// # 返回值
    ///
    /// - `true`: 这是一个新键，已被记录，请求应该被处理
    /// - `false`: 这是一个重复键，请求应该被忽略或返回之前的结果
    ///
    /// # 淘汰策略
    ///
    /// 当达到最大键数限制时，会淘汰最早记录的键（LRU 策略）。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let store = IdempotencyStore::new(Duration::from_secs(60), 100);
    ///
    /// // 首次请求 - 允许并记录
    /// assert!(store.record_if_new("order_123"));
    ///
    /// // 重复请求 - 拒绝
    /// assert!(!store.record_if_new("order_123"));
    ///
    /// // 不同的请求 - 允许并记录
    /// assert!(store.record_if_new("order_456"));
    /// ```
    pub fn record_if_new(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut keys = self.keys.lock();

        // 移除所有过期的幂等性记录（TTL 机制）
        keys.retain(|_, seen_at| now.duration_since(*seen_at) < self.ttl);

        // 如果键已存在，说明是重复请求，返回 false
        if keys.contains_key(key) {
            return false;
        }

        // 如果已达到最大键数限制，执行淘汰策略
        // 淘汰最早记录的键（最不活跃的）
        if keys.len() >= self.max_keys {
            let evict_key = keys.iter().min_by_key(|(_, seen_at)| *seen_at).map(|(k, _)| k.clone());
            if let Some(evict_key) = evict_key {
                keys.remove(&evict_key);
            }
        }

        // 记录新的幂等性键
        keys.insert(key.to_owned(), now);
        true
    }
}

#[cfg(test)]
#[path = "limits_tests.rs"]
mod limits_tests;

#[cfg(test)]
#[path = "limits_idempotency_tests.rs"]
mod idempotency_tests;
#[cfg(test)]
#[path = "limits_rate_limiter_tests.rs"]
mod rate_limiter_tests;
