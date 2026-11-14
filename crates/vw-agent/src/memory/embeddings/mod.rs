//! 文本嵌入模块
//!
//! 本模块提供文本嵌入（Text Embedding）功能的抽象接口和具体实现。
//! 文本嵌入是将文本转换为高维向量表示的技术，广泛应用于语义搜索、
//! 相似度计算、聚类分析等场景。
//!
//! # 核心组件
//!
//! - [`EmbeddingProvider`]: 嵌入提供者的核心 trait，定义了所有嵌入服务必须实现的接口
//! - [`OpenAiEmbedding`]: OpenAI 兼容的嵌入服务实现
//! - [`AlibabaEmbedding`]: 阿里 DashScope OpenAI 兼容嵌入服务实现
//! - [`NoopEmbedding`]: 空操作的嵌入实现，用于禁用嵌入功能
//! - [`create_embedding_provider`]: 工厂函数，根据配置创建对应的嵌入提供者
//!
//! # 平台适配
//!
//! 本模块通过条件编译支持多种平台：
//! - 原生平台：要求嵌入提供者实现 `Send + Sync`
//! - WebAssembly 平台：放宽了 `Send` 约束以适应单线程环境

use async_trait::async_trait;

mod alibaba;

pub use alibaba::AlibabaEmbedding;

const ALIBABA_EMBEDDING_BASE_URL: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1";
const ALIBABA_CN_EMBEDDING_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

pub use memory_types::memory::embeddings::{
    EmbeddingProvider, EmbeddingProviderBounds, NoopEmbedding,
};

/// OpenAI 兼容的嵌入提供者
///
/// 该结构体实现了与 OpenAI Embedding API 兼容的嵌入服务。
/// 支持以下服务提供商：
/// - OpenAI 官方 API
/// - OpenRouter 代理服务
/// - 任何兼容 OpenAI API 的自定义服务
///
/// # 字段说明
///
/// - `base_url`: API 基础 URL（不含尾部斜杠）
/// - `api_key`: API 认证密钥
/// - `model`: 使用的嵌入模型名称
/// - `dims`: 嵌入向量的维度
///
/// # 示例
///
/// ```ignore
/// use vibe_agent::memory::embeddings::OpenAiEmbedding;
///
/// let embedding = OpenAiEmbedding::new(
///     "https://api.openai.com",
///     "sk-...",
///     "text-embedding-ada-002",
///     1536
/// );
/// ```
pub struct OpenAiEmbedding {
    /// API 基础 URL
    pub base_url: String,
    /// API 认证密钥
    api_key: String,
    /// 嵌入模型名称
    model: String,
    /// 嵌入向量维度
    dims: usize,
}

