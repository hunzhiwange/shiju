//! # LucidMemory 模块
//!
//! 本模块实现了 `LucidMemory` 记忆存储后端，它是一个**双层混合记忆系统**，
//! 将本地 SQLite 存储与 Lucid 命令行工具相结合，提供高效、可靠的记忆检索与存储能力。
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//!              LucidMemory                  │
//! ├─────────────────────────────────────────┤
//! │  本地层 (SqliteMemory) ←→ Lucid CLI     │
//! │     ↑ 权威数据源          ↑ 增强检索    │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## 核心特性
//!
//! - **本地优先策略**：所有写入首先持久化到本地 SQLite，确保数据不丢失
//! - **智能降级**：当 Lucid 服务不可用或处于冷却期时，自动回退到本地存储
//! - **结果合并**：将本地检索结果与 Lucid 检索结果智能合并去重
//! - **可配置超时**：通过环境变量灵活配置各项超时参数
//!
//! ## 环境变量配置
//!
//! | 环境变量 | 默认值 | 说明 |
//! |---------|-------|------|
//! | `VIBEWINDOW_LUCID_CMD` | `lucid` | Lucid 命令路径 |
//! | `VIBEWINDOW_LUCID_BUDGET` | `200` | Token 预算限制 |
//! | `VIBEWINDOW_LUCID_RECALL_TIMEOUT_MS` | `500` | 检索超时（毫秒） |
//! | `VIBEWINDOW_LUCID_STORE_TIMEOUT_MS` | `800` | 存储超时（毫秒） |
//! | `VIBEWINDOW_LUCID_LOCAL_HIT_THRESHOLD` | `3` | 本地结果阈值 |
//! | `VIBEWINDOW_LUCID_FAILURE_COOLDOWN_MS` | `15000` | 失败冷却时间（毫秒） |

use memory_sqlite::memory::sqlite::SqliteMemory;
use super::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;
use chrono::Local;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// LucidMemory - 双层混合记忆存储实现
///
/// 该结构体组合了本地 SQLite 存储与 Lucid 命令行工具，实现了一个具有
/// 智能降级、结果合并和高可用性的记忆系统。
///
/// # 设计原则
///
/// 1. **本地优先**：SQLite 是权威数据源，所有操作首先作用于本地
/// 2. **异步同步**：存储操作同步到本地后，异步同步到 Lucid
/// 3. **失败隔离**：Lucid 失败不会影响核心功能，并会触发冷却机制
/// 4. **阈值优化**：当本地结果足够时，跳过 Lucid 检索以节省资源
///
/// # 线程安全
///
/// - 所有字段均为只读配置（在构造后不变）
/// - `last_failure_at` 使用 `Mutex` 保护，确保并发安全
pub struct LucidMemory {
    /// 本地 SQLite 记忆存储实例，作为权威数据源
    local: SqliteMemory,
    /// Lucid 命令行工具的路径或名称
    lucid_cmd: String,
    /// 检索时分配给 Lucid 的 token 预算上限
    token_budget: usize,
    /// 工作空间目录路径，用于 Lucid 项目上下文
    workspace_dir: PathBuf,
    /// Lucid 检索操作的超时时间
    recall_timeout: Duration,
    /// Lucid 存储操作的超时时间
    store_timeout: Duration,
    /// 本地检索结果的阈值：当本地结果数量达到此值时，跳过 Lucid 检索
    local_hit_threshold: usize,
    /// Lucid 失败后的冷却时间，在此期间不会尝试调用 Lucid
    failure_cooldown: Duration,
    /// 记录最后一次 Lucid 调用失败的时间点，用于冷却判断
    last_failure_at: Mutex<Option<Instant>>,
}

