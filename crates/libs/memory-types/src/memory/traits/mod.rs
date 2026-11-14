//! # Memory Traits 模块
//!
//! 本模块定义了 VibeWindow 代理系统的记忆存储核心抽象和接口。
//!
//! ## 主要功能
//!
//! - 定义 `Memory` trait，作为所有记忆存储后端的统一接口
//! - 提供标准化的记忆条目结构 `MemoryEntry`
//! - 支持多种记忆分类（核心、日常、会话、自定义）
//! - 提供跨平台兼容性（支持 WebAssembly 和原生环境）
//!
//! ## 架构说明
//!
//! 本模块采用 trait 驱动设计，遵循项目核心架构原则：
//! - 通过实现 `Memory` trait 扩展新的存储后端
//! - 使用 `MemoryBounds` trait 约束跨平台同步语义
//! - 序列化/反序列化逻辑与业务逻辑分离
//!
//! ## 扩展点
//!
//! 要添加新的记忆存储后端：
//! 1. 在对应模块中实现 `Memory` trait
//! 2. 在工厂模块中注册新后端
//! 3. 确保实现满足 `MemoryBounds` 约束（平台相关）

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 单条记忆条目
///
/// 代表存储在记忆系统中的单个条目，包含唯一标识符、键、内容、分类、
/// 时间戳以及可选的会话关联和相关性评分。
///
/// # 字段说明
///
/// - `id`: 记忆条目的唯一标识符
/// - `key`: 用于检索和索引的键名
/// - `content`: 记忆的实际内容
/// - `category`: 记忆分类，用于组织和过滤
/// - `timestamp`: 创建或更新时间戳（ISO 8601 格式）
/// - `session_id`: 可选的会话标识符，用于会话范围的记忆隔离
/// - `score`: 可选的相关性评分，用于排序和筛选（0.0-1.0）
///
/// # 示例
///
/// ```rust
/// use memory_types::memory::{MemoryEntry, MemoryCategory};
///
/// let entry = MemoryEntry {
///     id: "mem_001".to_string(),
///     key: "user_preference".to_string(),
///     content: "用户偏好使用深色主题".to_string(),
///     category: MemoryCategory::Core,
///     timestamp: "2024-01-15T10:30:00Z".to_string(),
///     session_id: Some("session_123".to_string()),
///     score: Some(0.95),
/// };
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 记忆条目的唯一标识符
    pub id: String,
    /// 用于检索和索引的键名
    pub key: String,
    /// 记忆的实际内容
    pub content: String,
    /// 记忆分类，用于组织和过滤
    pub category: MemoryCategory,
    /// 创建或更新时间戳（ISO 8601 格式）
    pub timestamp: String,
    /// 可选的会话标识符，用于会话范围的记忆隔离
    pub session_id: Option<String>,
    /// 可选的相关性评分，用于排序和筛选（0.0-1.0）
    pub score: Option<f64>,
}

/// 为 MemoryEntry 实现 Debug trait
///
/// 提供结构化的调试输出，但排除 `session_id` 字段以保持输出简洁。
/// 使用 `finish_non_exhaustive()` 确保未来添加字段时不会破坏现有代码。
impl std::fmt::Debug for MemoryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryEntry")
            .field("id", &self.id)
            .field("key", &self.key)
            .field("content", &self.content)
            .field("category", &self.category)
            .field("timestamp", &self.timestamp)
            .field("score", &self.score)
            .finish_non_exhaustive()
    }
}

/// 记忆分类枚举
///
/// 定义不同类型的记忆分类，用于组织和过滤记忆条目。
/// 支持预定义的核心分类以及用户自定义分类。
///
/// # 变体说明
///
/// - `Core`: 核心记忆，包含长期事实、用户偏好和重要决策
/// - `Daily`: 日常记忆，包含每日会话日志和临时信息
/// - `Conversation`: 会话记忆，包含对话上下文和交互历史
/// - `Custom`: 自定义分类，允许用户定义特定场景的分类
///
/// # 示例
///
/// ```rust
/// use memory_types::memory::MemoryCategory;
///
/// let core = MemoryCategory::Core;
/// let custom = MemoryCategory::Custom("project_alpha".to_string());
///
/// assert_eq!(core.to_string(), "core");
/// assert_eq!(custom.to_string(), "project_alpha");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryCategory {
    /// 核心记忆：长期事实、用户偏好和重要决策
    Core,
    /// 日常记忆：每日会话日志和临时信息
    Daily,
    /// 会话记忆：对话上下文和交互历史
    Conversation,
    /// 自定义分类：用户定义的特定场景分类
    Custom(String),
}

