//! 验证旧入口与独立记忆契约包的类型身份及空实现行为。

use crate::memory::{Memory, MemoryCategory, NoneMemory};
use crate::memory::embeddings::NoopEmbedding;

#[tokio::test]
async fn memory_types_production_boundary() {
    let embedding = NoopEmbedding;
    let provider: &dyn memory_types::memory::embeddings::EmbeddingProvider = &embedding;
    assert_eq!(provider.dimensions(), 0);
    assert!(provider.embed(&["local boundary"]).await.unwrap().is_empty());

    let memory: memory_types::memory::NoneMemory = NoneMemory::new();
    let category: memory_types::memory::MemoryCategory = MemoryCategory::Core;
    let _: &dyn memory_types::memory::Memory = &memory;
    memory.store("boundary", "value", category, None).await.unwrap();
    let entry: Option<memory_types::memory::MemoryEntry> = memory.get("boundary").await.unwrap();
    assert!(entry.is_none());
    assert_eq!(memory.count().await.unwrap(), 0);
}
