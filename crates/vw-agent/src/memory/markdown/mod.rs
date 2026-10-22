//! Markdown 文件存储后端模块
//!
//! 本模块实现了基于 Markdown 文件的内存存储系统，将文件作为单一数据源。
//! 适用于需要人类可读、版本控制友好的场景。
//!
//! # 架构设计
//!
//! 存储布局：
//! - `~/.shiju-agent/worktree/<workspace-hash>/MEMORY.md` - 长期记忆的核心文件
//! - `~/.shiju-agent/worktree/<workspace-hash>/memory/YYYY-MM-DD.md` - 每日日志文件
//!
//! # 核心特性
//!
//! - **人类可读**：所有记忆以 Markdown 格式存储
//! - **审计追踪**：每日日志仅追加，保证历史完整性
//! - **简单可靠**：基于文件系统，无需数据库依赖
//!
//! # 示例
//!
//! ```rust,no_run
//! use std::path::Path;
//! use vibe_agent::memory::markdown::MarkdownMemory;
//! use vibe_agent::memory::traits::{Memory, MemoryCategory};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 创建内存存储实例
//! let workspace = Path::new("./workspace");
//! let memory = MarkdownMemory::new(workspace);
//!
//! // 存储核心记忆
//! memory.store(
//!     "project_goal",
//!     "构建高性能 AI 代理运行时",
//!     MemoryCategory::Core,
//!     None
//! ).await?;
//!
//! // 回忆相关内容
//! let entries = memory.recall("代理", 10, None).await?;
//! # Ok(())
//! # }
//! ```

use super::paths;
use super::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;
use chrono::Local;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use tokio::fs;

