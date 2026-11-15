//! # Scout 模块 — 技能发现与外部源搜索
//!
//! 本模块提供从外部源（如 GitHub、ClawHub、HuggingFace 等）自动发现和搜索技能包的能力。
//!
//! ## 核心功能
//!
//! - **多源搜索**：支持从多个外部平台搜索技能相关的开源仓库
//! - **统一接口**：通过 `Scout` trait 提供统一的技能发现接口
//! - **结果聚合**：支持从多个查询词聚合搜索结果，并自动去重
//!
//! ## 主要组件
//!
//! - [`ScoutSource`]：枚举类型，表示技能来源平台
//! - [`ScoutResult`]：搜索结果结构体，包含仓库的元数据信息
//! - [`Scout`]：异步 trait，定义技能发现的行为契约
//! - [`GitHubScout`]：GitHub 平台的技能搜索实现
//!
//! ## 使用示例
//!
//! ```ignore
//! use skill_forge::scout::{GitHubScout, Scout};
//!
//! async fn discover_skills() -> anyhow::Result<()> {
//!     let scout = GitHubScout::new(None);
//!     let results = scout.discover().await?;
//!
//!     for skill in results {
//!         println!("{} - {} ({} stars)", skill.name, skill.description, skill.stars);
//!     }
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

// ---------------------------------------------------------------------------

/// 技能来源平台枚举
///
/// 定义技能搜索支持的外部源平台。每个平台代表一个可能的技能包托管位置。
///
/// # 变体
///
/// - `GitHub`：GitHub 代码托管平台，最常用的技能源
/// - `ClawHub`：ClawHub 技能仓库平台
/// - `HuggingFace`：HuggingFace 模型与数据集平台（别名 "hf"）
///
/// # 示例
///
/// ```ignore
/// use std::str::FromStr;
/// use skill_forge::scout::ScoutSource;
///
/// let source: ScoutSource = "github".parse().unwrap();
/// assert_eq!(source, ScoutSource::GitHub);
///
/// let source: ScoutSource = "hf".parse().unwrap();
/// assert_eq!(source, ScoutSource::HuggingFace);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoutSource {
    /// GitHub 代码托管平台
    GitHub,
    /// ClawHub 技能仓库平台
    ClawHub,
    /// HuggingFace 模型与数据集平台
    HuggingFace,
}

/// 为 ScoutSource 实现字符串解析
///
/// 支持从字符串解析为对应的枚举值，不区分大小写。
/// 当遇到未知的来源标识时，会记录警告日志并默认返回 GitHub。
impl std::str::FromStr for ScoutSource {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "github" => Self::GitHub,
            "clawhub" => Self::ClawHub,
            "huggingface" | "hf" => Self::HuggingFace,
            _ => {
                warn!(source = s, "Unknown scout source, defaulting to GitHub");
                Self::GitHub
            }
        })
    }
}

// ---------------------------------------------------------------------------

/// 技能搜索结果结构体
///
/// 表示从外部源搜索到的单个技能仓库的元数据信息。
/// 该结构体包含了评估和展示技能所需的关键信息。
///
/// # 字段说明
///
/// - `name`：仓库名称（不含所有者前缀）
/// - `url`：仓库的完整访问 URL
/// - `description`：仓库描述信息，用于展示技能用途
/// - `stars`：星标数量，用于评估技能受欢迎程度
/// - `language`：主要编程语言（可选）
/// - `updated_at`：最后更新时间（可选）
/// - `source`：技能来源平台
/// - `owner`：仓库所有者/组织名
/// - `has_license`：是否包含开源许可证文件
///
/// # 示例
///
/// ```ignore
/// use skill_forge::scout::{ScoutResult, ScoutSource};
/// use chrono::{DateTime, Utc};
///
/// let result = ScoutResult {
///     name: "awesome-skill".to_string(),
///     url: "https://github.com/user/awesome-skill".to_string(),
///     description: "An awesome skill for AI agents".to_string(),
///     stars: 100,
///     language: Some("Rust".to_string()),
///     updated_at: Some(Utc::now()),
///     source: ScoutSource::GitHub,
///     owner: "user".to_string(),
///     has_license: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutResult {
    /// 仓库名称（不含所有者前缀）
    pub name: String,
    /// 仓库的完整访问 URL
    pub url: String,
    /// 仓库描述信息
    pub description: String,
    /// 星标数量，表示受欢迎程度
    pub stars: u64,
    /// 主要编程语言（可选）
    pub language: Option<String>,
    /// 最后更新时间（可选）
    pub updated_at: Option<DateTime<Utc>>,
    /// 技能来源平台
    pub source: ScoutSource,
    /// 仓库所有者/组织名，从 URL 或 API 响应中提取
    pub owner: String,
    /// 是否包含开源许可证文件
    pub has_license: bool,
}

