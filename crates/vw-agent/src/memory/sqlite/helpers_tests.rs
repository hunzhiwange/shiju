// 验证 SQLite 记忆辅助函数的分类转换与内容哈希稳定性。
use super::*;
use crate::memory::traits::MemoryCategory;

#[test]
fn sqlite_category_round_trips_known_and_custom_values() {
    assert_eq!(SqliteMemory::category_to_str(&MemoryCategory::Conversation), "conversation");
    assert_eq!(SqliteMemory::str_to_category("core"), MemoryCategory::Core);
    assert_eq!(SqliteMemory::str_to_category("project"), MemoryCategory::Custom("project".into()));
}

#[test]
fn content_hash_is_deterministic_and_compact() {
    let first = SqliteMemory::content_hash("same content");
    assert_eq!(first, SqliteMemory::content_hash("same content"));
    assert_eq!(first.len(), 16);
    assert_ne!(first, SqliteMemory::content_hash("other content"));
}
