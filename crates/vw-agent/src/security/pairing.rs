//! # 网关 skey 鉴权管理模块
//!
//! 本模块保留历史 `PairingGuard` 类型名，但运行时鉴权已切换为服务端配置的 skey。
//!
//! ## 工作原理
//!
//! 1. **配置阶段**：服务端读取 `[gateway].skeys` 配置。
//! 2. **哈希存储**：原始 skey 只在加载时使用，运行时仅保留 `skey_hash` 和过期时间。
//! 3. **请求认证**：客户端通过 `Authorization: Bearer <skey>` 携带 skey。
//! 4. **默认关闭**：`auth_enabled = false` 时不要求 skey，开启后才校验。
//!
//! ## 安全特性
//!
//! - **可选过期时间**：过期 skey 自动失效
//! - **恒定时间比较**：使用恒定时间算法比较 skey 哈希，防止时序攻击
//! - **skey 哈希存储**：skey 以 SHA-256 哈希形式存储，避免明文暴露
//! - **内存边界保护**：限制追踪的客户端数量，防止内存无限增长

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use vw_config_types::gateway::GatewaySkey;

/// 配对尝试失败的最大次数阈值
///
/// 超过此次数后，该客户端将被锁定，无法继续尝试配对
const MAX_PAIR_ATTEMPTS: u32 = 5;

/// 配对锁定持续时间（秒）
///
/// 客户端因暴力破解被锁定后的等待时间（5分钟）
const PAIR_LOCKOUT_SECS: u64 = 300;

/// 最大追踪客户端数量
///
/// 限制内存中保存的失败尝试记录数量，防止内存无限增长
const MAX_TRACKED_CLIENTS: usize = 10_000;

/// 失败尝试记录的保留时间（秒）
///
/// 超过此时间无活动的失败尝试记录将被清理（15分钟）
const FAILED_ATTEMPT_RETENTION_SECS: u64 = 900;

/// 失败尝试记录清理扫描的最小间隔（秒）
///
/// 定期清理过期记录的时间间隔（5分钟）
const FAILED_ATTEMPT_SWEEP_INTERVAL_SECS: u64 = 300;

/// 单个客户端的失败尝试状态
///
/// 跟踪每个客户端的配对失败次数、锁定状态和最后尝试时间，
/// 用于实现防暴力破解机制。
#[derive(Debug, Clone, Copy)]
struct FailedAttemptState {
    /// 累计失败次数
    count: u32,
    /// 锁定截止时间（如果被锁定的话）
    lockout_until: Option<Instant>,
    /// 最后一次尝试的时间戳
    last_attempt: Instant,
}

#[derive(Debug, Clone)]
struct SkeyEntry {
    enabled: bool,
    hash: String,
    expires_at: Option<DateTime<Utc>>,
}

/// 网关配对状态管理器
///
/// 负责管理整个配对流程的状态，包括配对码、已配对令牌和防暴力破解机制。
///
/// ## 令牌存储策略
///
/// 持有者令牌以 SHA-256 哈希形式存储，防止配置文件中暴露明文令牌。
/// 当生成新令牌时，明文令牌仅返回给客户端一次，服务器仅保留哈希值。
///
/// ## 线程安全性
///
/// 使用 `parking_lot::Mutex` 保证线程安全。
/// 注意：未来应考虑迁移到异步互斥锁（flume 或 tokio::sync::Mutex）
///
/// # 示例
///
/// ```no_run
/// use vibe_agent::security::pairing::PairingGuard;
///
/// // 创建需要配对的管理器
/// let guard = PairingGuard::new(true, &[]);
///
/// // 获取配对码
/// if let Some(code) = guard.pairing_code() {
///     println!("配对码: {}", code);
/// }
///
/// // 尝试配对
/// let result = guard.try_pair("123456", "client-1").await;
/// ```
#[derive(Debug, Clone)]
pub struct PairingGuard {
    /// 是否启用配对要求
    ///
    /// 为 `false` 时，所有请求都视为已认证
    require_pairing: Arc<AtomicBool>,

    /// 一次性配对码
    ///
    /// 启动时生成，首次成功配对后会被消费（置为 `None`）
    pairing_code: Arc<Mutex<Option<String>>>,