/// 为 MemoryCategory 实现 Display trait
///
/// 将分类枚举转换为字符串表示：
/// - `Core` -> "core"
/// - `Daily` -> "daily"
/// - `Conversation` -> "conversation"
/// - `Custom(name)` -> name（保持原样）
impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Daily => write!(f, "daily"),
            Self::Conversation => write!(f, "conversation"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// 为 MemoryCategory 实现 Serialize trait
///
/// 通过将枚举转换为字符串进行序列化，确保与 JSON/YAML 等格式的兼容性。
impl Serialize for MemoryCategory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// 为 MemoryCategory 实现 Deserialize trait
///
/// 从字符串反序列化为枚举，支持标准分类名称和自定义分类。
///
/// # 反序列化规则
///
/// - "core" -> `Core`
/// - "daily" -> `Daily`
/// - "conversation" -> `Conversation`
/// - 其他任何字符串 -> `Custom(String)`
impl<'de> Deserialize<'de> for MemoryCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "core" => Self::Core,
            "daily" => Self::Daily,
            "conversation" => Self::Conversation,
            other => Self::Custom(other.to_string()),
        })
    }
}

/// 记忆系统边界约束 trait（非 WASM 环境）
///
/// 在原生环境中，要求记忆实现必须满足 `Send + Sync`，
/// 以支持跨线程的安全共享和传递。
#[cfg(not(target_arch = "wasm32"))]
pub trait MemoryBounds: Send + Sync {}

/// MemoryBounds 的 blanket 实现（非 WASM 环境）
///
/// 为所有满足 `Send + Sync` 的类型自动实现 MemoryBounds。
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> MemoryBounds for T {}

/// 记忆系统边界约束 trait（WASM 环境）
///
/// 在 WebAssembly 环境中，不要求 `Send + Sync`，
/// 因为 WASM 是单线程执行模型。
#[cfg(target_arch = "wasm32")]
pub trait MemoryBounds {}

/// MemoryBounds 的 blanket 实现（WASM 环境）
///
/// 在 WASM 环境中为所有类型自动实现 MemoryBounds。
#[cfg(target_arch = "wasm32")]
impl<T> MemoryBounds for T {}

