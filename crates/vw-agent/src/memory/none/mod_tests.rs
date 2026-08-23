// 验证禁用记忆后端保持无状态并返回约定的空结果。
use super::*;

#[test]
fn none_memory_is_zero_sized_copyable_and_named() {
    let memory = NoneMemory::new();
    let copied = memory;
    let cloned = copied;

    assert_eq!(std::mem::size_of::<NoneMemory>(), 0);
    assert_eq!(format!("{cloned:?}"), "NoneMemory");
    assert_eq!(NoneMemory::default().name(), "none");
}

#[tokio::test]
async fn store_and_forget_accept_all_inputs_without_state() {
    let memory = NoneMemory::new();

    memory.store("", "", MemoryCategory::Daily, None).await.unwrap();
    memory
        .store(
            "session-key",
            "content that would normally be persisted",
            MemoryCategory::Custom("custom".into()),
            Some("session-1"),
        )
        .await
        .unwrap();

    assert!(!memory.forget("").await.unwrap());
    assert!(!memory.forget("session-key").await.unwrap());
    assert_eq!(memory.count().await.unwrap(), 0);
}

#[tokio::test]
async fn reads_always_return_empty_results_for_every_filter_shape() {
    let memory = NoneMemory::new();
    let category = MemoryCategory::Conversation;

    assert!(memory.get("").await.unwrap().is_none());
    assert!(memory.get("missing").await.unwrap().is_none());
    assert!(memory.recall("", 0, None).await.unwrap().is_empty());
    assert!(memory.recall("anything", usize::MAX, Some("session")).await.unwrap().is_empty());
    assert!(memory.list(None, None).await.unwrap().is_empty());
    assert!(memory.list(Some(&category), Some("session")).await.unwrap().is_empty());
}

#[tokio::test]
async fn health_check_is_always_true() {
    assert!(NoneMemory::new().health_check().await);
}