    /// 网关 skey 哈希集合
    ///
    /// 以 SHA-256 哈希形式存储，原始 skey 不进入运行时持久状态
    skeys: Arc<Mutex<Vec<SkeyEntry>>>,

    /// 防暴力破解状态
    ///
    /// 包含：客户端失败尝试记录映射 + 上次清理扫描的时间戳
    failed_attempts: Arc<Mutex<(HashMap<String, FailedAttemptState>, Instant)>>,
}

impl PairingGuard {
    /// 创建新的配对管理器实例
    ///
    /// # 参数
    ///
    /// - `require_pairing`: 是否启用配对要求
    ///   - `true`: 需要配对认证，若无现有令牌则生成新配对码
    ///   - `false`: 禁用配对，所有请求都视为已认证
    /// - `existing_tokens`: 现有的令牌列表（从配置加载）
    ///   - 支持两种格式：
    ///     - 明文令牌（`zc_...`）：加载时自动哈希处理（向后兼容）
    ///     - 已哈希令牌（64字符十六进制）：直接存储
    ///
    /// # 返回
    ///
    /// 返回初始化完成的 `PairingGuard` 实例
    ///
    /// # 示例
    ///
    /// ```no_run
    /// // 无现有令牌，创建需要配对的管理器
    /// let guard = PairingGuard::new(true, &[]);
    /// assert!(guard.pairing_code().is_some());
    ///
    /// // 有现有令牌，跳过配对码生成
    /// let guard = PairingGuard::new(true, &["existing_token_hash".to_string()]);
    /// assert!(guard.pairing_code().is_none());
    /// ```
    pub fn new(require_pairing: bool, existing_tokens: &[String]) -> Self {
        // 处理现有令牌：识别格式并统一转换为哈希形式
        let tokens: HashSet<String> = existing_tokens
            .iter()
            .map(|t| if is_token_hash(t) { t.clone() } else { hash_token(t) })
            .collect();
        let skeys = tokens
            .iter()
            .map(|hash| SkeyEntry { enabled: true, hash: hash.clone(), expires_at: None })
            .collect::<Vec<_>>();

        // 仅在需要配对且无现有令牌时生成配对码
        let code = if require_pairing && tokens.is_empty() { Some(generate_code()) } else { None };

        Self {
            require_pairing: Arc::new(AtomicBool::new(require_pairing)),
            pairing_code: Arc::new(Mutex::new(code)),
            skeys: Arc::new(Mutex::new(skeys)),
            failed_attempts: Arc::new(Mutex::new((HashMap::new(), Instant::now()))),
        }
    }

    /// 使用服务端网关 skey 配置创建鉴权管理器。
    ///
    /// 原始 skey 如果出现在配置对象中，只会在这里被哈希后使用；运行时状态只保留哈希和过期时间。
    pub fn from_skeys(auth_enabled: bool, configured_skeys: &[GatewaySkey]) -> Self {
        let skeys = configured_skeys.iter().filter_map(skey_entry_from_config).collect::<Vec<_>>();

        Self {
            require_pairing: Arc::new(AtomicBool::new(auth_enabled)),
            pairing_code: Arc::new(Mutex::new(None)),
            skeys: Arc::new(Mutex::new(skeys)),
            failed_attempts: Arc::new(Mutex::new((HashMap::new(), Instant::now()))),
        }
    }

    /// 热更新服务端网关 skey 鉴权配置。
    ///
    /// Gateway 进程启动后，桌面端或 dashboard 可能会更新 `auth_enabled` / `skeys`。
    /// 该方法让运行中的 handler 立即看到新状态，无需重启 gateway。
    pub fn update_from_skeys(&self, auth_enabled: bool, configured_skeys: &[GatewaySkey]) {
        let skeys = configured_skeys.iter().filter_map(skey_entry_from_config).collect::<Vec<_>>();
        *self.skeys.lock() = skeys;
        self.require_pairing.store(auth_enabled, Ordering::Relaxed);
    }