impl LucidMemory {
    /// 默认的 Lucid 命令名称
    const DEFAULT_LUCID_CMD: &'static str = "lucid";
    /// 默认的 token 预算（200 tokens）
    const DEFAULT_TOKEN_BUDGET: usize = 200;
    /// 默认的检索超时时间（500 毫秒）
    const DEFAULT_RECALL_TIMEOUT_MS: u64 = 500;
    /// 默认的存储超时时间（800 毫秒）
    const DEFAULT_STORE_TIMEOUT_MS: u64 = 800;
    /// 默认的本地结果阈值（3 条）
    const DEFAULT_LOCAL_HIT_THRESHOLD: usize = 3;
    /// 默认的失败冷却时间（15 秒）
    const DEFAULT_FAILURE_COOLDOWN_MS: u64 = 15_000;

    /// 创建新的 LucidMemory 实例
    ///
    /// 从环境变量读取配置，未设置时使用默认值。
    /// 这是在生产环境中构造实例的推荐方式。
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作空间目录路径，用于 Lucid 项目上下文识别
    /// - `local`: 底层 SQLite 记忆存储实例
    ///
    /// # 环境变量
    ///
    /// - `VIBEWINDOW_LUCID_CMD`: Lucid 命令路径（默认: `lucid`）
    /// - `VIBEWINDOW_LUCID_BUDGET`: Token 预算（默认: `200`，最小: `1`）
    /// - `VIBEWINDOW_LUCID_RECALL_TIMEOUT_MS`: 检索超时毫秒（默认: `500`，最小: `20`）
    /// - `VIBEWINDOW_LUCID_STORE_TIMEOUT_MS`: 存储超时毫秒（默认: `800`，最小: `50`）
    /// - `VIBEWINDOW_LUCID_LOCAL_HIT_THRESHOLD`: 本地阈值（默认: `3`，最小: `1`）
    /// - `VIBEWINDOW_LUCID_FAILURE_COOLDOWN_MS`: 冷却时间毫秒（默认: `15000`，最小: `100`）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::path::Path;
    /// let sqlite = SqliteMemory::new(Path::new("./data/memory.db"))?;
    /// let lucid = LucidMemory::new(Path::new("./workspace"), sqlite);
    /// ```
    pub fn new(workspace_dir: &Path, local: SqliteMemory) -> Self {
        // 从环境变量读取 Lucid 命令路径，未设置时使用默认值
        let lucid_cmd = std::env::var("VIBEWINDOW_LUCID_CMD")
            .unwrap_or_else(|_| Self::DEFAULT_LUCID_CMD.to_string());

        // 从环境变量读取 token 预算，确保为正整数
        let token_budget = std::env::var("VIBEWINDOW_LUCID_BUDGET")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(Self::DEFAULT_TOKEN_BUDGET);

        // 读取检索超时配置（最小 20ms）
        let recall_timeout = Self::read_env_duration_ms(
            "VIBEWINDOW_LUCID_RECALL_TIMEOUT_MS",
            Self::DEFAULT_RECALL_TIMEOUT_MS,
            20,
        );
        // 读取存储超时配置（最小 50ms）
        let store_timeout = Self::read_env_duration_ms(
            "VIBEWINDOW_LUCID_STORE_TIMEOUT_MS",
            Self::DEFAULT_STORE_TIMEOUT_MS,
            50,
        );
        // 读取本地结果阈值（最小 1）
        let local_hit_threshold = Self::read_env_usize(
            "VIBEWINDOW_LUCID_LOCAL_HIT_THRESHOLD",
            Self::DEFAULT_LOCAL_HIT_THRESHOLD,
            1,
        );
        // 读取失败冷却时间（最小 100ms）
        let failure_cooldown = Self::read_env_duration_ms(
            "VIBEWINDOW_LUCID_FAILURE_COOLDOWN_MS",
            Self::DEFAULT_FAILURE_COOLDOWN_MS,
            100,
        );

        Self {
            local,
            lucid_cmd,
            token_budget,
            workspace_dir: workspace_dir.to_path_buf(),
            recall_timeout,
            store_timeout,
            local_hit_threshold,
            failure_cooldown,
            last_failure_at: Mutex::new(None),
        }
    }