// ---------------------------------------------------------------------------

/// Scout trait 的边界约束标记
///
/// 根据目标平台自动调整 trait 的 Send + Sync 约束：
/// - 非 WASM 平台：要求 Send + Sync，支持多线程环境
/// - WASM 平台：不要求 Send + Sync，适配单线程 WASM 运行时
///
/// 这种条件编译设计使得 Scout 可以同时在原生环境和 WASM 环境中使用。
#[cfg(not(target_arch = "wasm32"))]
pub trait ScoutBounds: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> ScoutBounds for T {}

/// Scout trait 的边界约束标记（WASM 版本）
///
/// 在 WASM 目标平台上，不要求 Send + Sync 约束，
/// 因为 WASM 运行时通常是单线程的。
#[cfg(target_arch = "wasm32")]
pub trait ScoutBounds {}
#[cfg(target_arch = "wasm32")]
impl<T> ScoutBounds for T {}

/// 技能发现 trait
///
/// 定义从外部源发现技能的异步接口。所有技能搜索器（如 GitHubScout）
/// 都需要实现此 trait。
///
/// # Trait 约束
///
/// - 自动根据目标平台应用 `ScoutBounds` 约束
/// - 使用 `async_trait` 支持异步操作
/// - 在 WASM 平台上使用 `?Send` 模式
///
/// # 必需方法
///
/// - [`discover`](Scout::discover)：执行技能发现，返回搜索结果列表
///
/// # 实现说明
///
/// 实现者应当：
/// - 处理网络错误和 API 限流
/// - 对结果进行去重
/// - 记录适当的调试和警告日志
///
/// # 示例
///
/// ```ignore
/// use async_trait::async_trait;
/// use skill_forge::scout::{Scout, ScoutResult};
///
/// struct MyScout;
///
/// #[async_trait]
/// impl Scout for MyScout {
///     async fn discover(&self) -> anyhow::Result<Vec<ScoutResult>> {
///         // 实现技能发现逻辑
///         Ok(vec![])
///     }
/// }
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Scout: ScoutBounds {
    /// 从外部源发现候选技能
    ///
    /// 执行技能搜索操作，返回符合条件的结果列表。
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<ScoutResult>)`：成功返回搜索结果列表
    /// - `Err(anyhow::Error)`：搜索过程中发生错误
    ///
    /// # 错误处理
    ///
    /// 实现者应当在以下情况返回错误：
    /// - 网络连接失败
    /// - API 认证失败
    /// - 响应解析失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use skill_forge::scout::{GitHubScout, Scout};
    ///
    /// async fn find_skills() -> anyhow::Result<()> {
    ///     let scout = GitHubScout::new(None);
    ///     let results = scout.discover().await?;
    ///     println!("Found {} skills", results.len());
    ///     Ok(())
    /// }
    /// ```
    async fn discover(&self) -> Result<Vec<ScoutResult>>;
}

// ---------------------------------------------------------------------------

/// GitHub 技能搜索器
///
/// 实现 `Scout` trait，用于从 GitHub 搜索与技能相关的开源仓库。
/// 使用 GitHub REST API 的搜索接口，按星标数降序排列结果。
///
/// # 搜索策略
///
/// 默认使用以下查询词进行搜索：
/// - "vibewindow skill"
/// - "ai agent skill"
///
/// # 认证
///
/// 支持可选的 Bearer Token 认证。如果提供 token，可以提高 API 限流阈值。
/// 如果未提供 token，则使用匿名访问（受更严格的限流限制）。
///
/// # 请求配置
///
/// - 超时时间：30 秒（非 WASM 平台）
/// - User-Agent：VibeWindow-SkillForge/0.1
/// - Accept：application/vnd.github+json
///
/// # 示例
///
/// ```ignore
/// use skill_forge::scout::{GitHubScout, Scout};
///
/// // 无认证令牌
/// let scout = GitHubScout::new(None);
///
/// // 使用认证令牌
/// let scout = GitHubScout::new(Some("ghp_xxxx".to_string()));
/// ```
pub struct GitHubScout {
    /// HTTP 客户端，用于发送 API 请求
    client: reqwest::Client,
    /// 搜索查询词列表
    queries: Vec<String>,
}

