//! Qdrant 向量数据库记忆后端模块
//!
//! 本模块提供了基于 Qdrant 向量数据库的记忆存储实现，支持语义搜索和向量检索。
//! Qdrant 是一个高性能的向量相似度搜索引擎，适用于需要基于语义的内存检索场景。
//!
//! # 主要功能
//!
//! - **向量存储**: 将记忆内容转换为向量并存储到 Qdrant
//! - **语义搜索**: 基于向量相似度进行记忆检索
//! - **分类过滤**: 支持按记忆类别和会话 ID 过滤
//! - **懒加载初始化**: 支持延迟初始化集合，适配同步工厂模式
//!
//! # 架构说明
//!
//! 该模块实现了 `Memory` trait，作为 VibeWindow 记忆系统的一个后端实现。
//! 需要 `EmbeddingProvider` 来将文本转换为向量，然后存储到 Qdrant 中。
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use agent_memory::qdrant::QdrantMemory;
//! use agent_memory::embeddings::EmbeddingProvider;
//!
//! async fn example(embedder: Arc<dyn EmbeddingProvider>) {
//!     let memory = QdrantMemory::new(
//!         "http://localhost:6333",
//!         "memories",
//!         None,
//!         embedder
//!     ).await.unwrap();
//! }
//! ```

