use super::super::traits::{MemoryCategory, MemoryEntry};
use super::PostgresMemory;
use anyhow::Result;
use chrono::{DateTime, Utc};
use postgres::Row;

impl PostgresMemory {
    /// 将记忆分类枚举转换为数据库中的字符串表示。
    pub(super) fn category_to_str(category: &MemoryCategory) -> String {
        match category {
            MemoryCategory::Core => "core".to_string(),
            MemoryCategory::Daily => "daily".to_string(),
            MemoryCategory::Conversation => "conversation".to_string(),
            MemoryCategory::Custom(name) => name.clone(),
        }
    }

    /// 将数据库中的分类字符串解析为记忆分类枚举。
    pub(super) fn parse_category(value: &str) -> MemoryCategory {
        match value {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
        }
    }

    /// 将 PostgreSQL 行映射为记忆条目。
    pub(super) fn row_to_entry(row: &Row) -> Result<MemoryEntry> {
        let timestamp: DateTime<Utc> = row.get(4);

        Ok(Self::entry_from_values(
            row.get(0),
            row.get(1),
            row.get(2),
            row.get::<_, String>(3),
            timestamp,
            row.get(5),
            row.try_get(6).ok(),
        ))
    }

    fn entry_from_values(
        id: String,
        key: String,
        content: String,
        category: String,
        timestamp: DateTime<Utc>,
        session_id: Option<String>,
        score: Option<f64>,
    ) -> MemoryEntry {
        MemoryEntry {
            id,
            key,
            content,
            category: Self::parse_category(&category),
            timestamp: timestamp.to_rfc3339(),
            session_id,
            score,
        }
    }
}

/// 验证 SQL 标识符是否合法。
pub(super) fn validate_identifier(value: &str, field_name: &str) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("{field_name} must not be empty");
    }

    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("{field_name} must not be empty");
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        anyhow::bail!("{field_name} must start with an ASCII letter or underscore; got '{value}'");
    }

    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        anyhow::bail!(
            "{field_name} can only contain ASCII letters, numbers, and underscores; got '{value}'"
        );
    }

    Ok(())
}

/// 将合法标识符包裹为双引号形式。
pub(super) fn quote_identifier(value: &str) -> String {
    format!("\"{value}\"")
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