    /// 使用自定义配置创建 LucidMemory 实例（仅用于测试）
    ///
    /// 该方法允许完全控制所有参数，主要用于单元测试和集成测试。
    /// 生产代码应使用 [`new`](Self::new) 方法。
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作空间目录
    /// - `local`: SQLite 记忆存储实例
    /// - `lucid_cmd`: Lucid 命令路径
    /// - `token_budget`: Token 预算
    /// - `local_hit_threshold`: 本地结果阈值（自动修正为至少 1）
    /// - `recall_timeout`: 检索超时
    /// - `store_timeout`: 存储超时
    /// - `failure_cooldown`: 失败冷却时间
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn with_options(
        workspace_dir: &Path,
        local: SqliteMemory,
        lucid_cmd: String,
        token_budget: usize,
        local_hit_threshold: usize,
        recall_timeout: Duration,
        store_timeout: Duration,
        failure_cooldown: Duration,
    ) -> Self {
        Self {
            local,
            lucid_cmd,
            token_budget,
            workspace_dir: workspace_dir.to_path_buf(),
            recall_timeout,
            store_timeout,
            // 确保阈值至少为 1，避免逻辑错误
            local_hit_threshold: local_hit_threshold.max(1),
            failure_cooldown,
            last_failure_at: Mutex::new(None),
        }
    }

    /// 从环境变量读取 usize 值，带最小值约束
    ///
    /// # 参数
    ///
    /// - `name`: 环境变量名称
    /// - `default`: 解析失败时的默认值
    /// - `min`: 允许的最小值，低于此值将被修正
    ///
    /// # 返回
    ///
    /// 解析并修正后的 usize 值
    fn read_env_usize(name: &str, default: usize, min: usize) -> usize {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map_or(default, |v| v.max(min))
    }

    /// 从环境变量读取毫秒数并转换为 Duration，带最小值约束
    ///
    /// # 参数
    ///
    /// - `name`: 环境变量名称
    /// - `default_ms`: 默认毫秒数
    /// - `min_ms`: 允许的最小毫秒数
    ///
    /// # 返回
    ///
    /// 转换后的 Duration 实例
    fn read_env_duration_ms(name: &str, default_ms: u64, min_ms: u64) -> Duration {
        let millis = std::env::var(name)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map_or(default_ms, |v| v.max(min_ms));
        Duration::from_millis(millis)
    }

    /// 检查当前是否处于失败冷却期
    ///
    /// 当 Lucid 调用失败后，会进入冷却期。在此期间，检索操作将直接
    /// 使用本地结果，不再尝试调用 Lucid，以避免频繁失败导致的资源浪费。
    ///
    /// # 返回
    ///
    /// - `true`: 当前处于冷却期，应跳过 Lucid 调用
    /// - `false`: 冷却期已过或从未失败，可以尝试调用 Lucid
    fn in_failure_cooldown(&self) -> bool {
        let guard = self.last_failure_at.lock();
        guard.as_ref().is_some_and(|last| last.elapsed() < self.failure_cooldown)
    }

    /// 标记当前时刻为 Lucid 失败时刻
    ///
    /// 调用此方法后，后续的检索操作将在 `failure_cooldown` 时长内
    /// 自动跳过 Lucid 调用。
    fn mark_failure_now(&self) {
        let mut guard = self.last_failure_at.lock();
        *guard = Some(Instant::now());
    }

    /// 清除失败状态
    ///
    /// 当 Lucid 调用成功后，清除之前的失败标记，
    /// 允许后续操作正常使用 Lucid。
    fn clear_failure(&self) {
        let mut guard = self.last_failure_at.lock();
        *guard = None;
    }