/// 核心记忆 trait - 为任意持久化后端实现此 trait
///
/// 这是 VibeWindow 代理系统记忆存储的核心抽象接口。
/// 所有记忆后端（如 Markdown、SQLite、PostgreSQL、Vector 等）
/// 都必须实现此 trait 以提供统一的存储和检索能力。
///
/// # 设计原则
///
/// - **trait 驱动扩展**: 通过实现此 trait 添加新后端
/// - **异步优先**: 所有 I/O 操作采用异步设计
/// - **平台兼容**: 通过 `MemoryBounds` 约束适配 WASM 和原生环境
/// - **最小接口**: 仅包含核心操作，扩展功能由具体实现提供
///
/// # 必需方法
///
/// - `name`: 返回后端标识符
/// - `store`: 存储记忆条目
/// - `recall`: 根据查询检索相关记忆
/// - `get`: 根据 key 精确获取记忆
/// - `list`: 列出符合条件的记忆
/// - `forget`: 删除指定记忆
/// - `count`: 统计记忆总数
/// - `health_check`: 健康检查
///
/// # 示例
///
/// ```rust
/// use memory_types::memory::{Memory, MemoryCategory, MemoryEntry};
/// use async_trait::async_trait;
///
/// struct InMemoryBackend {
///     entries: std::collections::HashMap<String, MemoryEntry>,
/// }
///
/// #[async_trait]
/// impl Memory for InMemoryBackend {
///     fn name(&self) -> &str {
///         "in-memory"
///     }
///
///     async fn store(
///         &self,
///         key: &str,
///         content: &str,
///         category: MemoryCategory,
///         session_id: Option<&str>,
///     ) -> anyhow::Result<()> {
///         // 实现存储逻辑
///         Ok(())
///     }
///
///     // ... 实现其他方法
///     # async fn recall(&self, _: &str, _: usize, _: Option<&str>) -> anyhow::Result<Vec<MemoryEntry>> { Ok(vec![]) }
///     # async fn get(&self, _: &str) -> anyhow::Result<Option<MemoryEntry>> { Ok(None) }
///     # async fn list(&self, _: Option<&MemoryCategory>, _: Option<&str>) -> anyhow::Result<Vec<MemoryEntry>> { Ok(vec![]) }
///     # async fn forget(&self, _: &str) -> anyhow::Result<bool> { Ok(false) }
///     # async fn count(&self) -> anyhow::Result<usize> { Ok(0) }
///     # async fn health_check(&self) -> bool { true }
/// }
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Memory: MemoryBounds {
    /// 返回后端名称
    ///
    /// 用于标识和日志记录，应返回简洁、可读的字符串标识符。
    ///
    /// # 返回值
    ///
    /// 后端名称的字符串切片引用
    fn name(&self) -> &str;

    /// 存储记忆条目
    ///
    /// 将新的记忆条目持久化存储，可选择关联到特定会话。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆的唯一键名，用于后续检索
    /// - `content`: 记忆的实际内容
    /// - `category`: 记忆分类，用于组织和过滤
    /// - `session_id`: 可选的会话标识符，用于会话范围隔离
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 存储成功
    /// - `Err(e)`: 存储失败，返回错误信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// memory.store(
    ///     "user_theme",
    ///     "深色主题",
    ///     MemoryCategory::Core,
    ///     None
    /// ).await?;
    /// ```
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()>;

    /// 根据查询检索相关记忆
    ///
    /// 执行关键词搜索，返回与查询最相关的记忆条目。
    /// 具体的相关性算法由后端实现决定（如全文搜索、向量相似度等）。
    ///
    /// # 参数
    ///
    /// - `query`: 搜索查询字符串
    /// - `limit`: 返回结果的最大数量
    /// - `session_id`: 可选的会话标识符，限制搜索范围
    ///
    /// # 返回值
    ///
    /// - `Ok(entries)`: 匹配的记忆条目列表，按相关性排序
    /// - `Err(e)`: 检索失败，返回错误信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let memories = memory.recall("用户偏好", 10, None).await?;
    /// for entry in memories {
    ///     println!("找到记忆: {} -> {}", entry.key, entry.content);
    /// }
    /// ```
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;

    /// 根据 key 精确获取记忆
    ///
    /// 使用唯一键名精确检索单个记忆条目。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆的唯一键名
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(entry))`: 找到记忆条目
    /// - `Ok(None)`: 未找到对应键的记忆
    /// - `Err(e)`: 检索失败，返回错误信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// if let Some(entry) = memory.get("user_theme").await? {
    ///     println!("用户主题偏好: {}", entry.content);
    /// }
    /// ```
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;

    /// 列出符合条件的记忆
    ///
    /// 根据分类和会话过滤条件列出记忆条目。
    ///
    /// # 参数
    ///
    /// - `category`: 可选的分类过滤器，为 None 时不过滤分类
    /// - `session_id`: 可选的会话过滤器，为 None 时不过滤会话
    ///
    /// # 返回值
    ///
    /// - `Ok(entries)`: 符合条件的记忆条目列表
    /// - `Err(e)`: 列表查询失败，返回错误信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 获取所有核心记忆
    /// let core_memories = memory.list(Some(&MemoryCategory::Core), None).await?;
    ///
    /// // 获取特定会话的所有记忆
    /// let session_memories = memory.list(None, Some("session_123")).await?;
    /// ```
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;

    /// 删除指定记忆
    ///
    /// 根据 key 删除单个记忆条目。
    ///
    /// # 参数
    ///
    /// - `key`: 要删除的记忆的唯一键名
    ///
    /// # 返回值
    ///
    /// - `Ok(true)`: 成功删除记忆
    /// - `Ok(false)`: 未找到对应键的记忆（无操作）
    /// - `Err(e)`: 删除失败，返回错误信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// if memory.forget("old_preference").await? {
    ///     println!("记忆已删除");
    /// } else {
    ///     println!("未找到该记忆");
    /// }
    /// ```
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;

    /// 统计记忆总数
    ///
    /// 返回当前存储的所有记忆条目总数（不区分分类和会话）。
    ///
    /// # 返回值
    ///
    /// - `Ok(count)`: 记忆条目总数
    /// - `Err(e)`: 统计失败，返回错误信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let total = memory.count().await?;
    /// println!("总共有 {} 条记忆", total);
    /// ```
    async fn count(&self) -> anyhow::Result<usize>;

    /// 健康检查
    ///
    /// 检查后端存储系统的健康状态，用于监控和诊断。
    ///
    /// # 返回值
    ///
    /// - `true`: 后端健康，可以正常提供服务
    /// - `false`: 后端异常，无法提供服务
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// if memory.health_check().await {
    ///     println!("记忆系统健康");
    /// } else {
    ///     eprintln!("记忆系统异常");
    /// }
    /// ```
    async fn health_check(&self) -> bool;
}

/// 单元测试模块
///
/// 测试文件位于 `tests.rs`，包含对 trait 实现和序列化逻辑的测试。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