impl OpenAiEmbedding {
    /// 创建新的 OpenAI 嵌入提供者实例
    ///
    /// # 参数
    ///
    /// - `base_url`: API 服务的基​​础 URL（会自动移除尾部斜杠）
    /// - `api_key`: API 认证密钥
    /// - `model`: 使用的嵌入模型名称
    /// - `dims`: 嵌入向量的维度
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `OpenAiEmbedding` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let provider = OpenAiEmbedding::new(
    ///     "https://api.openai.com/",
    ///     "sk-xxxxx",
    ///     "text-embedding-ada-002",
    ///     1536
    /// );
    /// ```
    pub fn new(base_url: &str, api_key: &str, model: &str, dims: usize) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            dims,
        }
    }

    /// 创建配置了代理的 HTTP 客户端
    ///
    /// 使用全局配置中的代理设置创建 HTTP 客户端，
    /// 用于与嵌入 API 服务进行通信。
    ///
    /// # 返回值
    ///
    /// 返回配置好的 reqwest 客户端实例
    fn http_client(&self) -> reqwest::Client {
        crate::app::agent::config::build_runtime_proxy_client("memory.embeddings")
    }

    /// 检查 base_url 是否包含显式的 API 路径
    ///
    /// 该方法用于判断 URL 中是否已经包含了 API 路径，
    /// 而不仅仅是协议和域名。
    ///
    /// # 返回值
    ///
    /// - `true`: URL 包含非空的路径（不是 "/"）
    /// - `false`: URL 路径为空或仅为 "/"
    ///
    /// # 示例
    ///
    /// - `https://api.example.com/v1` -> true
    /// - `https://api.example.com/` -> false
    fn has_explicit_api_path(&self) -> bool {
        let Ok(url) = reqwest::Url::parse(&self.base_url) else {
            return false;
        };

        let path = url.path().trim_end_matches('/');
        !path.is_empty() && path != "/"
    }

    /// 检查 URL 是否已经包含 embeddings 端点
    ///
    /// 该方法用于判断 URL 是否已经完整地包含了 embeddings API 端点。
    ///
    /// # 返回值
    ///
    /// - `true`: URL 路径以 "/embeddings" 结尾
    /// - `false`: URL 路径不以 "/embeddings" 结尾
    fn has_embeddings_endpoint(&self) -> bool {
        let Ok(url) = reqwest::Url::parse(&self.base_url) else {
            return false;
        };

        url.path().trim_end_matches('/').ends_with("/embeddings")
    }

    /// 构建完整的 embeddings API URL
    ///
    /// 根据不同的 URL 格式智能构建完整的 API 端点地址。
    ///
    /// # URL 构建规则
    ///
    /// 1. 如果 URL 已包含 "/embeddings" 端点，直接使用原 URL
    /// 2. 如果 URL 包含显式路径（如 "/v1"），追加 "/embeddings"
    /// 3. 如果 URL 仅为域名，追加 "/v1/embeddings"
    ///
    /// # 返回值
    ///
    /// 返回完整的 embeddings API URL 字符串
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 情况 1: 已包含端点
    /// assert_eq!(provider.embeddings_url(), "https://api.example.com/embeddings");
    ///
    /// // 情况 2: 包含显式路径
    /// assert_eq!(provider.embeddings_url(), "https://api.example.com/v1/embeddings");
    ///
    /// // 情况 3: 仅域名
    /// assert_eq!(provider.embeddings_url(), "https://api.example.com/v1/embeddings");
    /// ```
    pub fn embeddings_url(&self) -> String {
        if self.has_embeddings_endpoint() {
            return self.base_url.clone();
        }

        if self.has_explicit_api_path() {
            format!("{}/embeddings", self.base_url)
        } else {
            format!("{}/v1/embeddings", self.base_url)
        }
    }
}

/// 为 OpenAiEmbedding 实现 EmbeddingProvider trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl EmbeddingProvider for OpenAiEmbedding {
    /// 返回提供者名称
    ///
    /// # 返回值
    ///
    /// 固定返回 "openai"
    fn name(&self) -> &str {
        "openai"
    }

    /// 返回嵌入向量维度
    ///
    /// # 返回值
    ///
    /// 返回创建时配置的维度值
    fn dimensions(&self) -> usize {
        self.dims
    }

    /// 调用 OpenAI API 获取文本嵌入向量
    ///
    /// 该方法向 OpenAI 兼容的 API 发送 POST 请求，
    /// 将文本转换为高维向量表示。
    ///
    /// # 参数
    ///
    /// - `texts`: 待嵌入的文本数组
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<Vec<f32>>)`: 成功时返回嵌入向量数组
    /// - `Err`: API 调用失败或响应解析错误
    ///
    /// # 错误
    ///
    /// - API 返回非成功状态码
    /// - 响应 JSON 缺少必需字段
    /// - 嵌入数据格式不正确
    ///
    /// # 请求格式
    ///
    /// ```json
    /// {
    ///   "model": "text-embedding-ada-002",
    ///   "input": ["文本1", "文本2"]
    /// }
    /// ```
    ///
    /// # 响应格式
    ///
    /// ```json
    /// {
    ///   "data": [
    ///     {"embedding": [0.1, 0.2, ...]},
    ///     {"embedding": [0.3, 0.4, ...]}
    ///   ]
    /// }
    /// ```
    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let resp = self
            .http_client()
            .post(self.embeddings_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Embedding API error {status}: {text}");
        }

        let json: serde_json::Value = resp.json().await?;

        let data = json
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid embedding response: missing 'data'"))?;

        let mut embeddings = Vec::with_capacity(data.len());

        for item in data {
            let embedding = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| anyhow::anyhow!("Invalid embedding item"))?;

            #[allow(clippy::cast_possible_truncation)]
            let vec: Vec<f32> =
                embedding.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();

            embeddings.push(vec);
        }

        Ok(embeddings)
    }
}