/// Markdown 内存存储实现
///
/// 使用文件系统中的 Markdown 文件作为存储介质，提供持久化的记忆能力。
/// 支持核心记忆和每日日志两种存储模式。
///
/// # 存储策略
///
/// - **Core（核心记忆）**：存储在 `MEMORY.md` 文件中，用于重要的长期记忆
/// - **Daily（每日日志）**：按日期存储在 `memory/YYYY-MM-DD.md` 文件中
///
/// # 线程安全
///
/// 该实现通过 tokio 的异步文件操作保证并发安全。
/// 注意：Markdown 内存采用仅追加设计，不支持删除操作。
pub struct MarkdownMemory {
    /// 用户态记忆数据目录路径
    workspace_dir: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl MarkdownMemory {
    /// 创建新的 Markdown 内存存储实例
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作空间根目录，仅用于派生用户态记忆数据目录
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::path::Path;
    /// use vibe_agent::memory::markdown::MarkdownMemory;
    ///
    /// let workspace = Path::new("./my_workspace");
    /// let memory = MarkdownMemory::new(workspace);
    /// ```
    pub fn new(workspace_dir: &Path) -> Self {
        let workspace_dir = paths::project_data_dir_best_effort(workspace_dir);
        let _ = std::fs::create_dir_all(&workspace_dir);
        Self { workspace_dir }
    }

    /// 获取每日日志目录路径
    ///
    /// 返回 `workspace_dir/memory` 路径，用于存储每日日志文件。
    fn memory_dir(&self) -> PathBuf {
        self.workspace_dir.join("memory")
    }

    /// 获取核心记忆文件路径
    ///
    /// 返回 `workspace_dir/MEMORY.md` 路径，用于存储核心长期记忆。
    fn core_path(&self) -> PathBuf {
        self.workspace_dir.join("MEMORY.md")
    }

    /// 获取当日日志文件路径
    ///
    /// 返回 `workspace_dir/memory/YYYY-MM-DD.md` 路径，
    /// 文件名基于当前日期自动生成。
    fn daily_path(&self) -> PathBuf {
        let date = Local::now().format("%Y-%m-%d").to_string();
        self.memory_dir().join(format!("{date}.md"))
    }

    /// 确保必要的目录结构存在
    ///
    /// 创建 `memory/` 目录（如果不存在）。
    /// 使用异步文件操作以避免阻塞。
    async fn ensure_dirs(&self) -> anyhow::Result<()> {
        fs::create_dir_all(self.memory_dir()).await?;
        Ok(())
    }

    /// 追加内容到文件
    ///
    /// 如果文件不存在，会创建新文件并添加适当的标题。
    /// 对于核心文件和日志文件，会使用不同的标题格式。
    ///
    /// # 参数
    ///
    /// - `path`: 目标文件路径
    /// - `content`: 要追加的内容（不含换行符）
    ///
    /// # 文件格式
    ///
    /// - 核心文件标题：`# Long-Term Memory`
    /// - 日志文件标题：`# Daily Log — YYYY-MM-DD`
    async fn append_to_file(&self, path: &Path, content: &str) -> anyhow::Result<()> {
        // 确保目录存在
        self.ensure_dirs().await?;

        // 读取现有内容，如果文件不存在则使用空字符串
        let existing = if path.exists() {
            fs::read_to_string(path).await.unwrap_or_default()
        } else {
            String::new()
        };

        // 根据是否为新文件决定是否添加标题
        let updated = if existing.is_empty() {
            // 为新文件添加标题
            let header = if path == self.core_path() {
                "# Long-Term Memory\n\n"
            } else {
                let date = Local::now().format("%Y-%m-%d").to_string();
                &format!("# Daily Log — {date}\n\n")
            };
            format!("{header}{content}\n")
        } else {
            // 追加到现有文件
            format!("{existing}\n{content}\n")
        };

        // 写入更新后的内容
        fs::write(path, updated).await?;
        Ok(())
    }

    /// 从文件内容解析记忆条目
    ///
    /// 将 Markdown 文件转换为结构化的记忆条目列表。
    /// 跳过空行和标题行（以 # 开头的行）。
    ///
    /// # 参数
    ///
    /// - `path`: 源文件路径（用于生成条目 ID）
    /// - `content`: 文件内容
    /// - `category`: 记忆分类（Core 或 Daily）
    ///
    /// # 返回值
    ///
    /// 返回解析后的记忆条目向量，每个条目包含：
    /// - `id`: 格式为 `文件名:行号`
    /// - `key`: 与 id 相同
    /// - `content`: 清理后的内容（去除 `- ` 前缀）
    /// - `category`: 传入的分类
    /// - `timestamp`: 文件名（作为时间戳）
    fn parse_entries_from_file(
        path: &Path,
        content: &str,
        category: &MemoryCategory,
    ) -> Vec<MemoryEntry> {
        // 提取文件名（不含扩展名），用于生成条目 ID
        let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        // 逐行解析，过滤空行和标题行
        content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .enumerate()
            .map(|(i, line)| {
                let trimmed = line.trim();
                // 移除 Markdown 列表项前缀（"- "）
                let clean = trimmed.strip_prefix("- ").unwrap_or(trimmed);
                MemoryEntry {
                    id: format!("{filename}:{i}"),
                    key: format!("{filename}:{i}"),
                    content: clean.to_string(),
                    category: category.clone(),
                    timestamp: filename.to_string(),
                    session_id: None,
                    score: None,
                }
            })
            .collect()
    }

    /// 读取所有记忆条目
    ///
    /// 从核心文件和所有日志文件中读取记忆条目，
    /// 并按时间戳降序排序（最新的在前）。
    ///
    /// # 返回值
    ///
    /// 返回所有记忆条目的向量，按时间降序排列。
    async fn read_all_entries(&self) -> anyhow::Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();

        // 读取核心记忆文件（MEMORY.md）
        let core_path = self.core_path();
        if core_path.exists() {
            let content = fs::read_to_string(&core_path).await?;
            entries.extend(Self::parse_entries_from_file(
                &core_path,
                &content,
                &MemoryCategory::Core,
            ));
        }

        // 读取每日日志文件
        let mem_dir = self.memory_dir();
        if mem_dir.exists() {
            let mut dir = fs::read_dir(&mem_dir).await?;
            // 遍历目录中的所有 .md 文件
            while let Some(entry) = dir.next_entry().await? {
                let path = entry.path();
                // 只处理 .md 文件
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    let content = fs::read_to_string(&path).await?;
                    entries.extend(Self::parse_entries_from_file(
                        &path,
                        &content,
                        &MemoryCategory::Daily,
                    ));
                }
            }
        }

