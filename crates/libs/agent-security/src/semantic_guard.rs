//! 基于向量相似度的语义提示词注入防护模块。
//!
//! 本模块复用现有的记忆嵌入设置和 Qdrant 连接，用于检测对抗改写的提示词注入攻击。
//!
//! # 核心功能
//!
//! - **语义检测**：通过向量相似度匹配，检测与已知攻击模式相似的提示词
//! - **语料库管理**：支持从内置、本地文件或远程 URL 加载攻击语料库
//! - **灵活配置**：可配置相似度阈值、启用/禁用状态和 Qdrant 集合名称
//!
//! # 工作原理
//!
//! 1. 将用户输入的提示词转换为向量嵌入
//! 2. 在 Qdrant 向量数据库中搜索相似度最高的已知攻击模式
//! 3. 如果相似度超过配置的阈值，则返回匹配结果
//!
//! # 使用场景
//!
//! - 防止越狱攻击（jailbreak attempts）
//! - 检测提示词泄露尝试
//! - 识别角色扮演攻击
//! - 阻止指令覆盖攻击

use agent_config::runtime::{Config, MemoryConfig};
use agent_memory::embeddings::{EmbeddingProvider, create_embedding_provider};
use agent_memory::{Memory, MemoryCategory, QdrantMemory};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;

/// 内置语料库来源标识符
const BUILTIN_SOURCE: &str = "builtin";

/// 内置攻击语料库（JSONL 格式）
/// 编译时从 crate 内的 `assets/security/attack-corpus-v1.jsonl` 加载
const BUILTIN_CORPUS_JSONL: &str = include_str!("../assets/security/attack-corpus-v1.jsonl");

/// 语义防护守卫
///
/// 负责检测提示词注入攻击的核心组件，通过向量相似度匹配来识别潜在的恶意输入。
///
/// # 字段说明
///
/// - `enabled` - 是否启用语义防护
/// - `collection` - Qdrant 中用于存储攻击语料库的集合名称
/// - `threshold` - 相似度阈值（0.0 到 1.0），超过此值的匹配将被报告
/// - `qdrant_url` - Qdrant 服务器的 URL
/// - `qdrant_api_key` - Qdrant API 密钥（可选）
/// - `embedder` - 嵌入向量提供者
///
/// # 示例
///
/// ```ignore
/// use agent_security::semantic_guard::SemanticGuard;
/// use agent_config::runtime::MemoryConfig;
///
/// let guard = SemanticGuard::from_config(
///     &memory_config,
///     true,
///     "attack-patterns",
///     0.85,
///     Some("your-api-key"),
/// );
///
/// let match_result = guard.detect("some user input").await;
/// ```
#[derive(Clone)]
pub struct SemanticGuard {
    enabled: bool,
    collection: String,
    threshold: f64,
    qdrant_url: Option<String>,
    qdrant_api_key: Option<String>,
    embedder: Arc<dyn EmbeddingProvider>,
}

/// 语义防护守卫启动状态
///
/// 描述语义防护守卫在启动时的状态，用于诊断配置问题。
///
/// # 字段说明
///
/// - `active` - 守卫是否处于活动状态
/// - `reason` - 如果未激活，说明原因
#[derive(Debug, Clone)]
pub struct SemanticGuardStartupStatus {
    pub active: bool,
    pub reason: Option<String>,
}

/// 语义匹配结果
///
/// 当检测到用户输入与已知攻击模式匹配时返回的结果。
///
/// # 字段说明
///
/// - `score` - 相似度分数（0.0 到 1.0），越高表示越相似
/// - `key` - 匹配的攻击模式在语料库中的唯一标识符
/// - `category` - 攻击类别（如 "jailbreak"、"prompt-leak" 等）
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub score: f64,
    pub key: String,
    pub category: String,
}

/// 防护语料库记录
///
/// 表示语料库中的单条攻击模式记录，用于导入到向量数据库。
///
/// # 字段说明
///
/// - `text` - 攻击模式的文本内容（必需）
/// - `category` - 攻击类别，将被规范化为小写并用下划线替换空格（必需）
/// - `source` - 记录来源（可选，用于追踪）
/// - `id` - 记录的唯一标识符（可选，如未提供将根据内容和类别自动生成）
///
/// # 示例
///
/// ```json
/// {
///   "text": "Ignore all previous instructions and...",
///   "category": "jailbreak",
///   "source": "community-report",
///   "id": "jb-001"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardCorpusRecord {
    pub text: String,
    pub category: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