/// 创建嵌入提供者的工厂函数
///
/// 根据提供者名称创建对应的嵌入提供者实例。
/// 该函数是创建嵌入提供者的推荐方式。
///
/// # 参数
///
/// - `provider`: 提供者类型标识符，支持以下值：
///   - `"openai"`: OpenAI 官方 API
///   - `"openrouter"`: OpenRouter 代理服务
///   - `"alibaba"`: 阿里 DashScope 国际站 OpenAI 兼容接口
///   - `"alibaba-cn"`: 阿里 DashScope 中国区 OpenAI 兼容接口
///   - `"custom:<url>"`: 自定义兼容服务（替换 <url> 为实际地址）
///   - 其他: 返回 NoopEmbedding（空操作）
/// - `api_key`: API 认证密钥（可选，未提供时使用空字符串）
/// - `model`: 嵌入模型名称
/// - `dims`: 嵌入向量维度
///
/// # 返回值
///
/// 返回装箱的 trait 对象，可通过统一接口调用
///
/// # 示例
///
/// ```ignore
/// use vibe_agent::memory::embeddings::create_embedding_provider;
///
/// // 创建 OpenAI 提供者
/// let provider = create_embedding_provider(
///     "openai",
///     Some("sk-xxxxx"),
///     "text-embedding-ada-002",
///     1536
/// );
///
/// // 创建 OpenRouter 提供者
/// let provider = create_embedding_provider(
///     "openrouter",
///     Some("sk-or-xxxxx"),
///     "openai/text-embedding-ada-002",
///     1536
/// );
///
/// // 创建自定义服务提供者
/// let provider = create_embedding_provider(
///     "custom:https://api.example.com",
///     Some("your-api-key"),
///     "embedding-model",
///     768
/// );
///
/// // 创建空操作提供者（禁用嵌入功能）
/// let provider = create_embedding_provider(
///     "none",
///     None,
///     "",
///     0
/// );
/// ```
pub fn create_embedding_provider(
    provider: &str,
    api_key: Option<&str>,
    model: &str,
    dims: usize,
) -> Box<dyn EmbeddingProvider> {
    let provider = provider.trim();
    let normalized_provider = provider.to_ascii_lowercase();
    match normalized_provider.as_str() {
        "openai" => {
            let key = api_key.unwrap_or("");
            Box::new(OpenAiEmbedding::new("https://api.openai.com", key, model, dims))
        }
        "openrouter" => {
            let key = api_key.unwrap_or("");
            Box::new(OpenAiEmbedding::new("https://openrouter.ai/api/v1", key, model, dims))
        }
        "alibaba" => {
            let key = api_key.unwrap_or("");
            Box::new(AlibabaEmbedding::new("alibaba", ALIBABA_EMBEDDING_BASE_URL, key, model, dims))
        }
        "alibaba-cn" => {
            let key = api_key.unwrap_or("");
            Box::new(AlibabaEmbedding::new(
                "alibaba-cn",
                ALIBABA_CN_EMBEDDING_BASE_URL,
                key,
                model,
                dims,
            ))
        }
        _ if provider.starts_with("custom:") => {
            let base_url = provider.strip_prefix("custom:").unwrap_or("");
            let key = api_key.unwrap_or("");
            Box::new(OpenAiEmbedding::new(base_url, key, model, dims))
        }
        _ => Box::new(NoopEmbedding),
    }
}

/// 模块单元测试
///
/// 测试文件位于 `tests.rs`，包含嵌入功能的各项测试用例。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