impl GitHubScout {
    /// 创建新的 GitHubScout 实例
    ///
    /// # 参数
    ///
    /// - `token`：可选的 GitHub 个人访问令牌（PAT）
    ///   - `None`：使用匿名访问
    ///   - `Some(token)`：使用 Bearer Token 认证
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `GitHubScout` 实例
    ///
    /// # 请求头配置
    ///
    /// 自动添加以下请求头：
    /// - `Accept: application/vnd.github+json`：指定 GitHub API 版本
    /// - `User-Agent: VibeWindow-SkillForge/0.1`：标识客户端
    /// - `Authorization: Bearer {token}`：如果提供了 token
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 匿名访问
    /// let scout = GitHubScout::new(None);
    ///
    /// // 使用令牌访问（更高的限流限制）
    /// let scout = GitHubScout::new(Some("ghp_your_token".to_string()));
    /// ```
    pub fn new(token: Option<String>) -> Self {
        use std::time::Duration;

        let mut headers = reqwest::header::HeaderMap::new();
        // 设置 GitHub API v3+ 的 JSON 响应格式
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.github+json".parse().expect("valid header"),
        );
        // 设置 User-Agent，GitHub API 要求必须有
        headers.insert(
            reqwest::header::USER_AGENT,
            "VibeWindow-SkillForge/0.1".parse().expect("valid header"),
        );
        // 如果提供了 token，添加认证头
        if let Some(ref t) = token {
            if let Ok(val) = format!("Bearer {t}").parse() {
                headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }

        // 构建 HTTP 客户端
        let client = reqwest::Client::builder();
        // 非 WASM 平台设置 30 秒超时
        #[cfg(not(target_arch = "wasm32"))]
        let client = client.timeout(Duration::from_secs(30));

        let client = client.default_headers(headers).build().expect("valid client");

        // 使用默认的技能相关查询词
        Self { client, queries: vec!["vibewindow skill".into(), "ai agent skill".into()] }
    }

    /// 解析 GitHub 搜索/仓库 API 的 JSON 响应
    ///
    /// 将 GitHub API 返回的 JSON 数组转换为 `ScoutResult` 列表。
    /// 对于解析失败的字段，使用合理的默认值。
    ///
    /// # 参数
    ///
    /// - `body`：GitHub API 返回的 JSON 响应体
    ///
    /// # 返回值
    ///
    /// 返回解析后的 `ScoutResult` 向量。如果响应中没有 `items` 字段，
    /// 则返回空向量。
    ///
    /// # 字段映射
    ///
    /// | JSON 字段 | ScoutResult 字段 | 默认值 |
    /// |-----------|------------------|--------|
    /// | name | name | 无（必需） |
    /// | html_url | url | 无（必需） |
    /// | description | description | 空字符串 |
    /// | stargazers_count | stars | 0 |
    /// | language | language | None |
    /// | updated_at | updated_at | None |
    /// | owner.login | owner | "unknown" |
    /// | license | has_license | false |
    fn parse_items(body: &serde_json::Value) -> Vec<ScoutResult> {
        // 尝试获取 items 数组，如果不存在则返回空列表
        let items = match body.get("items").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return vec![],
        };

        items
            .iter()
            .filter_map(|item| {
                // 使用 filter_map 自动跳过解析失败的项
                let name = item.get("name")?.as_str()?.to_string();
                let url = item.get("html_url")?.as_str()?.to_string();
                let description =
                    item.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let stars = item.get("stargazers_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let language = item.get("language").and_then(|v| v.as_str()).map(String::from);
                // 解析 ISO 8601 格式的时间字符串
                let updated_at = item
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok());
                // 提取所有者信息
                let owner = item
                    .get("owner")
                    .and_then(|o| o.get("login"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                // 检查是否存在许可证（license 字段非 null）
                let has_license = item.get("license").map(|v| !v.is_null()).unwrap_or(false);

                Some(ScoutResult {
                    name,
                    url,
                    description,
                    stars,
                    language,
                    updated_at,
                    source: ScoutSource::GitHub,
                    owner,
                    has_license,
                })
            })
            .collect()
    }
}

/// 为 GitHubScout 实现 Scout trait
///
/// 执行多查询的 GitHub 搜索，聚合结果并去重。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Scout for GitHubScout {
    /// 执行技能发现操作
    ///
    /// 对配置的所有查询词执行 GitHub API 搜索，聚合结果并去重。
    ///
    /// # 搜索流程
    ///
    /// 1. 遍历所有查询词
    /// 2. 对每个查询词构建搜索 URL（按星标数降序，每页 30 条）
    /// 3. 发送 HTTP GET 请求到 GitHub API
    /// 4. 解析 JSON 响应并提取仓库信息
    /// 5. 聚合所有查询结果
    /// 6. 按 URL 去重
    ///
    /// # 错误处理
    ///
    /// - 单个查询失败时记录警告并继续下一个查询
    /// - 不会因为单个查询失败而中断整个搜索
    /// - 网络错误、非 200 响应、JSON 解析错误都会被优雅处理
    ///
    /// # 返回值
    ///
    /// 返回去重后的搜索结果列表。即使所有查询都失败，也返回空列表而非错误。
    async fn discover(&self) -> Result<Vec<ScoutResult>> {
        let mut all: Vec<ScoutResult> = Vec::new();

        // 遍历所有查询词，执行搜索
        for query in &self.queries {
            // 构建 GitHub 搜索 API URL
            // - sort=stars：按星标数排序
            // - order=desc：降序排列
            // - per_page=30：每页返回 30 条结果
            let url = format!(
                "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page=30",
                urlencoding(query)
            );
            debug!(query = query.as_str(), "Searching GitHub");

            // 发送 HTTP 请求，失败时跳过此查询
            let resp = match self.client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!(
                        query = query.as_str(),
                        error = %e,
                        "GitHub API request failed, skipping query"
                    );
                    continue;
                }
            };