/// 防护语料库更新报告
///
/// 描述语料库更新操作的结果，包含详细的统计信息。
///
/// # 字段说明
///
/// - `source` - 语料库来源（"builtin"、文件路径或 URL）
/// - `sha256` - 语料库内容的 SHA256 哈希值
/// - `parsed_records` - 成功解析的记录数量
/// - `upserted_records` - 成功写入向量数据库的记录数量
/// - `collection` - 目标 Qdrant 集合名称
#[derive(Debug, Clone)]
pub struct GuardCorpusUpdateReport {
    pub source: String,
    pub sha256: String,
    pub parsed_records: usize,
    pub upserted_records: usize,
    pub collection: String,
}

impl SemanticGuard {
    /// 从配置创建语义防护守卫实例
    ///
    /// # 参数
    ///
    /// - `memory` - 记忆配置，包含 Qdrant 连接和嵌入提供者设置
    /// - `enabled` - 是否启用语义防护
    /// - `collection` - Qdrant 集合名称，用于存储攻击语料库
    /// - `threshold` - 相似度阈值，范围 [0.0, 1.0]，超出范围会被自动裁剪
    /// - `embedding_api_key` - 嵌入提供者的 API 密钥（可选）
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `SemanticGuard` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let guard = SemanticGuard::from_config(
    ///     &config.memory,
    ///     true,
    ///     "attack-patterns",
    ///     0.85,
    ///     config.api_key.as_deref(),
    /// );
    /// ```
    pub fn from_config(
        memory: &MemoryConfig,
        enabled: bool,
        collection: &str,
        threshold: f64,
        embedding_api_key: Option<&str>,
    ) -> Self {
        // 从记忆配置中解析 Qdrant URL 和 API 密钥
        let qdrant_url = resolve_qdrant_url(memory);
        let qdrant_api_key = resolve_qdrant_api_key(memory);

        // 创建嵌入向量提供者
        let embedder: Arc<dyn EmbeddingProvider> = Arc::from(create_embedding_provider(
            memory.embedding_provider.trim(),
            embedding_api_key,
            memory.embedding_model.trim(),
            memory.embedding_dimensions,
        ));

        Self {
            enabled,
            collection: collection.trim().to_string(),
            threshold: threshold.clamp(0.0, 1.0), // 确保阈值在有效范围内
            qdrant_url,
            qdrant_api_key,
            embedder,
        }
    }

    /// 为测试创建带有自定义嵌入提供者的守卫实例
    ///
    /// 此方法仅在测试代码中可用，允许注入模拟的嵌入提供者。
    #[cfg(test)]
    fn with_embedder_for_tests(
        enabled: bool,
        collection: &str,
        threshold: f64,
        qdrant_url: Option<String>,
        qdrant_api_key: Option<String>,
        embedder: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            enabled,
            collection: collection.to_string(),
            threshold,
            qdrant_url,
            qdrant_api_key,
            embedder,
        }
    }

    /// 获取守卫启动状态
    ///
    /// 检查语义防护守卫是否处于可用状态，并返回诊断信息。
    ///
    /// # 返回值
    ///
    /// 返回 `SemanticGuardStartupStatus`，包含：
    /// - `active: true` - 守卫已就绪，可以进行检测
    /// - `active: false` - 守卫未激活，`reason` 字段说明原因
    ///
    /// # 激活条件
    ///
    /// 守卫处于活动状态需要满足以下所有条件：
    /// 1. `enabled` 为 `true`
    /// 2. `collection` 不为空
    /// 3. `qdrant_url` 已配置
    /// 4. 嵌入维度大于 0（即嵌入功能已启用）
    pub fn startup_status(&self) -> SemanticGuardStartupStatus {
        // 检查是否启用
        if !self.enabled {
            return SemanticGuardStartupStatus {
                active: false,
                reason: Some("security.semantic_guard=false".to_string()),
            };
        }

        // 检查集合名称是否为空
        if self.collection.trim().is_empty() {
            return SemanticGuardStartupStatus {
                active: false,
                reason: Some("security.semantic_guard_collection is empty".to_string()),
            };
        }

        // 检查 Qdrant URL 是否配置
        if self.qdrant_url.is_none() {
            return SemanticGuardStartupStatus {
                active: false,
                reason: Some("memory.qdrant.url (or QDRANT_URL) is not configured".to_string()),
            };
        }

        // 检查嵌入提供者是否可用
        if self.embedder.dimensions() == 0 {
            return SemanticGuardStartupStatus {
                active: false,
                reason: Some(
                    "memory embeddings are disabled (embedding dimensions are zero)".to_string(),
                ),
            };
        }

        SemanticGuardStartupStatus { active: true, reason: None }
    }

