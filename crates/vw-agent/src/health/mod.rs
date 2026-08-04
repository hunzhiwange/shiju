//! # 健康检查模块
//!
//! 本模块提供 agent 运行时的健康状态监控功能，用于跟踪各个组件的运行状态。
//!
//! ## 主要功能
//!
//! - **组件健康跟踪**：记录各个组件的当前状态、最后成功时间、最后错误信息等
//! - **健康快照**：生成包含进程信息和所有组件状态的健康报告
//! - **全局注册表**：通过线程安全的全局注册表管理健康状态
//!
//! ## 使用示例
//!
//! ```rust
//! use vibe_window::app::agent::health::{mark_component_ok, mark_component_error, snapshot};
//!
//! // 标记组件状态为正常
//! mark_component_ok("database");
//!
//! // 标记组件状态为错误
//! mark_component_error("cache", "connection timeout");
//!
//! // 获取健康快照
//! let health = snapshot();
//! println!("Uptime: {} seconds", health.uptime_seconds);
//! ```

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// 组件健康状态信息
///
/// 记录单个组件的健康状态详情，包括当前状态、时间戳、错误信息等。
/// 该结构体可序列化为 JSON 格式，便于通过 API 暴露健康状态。
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    /// 组件当前状态
    ///
    /// 可能的值包括：
    /// - `"starting"` - 组件正在启动
    /// - `"ok"` - 组件运行正常
    /// - `"error"` - 组件发生错误
    pub status: String,

    /// 状态最后更新时间（RFC3339 格式）
    ///
    /// 每次调用更新函数时都会自动更新此字段
    pub updated_at: String,

    /// 组件最后处于正常状态的时间（RFC3339 格式）
    ///
    /// 当调用 `mark_component_ok` 时会更新此字段
    pub last_ok: Option<String>,

    /// 组件最后发生的错误信息
    ///
    /// 当调用 `mark_component_error` 时会记录错误信息，
    /// 调用 `mark_component_ok` 时会清空此字段
    pub last_error: Option<String>,

    /// 组件重启次数
    ///
    /// 通过 `bump_component_restart` 递增，用于追踪组件的不稳定性
    pub restart_count: u64,
}

/// 系统健康状态快照
///
/// 包含整个 agent 进程的健康状态信息，包括进程 ID、运行时长、
/// 以及所有被跟踪组件的健康状态。
#[derive(Debug, Clone, Serialize)]
pub struct HealthSnapshot {
    /// 当前进程 ID
    pub pid: u32,

    /// 快照生成时间（RFC3339 格式）
    pub updated_at: String,

    /// 进程启动以来的运行时长（秒）
    pub uptime_seconds: u64,

    /// 所有组件的健康状态映射
    ///
    /// 键为组件名称，值为对应的健康状态信息
    pub components: BTreeMap<String, ComponentHealth>,
}

/// 健康状态注册表（内部结构）
///
/// 全局单例，存储所有组件的健康状态信息。
/// 使用 `Mutex` 保证线程安全。
struct HealthRegistry {
    /// 进程启动时间点，用于计算运行时长
    started_at: Instant,

    /// 组件健康状态映射表
    ///
    /// 使用 `Mutex<BTreeMap>` 保证并发安全，
    /// `BTreeMap` 保证组件名称的有序性，便于序列化输出
    components: Mutex<BTreeMap<String, ComponentHealth>>,
}

/// 全局健康状态注册表
///
/// 使用 `OnceLock` 实现线程安全的延迟初始化，
/// 首次访问时自动创建注册表实例
static REGISTRY: OnceLock<HealthRegistry> = OnceLock::new();
static RUNTIME_BUILD_ID: OnceLock<String> = OnceLock::new();

/// Records the current executable digest before any on-disk replacement can occur.
pub fn initialize_runtime_build_id() -> Result<&'static str, String> {
    if let Some(build_id) = RUNTIME_BUILD_ID.get() {
        return Ok(build_id);
    }

    let build_id = vw_shared::build_identity::current_executable_build_id()
        .map_err(|error| format!("failed to calculate runtime build identity: {error}"))?;
    let _ = RUNTIME_BUILD_ID.set(build_id);
    RUNTIME_BUILD_ID
        .get()
        .map(String::as_str)
        .ok_or_else(|| "failed to store runtime build identity".to_string())
}

/// Returns the executable digest recorded for the running gateway process.
pub fn runtime_build_id() -> Option<&'static str> {
    RUNTIME_BUILD_ID.get().map(String::as_str)
}

/// 获取全局健康状态注册表
///
/// 如果注册表尚未初始化，会自动创建一个新的实例。
/// 注册表在进程生命周期内只会被初始化一次。
///
/// # 返回值
///
/// 返回全局注册表的静态引用
fn registry() -> &'static HealthRegistry {
    REGISTRY.get_or_init(|| HealthRegistry {
        started_at: Instant::now(),
        components: Mutex::new(BTreeMap::new()),
    })
}

/// 生成当前时间的 RFC3339 格式字符串
///
/// 使用 UTC 时区生成符合 RFC3339 标准的时间戳字符串。
/// 如果格式化失败（理论上不会发生），返回 Unix 纪元时间。
///
/// # 返回值
///
/// RFC3339 格式的时间字符串，例如 `"2024-01-15T10:30:00Z"`
fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