use super::embeddings::EmbeddingProvider;
use super::traits::{Memory, MemoryCategory, MemoryEntry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;
use uuid::Uuid;

/// Qdrant 向量数据库记忆后端
///
/// 使用 Qdrant 的 REST API 进行向量存储和语义搜索。
/// 需要一个嵌入提供者（EmbeddingProvider）来将文本转换为向量。
///
/// # 字段说明
///
/// - `client`: HTTP 客户端，用于与 Qdrant REST API 通信
/// - `base_url`: Qdrant 服务器的基础 URL
/// - `collection`: 集合名称，用于存储记忆向量
/// - `api_key`: 可选的 API 密钥，用于 Qdrant Cloud 认证
/// - `embedder`: 嵌入提供者，负责将文本转换为向量
/// - `initialized`: 跟踪集合是否已初始化（用于懒加载工厂模式的同步初始化）
pub struct QdrantMemory {
    /// HTTP 客户端
    client: reqwest::Client,
    /// Qdrant 服务器基础 URL
    base_url: String,
    /// 集合名称
    collection: String,
    /// 可选的 API 密钥
    api_key: Option<String>,
    /// 嵌入提供者
    embedder: Arc<dyn EmbeddingProvider>,
    /// 跟踪集合是否已初始化（懒加载支持）
    initialized: OnceCell<()>,
}

impl QdrantMemory {
    /// 创建一个新的 Qdrant 记忆后端实例
    ///
    /// 该方法会立即初始化集合，确保集合存在并具有正确的 schema。
    /// 如果你需要延迟初始化（例如在同步上下文中调用），请使用 `new_lazy` 方法。
    ///
    /// # 参数
    ///
    /// - `url`: Qdrant 服务器 URL（例如 "http://localhost:6333"）
    /// - `collection`: 用于存储记忆的集合名称
    /// - `api_key`: 可选的 API 密钥，用于 Qdrant Cloud 认证
    /// - `embedder`: 嵌入提供者，用于向量转换
    ///
    /// # 返回值
    ///
    /// 成功时返回初始化完成的 `QdrantMemory` 实例，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 无法连接到 Qdrant 服务器
    /// - 无法创建集合
    /// - 嵌入提供者配置错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use agent_memory::qdrant::QdrantMemory;
    ///
    /// async fn create_memory(embedder: Arc<dyn EmbeddingProvider>) -> Result<QdrantMemory> {
    ///     QdrantMemory::new(
    ///         "http://localhost:6333",
    ///         "my_memories",
    ///         None,
    ///         embedder
    ///     ).await
    /// }
    /// ```
    pub async fn new(
        url: &str,
        collection: &str,
        api_key: Option<String>,
        embedder: Arc<dyn EmbeddingProvider>,
    ) -> Result<Self> {
        // 使用懒加载方式创建实例
        let mem = Self::new_lazy(url, collection, api_key, embedder);

        // 确保集合存在并具有正确的 schema
        mem.ensure_collection().await?;
        // 标记为已初始化
        mem.initialized.set(()).ok();

        Ok(mem)
    }

    /// 创建一个带有延迟初始化的 Qdrant 记忆后端
    ///
    /// 集合将在第一次操作时创建。当从同步上下文（例如记忆工厂）调用时，请使用此方法。
    ///
    /// # 参数
    ///
    /// - `url`: Qdrant 服务器 URL（例如 "http://localhost:6333"）
    /// - `collection`: 用于存储记忆的集合名称
    /// - `api_key`: 可选的 API 密钥，用于 Qdrant Cloud 认证
    /// - `embedder`: 嵌入提供者，用于向量转换
    ///
    /// # 返回值
    ///
    /// 返回一个未初始化的 `QdrantMemory` 实例，集合将在首次操作时创建
    ///
    /// # 使用场景
    ///
    /// 适用于无法使用异步初始化的同步上下文，例如：
    /// - 记忆工厂的注册函数
    /// - 静态初始化场景
    /// - 配置加载阶段
    pub fn new_lazy(
        url: &str,
        collection: &str,
        api_key: Option<String>,
        embedder: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        // 移除 URL 末尾的斜杠，避免重复斜杠
        let base_url = url.trim_end_matches('/').to_string();
        // 构建支持代理配置的 HTTP 客户端
        let client = agent_config::schema::build_runtime_proxy_client("memory.qdrant");

        Self {
            client,
            base_url,
            collection: collection.to_string(),
            api_key,
            embedder,
            initialized: OnceCell::new(),
        }
    }

    /// 确保集合已初始化（在首次操作时懒加载调用）
    ///
    /// 该方法是线程安全的，只会执行一次初始化逻辑。
    /// 使用 `OnceCell` 确保即使在并发环境下也只初始化一次。
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    async fn ensure_initialized(&self) -> Result<()> {
        self.initialized
            .get_or_try_init(|| async {
                self.ensure_collection().await?;
                Ok::<(), anyhow::Error>(())
            })
            .await?;
        Ok(())
    }

    /// 创建一个 HTTP 请求构建器
    ///
    /// 该方法会自动添加必要的请求头（Content-Type 和可选的 api-key）。
    ///
    /// # 参数
    ///
    /// - `method`: HTTP 方法（GET、POST、PUT 等）
    /// - `path`: API 路径（相对于 base_url）
    ///
    /// # 返回值
    ///
    /// 返回配置好基础头部的 `reqwest::RequestBuilder`
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        // 构建完整的 URL
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);

        // 如果配置了 API 密钥，添加到请求头
        if let Some(ref key) = self.api_key {
            req = req.header("api-key", key);
        }

        // 添加 Content-Type 头
        req.header("Content-Type", "application/json")
    }

    /// 确保 Qdrant 集合存在并具有正确的配置
    ///
    /// 该方法会检查集合是否存在，如果不存在则创建。
    /// 创建的集合使用 Cosine 距离度量，向量维度由嵌入提供者决定。
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 连接 Qdrant 服务器失败
    /// - 集合检查失败（非 200/404 响应）
    /// - 集合创建失败
    ///
    /// # 特殊情况
    ///
    /// 如果嵌入提供者返回 0 维度（noop embedder），则会跳过向量集合设置，
    /// 因为无法创建维度为 0 的向量索引。在这种情况下，向量搜索将被禁用。
    async fn ensure_collection(&self) -> Result<()> {
        // 获取嵌入维度
        let dims = self.embedder.dimensions();

        // 如果是 noop embedder（0 维度），跳过向量集合设置
        if dims == 0 {
            tracing::warn!(
                "Qdrant memory using noop embedder (0 dimensions); vector search disabled"
            );
            return Ok(());
        }

        // 检查集合是否已存在
        let resp = self
            .request(reqwest::Method::GET, &format!("/collections/{}", self.collection))
            .send()
            .await;

        match resp {
            // 集合已存在，无需创建
            Ok(r) if r.status().is_success() => {
                return Ok(());
            }
            // 集合不存在（404），继续创建
            Ok(r) if r.status().as_u16() == 404 => {
                // 集合不存在，需要创建
            }
            // 其他响应状态，返回错误
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                anyhow::bail!("Qdrant collection check failed ({status}): {text}");
            }
            // 连接失败
            Err(e) => {
                anyhow::bail!("Qdrant connection failed: {e}");
            }
        }

        // 创建集合，配置向量参数：维度和距离度量（Cosine）
        let create_body = serde_json::json!({
            "vectors": {
                "size": dims,
                "distance": "Cosine"  // 使用余弦相似度作为距离度量
            }
        });

        // 发送创建集合请求
        let resp = self
            .request(reqwest::Method::PUT, &format!("/collections/{}", self.collection))
            .json(&create_body)
            .send()
            .await
            .context("failed to create Qdrant collection")?;

        // 检查创建结果
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant collection creation failed ({status}): {text}");
        }

        tracing::info!("Created Qdrant collection '{}' with {} dimensions", self.collection, dims);

        Ok(())
    }

    /// 将 MemoryCategory 枚举转换为字符串
    ///
    /// 用于在 Qdrant payload 中存储类别信息。
    ///
    /// # 参数
    ///
    /// - `category`: 记忆类别枚举
    ///
    /// # 返回值
    ///
    /// 返回类别的字符串表示
    fn category_to_str(category: &MemoryCategory) -> String {
        match category {
            MemoryCategory::Core => "core".to_string(),
            MemoryCategory::Daily => "daily".to_string(),
            MemoryCategory::Conversation => "conversation".to_string(),
            MemoryCategory::Custom(name) => name.clone(),
        }
    }

    /// 将字符串解析为 MemoryCategory 枚举
    ///
    /// 从 Qdrant payload 中读取类别信息并转换为枚举。
    ///
    /// # 参数
    ///
    /// - `value`: 类别字符串
    ///
    /// # 返回值
    ///
    /// 返回对应的 MemoryCategory 枚举值
    fn parse_category(value: &str) -> MemoryCategory {
        match value {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
        }
    }
}