    /// 获取当前的一次性配对码
    ///
    /// 仅在尚未配对（无任何已配对令牌）时返回配对码。
    /// 配对成功后，配对码会被消费，此方法将返回 `None`。
    ///
    /// # 返回
    ///
    /// - `Some(String)`: 6位数字配对码
    /// - `None`: 已配对或不需要配对
    pub fn pairing_code(&self) -> Option<String> {
        self.pairing_code.lock().clone()
    }

    /// 返回当前可用的引导配对码，必要时为本地 bootstrap 重新生成一个。
    ///
    /// 与 [`pairing_code`](Self::pairing_code) 不同，这个方法在已经存在已配对令牌时
    /// 也允许重新生成一次性配对码，供旧版受信任的本地 loopback 客户端重新获取令牌。
    ///
    /// # 返回
    ///
    /// - `Some(String)`: 当前可用的 6 位数字配对码
    /// - `None`: 未启用配对
    pub fn ensure_pairing_code(&self) -> Option<String> {
        if !self.require_pairing() {
            return None;
        }

        let mut pairing_code = self.pairing_code.lock();
        if pairing_code.is_none() {
            *pairing_code = Some(generate_code());
        }
        pairing_code.clone()
    }

    /// 检查是否启用了配对要求
    ///
    /// # 返回
    ///
    /// - `true`: 需要配对认证
    /// - `false`: 禁用配对，所有请求都视为已认证
    pub fn require_pairing(&self) -> bool {
        self.require_pairing.load(Ordering::Relaxed)
    }

    /// 检查服务端网关是否启用了 skey 鉴权。
    pub fn auth_enabled(&self) -> bool {
        self.require_pairing()
    }

    /// 尝试配对的内部阻塞实现
    ///
    /// 执行配对验证的核心逻辑，包括：
    /// 1. 检查客户端是否被锁定
    /// 2. 验证配对码
    /// 3. 成功时生成并返回令牌
    /// 4. 失败时更新失败计数
    ///
    /// # 参数
    ///
    /// - `code`: 客户端提交的配对码
    /// - `client_id`: 客户端标识（用于追踪失败尝试）
    ///
    /// # 返回
    ///
    /// - `Ok(Some(token))`: 配对成功，返回新生成的持有者令牌
    /// - `Ok(None)`: 配对失败（配对码错误）
    /// - `Err(remaining_secs)`: 客户端被锁定，返回剩余锁定时间（秒）
    fn try_pair_blocking(&self, code: &str, client_id: &str) -> Result<Option<String>, u64> {
        let client_id = normalize_client_key(client_id);
        let now = Instant::now();

        // 阶段1：定期清理和锁定检查
        {
            let mut guard = self.failed_attempts.lock();
            let (ref mut map, ref mut last_sweep) = *guard;

            // 按间隔清理过期的失败尝试记录
            if now.duration_since(*last_sweep).as_secs() >= FAILED_ATTEMPT_SWEEP_INTERVAL_SECS {
                prune_failed_attempts(map, now);
                *last_sweep = now;
            }

            // 检查该客户端是否被锁定
            if let Some(state) = map.get(&client_id) {
                if let Some(until) = state.lockout_until {
                    if now < until {
                        // 仍在锁定期内，返回剩余时间
                        let remaining = (until - now).as_secs();
                        return Err(remaining.max(1));
                    }
                    // 锁定期已过，移除记录
                    map.remove(&client_id);
                }
            }
        }

        // 阶段2：验证配对码
        {
            let mut pairing_code = self.pairing_code.lock();
            if let Some(ref expected) = *pairing_code {
                // 使用恒定时间比较防止时序攻击
                if constant_time_eq(code.trim(), expected.trim()) {
                    // 配对成功：清除该客户端的失败记录
                    {
                        let mut guard = self.failed_attempts.lock();
                        guard.0.remove(&client_id);
                    }

                    // 生成新令牌并存储哈希值
                    let token = generate_token();
                    let mut skeys = self.skeys.lock();
                    let hash = hash_token(&token);
                    if !skeys.iter().any(|entry| entry.hash == hash) {
                        skeys.push(SkeyEntry { enabled: true, hash, expires_at: None });
                    }

                    // 消费配对码，防止重复使用
                    *pairing_code = None;

                    return Ok(Some(token));
                }
            }
        }

        // 阶段3：配对失败，更新失败计数
        {
            let mut guard = self.failed_attempts.lock();
            let (ref mut map, _) = *guard;

            // 容量管理：先清理过期记录，再按LRU策略驱逐
            if map.len() >= MAX_TRACKED_CLIENTS {
                prune_failed_attempts(map, now);
            }
            if map.len() >= MAX_TRACKED_CLIENTS {
                // 驱逐最久未活跃的条目
                if let Some(lru_key) =
                    map.iter().min_by_key(|(_, s)| s.last_attempt).map(|(k, _)| k.clone())
                {
                    map.remove(&lru_key);
                }
            }

            // 更新或创建失败记录
            let entry = map.entry(client_id).or_insert(FailedAttemptState {
                count: 0,
                lockout_until: None,
                last_attempt: now,
            });

            entry.last_attempt = now;
            entry.count += 1;

            // 达到阈值时实施锁定
            if entry.count >= MAX_PAIR_ATTEMPTS {
                entry.lockout_until = Some(now + std::time::Duration::from_secs(PAIR_LOCKOUT_SECS));
            }
        }

        Ok(None)
    }