    /// 将内部记忆分类转换为 Lucid 类型标签
    ///
    /// Lucid 使用自己的类型系统，此方法建立两者之间的映射关系。
    ///
    /// # 映射规则
    ///
    /// | MemoryCategory | Lucid 类型 |
    /// |----------------|-----------|
    /// | Core | decision |
    /// | Daily | context |
    /// | Conversation | conversation |
    /// | Custom(_) | learning |
    fn to_lucid_type(category: &MemoryCategory) -> &'static str {
        match category {
            MemoryCategory::Core => "decision",
            MemoryCategory::Daily => "context",
            MemoryCategory::Conversation => "conversation",
            MemoryCategory::Custom(_) => "learning",
        }
    }

    /// 将 Lucid 类型标签转换为内部记忆分类
    ///
    /// 这是 [`to_lucid_type`](Self::to_lucid_type) 的逆向转换，
    /// 用于解析 Lucid 返回的结果。
    ///
    /// # 转换规则
    ///
    /// - 包含 "visual" 的标签 → `Custom("visual")`
    /// - "decision" / "learning" / "solution" → `Core`
    /// - "context" / "conversation" → `Conversation`
    /// - "bug" → `Daily`
    /// - 其他 → `Custom(原标签)`
    fn to_memory_category(label: &str) -> MemoryCategory {
        let normalized = label.to_lowercase();
        // 特殊处理视觉相关记忆
        if normalized.contains("visual") {
            return MemoryCategory::Custom("visual".to_string());
        }

        match normalized.as_str() {
            "decision" | "learning" | "solution" => MemoryCategory::Core,
            "context" | "conversation" => MemoryCategory::Conversation,
            "bug" => MemoryCategory::Daily,
            other => MemoryCategory::Custom(other.to_string()),
        }
    }

    /// 合并本地和 Lucid 的检索结果
    ///
    /// 将两组结果按顺序合并，并基于 (key + content) 组合进行去重。
    /// 比较时不区分大小写，以应对不同来源的格式差异。
    ///
    /// # 参数
    ///
    /// - `primary_results`: 主结果集（通常是本地结果），优先级更高
    /// - `secondary_results`: 次结果集（通常是 Lucid 结果），用于补充
    /// - `limit`: 最大返回数量，0 表示返回空列表
    ///
    /// # 返回
    ///
    /// 合并去重后的结果列表，长度不超过 `limit`
    ///
    /// # 去重逻辑
    ///
    /// 使用 `key\0content` 的组合签名进行去重，忽略大小写。
    /// 这样既能去除完全重复的条目，也能合并内容相同但来源不同的记录。
    fn merge_results(
        primary_results: Vec<MemoryEntry>,
        secondary_results: Vec<MemoryEntry>,
        limit: usize,
    ) -> Vec<MemoryEntry> {
        // limit 为 0 时直接返回空，避免无意义处理
        if limit == 0 {
            return Vec::new();
        }

        let mut merged = Vec::new();
        let mut seen = HashSet::new();

        // 链式迭代：先遍历主结果，再遍历次结果
        for entry in primary_results.into_iter().chain(secondary_results) {
            // 构建去重签名：key + NULL分隔符 + content，均转为小写
            let signature =
                format!("{}\u{0}{}", entry.key.to_lowercase(), entry.content.to_lowercase());

            // 仅保留首次出现的条目
            if seen.insert(signature) {
                merged.push(entry);
                // 达到限制数量后停止
                if merged.len() >= limit {
                    break;
                }
            }
        }

        merged
    }

    /// 解析 Lucid 返回的上下文块
    ///
    /// Lucid 返回的上下文格式为 XML 风格的标记列表：
    /// ```text
    /// <lucid-context>
    /// - [decision] 实现了用户认证模块
    /// - [context] 项目使用 Rust 异步运行时
    /// </lucid-context>
    /// ```
    ///
    /// # 参数
    ///
    /// - `raw`: Lucid 命令的原始输出字符串
    ///
    /// # 返回
    ///
    /// 解析后的 `MemoryEntry` 向量。每个条目包含：
    /// - `id`: 格式为 `lucid:{序号}`
    /// - `key`: 格式为 `lucid_{序号}`
    /// - `content`: 实际内容文本
    /// - `category`: 根据 Lucid 标签转换的分类
    /// - `timestamp`: 当前时间（RFC3339 格式）
    /// - `score`: 根据位置计算的衰减分数（0.1 ~ 1.0）
    fn parse_lucid_context(raw: &str) -> Vec<MemoryEntry> {
        let mut in_context_block = false;
        let mut entries = Vec::new();
        let now = Local::now().to_rfc3339();

        for line in raw.lines().map(str::trim) {
            // 检测上下文块开始标记
            if line == "<lucid-context>" {
                in_context_block = true;
                continue;
            }

            // 检测上下文块结束标记，停止解析
            if line == "</lucid-context>" {
                break;
            }

            // 跳过块外内容和空行
            if !in_context_block || line.is_empty() {
                continue;
            }

            // 解析格式: "- [label] content"
            let Some(rest) = line.strip_prefix("- [") else {
                continue;
            };

            let Some((label, content_part)) = rest.split_once(']') else {
                continue;
            };

            let content = content_part.trim();
            // 跳过空内容
            if content.is_empty() {
                continue;
            }

            // 使用条目序号作为唯一标识
            let rank = entries.len();
            entries.push(MemoryEntry {
                id: format!("lucid:{rank}"),
                key: format!("lucid_{rank}"),
                content: content.to_string(),
                category: Self::to_memory_category(label.trim()),
                timestamp: now.clone(),
                session_id: None,
                // 计算位置衰减分数：越靠前分数越高，每条递减 0.05
                score: Some((1.0 - rank as f64 * 0.05).max(0.1)),
            });
        }

        entries
    }

    /// 执行 Lucid 命令并返回原始输出（静态版本）
    ///
    /// 这是不依赖 `self` 的静态方法，可在构造实例前使用。
    ///
    /// # 参数
    ///
    /// - `lucid_cmd`: Lucid 命令路径
    /// - `args`: 命令行参数列表
    /// - `timeout_window`: 执行超时时间
    ///
    /// # 错误
    ///
    /// - 命令执行超时
    /// - 命令返回非零退出码
    /// - 输出无法解析为 UTF-8
    async fn run_lucid_command_raw(
        lucid_cmd: &str,
        args: &[String],
        timeout_window: Duration,
    ) -> anyhow::Result<String> {
        let mut cmd = Command::new(lucid_cmd);
        cmd.args(args);

        // 使用 tokio::time::timeout 包装命令执行，超时后返回错误
        let output = timeout(timeout_window, cmd.output()).await.map_err(|_| {
            anyhow::anyhow!("lucid command timed out after {}ms", timeout_window.as_millis())
        })??;

        // 检查命令执行状态码
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("lucid command failed: {stderr}");
        }

        // 将标准输出转换为字符串返回
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 执行 Lucid 命令并返回原始输出（实例方法版本）
    ///
    /// 使用实例配置的命令路径执行 Lucid。
    ///
    /// # 参数
    ///
    /// - `args`: 命令行参数列表
    /// - `timeout_window`: 执行超时时间
    ///
    /// # 错误
    ///
    /// 参见 [`run_lucid_command_raw`](Self::run_lucid_command_raw)
    async fn run_lucid_command(
        &self,
        args: &[String],
        timeout_window: Duration,
    ) -> anyhow::Result<String> {
        Self::run_lucid_command_raw(&self.lucid_cmd, args, timeout_window).await
    }

    /// 构建存储操作的命令行参数
    ///
    /// 生成调用 `lucid store` 所需的完整参数列表。
    ///
    /// # 生成的命令格式
    ///
    /// ```text
    /// lucid store "{key}: {content}" --type={type} --project={workspace}
    /// ```
    fn build_store_args(&self, key: &str, content: &str, category: &MemoryCategory) -> Vec<String> {
        let payload = format!("{key}: {content}");
        vec![
            "store".to_string(),
            payload,
            format!("--type={}", Self::to_lucid_type(category)),
            format!("--project={}", self.workspace_dir.display()),
        ]
    }

    /// 构建检索操作的命令行参数
    ///
    /// 生成调用 `lucid context` 所需的完整参数列表。
    ///
    /// # 生成的命令格式
    ///
    /// ```text
    /// lucid context "{query}" --budget={budget} --project={workspace}
    /// ```
    fn build_recall_args(&self, query: &str) -> Vec<String> {
        vec![
            "context".to_string(),
            query.to_string(),
            format!("--budget={}", self.token_budget),
            format!("--project={}", self.workspace_dir.display()),
        ]
    }

    /// 异步同步记忆条目到 Lucid
    ///
    /// 该方法在后台将本地存储的条目同步到 Lucid。
    /// 即使同步失败也不影响主流程，仅记录调试日志。
    ///
    /// # 设计说明
    ///
    /// - 同步失败不会传播错误，保证本地存储的权威性
    /// - 失败会通过日志记录，便于问题排查
    /// - 该方法当前为同步等待实现，未来可改为真正的后台任务
    async fn sync_to_lucid_async(&self, key: &str, content: &str, category: &MemoryCategory) {
        let args = self.build_store_args(key, content, category);
        if let Err(error) = self.run_lucid_command(&args, self.store_timeout).await {
            tracing::debug!(
                command = %self.lucid_cmd,
                error = %error,
                "Lucid store sync failed; sqlite remains authoritative"
            );
        }
    }

    /// 从 Lucid 检索相关记忆
    ///
    /// 执行 Lucid context 命令并解析返回的上下文块。
    ///
    /// # 参数
    ///
    /// - `query`: 检索查询字符串
    ///
    /// # 返回
    ///
    /// 解析后的 `MemoryEntry` 向量
    ///
    /// # 错误
    ///
    /// - Lucid 命令执行失败
    /// - 命令执行超时
    async fn recall_from_lucid(&self, query: &str) -> anyhow::Result<Vec<MemoryEntry>> {
        let args = self.build_recall_args(query);
        let output = self.run_lucid_command(&args, self.recall_timeout).await?;
        Ok(Self::parse_lucid_context(&output))
    }
}