/// 更新或插入组件健康状态
///
/// 这是一个内部辅助函数，用于原子性地更新组件的健康状态。
/// 如果组件不存在，会先创建一个默认的健康状态条目，
/// 然后应用传入的更新函数。
///
/// # 参数
///
/// - `component` - 组件名称，作为映射表的键
/// - `update` - 更新函数，接收可变引用并修改健康状态
///
/// # 线程安全
///
/// 该函数会锁定全局注册表的互斥锁，确保并发安全。
/// 即使互斥锁被污染（线程 panic），也会恢复并继续使用内部数据。
fn upsert_component<F>(component: &str, update: F)
where
    F: FnOnce(&mut ComponentHealth),
{
    // 锁定互斥锁，处理可能的污染情况
    let mut map = registry().components.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    // 获取当前时间戳
    let now = now_rfc3339();

    // 获取或创建组件条目，然后应用更新
    let entry = map.entry(component.to_string()).or_insert_with(|| ComponentHealth {
        status: "starting".into(),
        updated_at: now.clone(),
        last_ok: None,
        last_error: None,
        restart_count: 0,
    });

    // 应用用户提供的更新函数
    update(entry);

    // 更新最后修改时间
    entry.updated_at = now;
}

/// 标记组件状态为正常
///
/// 将指定组件的健康状态标记为 "ok"，并记录当前时间作为最后成功时间。
/// 同时清空之前的错误信息。
///
/// # 参数
///
/// - `component` - 组件名称
///
/// # 示例
///
/// ```rust
/// use vibe_window::app::agent::health::mark_component_ok;
///
/// mark_component_ok("database");
/// mark_component_ok("cache");
/// ```
pub fn mark_component_ok(component: &str) {
    upsert_component(component, |entry| {
        entry.status = "ok".into();
        entry.last_ok = Some(now_rfc3339());
        entry.last_error = None;
    });
}

/// 标记组件状态为错误
///
/// 将指定组件的健康状态标记为 "error"，并记录错误信息。
/// 不会清空 `last_ok` 字段，保留最后成功的时间记录。
///
/// # 参数
///
/// - `component` - 组件名称
/// - `error` - 错误信息，可以是任何实现了 `ToString` trait 的类型
///
/// # 示例
///
/// ```rust
/// use vibe_window::app::agent::health::mark_component_error;
///
/// mark_component_error("database", "connection refused");
/// mark_component_error("cache", String::from("timeout after 30s"));
/// ```
pub fn mark_component_error(component: &str, error: impl ToString) {
    let err = error.to_string();
    upsert_component(component, move |entry| {
        entry.status = "error".into();
        entry.last_error = Some(err);
    });
}

/// 递增组件重启计数
///
/// 将指定组件的重启计数加一。用于跟踪组件的不稳定性。
/// 使用 `saturating_add` 防止溢出。
///
/// # 参数
///
/// - `component` - 组件名称
///
/// # 示例
///
/// ```rust
/// use vibe_window::app::agent::health::bump_component_restart;
///
/// bump_component_restart("worker");
/// ```
pub fn bump_component_restart(component: &str) {
    upsert_component(component, |entry| {
        entry.restart_count = entry.restart_count.saturating_add(1);
    });
}

/// 生成健康状态快照
///
/// 创建包含当前进程信息和所有组件健康状态的快照。
/// 快照包括进程 ID、运行时长、生成时间和所有组件的状态信息。
///
/// # 返回值
///
/// 返回 `HealthSnapshot` 实例，包含完整的健康状态信息
///
/// # 线程安全
///
/// 该函数会锁定全局注册表的互斥锁进行读取，但克隆操作很快，
/// 不会长时间持有锁。
///
/// # 示例
///
/// ```rust
/// use vibe_window::app::agent::health::{snapshot, mark_component_ok};
///
/// mark_component_ok("api");
/// let health = snapshot();
///
/// println!("Process ID: {}", health.pid);
/// println!("Uptime: {} seconds", health.uptime_seconds);
/// println!("Components: {:?}", health.components.keys().collect::<Vec<_>>());
/// ```
pub fn snapshot() -> HealthSnapshot {
    // 锁定并克隆组件状态映射
    let components =
        registry().components.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();

    // 构建健康快照
    HealthSnapshot {
        pid: std::process::id(),
        updated_at: now_rfc3339(),
        uptime_seconds: registry().started_at.elapsed().as_secs(),
        components,
    }
}

/// 生成 JSON 格式的健康状态快照
///
/// 将健康快照序列化为 `serde_json::Value`，便于通过 HTTP API 返回。
/// 如果序列化失败，返回一个包含错误信息的 JSON 对象。
///
/// # 返回值
///
/// 返回 `serde_json::Value`，通常是健康快照的 JSON 表示。
/// 序列化失败时返回 `{"status": "error", "message": "..."}`。
///
/// # 示例
///
/// ```rust
/// use vibe_window::app::agent::health::snapshot_json;
///
/// let json = snapshot_json();
/// println!("{}", serde_json::to_string_pretty(&json).unwrap());
/// ```
pub fn snapshot_json() -> serde_json::Value {
    serde_json::to_value(snapshot()).unwrap_or_else(|_| {
        serde_json::json!({
            "status": "error",
            "message": "failed to serialize health snapshot"
        })
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