    /// 尝试使用配对码进行配对
    ///
    /// 这是配对流程的主要入口点。客户端提交配对码，如果正确则获得持有者令牌。
    ///
    /// # 参数
    ///
    /// - `code`: 6位数字配对码（从终端获取）
    /// - `client_id`: 客户端唯一标识（用于防暴力破解追踪）
    ///   - 建议使用 IP 地址或其他可识别信息
    ///
    /// # 返回
    ///
    /// - `Ok(Some(token))`: 配对成功，返回新生成的持有者令牌
    ///   - 令牌格式：`zc_` 前缀 + 64字符十六进制
    /// - `Ok(None)`: 配对失败（配对码错误或已过期）
    /// - `Err(lockout_seconds)`: 客户端被锁定，返回剩余锁定秒数
    ///
    /// # 平台差异
    ///
    /// - 非WASM平台：使用 `spawn_blocking` 在专用线程执行
    /// - WASM平台：直接执行（无异步运行时）
    ///
    /// # 示例
    ///
    /// ```no_run
    /// let result = guard.try_pair("123456", "192.168.1.100").await;
    /// match result {
    ///     Ok(Some(token)) => println!("配对成功，令牌: {}", token),
    ///     Ok(None) => println!("配对码错误"),
    ///     Err(secs) => println!("被锁定，请等待 {} 秒", secs),
    /// }
    /// ```
    pub async fn try_pair(&self, code: &str, client_id: &str) -> Result<Option<String>, u64> {
        let this = self.clone();
        let code = code.to_string();
        let client_id = client_id.to_string();

        #[cfg(not(target_arch = "wasm32"))]
        {
            // 非WASM平台：使用阻塞线程池执行
            let handle =
                tokio::task::spawn_blocking(move || this.try_pair_blocking(&code, &client_id));

            match handle.await {
                Ok(result) => result,
                Err(err) => {
                    tracing::error!("配对工作线程失败: {err}");
                    Ok(None)
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // WASM平台：直接执行
            this.try_pair_blocking(&code, &client_id)
        }
    }

    /// 验证 skey 是否有效
    ///
    /// 将提供的 skey 进行哈希后，与已存储的 skey 哈希集合比对。
    ///
    /// # 参数
    ///
    /// - `token`: 待验证的 skey
    ///
    /// # 返回
    ///
    /// - `true`: skey 有效或不需要鉴权
    /// - `false`: skey 无效、缺失或已过期
    ///
    /// # 注意
    ///
    /// 如果 `require_pairing` 为 `false`，此方法始终返回 `true`
    pub fn is_authenticated(&self, token: &str) -> bool {
        if !self.require_pairing() {
            return true;
        }
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return false;
        }
        let hashed = hash_token(trimmed);
        let now = Utc::now();
        let skeys = self.skeys.lock();
        skeys.iter().any(|entry| {
            entry.enabled
                && !entry.expires_at.as_ref().is_some_and(|expires_at| now >= *expires_at)
                && constant_time_eq(&entry.hash, &hashed)
        })
    }

    /// 检查网关是否已完成配对
    ///
    /// 判断是否至少有一个有效的已配对令牌。
    ///
    /// # 返回
    ///
    /// - `true`: 已有至少一个配对的令牌
    /// - `false`: 尚未配对
    pub fn is_paired(&self) -> bool {
        self.active_skey_count() > 0
    }

    /// 返回当前未过期 skey 数量。
    pub fn active_skey_count(&self) -> usize {
        let now = Utc::now();
        let skeys = self.skeys.lock();
        skeys
            .iter()
            .filter(|entry| {
                entry.enabled
                    && !entry.expires_at.as_ref().is_some_and(|expires_at| now >= *expires_at)
            })
            .count()
    }

    /// 获取所有已配对令牌的哈希值
    ///
    /// 用于将令牌持久化到配置文件。
    ///
    /// # 返回
    ///
    /// 返回所有令牌哈希的向量（64字符十六进制字符串）
    ///
    /// # 注意
    ///
    /// 返回的是哈希值，不是原始令牌。原始令牌仅在配对时返回一次。
    pub fn tokens(&self) -> Vec<String> {
        let skeys = self.skeys.lock();
        skeys.iter().map(|entry| entry.hash.clone()).collect()
    }
}

fn skey_entry_from_config(config: &GatewaySkey) -> Option<SkeyEntry> {
    let raw_skey = config.skey.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let hash = match raw_skey {
        Some(skey) => hash_token(skey),
        None => config.skey_hash.trim().to_ascii_lowercase(),
    };
    if !is_token_hash(&hash) {
        return None;
    }

    let expires_at = match config.expires_at.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => match DateTime::parse_from_rfc3339(value) {
            Ok(parsed) => Some(parsed.with_timezone(&Utc)),
            Err(_) => return None,
        },
        _ => None,
    };

