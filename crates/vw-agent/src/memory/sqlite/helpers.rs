//! SQLite 记忆后端的局部转换工具。
//!
//! 这里集中放置分类序列化和内容哈希逻辑，避免存储、检索、缓存实现各自维护一套映射规则。

use super::SqliteMemory;
use crate::memory::traits::MemoryCategory;

impl SqliteMemory {
    /// 将内存中的记忆分类转换为数据库中存储的字符串。
    pub(super) fn category_to_str(cat: &MemoryCategory) -> String {
        match cat {
            MemoryCategory::Core => "core".into(),
            MemoryCategory::Daily => "daily".into(),
            MemoryCategory::Conversation => "conversation".into(),
            MemoryCategory::Custom(name) => name.clone(),
        }
    }

    /// 将数据库中的分类字符串还原为记忆分类。
    ///
    /// 未知字符串会保留为自定义分类，避免读取旧数据或扩展分类时丢失语义。
    pub(super) fn str_to_category(s: &str) -> MemoryCategory {
        match s {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
        }
    }

    /// 为文本生成短哈希，作为嵌入缓存键。
    ///
    /// 使用 SHA-256 的前 64 位可以保持确定性，同时让缓存键比完整哈希更紧凑。
    pub(super) fn content_hash(text: &str) -> String {
        use sha2::{Digest, Sha256};

        let hash = Sha256::digest(text.as_bytes());
        format!(
            "{:016x}",
            u64::from_be_bytes(hash[..8].try_into().expect("SHA-256 总是产生 >= 8 字节"))
        )
    }
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