            // 检查响应状态码，非 200 时跳过此查询
            if !resp.status().is_success() {
                warn!(
                    status = %resp.status(),
                    query = query.as_str(),
                    "GitHub search returned non-200"
                );
                continue;
            }

            // 解析 JSON 响应体，失败时跳过此查询
            let body: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        query = query.as_str(),
                        error = %e,
                        "Failed to parse GitHub response, skipping query"
                    );
                    continue;
                }
            };

            // 解析仓库项并添加到结果列表
            let mut items = Self::parse_items(&body);
            debug!(count = items.len(), query = query.as_str(), "Parsed items");
            all.append(&mut items);
        }

        // 按 URL 去重，保留首次出现的项
        dedup(&mut all);
        Ok(all)
    }
}

// ---------------------------------------------------------------------------

/// URL 查询字符串的最小化编码
///
/// 对查询字符串进行简单的 URL 编码，处理常见特殊字符。
/// 这是一个轻量级的实现，仅编码必要的字符。
///
/// # 编码规则
///
/// - 空格 ` ` → `+`
/// - 和号 `&` → `%26`
/// - 井号 `#` → `%23`
///
/// # 参数
///
/// - `s`：待编码的字符串
///
/// # 返回值
///
/// 返回编码后的字符串
///
/// # 示例
///
/// ```ignore
/// use skill_forge::scout::urlencoding;
///
/// assert_eq!(urlencoding("vibewindow skill"), "vibewindow+skill");
/// assert_eq!(urlencoding("rust&web"), "rust%26web");
/// ```
fn urlencoding(s: &str) -> String {
    s.replace(' ', "+").replace('&', "%26").replace('#', "%23")
}

/// 对搜索结果进行去重
///
/// 根据 URL 字段移除重复的搜索结果，保留首次出现的项。
///
/// # 参数
///
/// - `results`：搜索结果向量的可变引用，会原地修改
///
/// # 去重规则
///
/// - 使用 URL 作为唯一标识
/// - 保留首次出现的项
/// - 后续重复项会被移除
///
/// # 复杂度
///
/// - 时间复杂度：O(n)，其中 n 是结果数量
/// - 空间复杂度：O(n)，用于存储已见 URL 的 HashSet
///
/// # 示例
///
/// ```ignore
/// use skill_forge::scout::{ScoutResult, ScoutSource, dedup};
///
/// let mut results = vec![
///     ScoutResult {
///         name: "skill1".to_string(),
///         url: "https://github.com/user/skill".to_string(),
///         // ... 其他字段
///     },
///     ScoutResult {
///         name: "skill2".to_string(),
///         url: "https://github.com/user/skill".to_string(), // 重复 URL
///         // ... 其他字段
///     },
/// ];
///
/// dedup(&mut results);
/// assert_eq!(results.len(), 1); // 只保留第一项
/// ```
pub fn dedup(results: &mut Vec<ScoutResult>) {
    let mut seen = std::collections::HashSet::new();
    results.retain(|r| seen.insert(r.url.clone()));
}

// ---------------------------------------------------------------------------

/// 单元测试模块
///
/// 测试文件位于同级目录的 tests.rs 中
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