        // 按时间戳降序排序（最新条目在前）
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(entries)
    }
}

/// WASM 平台的构造函数实现（受限版本）
///
/// 在 WASM 环境下，文件系统操作受限，
/// 仅提供基本的构造函数，实际存储功能为空实现。
#[cfg(target_arch = "wasm32")]
impl MarkdownMemory {
    /// 创建新的 Markdown 内存存储实例（WASM 版本）
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作空间根目录（在 WASM 中可能无法访问）
    pub fn new(workspace_dir: &Path) -> Self {
        let workspace_dir = paths::project_data_dir_best_effort(workspace_dir);
        Self { workspace_dir }
    }
}

/// WASM 平台的 Memory trait 实现（空实现）
///
/// 由于 WASM 环境文件系统访问受限，所有方法返回空结果。
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Memory for MarkdownMemory {
    /// 返回存储后端名称
    fn name(&self) -> &str {
        "markdown"
    }

    /// 存储记忆（空实现）
    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// 回忆记忆（空实现）
    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    /// 获取单个记忆条目（空实现）
    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    /// 列出记忆条目（空实现）
    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    /// 删除记忆（空实现）
    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    /// 统计记忆数量（空实现）
    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    /// 健康检查（始终返回 true）
    async fn health_check(&self) -> bool {
        true
    }
}

