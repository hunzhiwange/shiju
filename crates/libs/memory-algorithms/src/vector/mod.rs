//! 向量操作模块
//!
//! 本模块提供向量检索和相似度计算的核心工具函数，支持语义搜索的底层计算需求。
//!
//! # 主要功能
//!
//! - **余弦相似度计算**：衡量两个向量之间的相似程度，返回 [0.0, 1.0] 范围的分数
//! - **向量序列化/反序列化**：在 f32 向量与字节序列之间转换，便于存储和传输
//! - **混合检索合并**：将向量检索结果与关键词检索结果加权融合，提升召回质量
//!
//! # 典型使用场景
//!
//! 1. 语义搜索中对查询向量和文档向量计算相似度
//! 2. 将嵌入向量持久化到数据库前进行字节序列化
//! 3. 混合检索（向量 + BM25）结果的最终排序

/// 计算两个向量之间的余弦相似度。
///
/// 余弦相似度衡量两个向量在方向上的相似程度，值域为 [0.0, 1.0]。
/// 对于语义嵌入向量，该值越接近 1.0 表示语义越相似。
///
/// # 参数
///
/// - `a`: 第一个向量（f32 切片）
/// - `b`: 第二个向量（f32 切片）
///
/// # 返回值
///
/// 返回余弦相似度分数，范围 [0.0, 1.0]。在以下情况返回 0.0：
/// - 两个向量长度不一致
/// - 向量为空
/// - 计算过程中出现非有限值（NaN 或无穷大）
/// - 分母（模长的乘积）为零或可忽略
///
/// # 算法说明
///
/// 余弦相似度公式：`sim(a, b) = (a · b) / (||a|| * ||b||)`
///
/// 其中 `a · b` 为点积，`||a||` 为向量的欧几里得范数（模长）。
/// 为避免浮点精度问题，内部使用 f64 进行累积计算。
///
/// # 示例
///
/// ```ignore
/// use memory_algorithms::vector::cosine_similarity;
///
/// let a = vec![1.0, 2.0, 3.0];
/// let b = vec![1.0, 2.0, 3.0];
/// assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
///
/// let c = vec![-1.0, -2.0, -3.0];
/// // 对于典型的正嵌入向量，负向量会被限制到 0.0
/// assert_eq!(cosine_similarity(&a, &c), 0.0);
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    // 边界检查：长度不一致或空向量直接返回 0.0
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    // 使用 f64 进行累积计算以提高精度
    let mut dot = 0.0_f64; // 点积累加器
    let mut norm_a = 0.0_f64; // 向量 a 的平方和累加器
    let mut norm_b = 0.0_f64; // 向量 b 的平方和累加器

    // 遍历两个向量，同时计算点积和各向量的平方和
    for (x, y) in a.iter().zip(b.iter()) {
        let x = f64::from(*x);
        let y = f64::from(*y);
        dot += x * y; // 累加点积
        norm_a += x * x; // 累加向量 a 的平方
        norm_b += y * y; // 累加向量 b 的平方
    }

    // 计算分母：两个向量模长的乘积
    let denom = norm_a.sqrt() * norm_b.sqrt();
    // 如果分母非有限或过小，返回 0.0 避免除零
    if !denom.is_finite() || denom < f64::EPSILON {
        return 0.0;
    }

    // 计算原始相似度
    let raw = dot / denom;
    // 防止浮点异常
    if !raw.is_finite() {
        return 0.0;
    }

    // 将结果限制在 [0, 1] 范围内——语义嵌入通常为正值
    // 对于方向相反的向量，相似度会被限制为 0.0
    #[allow(clippy::cast_possible_truncation)]
    let sim = raw.clamp(0.0, 1.0) as f32;
    sim
}

