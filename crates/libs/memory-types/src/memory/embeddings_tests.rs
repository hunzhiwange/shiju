//! 空嵌入行为测试。

use super::*;

/// 测试 NoopEmbedding 的名称和维度属性
///
/// # 验证点
///
/// - `name()` 返回 `"none"`
/// - `dimensions()` 返回 `0`
#[test]
fn noop_name() {
    let p = NoopEmbedding;
    assert_eq!(p.name(), "none");
    assert_eq!(p.dimensions(), 0);
}

/// 测试 NoopEmbedding 的 embed 方法返回空结果
///
/// # 验证点
///
/// - 对非空输入数组，`embed()` 应返回空向量
/// - 不应返回错误
#[tokio::test]
async fn noop_embed_returns_empty() {
    let p = NoopEmbedding;
    let result = p.embed(&["hello"]).await.unwrap();
    assert!(result.is_empty());
}

/// 测试 NoopEmbedding 的 embed_one 方法返回错误
///
/// # 验证点
///
/// - `embed_one()` 应返回错误而非空向量
/// - 区别于 `embed()` 方法的行为
#[tokio::test]
async fn noop_embed_one_returns_error() {
    let p = NoopEmbedding;
    let result = p.embed_one("hello").await;
    assert!(result.is_err());
}

/// 测试 NoopEmbedding 处理空批次
///
/// # 验证点
///
/// - 对空数组调用 `embed()` 应返回空向量
/// - 不应产生错误
#[tokio::test]
async fn noop_embed_empty_batch() {
    let p = NoopEmbedding;
    let result = p.embed(&[]).await.unwrap();
    assert!(result.is_empty());
}

/// 测试 NoopEmbedding 处理多个文本
///
/// # 验证点
///
/// - 无论输入多少个文本，NoopEmbedding 始终返回空结果
/// - 这是空操作提供者的预期行为
#[tokio::test]
async fn noop_embed_multiple_texts() {
    let p = NoopEmbedding;
    let result = p.embed(&["a", "b", "c"]).await.unwrap();
    assert!(result.is_empty());
}
