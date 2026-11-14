//! 嵌入提供者契约与空实现。

use async_trait::async_trait;

/// 嵌入提供者的平台相关约束 trait
///
/// 该 trait 用于在不同平台上定义嵌入提供者的类型约束。
/// 通过条件编译，在原生平台上要求 `Send + Sync`，
/// 在 WebAssembly 平台上放宽这些约束。
#[cfg(not(target_arch = "wasm32"))]
pub trait EmbeddingProviderBounds: Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> EmbeddingProviderBounds for T {}

/// 嵌入提供者的平台相关约束 trait（WebAssembly 版本）
///
/// 在 WebAssembly 平台上，由于单线程环境的限制，
/// 不要求实现 `Send` trait。
#[cfg(target_arch = "wasm32")]
pub trait EmbeddingProviderBounds {}

#[cfg(target_arch = "wasm32")]
impl<T> EmbeddingProviderBounds for T {}

/// 嵌入提供者的核心接口
///
/// 所有嵌入服务都必须实现此 trait。该接口定义了获取嵌入向量的标准方法，
/// 支持批量处理和单个文本处理两种模式。
///
/// # 类型参数
///
/// 继承自 [`EmbeddingProviderBounds`]，在不同平台上有不同的约束要求。
///
/// # 示例
///
/// ```ignore
/// use memory_types::memory::embeddings::EmbeddingProvider;
///
/// async fn get_embeddings(provider: &dyn EmbeddingProvider) {
///     // 批量获取嵌入向量
///     let texts = vec!["你好世界", "Hello World"];
///     let embeddings = provider.embed(&texts).await.unwrap();
///
///     // 获取单个文本的嵌入向量
///     let single = provider.embed_one("测试文本").await.unwrap();
/// }
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait EmbeddingProvider: EmbeddingProviderBounds {
    /// 返回嵌入提供者的名称
    ///
    /// # 返回值
    ///
    /// 提供者的标识名称，如 "openai"、"none" 等
    fn name(&self) -> &str;

    /// 返回嵌入向量的维度
    ///
    /// 不同的嵌入模型会生成不同维度的向量。
    /// 例如，OpenAI 的 text-embedding-ada-002 生成 1536 维向量。
    ///
    /// # 返回值
    ///
    /// 嵌入向量的维度数量
    fn dimensions(&self) -> usize;

    /// 批量获取多个文本的嵌入向量
    ///
    /// 这是嵌入功能的核心方法，将一组文本转换为对应的高维向量。
    /// 批量处理通常比单独处理更高效。
    ///
    /// # 参数
    ///
    /// - `texts`: 待嵌入的文本切片数组
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<Vec<f32>>)`: 成功时返回嵌入向量数组，每个内部 Vec 对应一个输入文本
    /// - `Err`: 请求失败或解析错误
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let texts = vec!["文本1", "文本2", "文本3"];
    /// let embeddings = provider.embed(&texts).await?;
    /// assert_eq!(embeddings.len(), 3);
    /// ```
    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>>;

    /// 获取单个文本的嵌入向量
    ///
    /// 这是 [`embed`](EmbeddingProvider::embed) 的便捷包装方法，
    /// 用于处理单个文本的场景。
    ///
    /// # 参数
    ///
    /// - `text`: 待嵌入的单个文本
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<f32>)`: 成功时返回嵌入向量
    /// - `Err`: 请求失败、解析错误或结果为空
    ///
    /// # 默认实现
    ///
    /// 默认实现调用 [`embed`](EmbeddingProvider::embed) 并提取第一个结果。
    /// 如果结果为空则返回错误。
    async fn embed_one(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut results = self.embed(&[text]).await?;
        results.pop().ok_or_else(|| anyhow::anyhow!("Empty embedding result"))
    }
}

/// 空操作的嵌入提供者
///
/// 该实现不执行任何实际的嵌入操作，始终返回空结果。
/// 主要用于以下场景：
/// - 禁用嵌入功能
/// - 测试和开发环境
/// - 不需要语义搜索功能的配置
///
/// # 特性
///
/// - 名称: "none"
/// - 维度: 0
/// - 嵌入结果: 始终为空数组
pub struct NoopEmbedding;

/// 为 NoopEmbedding 实现 EmbeddingProvider trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl EmbeddingProvider for NoopEmbedding {
    /// 返回提供者名称
    ///
    /// # 返回值
    ///
    /// 固定返回 "none"
    fn name(&self) -> &str {
        "none"
    }

    /// 返回嵌入向量维度
    ///
    /// # 返回值
    ///
    /// 固定返回 0，表示不提供实际的嵌入功能
    fn dimensions(&self) -> usize {
        0
    }

    /// 执行嵌入操作（空实现）
    ///
    /// # 参数
    ///
    /// - `_texts`: 忽略所有输入文本
    ///
    /// # 返回值
    ///
    /// 始终返回空的向量数组
    async fn embed(&self, _texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
#[path = "embeddings_tests.rs"]
mod tests;