/// Qdrant 点的负载数据结构
///
/// 存储记忆条目的元数据，作为向量点的 payload 存储。
/// 该结构体实现了序列化和反序列化，用于与 Qdrant API 交互。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryPayload {
    /// 记忆键，用于唯一标识和检索
    key: String,
    /// 记忆内容
    content: String,
    /// 记忆类别（core/daily/conversation 或自定义）
    category: String,
    /// 时间戳（RFC3339 格式）
    timestamp: String,
    /// 可选的会话 ID，用于按会话过滤
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

/// Qdrant 搜索结果结构
///
/// Qdrant 向量搜索 API 的响应结构。
#[derive(Debug, Deserialize)]
struct QdrantSearchResult {
    /// 搜索结果列表，包含得分点
    result: Vec<QdrantScoredPoint>,
}

/// Qdrant 得分点结构
///
/// 表示一个带有相似度得分的向量点。
#[derive(Debug, Deserialize)]
struct QdrantScoredPoint {
    /// 点的 ID（可以是字符串或数字）
    id: serde_json::Value,
    /// 相似度得分（越高表示越相似）
    score: f64,
    /// 点的负载数据
    payload: Option<MemoryPayload>,
}

/// Qdrant 滚动查询结果结构
///
/// Qdrant scroll API 的响应结构，用于遍历集合中的点。
#[derive(Debug, Deserialize)]
struct QdrantScrollResult {
    /// 滚动查询结果
    result: QdrantScrollPoints,
}

/// Qdrant 滚动查询的点列表
#[derive(Debug, Deserialize)]
struct QdrantScrollPoints {
    /// 点列表
    points: Vec<QdrantPoint>,
}

/// Qdrant 点结构
///
/// 表示集合中的一个向量点（不带得分）。
#[derive(Debug, Deserialize)]
struct QdrantPoint {
    /// 点的 ID（可以是字符串或数字）
    id: serde_json::Value,
    /// 点的负载数据
    payload: Option<MemoryPayload>,
}