    /// 创建记忆后端实例
    ///
    /// 内部方法，用于创建与 Qdrant 交互的记忆后端。
    ///
    /// # 错误
    ///
    /// 如果守卫未处于活动状态或 Qdrant URL 缺失，返回错误。
    fn create_memory(&self) -> Result<Arc<dyn Memory>> {
        let status = self.startup_status();
        if !status.active {
            bail!(
                "semantic guard is unavailable: {}",
                status.reason.unwrap_or_else(|| "unknown reason".to_string())
            );
        }

        let Some(url) = self.qdrant_url.as_deref() else {
            bail!("missing qdrant url");
        };

        // 创建懒加载的 Qdrant 记忆后端
        let backend = QdrantMemory::new_lazy(
            url,
            self.collection.trim(),
            self.qdrant_api_key.clone(),
            Arc::clone(&self.embedder),
        );

        let memory: Arc<dyn Memory> = Arc::new(backend);
        Ok(memory)
    }

    /// 检测语义提示词注入匹配
    ///
    /// 对用户输入进行语义分析，检查是否与已知的攻击模式匹配。
    ///
    /// # 参数
    ///
    /// - `prompt` - 待检测的用户输入文本
    ///
    /// # 返回值
    ///
    /// - `Some(SemanticMatch)` - 检测到匹配，包含相似度分数、标识符和类别
    /// - `None` - 未检测到匹配，或守卫已禁用/不可用，或发生后端错误
    ///
    /// # 安全行为
    ///
    /// 当向量基础设施不可用时（如 Qdrant 连接失败），此方法返回 `None`
    /// 而不是返回错误，以确保在基础设施故障时保持安全的无操作行为。
    /// 这避免了因防护系统故障而阻塞正常用户请求。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let guard = SemanticGuard::from_config(/* ... */);
    ///
    /// if let Some(match_result) = guard.detect("user input").await {
    ///     if match_result.score > 0.9 {
    ///         println!("高风险攻击: {} (分数: {})", match_result.category, match_result.score);
    ///     }
    /// }
    /// ```
    pub async fn detect(&self, prompt: &str) -> Option<SemanticMatch> {
        // 跳过空输入
        if prompt.trim().is_empty() {
            return None;
        }

        // 创建记忆后端，失败时返回 None（静默失败策略）
        let memory = match self.create_memory() {
            Ok(memory) => memory,
            Err(error) => {
                tracing::debug!("semantic guard disabled for this request: {error}");
                return None;
            }
        };

        // 在向量数据库中搜索最相似的攻击模式
        let entries = match memory.recall(prompt, 1, None).await {
            Ok(entries) => entries,
            Err(error) => {
                tracing::debug!("semantic guard recall failed; continuing without block: {error}");
                return None;
            }
        };

        // 获取最相似的匹配结果
        let Some(entry) = entries.into_iter().next() else {
            return None;
        };

        let score = entry.score.unwrap_or(0.0);

        // 检查相似度是否超过阈值
        if score < self.threshold {
            return None;
        }

        Some(SemanticMatch {
            score,
            key: entry.key,
            category: category_name_from_memory(&entry.category),
        })
    }