/// 将 f32 向量序列化为字节数组（小端序）。
///
/// 每个 f32 值转换为 4 字节的小端序表示，适用于：
/// - 将嵌入向量存储到数据库的 BLOB 字段
/// - 网络传输向量数据
/// - 文件持久化
///
/// # 参数
///
/// - `v`: 要序列化的 f32 向量切片
///
/// # 返回值
///
/// 返回字节数组，长度为 `v.len() * 4`。
///
/// # 示例
///
/// ```ignore
/// use memory_algorithms::vector::vec_to_bytes;
///
/// let v = vec![1.0_f32, 2.0, 3.0];
/// let bytes = vec_to_bytes(&v);
/// assert_eq!(bytes.len(), 12); // 3 个 f32 * 4 字节
/// ```
pub fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    // 预分配容量：每个 f32 占 4 字节
    let mut bytes = Vec::with_capacity(v.len() * 4);
    // 遍历每个 f32 值，转换小端序字节并追加
    for &f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

/// 将字节数组反序列化为 f32 向量（小端序）。
///
/// 从 [`vec_to_bytes`] 生成的字节数组中恢复原始向量。
/// 每连续 4 字节被解析为一个 f32 值。
///
/// # 参数
///
/// - `bytes`: 小端序编码的字节数组切片
///
/// # 返回值
///
/// 返回解析后的 f32 向量。如果字节数组长度不是 4 的倍数，
/// 末尾不足 4 字节的部分将被忽略（解析为零值）。
///
/// # 注意事项
///
/// - 输入字节数组长度应为 4 的倍数
/// - 与 [`vec_to_bytes`] 配套使用以确保正确性
///
/// # 示例
///
/// ```ignore
/// use memory_algorithms::vector::{vec_to_bytes, bytes_to_vec};
///
/// let original = vec![1.0_f32, 2.0, 3.0];
/// let bytes = vec_to_bytes(&original);
/// let restored = bytes_to_vec(&bytes);
/// assert_eq!(original, restored);
/// ```
pub fn bytes_to_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        // 将字节切片按每 4 字节分组
        .chunks_exact(4)
        .map(|chunk| {
            // 尝试将 4 字节转换为数组，失败则使用零值
            let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
            // 从小端序字节解析 f32
            f32::from_le_bytes(arr)
        })
        .collect()
}

/// 带分数的检索结果。
///
/// 用于混合检索合并时，存储来自不同检索源（向量检索、关键词检索）
/// 的分数信息及最终融合分数。
///
/// # 字段说明
///
/// - `id`: 结果项的唯一标识符
/// - `vector_score`: 向量检索的相似度分数（来自余弦相似度，范围 [0.0, 1.0]）
/// - `keyword_score`: 关键词检索的相关性分数（来自 BM25，已归一化到 [0.0, 1.0]）
/// - `final_score`: 最终融合分数，由两个分数加权计算得出
#[derive(Debug, Clone)]
pub struct ScoredResult {
    /// 结果项的唯一标识符
    pub id: String,
    /// 向量检索分数（余弦相似度，0.0–1.0），若该结果未出现在向量检索中则为 None
    pub vector_score: Option<f32>,
    /// 关键词检索分数（BM25 归一化后，0.0–1.0），若该结果未出现在关键词检索中则为 None
    pub keyword_score: Option<f32>,
    /// 最终融合分数，由向量分数和关键词分数加权求和得出
    pub final_score: f32,
}