/// Memory trait 的完整实现（非 WASM 平台）
///
/// 提供基于 Markdown 文件的完整内存存储功能。
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Memory for MarkdownMemory {
    /// 返回存储后端名称
    ///
    /// # 返回值
    ///
    /// 固定返回 `"markdown"`
    fn name(&self) -> &str {
        "markdown"
    }

    /// 存储新的记忆条目
    ///
    /// 将记忆内容以 Markdown 列表项格式追加到相应文件。
    /// Core 类型的记忆存储到 `MEMORY.md`，其他类型存储到当日日志。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆键名（用于标识）
    /// - `content`: 记忆内容
    /// - `category`: 记忆分类（Core 或 Daily）
    /// - `_session_id`: 会话 ID（当前未使用）
    ///
    /// # 存储格式
    ///
    /// ```markdown
    /// - **key**: content
    /// ```
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use vibe_agent::memory::markdown::MarkdownMemory;
    /// # use vibe_agent::memory::traits::{Memory, MemoryCategory};
    /// # async fn example(memory: MarkdownMemory) -> anyhow::Result<()> {
    /// memory.store(
    ///     "user_preference",
    ///     "用户偏好使用暗色主题",
    ///     MemoryCategory::Core,
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        // 格式化为 Markdown 列表项
        let entry = format!("- **{key}**: {content}");
        // 根据分类选择目标文件
        let path = match category {
            MemoryCategory::Core => self.core_path(),
            _ => self.daily_path(),
        };
        self.append_to_file(&path, &entry).await
    }

    /// 根据查询关键词回忆相关记忆
    ///
    /// 使用简单的关键词匹配算法，对记忆内容进行相关性评分。
    /// 匹配的关键词越多，评分越高。
    ///
    /// # 参数
    ///
    /// - `query`: 查询字符串（支持多个关键词，用空格分隔）
    /// - `limit`: 返回结果的最大数量
    /// - `_session_id`: 会话 ID（当前未使用）
    ///
    /// # 返回值
    ///
    /// 返回按相关性评分降序排列的记忆条目列表。
    /// 每个条目的 `score` 字段包含匹配度评分（0.0 到 1.0）。
    ///
    /// # 评分算法
    ///
    /// ```text
    /// score = 匹配的关键词数量 / 总关键词数量
    /// ```
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use vibe_agent::memory::markdown::MarkdownMemory;
    /// # use vibe_agent::memory::traits::Memory;
    /// # async fn example(memory: MarkdownMemory) -> anyhow::Result<()> {
    /// let results = memory.recall("Rust 性能 优化", 5, None).await?;
    /// for entry in results {
    ///     println!("评分: {:?}, 内容: {}", entry.score, entry.content);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        // 读取所有记忆条目
        let all = self.read_all_entries().await?;
        // 将查询转换为小写并分割为关键词
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();

        // 对每个条目计算匹配分数
        let mut scored: Vec<MemoryEntry> = all
            .into_iter()
            .filter_map(|mut entry| {
                let content_lower = entry.content.to_lowercase();
                // 统计匹配的关键词数量
                let matched = keywords.iter().filter(|kw| content_lower.contains(**kw)).count();
                // 只保留至少匹配一个关键词的条目
                if matched > 0 {
                    // 计算匹配度分数
                    #[allow(clippy::cast_precision_loss)]
                    let score = matched as f64 / keywords.len() as f64;
                    entry.score = Some(score);
                    Some(entry)
                } else {
                    None
                }
            })
            .collect();

        // 按分数降序排序
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        // 限制返回数量
        scored.truncate(limit);
        Ok(scored)
    }

    /// 根据键名或内容获取单个记忆条目
    ///
    /// 首先尝试精确匹配键名，如果失败则进行内容包含匹配。
    ///
    /// # 参数
    ///
    /// - `key`: 要查找的键名或内容片段
    ///
    /// # 返回值
    ///
    /// - `Some(MemoryEntry)`: 找到匹配的条目
    /// - `None`: 未找到匹配
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let all = self.read_all_entries().await?;
        // 优先匹配键名，其次匹配内容
        Ok(all.into_iter().find(|e| e.key == key || e.content.contains(key)))
    }

    /// 列出记忆条目
    ///
    /// 可选地按分类过滤记忆条目。
    ///
    /// # 参数
    ///
    /// - `category`: 可选的分类过滤器（None 表示返回所有）
    /// - `_session_id`: 会话 ID（当前未使用）
    ///
    /// # 返回值
    ///
    /// 返回符合过滤条件的记忆条目列表。
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let all = self.read_all_entries().await?;
        match category {
            Some(cat) => Ok(all.into_iter().filter(|e| &e.category == cat).collect()),
            None => Ok(all),
        }
    }

    /// 删除记忆条目（不支持）
    ///
    /// Markdown 内存采用仅追加设计以保证审计追踪的完整性。
    /// 此方法始终返回 `false`，表示条目未被删除。
    ///
    /// # 设计原因
    ///
    /// - **审计追踪**：保留所有历史记录用于审计
    /// - **数据完整性**：避免意外删除重要信息
    /// - **版本控制友好**：适合 Git 等版本控制系统
    ///
    /// # 参数
    ///
    /// - `_key`: 要删除的键名（忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(false)`
    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    /// 统计记忆条目总数
    ///
    /// # 返回值
    ///
    /// 返回所有记忆条目的总数（包括核心记忆和日志）。
    async fn count(&self) -> anyhow::Result<usize> {
        let all = self.read_all_entries().await?;
        Ok(all.len())
    }

    /// 检查存储后端健康状态
    ///
    /// 验证工作空间目录是否存在且可访问。
    ///
    /// # 返回值
    ///
    /// - `true`: 工作空间目录存在
    /// - `false`: 工作空间目录不存在
    async fn health_check(&self) -> bool {
        self.workspace_dir.exists()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