    /// 批量插入或更新防护语料库记录
    ///
    /// 将攻击模式记录写入 Qdrant 向量数据库，用于后续的语义匹配检测。
    ///
    /// # 参数
    ///
    /// - `records` - 待插入的语料库记录数组
    ///
    /// # 返回值
    ///
    /// 返回成功写入的记录数量
    ///
    /// # 错误
    ///
    /// - 如果守卫未激活，返回错误
    /// - 如果类别格式无效，返回错误
    /// - 如果写入向量数据库失败，返回错误
    ///
    /// # 说明
    ///
    /// - 记录的 `id` 字段如果为空，将根据类别和文本内容自动生成
    /// - 类别会被存储为 `semantic_guard:{category}` 格式，以便与其他记忆数据区分
    pub async fn upsert_corpus(&self, records: &[GuardCorpusRecord]) -> Result<usize> {
        let memory = self.create_memory()?;

        let mut upserted = 0usize;
        for record in records {
            // 规范化类别名称
            let category = normalize_corpus_category(&record.category)?;

            // 生成记录键：使用提供的 ID 或自动生成
            let key = record
                .id
                .clone()
                .filter(|id| !id.trim().is_empty())
                .unwrap_or_else(|| corpus_record_key(&category, &record.text));

            // 将记录存储到向量数据库，类别添加前缀以便识别
            memory
                .store(
                    &key,
                    record.text.trim(),
                    MemoryCategory::Custom(format!("semantic_guard:{category}")),
                    None,
                )
                .await
                .with_context(|| format!("failed to upsert semantic guard corpus key '{key}'"))?;
            upserted += 1;
        }

        Ok(upserted)
    }
}

/// 更新防护语料库
///
/// 从指定来源加载攻击语料库并写入 Qdrant 向量数据库。
/// 这是更新语义防护守卫语料库的主要入口函数。
///
/// # 参数
///
/// - `config` - 应用配置，包含记忆和安全设置
/// - `source` - 语料库来源：
///   - `None` 或 `"builtin"` - 使用内置语料库
///   - `"http://"` 或 `"https://"` 开头 - 从 URL 下载
///   - 其他 - 视为本地文件路径
/// - `expected_sha256` - 期望的 SHA256 哈希值（可选），用于验证语料库完整性
///
/// # 返回值
///
/// 返回 `GuardCorpusUpdateReport`，包含更新操作的详细统计信息
///
/// # 错误
///
/// - 如果哈希值验证失败，返回错误
/// - 如果语料库格式无效，返回错误
/// - 如果语义防护守卫不可用，返回错误
/// - 如果写入向量数据库失败，返回错误
///
/// # 示例
///
/// ```ignore
/// use agent_security::semantic_guard::update_guard_corpus;
/// use agent_config::runtime::Config;
///
/// // 使用内置语料库
/// let report = update_guard_corpus(&config, None, None).await?;
/// println!("更新了 {} 条记录", report.upserted_records);
///
/// // 从 URL 下载并验证哈希
/// let report = update_guard_corpus(
///     &config,
///     Some("https://example.com/corpus.jsonl"),
///     Some("abc123..."),
/// ).await?;
/// ```
pub async fn update_guard_corpus(
    config: &Config,
    source: Option<&str>,
    expected_sha256: Option<&str>,
) -> Result<GuardCorpusUpdateReport> {
    // 确定语料库来源，默认为内置语料库
    let source = source.unwrap_or(BUILTIN_SOURCE).trim();
    let payload = load_corpus_source(source).await?;

    // 计算实际哈希值
    let actual_sha256 = sha256_hex(payload.as_bytes());

    // 如果提供了期望的哈希值，进行验证
    if let Some(expected) = expected_sha256.map(str::trim).filter(|value| !value.is_empty()) {
        if !expected.eq_ignore_ascii_case(&actual_sha256) {
            bail!("guard corpus checksum mismatch: expected {expected}, got {actual_sha256}");
        }
    }

    // 解析 JSONL 格式的语料库
    let records = parse_guard_corpus_jsonl(&payload)?;

    // 创建语义防护守卫实例
    let semantic_guard = SemanticGuard::from_config(
        &config.memory,
        true,
        &config.security.semantic_guard_collection,
        config.security.semantic_guard_threshold,
        config.api_key.as_deref(),
    );

    // 检查守卫是否可用
    let status = semantic_guard.startup_status();
    if !status.active {
        bail!(
            "semantic guard corpus update unavailable: {}",
            status.reason.unwrap_or_else(|| "unknown reason".to_string())
        );
    }

    // 执行批量插入
    let upserted_records = semantic_guard.upsert_corpus(&records).await?;

    Ok(GuardCorpusUpdateReport {
        source: source.to_string(),
        sha256: actual_sha256,
        parsed_records: records.len(),
        upserted_records,
        collection: config.security.semantic_guard_collection.clone(),
    })
}