/// 混合检索结果合并：将向量检索和关键词检索结果加权融合。
///
/// 该函数实现混合检索（Hybrid Search）的核心逻辑，通过加权组合
/// 语义相似度（向量检索）和词汇匹配度（关键词检索/BM25）来提升检索质量。
///
/// # 算法流程
///
/// 1. 将两组结果按 id 合并去重
/// 2. 对关键词分数进行归一化（BM25 分数范围不确定，归一化到 [0, 1]）
///    向量分数已由余弦相似度保证在 [0, 1] 范围内
/// 3. 计算最终分数：`final_score = vector_weight * vector_score + keyword_weight * keyword_score`
/// 4. 按最终分数降序排序并截取前 N 个结果
///
/// # 参数
///
/// - `vector_results`: 向量检索结果列表，每项为 `(id, cosine_similarity)` 元组
/// - `keyword_results`: 关键词检索结果列表，每项为 `(id, bm25_score)` 元组
/// - `vector_weight`: 向量分数的权重（建议 0.0–1.0）
/// - `keyword_weight`: 关键词分数的权重（建议 0.0–1.0）
/// - `limit`: 返回结果的最大数量
///
/// # 返回值
///
/// 返回按最终分数降序排列的 [`ScoredResult`] 列表，最多包含 `limit` 个元素。
///
/// # 权重建议
///
/// - 均衡融合：`vector_weight = 0.5`, `keyword_weight = 0.5`
/// - 偏向语义：`vector_weight = 0.7`, `keyword_weight = 0.3`
/// - 偏向精确匹配：`vector_weight = 0.3`, `keyword_weight = 0.7`
///
/// # 示例
///
/// ```ignore
/// use memory_algorithms::vector::hybrid_merge;
///
/// let vector_results = vec![
///     ("doc1".to_string(), 0.95),
///     ("doc2".to_string(), 0.80),
/// ];
/// let keyword_results = vec![
///     ("doc1".to_string(), 5.0),  // BM25 原始分数
///     ("doc3".to_string(), 4.0),
/// ];
///
/// let merged = hybrid_merge(&vector_results, &keyword_results, 0.6, 0.4, 10);
/// // merged[0].id == "doc1"（同时出现在两组中，综合分数最高）
/// ```
pub fn hybrid_merge(
    vector_results: &[(String, f32)], // (id, cosine_similarity) 向量检索结果
    keyword_results: &[(String, f32)], // (id, bm25_score) 关键词检索结果
    vector_weight: f32,               // 向量分数权重
    keyword_weight: f32,              // 关键词分数权重
    limit: usize,                     // 返回结果数量上限
) -> Vec<ScoredResult> {
    use std::collections::HashMap;

    // 使用 HashMap 按 id 合并两组结果
    let mut map: HashMap<String, ScoredResult> = HashMap::new();

    // 处理向量检索结果
    // 向量分数（余弦相似度）已经在 [0, 1] 范围内，无需归一化
    for (id, score) in vector_results {
        map.entry(id.clone())
            .and_modify(|r| r.vector_score = Some(*score)) // 更新已存在结果的向量分数
            .or_insert_with(|| {
                // 插入新结果
                ScoredResult {
                    id: id.clone(),
                    vector_score: Some(*score),
                    keyword_score: None,
                    final_score: 0.0,
                }
            });
    }

    // 计算关键词分数的最大值用于归一化
    // BM25 分数范围不确定，需要归一化到 [0, 1]
    let max_kw = keyword_results.iter().map(|(_, s)| *s).fold(0.0_f32, f32::max);
    // 避免除零：最大值为零时使用 1.0 作为除数
    let max_kw = if max_kw < f32::EPSILON { 1.0 } else { max_kw };

    // 处理关键词检索结果
    for (id, score) in keyword_results {
        // 归一化关键词分数到 [0, 1] 范围
        let normalized = score / max_kw;
        map.entry(id.clone())
            .and_modify(|r| r.keyword_score = Some(normalized)) // 更新已存在结果的关键词分数
            .or_insert_with(|| {
                // 插入新结果
                ScoredResult {
                    id: id.clone(),
                    vector_score: None,
                    keyword_score: Some(normalized),
                    final_score: 0.0,
                }
            });
    }

    // 计算最终融合分数
    let mut results: Vec<ScoredResult> = map
        .into_values()
        .map(|mut r| {
            // 缺失的分数用 0.0 填充
            let vs = r.vector_score.unwrap_or(0.0);
            let ks = r.keyword_score.unwrap_or(0.0);
            // 加权求和计算最终分数
            r.final_score = vector_weight * vs + keyword_weight * ks;
            r
        })
        .collect();

    // 按最终分数降序排序
    results.sort_by(|a, b| {
        b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal)
    });
    // 截取前 limit 个结果
    results.truncate(limit);
    results
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