    Some(SkeyEntry { enabled: config.enabled, hash, expires_at })
}

/// 规范化客户端标识符
///
/// 去除首尾空白字符，并将空字符串映射为 `"unknown"`。
///
/// # 参数
///
/// - `key`: 原始客户端标识符
///
/// # 返回
///
/// 规范化后的客户端标识符
fn normalize_client_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() { "unknown".to_string() } else { trimmed.to_string() }
}

/// 清理过期的失败尝试记录
///
/// 移除 `last_attempt` 早于保留窗口的条目，防止内存无限增长。
///
/// # 参数
///
/// - `map`: 失败尝试记录映射
/// - `now`: 当前时间戳
fn prune_failed_attempts(map: &mut HashMap<String, FailedAttemptState>, now: Instant) {
    map.retain(|_, state| {
        now.duration_since(state.last_attempt).as_secs() < FAILED_ATTEMPT_RETENTION_SECS
    });
}

/// 生成6位数字配对码
///
/// 使用密码学安全随机数生成器生成均匀分布的6位数字配对码。
///
/// # 算法说明
///
/// 1. 使用 UUID v4 作为随机源（底层使用 `/dev/urandom`、`BCryptGenRandom` 或 `SecRandomCopyBytes`）
/// 2. 从 UUID 提取4字节转换为 u32
/// 3. 使用拒绝采样消除模偏倚：
///    - 仅接受小于 `REJECT_THRESHOLD` 的值
///    - 拒绝概率约0.02%，几乎总在第一次循环就返回
///
/// # 返回
///
/// 6位数字字符串（000000-999999）
fn generate_code() -> String {
    // 模运算的上界（1,000,000种可能）
    const UPPER_BOUND: u32 = 1_000_000;
    // 拒绝阈值：u32范围内1,000,000的最大倍数
    const REJECT_THRESHOLD: u32 = (u32::MAX / UPPER_BOUND) * UPPER_BOUND;

    loop {
        // 生成随机UUID
        let uuid = uuid::Uuid::new_v4();
        let bytes = uuid.as_bytes();
        // 提取前4字节作为随机数
        let raw = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

        // 拒绝采样：仅接受低于阈值的值以消除模偏倚
        if raw < REJECT_THRESHOLD {
            return format!("{:06}", raw % UPPER_BOUND);
        }
    }
}