/// 解析 Qdrant URL
///
/// 从记忆配置或环境变量中获取 Qdrant 服务器 URL。
/// 优先使用配置文件中的设置，如果未配置则回退到环境变量 `QDRANT_URL`。
///
/// # 参数
///
/// - `memory` - 记忆配置
///
/// # 返回值
///
/// 如果找到有效的 URL，返回 `Some(String)`，否则返回 `None`
fn resolve_qdrant_url(memory: &MemoryConfig) -> Option<String> {
    memory
        .qdrant
        .url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            // 回退到环境变量
            std::env::var("QDRANT_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

/// 解析 Qdrant API 密钥
///
/// 从记忆配置或环境变量中获取 Qdrant API 密钥。
/// 优先使用配置文件中的设置，如果未配置则回退到环境变量 `QDRANT_API_KEY`。
///
/// # 参数
///
/// - `memory` - 记忆配置
///
/// # 返回值
///
/// 如果找到有效的 API 密钥，返回 `Some(String)`，否则返回 `None`
fn resolve_qdrant_api_key(memory: &MemoryConfig) -> Option<String> {
    memory
        .qdrant
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            // 回退到环境变量
            std::env::var("QDRANT_API_KEY")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

/// 从记忆类别中提取语义防护类别名称
///
/// 语义防护记录存储时会添加 `semantic_guard:` 前缀，
/// 此函数用于从存储的类别中提取原始类别名称。
///
/// # 参数
///
/// - `category` - 记忆类别
///
/// # 返回值
///
/// 返回去除前缀后的类别名称
fn category_name_from_memory(category: &MemoryCategory) -> String {
    match category {
        MemoryCategory::Custom(name) => {
            // 移除 "semantic_guard:" 前缀
            name.strip_prefix("semantic_guard:").unwrap_or(name).to_string()
        }
        other => other.to_string(),
    }
}

/// 规范化语料库类别名称
///
/// 将类别名称转换为标准格式：小写、下划线替换空格。
///
/// # 参数
///
/// - `raw` - 原始类别名称
///
/// # 返回值
///
/// 返回规范化后的类别名称
///
/// # 错误
///
/// - 如果类别为空，返回错误
/// - 如果包含非法字符（非字母数字、下划线或连字符），返回错误
fn normalize_corpus_category(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_lowercase().replace(' ', "_");

    // 验证类别不为空
    if normalized.is_empty() {
        bail!("category must not be empty");
    }

    // 验证字符合法性
    if !normalized.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
        bail!("category contains unsupported characters: {normalized}");
    }

    Ok(normalized)
}

/// 生成语料库记录的唯一键
///
/// 基于类别和文本内容生成 SHA256 哈希作为记录的唯一标识符。
///
/// # 参数
///
/// - `category` - 记录类别
/// - `text` - 记录文本内容
///
/// # 返回值
///
/// 返回格式为 `"sg-{hash}"` 的唯一键
fn corpus_record_key(category: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(category.as_bytes());
    hasher.update([0]); // 添加分隔符
    hasher.update(text.trim().as_bytes());
    format!("sg-{}", hex::encode(hasher.finalize()))
}

/// 计算字节数组的 SHA256 哈希（十六进制格式）
///
/// # 参数
///
/// - `bytes` - 待哈希的字节数组
///
/// # 返回值
///
/// 返回十六进制格式的 SHA256 哈希字符串
fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// 解析 JSONL 格式的防护语料库
///
/// 将 JSONL 格式的文本解析为 `GuardCorpusRecord` 数组。
/// 支持注释行（以 `#` 开头）和空行。
///
/// # 参数
///
/// - `raw` - JSONL 格式的原始文本
///
/// # 返回值
///
/// 返回解析后的记录数组（已去重）
///
/// # 错误
///
/// - 如果某行 JSON 格式无效，返回错误（包含行号）
/// - 如果 `text` 字段为空，返回错误
/// - 如果 `category` 字段为空或格式无效，返回错误
/// - 如果解析后没有任何有效记录，返回错误
///
/// # JSONL 格式要求
///
/// 每行必须是一个有效的 JSON 对象，包含以下字段：
/// - `text`（必需）- 攻击模式文本
/// - `category`（必需）- 攻击类别
/// - `source`（可选）- 记录来源
/// - `id`（可选）- 唯一标识符
fn parse_guard_corpus_jsonl(raw: &str) -> Result<Vec<GuardCorpusRecord>> {
    let mut records = Vec::new();
    let mut seen = HashSet::new(); // 用于去重

    for (idx, line) in raw.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();

        // 跳过空行和注释行
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // 解析 JSON 行
        let mut record: GuardCorpusRecord = serde_json::from_str(trimmed).with_context(|| {
            format!("Invalid guard corpus JSONL schema at line {line_no}: expected JSON object")
        })?;

        // 验证必需字段
        if record.text.trim().is_empty() {
            bail!("Invalid guard corpus JSONL schema at line {line_no}: `text` is required");
        }
        if record.category.trim().is_empty() {
            bail!("Invalid guard corpus JSONL schema at line {line_no}: `category` is required");
        }

        // 规范化字段值
        record.text = record.text.trim().to_string();
        record.category = normalize_corpus_category(&record.category).with_context(|| {
            format!("Invalid guard corpus JSONL schema at line {line_no}: invalid `category` value")
        })?;

        // 清理空的 ID 字段
        if let Some(id) = record.id.as_deref().map(str::trim) {
            if id.is_empty() {
                record.id = None;
            }
        }

        // 基于类别和文本（小写）去重
        let dedupe_key = format!("{}:{}", record.category, record.text.to_ascii_lowercase());
        if seen.insert(dedupe_key) {
            records.push(record);
        }
    }

    // 确保至少有一条有效记录
    if records.is_empty() {
        bail!("Guard corpus is empty after parsing");
    }

    Ok(records)
}

/// 加载语料库来源
///
/// 根据来源标识符加载语料库内容。支持三种来源类型：
/// 1. 内置语料库（`"builtin"`）
/// 2. 远程 URL（`http://` 或 `https://`）
/// 3. 本地文件路径
///
/// # 参数
///
/// - `source` - 来源标识符
///
/// # 返回值
///
/// 返回语料库的文本内容
///
/// # 错误
///
/// - 如果 URL 下载失败，返回错误
/// - 如果 HTTP 响应状态码表示失败，返回错误
/// - 如果读取本地文件失败，返回错误
/// - 在 WASM 环境中尝试读取本地文件，返回错误
///
/// # 平台差异
///
/// - 在 WASM 环境中，不支持读取本地文件
/// - URL 下载使用配置的 HTTP 代理设置
async fn load_corpus_source(source: &str) -> Result<String> {
    // 处理内置语料库
    if source.eq_ignore_ascii_case(BUILTIN_SOURCE) {
        return Ok(BUILTIN_CORPUS_JSONL.to_string());
    }

    // 处理远程 URL
    if source.starts_with("http://") || source.starts_with("https://") {
        let response = agent_config::runtime::build_runtime_proxy_client("memory.qdrant")
            .get(source)
            .send()
            .await
            .with_context(|| format!("failed to download guard corpus from {source}"))?;

        // 检查 HTTP 状态码
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("guard corpus download failed ({status}): {body}");
        }

        return response.text().await.context("failed to read downloaded guard corpus body");
    }

    // 处理本地文件（非 WASM 环境）
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::fs::read_to_string(source)
            .await
            .with_context(|| format!("failed to read guard corpus file at {source}"))
    }

    // WASM 环境不支持本地文件访问
    #[cfg(target_arch = "wasm32")]
    {
        anyhow::bail!("Cannot read guard corpus file in WASM: {source}")
    }
}

#[cfg(test)]
#[path = "semantic_guard_tests.rs"]
mod semantic_guard_tests;