/// Memory trait 实现
///
/// 为 LucidMemory 实现标准的 Memory 接口，使其可被记忆系统统一调用。
/// 所有操作以本地 SQLite 为主，Lucid 为辅（增强检索）。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for LucidMemory {
    /// 返回存储后端名称标识
    ///
    /// # 返回
    ///
    /// 固定返回 `"lucid"` 字符串
    fn name(&self) -> &str {
        "lucid"
    }

    /// 存储记忆条目
    ///
    /// 首先将条目持久化到本地 SQLite（同步），然后异步同步到 Lucid。
    /// 即使 Lucid 同步失败，本地存储仍然成功。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆条目的唯一键
    /// - `content`: 记忆内容文本
    /// - `category`: 记忆分类（Core/Daily/Conversation/Custom）
    /// - `session_id`: 可选的会话标识符
    ///
    /// # 错误
    ///
    /// 仅当本地 SQLite 存储失败时返回错误
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        // 首先存储到本地，确保数据持久化
        self.local.store(key, content, category.clone(), session_id).await?;
        // 异步同步到 Lucid，失败不影响主流程
        self.sync_to_lucid_async(key, content, &category).await;
        Ok(())
    }

    /// 检索相关记忆
    ///
    /// 实现智能双层检索策略：
    /// 1. 首先从本地 SQLite 检索
    /// 2. 如果本地结果不足且不在冷却期，则调用 Lucid 增强检索
    /// 3. 合并去重两组结果后返回
    ///
    /// # 快速路径（跳过 Lucid）
    ///
    /// - `limit` 为 0
    /// - 本地结果数量已达到 `limit`
    /// - 本地结果数量已达到 `local_hit_threshold`
    /// - 当前处于失败冷却期
    ///
    /// # 参数
    ///
    /// - `query`: 检索查询字符串
    /// - `limit`: 最大返回数量
    /// - `session_id`: 可选的会话标识符（仅用于本地检索）
    ///
    /// # 返回
    ///
    /// 匹配的记忆条目列表，按相关性排序
    ///
    /// # 错误
    ///
    /// 本地 SQLite 检索失败时返回错误（Lucid 失败不会传播）
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        // 首先执行本地检索
        let local_results: Vec<MemoryEntry> = self.local.recall(query, limit, session_id).await?;

        // 快速路径：本地结果已足够，直接返回
        if limit == 0
            || local_results.len() >= limit
            || local_results.len() >= self.local_hit_threshold
        {
            return Ok(local_results);
        }

        // 快速路径：处于失败冷却期，避免重复调用失败的 Lucid
        if self.in_failure_cooldown() {
            return Ok(local_results);
        }

        // 尝试从 Lucid 获取增强结果
        match self.recall_from_lucid(query).await {
            // Lucid 返回非空结果：清除失败标记，合并结果
            Ok(lucid_results) if !lucid_results.is_empty() => {
                self.clear_failure();
                Ok(Self::merge_results(local_results, lucid_results, limit))
            }
            // Lucid 返回空结果：清除失败标记，返回本地结果
            Ok(_) => {
                self.clear_failure();
                Ok(local_results)
            }
            // Lucid 调用失败：标记失败时刻，降级到本地结果
            Err(error) => {
                self.mark_failure_now();
                tracing::debug!(
                    command = %self.lucid_cmd,
                    error = %error,
                    "Lucid context unavailable; using local sqlite results"
                );
                Ok(local_results)
            }
        }
    }

    /// 根据键精确获取单个记忆条目
    ///
    /// 仅从本地 SQLite 获取，不涉及 Lucid。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆条目的唯一键
    ///
    /// # 返回
    ///
    /// - `Some(entry)`: 找到匹配条目
    /// - `None`: 未找到匹配条目
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        self.local.get(key).await
    }

    /// 列出记忆条目
    ///
    /// 仅从本地 SQLite 列出，不涉及 Lucid。
    ///
    /// # 参数
    ///
    /// - `category`: 可选的分类过滤器
    /// - `session_id`: 可选的会话过滤器
    ///
    /// # 返回
    ///
    /// 符合条件的所有记忆条目
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.local.list(category, session_id).await
    }

    /// 删除指定键的记忆条目
    ///
    /// 仅从本地 SQLite 删除，不同步到 Lucid。
    /// 注意：这可能导致 Lucid 中残留已删除的数据。
    ///
    /// # 参数
    ///
    /// - `key`: 要删除的记忆条目键
    ///
    /// # 返回
    ///
    /// - `true`: 成功删除条目
    /// - `false`: 未找到匹配条目
    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        self.local.forget(key).await
    }

    /// 获取记忆条目总数
    ///
    /// 仅统计本地 SQLite 中的条目数。
    async fn count(&self) -> anyhow::Result<usize> {
        self.local.count().await
    }

    /// 健康检查
    ///
    /// 检查底层 SQLite 存储的可用性。
    /// 不检查 Lucid 服务的可用性。
    ///
    /// # 返回
    ///
    /// - `true`: 本地存储健康可用
    /// - `false`: 本地存储不可用
    async fn health_check(&self) -> bool {
        self.local.health_check().await
    }
}

/// 单元测试模块
///
/// 仅在 Unix 平台且测试配置下编译。
#[cfg(all(test, unix))]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