/// 为 QdrantMemory 实现 Memory trait
///
/// 该实现提供了完整的记忆管理功能，包括存储、检索、列表、删除和健康检查。
/// 所有操作都支持基于向量的语义搜索和传统的键值检索。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for QdrantMemory {
    /// 获取记忆后端的名称
    ///
    /// # 返回值
    ///
    /// 返回 "qdrant" 字符串
    fn name(&self) -> &str {
        "qdrant"
    }

    /// 存储一条记忆到 Qdrant
    ///
    /// 该方法会将键和内容组合后生成嵌入向量，然后存储到 Qdrant。
    /// 如果已存在相同键的记忆，会先删除旧记录再插入新记录。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆的唯一标识键
    /// - `content`: 记忆内容
    /// - `category`: 记忆类别
    /// - `session_id`: 可选的会话 ID
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 嵌入生成失败
    /// - 嵌入向量为空（维度为 0）
    /// - Qdrant upsert 操作失败
    ///
    /// # 实现细节
    ///
    /// 1. 组合键和内容生成嵌入向量
    /// 2. 生成唯一的 UUID 作为点 ID
    /// 3. 记录当前时间戳
    /// 4. 删除已存在的相同键的记忆
    /// 5. 使用 upsert 操作插入新点
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()> {
        // 确保集合已初始化
        self.ensure_initialized().await?;

        // 组合键和内容生成嵌入向量，提高检索质量
        let combined_text = format!("{}\n{}", key, content);
        let embedding = self.embedder.embed_one(&combined_text).await?;

        // 检查嵌入是否有效
        if embedding.is_empty() {
            anyhow::bail!("Qdrant requires non-zero dimensional embeddings");
        }

        // 生成唯一的点 ID 和时间戳
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        // 构建负载数据
        let payload = MemoryPayload {
            key: key.to_string(),
            content: content.to_string(),
            category: Self::category_to_str(&category),
            timestamp,
            session_id: session_id.map(str::to_string),
        };

        // 先删除已存在的相同键的记忆，避免重复
        let _ = self.forget(key).await;

        // 构建并执行 upsert 请求
        let upsert_body = serde_json::json!({
            "points": [{
                "id": id,
                "vector": embedding,
                "payload": payload
            }]
        });

        let resp = self
            .request(reqwest::Method::PUT, &format!("/collections/{}/points", self.collection))
            .query(&[("wait", "true")]) // 等待操作完成
            .json(&upsert_body)
            .send()
            .await
            .context("failed to upsert point to Qdrant")?;

        // 检查响应状态
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant upsert failed ({status}): {text}");
        }

        Ok(())
    }

    /// 从 Qdrant 检索记忆（基于向量相似度搜索）
    ///
    /// 该方法会将查询文本转换为向量，然后在 Qdrant 中进行相似度搜索。
    /// 如果查询为空或嵌入不可用，会回退到列表所有记忆。
    ///
    /// # 参数
    ///
    /// - `query`: 查询文本
    /// - `limit`: 返回结果的最大数量
    /// - `session_id`: 可选的会话 ID 过滤器
    ///
    /// # 返回值
    ///
    /// 成功时返回记忆条目列表，失败时返回错误
    ///
    /// # 实现细节
    ///
    /// 1. 如果查询为空，回退到 list 方法
    /// 2. 生成查询文本的嵌入向量
    /// 3. 如果嵌入为空，回退到 list 方法
    /// 4. 构建过滤条件（如果有 session_id）
    /// 5. 执行向量相似度搜索
    /// 6. 将结果转换为 MemoryEntry 列表
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        // 如果查询为空，直接列出所有记忆
        if query.trim().is_empty() {
            return self.list(None, session_id).await;
        }

        // 确保集合已初始化
        self.ensure_initialized().await?;

        // 生成查询文本的嵌入向量
        let embedding = self.embedder.embed_one(query).await?;

        // 如果嵌入不可用，回退到列表模式
        if embedding.is_empty() {
            return self.list(None, session_id).await;
        }

        // 构建 session_id 过滤器
        let filter = session_id.map(|sid| {
            serde_json::json!({
                "must": [{
                    "key": "session_id",
                    "match": { "value": sid }
                }]
            })
        });

        // 构建搜索请求体
        let mut search_body = serde_json::json!({
            "vector": embedding,
            "limit": limit,
            "with_payload": true  // 包含负载数据
        });

        // 如果有过滤条件，添加到请求体
        if let Some(f) = filter {
            search_body["filter"] = f;
        }

        // 执行搜索请求
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/collections/{}/points/search", self.collection),
            )
            .json(&search_body)
            .send()
            .await
            .context("failed to search Qdrant")?;

        // 检查响应状态
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant search failed ({status}): {text}");
        }

        // 解析响应
        let result: QdrantSearchResult = resp.json().await?;

        // 将搜索结果转换为 MemoryEntry 列表
        let entries = result
            .result
            .into_iter()
            .filter_map(|point| {
                // 提取负载数据
                let payload = point.payload?;

                // 解析点 ID（支持字符串和数字格式）
                let id = match &point.id {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => return None,
                };

                Some(MemoryEntry {
                    id,
                    key: payload.key,
                    content: payload.content,
                    category: Self::parse_category(&payload.category),
                    timestamp: payload.timestamp,
                    session_id: payload.session_id,
                    score: Some(point.score), // 包含相似度得分
                })
            })
            .collect();

        Ok(entries)
    }

    /// 根据键获取单条记忆
    ///
    /// 使用 Qdrant 的 scroll API 和过滤器精确匹配键。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆的唯一标识键
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Some(MemoryEntry)`（如果找到）或 `None`（如果未找到），失败时返回错误
    ///
    /// # 实现细节
    ///
    /// 使用 scroll API 而不是 search API，因为我们进行精确键匹配，
    /// 不需要向量相似度计算。
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        // 确保集合已初始化
        self.ensure_initialized().await?;

        // 构建 scroll 请求，使用键过滤器进行精确匹配
        let scroll_body = serde_json::json!({
            "filter": {
                "must": [{
                    "key": "key",
                    "match": { "value": key }
                }]
            },
            "limit": 1,  // 只需要第一条匹配结果
            "with_payload": true
        });

        // 执行 scroll 请求
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/collections/{}/points/scroll", self.collection),
            )
            .json(&scroll_body)
            .send()
            .await
            .context("failed to scroll Qdrant")?;

        // 检查响应状态
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant scroll failed ({status}): {text}");
        }

        // 解析响应
        let result: QdrantScrollResult = resp.json().await?;

        // 提取第一条结果（如果存在）
        let entry = result.result.points.into_iter().next().and_then(|point| {
            let payload = point.payload?;

            // 解析点 ID
            let id = match &point.id {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => return None,
            };

            Some(MemoryEntry {
                id,
                key: payload.key,
                content: payload.content,
                category: Self::parse_category(&payload.category),
                timestamp: payload.timestamp,
                session_id: payload.session_id,
                score: None, // 精确查询不包含相似度得分
            })
        });

        Ok(entry)
    }

    /// 列出记忆条目
    ///
    /// 使用 scroll API 遍历集合中的点，支持按类别和会话 ID 过滤。
    ///
    /// # 参数
    ///
    /// - `category`: 可选的类别过滤器
    /// - `session_id`: 可选的会话 ID 过滤器
    ///
    /// # 返回值
    ///
    /// 成功时返回记忆条目列表，失败时返回错误
    ///
    /// # 注意
    ///
    /// 当前实现限制最多返回 1000 条结果。如果需要支持更大的结果集，
    /// 需要实现分页或滚动迭代。
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        // 确保集合已初始化
        self.ensure_initialized().await?;

        // 构建过滤条件
        let mut must_conditions = Vec::new();

        // 添加类别过滤器
        if let Some(cat) = category {
            must_conditions.push(serde_json::json!({
                "key": "category",
                "match": { "value": Self::category_to_str(cat) }
            }));
        }

        // 添加会话 ID 过滤器
        if let Some(sid) = session_id {
            must_conditions.push(serde_json::json!({
                "key": "session_id",
                "match": { "value": sid }
            }));
        }

        // 构建 scroll 请求体
        let mut scroll_body = serde_json::json!({
            "limit": 1000,  // 限制最大返回数量
            "with_payload": true
        });

        // 如果有过滤条件，添加到请求体
        if !must_conditions.is_empty() {
            scroll_body["filter"] = serde_json::json!({ "must": must_conditions });
        }

        // 执行 scroll 请求
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/collections/{}/points/scroll", self.collection),
            )
            .json(&scroll_body)
            .send()
            .await
            .context("failed to scroll Qdrant")?;

        // 检查响应状态
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant scroll failed ({status}): {text}");
        }

        // 解析响应
        let result: QdrantScrollResult = resp.json().await?;

        // 将结果转换为 MemoryEntry 列表
        let entries = result
            .result
            .points
            .into_iter()
            .filter_map(|point| {
                let payload = point.payload?;

                // 解析点 ID
                let id = match &point.id {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => return None,
                };

                Some(MemoryEntry {
                    id,
                    key: payload.key,
                    content: payload.content,
                    category: Self::parse_category(&payload.category),
                    timestamp: payload.timestamp,
                    session_id: payload.session_id,
                    score: None, // 列表查询不包含相似度得分
                })
            })
            .collect();

        Ok(entries)
    }

    /// 删除指定键的记忆
    ///
    /// 使用过滤器删除所有匹配指定键的点。
    ///
    /// # 参数
    ///
    /// - `key`: 要删除的记忆键
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(true)`，失败时返回错误
    ///
    /// # 注意
    ///
    /// Qdrant 不容易返回删除的数量，因此我们假设删除总是成功的。
    /// 如果需要更精确的删除确认，需要额外的查询逻辑。
    async fn forget(&self, key: &str) -> Result<bool> {
        // 确保集合已初始化
        self.ensure_initialized().await?;

        // 构建删除请求，使用键过滤器
        let delete_body = serde_json::json!({
            "filter": {
                "must": [{
                    "key": "key",
                    "match": { "value": key }
                }]
            }
        });

        // 执行删除请求
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/collections/{}/points/delete", self.collection),
            )
            .query(&[("wait", "true")]) // 等待操作完成
            .json(&delete_body)
            .send()
            .await
            .context("failed to delete from Qdrant")?;

        // 检查响应状态
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant delete failed ({status}): {text}");
        }

        // Qdrant 不容易返回删除的数量，假设删除成功
        Ok(true)
    }

    /// 统计记忆条目总数
    ///
    /// 查询 Qdrant 集合的元数据获取点数量。
    ///
    /// # 返回值
    ///
    /// 成功时返回记忆条目总数，失败时返回错误
    async fn count(&self) -> Result<usize> {
        // 确保集合已初始化
        self.ensure_initialized().await?;

        // 获取集合信息
        let resp = self
            .request(reqwest::Method::GET, &format!("/collections/{}", self.collection))
            .send()
            .await
            .context("failed to get Qdrant collection info")?;

        // 检查响应状态
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant collection info failed ({status}): {text}");
        }

        // 解析响应 JSON
        let json: serde_json::Value = resp.json().await?;

        // 提取点数量，路径：result.points_count
        let count = json
            .get("result")
            .and_then(|r| r.get("points_count"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0);

        // 将 u64 转换为 usize
        let count =
            usize::try_from(count).context("Qdrant returned a points count that exceeds usize")?;

        Ok(count)
    }

    /// 执行健康检查
    ///
    /// 向 Qdrant 服务器发送一个简单的 GET 请求，检查连接是否正常。
    ///
    /// # 返回值
    ///
    /// 如果连接正常返回 `true`，否则返回 `false`
    async fn health_check(&self) -> bool {
        // 向根路径发送 GET 请求
        let resp = self.request(reqwest::Method::GET, "/").send().await;

        // 如果请求成功且状态码为成功，返回 true
        matches!(resp, Ok(r) if r.status().is_success())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