/// 生成具有256位熵的持有者令牌
///
/// 使用操作系统密码学安全随机数生成器生成高强度令牌。
///
/// # 随机源
///
/// - Linux: `/dev/urandom`
/// - Windows: `BCryptGenRandom`
/// - macOS: `SecRandomCopyBytes`
///
/// # 格式
///
/// - 前缀：`zc_`（VibeWindow标识）
/// - 主体：32字节随机数的十六进制编码（64字符）
/// - 总长度：67字符
/// - 熵：256位
///
/// # 返回
///
/// 格式为 `zc_<64位十六进制>` 的令牌字符串
fn generate_token() -> String {
    let bytes: [u8; 32] = rand::random();
    format!("zc_{}", hex::encode(bytes))
}

/// 对令牌进行 SHA-256 哈希
///
/// 用于安全存储令牌，避免配置文件中暴露明文。
///
/// # 参数
///
/// - `token`: 原始令牌字符串
///
/// # 返回
///
/// 小写十六进制格式的 SHA-256 哈希值（64字符）
fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// 对原始 skey 进行 SHA-256 哈希，供配置保存前归一化使用。
pub fn hash_skey(skey: &str) -> String {
    hash_token(skey.trim())
}

/// 生成 skey 的脱敏展示名。
pub fn masked_skey_name(skey: &str) -> String {
    let trimmed = skey.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let chars = trimmed.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "*".repeat(chars.len().max(1));
    }

    let prefix = chars.iter().take(4).collect::<String>();
    let suffix =
        chars.iter().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<String>();
    format!("{prefix}***{suffix}")
}

/// 检查存储值是否为 SHA-256 哈希格式
///
/// 通过长度（64字符）和字符集（十六进制）判断。
///
/// # 参数
///
/// - `value`: 待检查的字符串
///
/// # 返回
///
/// - `true`: 符合 SHA-256 哈希格式（64个十六进制字符）
/// - `false`: 可能是明文令牌
fn is_token_hash(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// 恒定时间字符串比较
///
/// 使用恒定时间算法比较两个字符串，防止通过执行时间推断内容的时序攻击。
///
/// # 算法特点
///
/// 1. 不在长度不匹配时提前返回，总是遍历较长的输入
/// 2. 使用 XOR 累积差异，而非逻辑短路
/// 3. 执行时间与输入长度相关，但与内容无关
///
/// # 参数
///
/// - `a`: 第一个字符串
/// - `b`: 第二个字符串
///
/// # 返回
///
/// - `true`: 两个字符串完全相同
/// - `false`: 字符串不同（长度或内容）
///
/// # 安全性说明
///
/// 此函数防止攻击者通过测量响应时间来逐字符猜测配对码。
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();

    // 使用 XOR 检测长度差异（非零表示不同）
    let len_diff = a.len() ^ b.len();

    // XOR 每个字节，较短输入用零填充
    // 遍历 max(a.len(), b.len()) 以避免时序差异
    let max_len = a.len().max(b.len());
    let mut byte_diff = 0u8;
    for i in 0..max_len {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        byte_diff |= x ^ y;
    }

    // 仅当长度和内容都相同时返回 true
    (len_diff == 0) & (byte_diff == 0)
}

/// 检查主机字符串是否为非本地绑定地址
///
/// 用于判断网关是否绑定在公网可访问的地址上，提示安全风险。
///
/// # 参数
///
/// - `host`: 主机地址字符串
///
/// # 返回
///
/// - `true`: 绑定在非本地地址（可能有安全风险）
/// - `false`: 绑定在本地地址（相对安全）
///
/// # 本地地址列表
///
/// - `127.0.0.1`: IPv4 本地回环
/// - `localhost`: 本地主机名
/// - `::1`: IPv6 本地回环
/// - `[::1]`: IPv6 本地回环（带方括号）
/// - `0:0:0:0:0:0:0:1`: IPv6 本地回环（完整形式）
pub fn is_public_bind(host: &str) -> bool {
    !matches!(host, "127.0.0.1" | "localhost" | "::1" | "[::1]" | "0:0:0:0:0:0:0:1")
}

#[cfg(test)]
#[path = "pairing_tests.rs"]
mod pairing_tests;
