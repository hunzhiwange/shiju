// 验证 PostgreSQL 记忆辅助函数的分类转换与标识符防注入约束。
use super::*;
use crate::memory::traits::MemoryCategory;
use anyhow::Result;
use chrono::{DateTime, Utc};

#[test]
fn postgres_category_round_trips_known_and_custom_values() {
    assert_eq!(PostgresMemory::category_to_str(&MemoryCategory::Core), "core");
    assert_eq!(PostgresMemory::category_to_str(&MemoryCategory::Daily), "daily");
    assert_eq!(PostgresMemory::category_to_str(&MemoryCategory::Conversation), "conversation");
    assert_eq!(
        PostgresMemory::category_to_str(&MemoryCategory::Custom("archive".into())),
        "archive"
    );
    assert_eq!(PostgresMemory::parse_category("core"), MemoryCategory::Core);
    assert_eq!(PostgresMemory::parse_category("daily"), MemoryCategory::Daily);
    assert_eq!(PostgresMemory::parse_category("conversation"), MemoryCategory::Conversation);
    assert_eq!(PostgresMemory::parse_category("archive"), MemoryCategory::Custom("archive".into()));
}

#[test]
fn parse_category_treats_only_exact_builtin_names_as_builtin() {
    assert_eq!(PostgresMemory::parse_category("Core"), MemoryCategory::Custom("Core".into()));
    assert_eq!(PostgresMemory::parse_category(" core"), MemoryCategory::Custom(" core".into()));
    assert_eq!(PostgresMemory::parse_category("core "), MemoryCategory::Custom("core ".into()));
    assert_eq!(PostgresMemory::parse_category(""), MemoryCategory::Custom(String::new()));
}

#[test]
fn identifier_validation_rejects_injection_shapes() {
    assert!(validate_identifier("valid_name_1", "field").is_ok());
    assert!(validate_identifier("_starts_with_underscore", "field").is_ok());
    assert!(validate_identifier("", "field").is_err());
    assert!(validate_identifier("1name", "field").is_err());
    assert!(validate_identifier("name;drop", "field").is_err());
    assert!(validate_identifier("with-hyphen", "field").is_err());
    assert!(validate_identifier("contains space", "field").is_err());
}

#[test]
fn identifier_validation_errors_name_the_invalid_field() {
    let empty = validate_identifier("", "storage schema").unwrap_err().to_string();
    let bad_start = validate_identifier("9bad", "storage table").unwrap_err().to_string();
    let bad_char = validate_identifier("bad.table", "storage table").unwrap_err().to_string();

    assert!(empty.contains("storage schema must not be empty"));
    assert!(bad_start.contains("storage table must start with an ASCII letter or underscore"));
    assert!(bad_char.contains("storage table can only contain ASCII letters"));
}

#[test]
fn identifier_validation_accepts_minimal_ascii_identifiers() {
    assert!(validate_identifier("a", "field").is_ok());
    assert!(validate_identifier("Z", "field").is_ok());
    assert!(validate_identifier("_", "field").is_ok());
}

#[test]
fn identifier_validation_rejects_non_ascii_characters() {
    let bad_start = validate_identifier("éclair", "storage schema").unwrap_err().to_string();
    let bad_body = validate_identifier("schema_名", "storage schema").unwrap_err().to_string();

    assert!(bad_start.contains("must start with an ASCII letter or underscore"));
    assert!(bad_body.contains("can only contain ASCII letters, numbers, and underscores"));
}

#[test]
fn quote_identifier_wraps_without_mutating_contents() {
    assert_eq!(quote_identifier("memories"), "\"memories\"");
    assert_eq!(quote_identifier("_table_1"), "\"_table_1\"");
}

#[test]
fn entry_from_values_maps_database_values_to_memory_entry() {
    let timestamp: DateTime<Utc> =
        DateTime::parse_from_rfc3339("2026-06-13T08:09:10Z").unwrap().with_timezone(&Utc);

    let entry = PostgresMemory::entry_from_values(
        "id-1".to_string(),
        "key".to_string(),
        "content".to_string(),
        "daily".to_string(),
        timestamp,
        Some("session".to_string()),
        Some(0.75),
    );

    assert_eq!(entry.id, "id-1");
    assert_eq!(entry.key, "key");
    assert_eq!(entry.content, "content");
    assert_eq!(entry.category, MemoryCategory::Daily);
    assert_eq!(entry.timestamp, "2026-06-13T08:09:10+00:00");
    assert_eq!(entry.session_id.as_deref(), Some("session"));
    assert_eq!(entry.score, Some(0.75));
}

#[test]
fn entry_from_values_preserves_missing_session_and_score() {
    let timestamp: DateTime<Utc> =
        DateTime::parse_from_rfc3339("2026-06-13T08:09:10Z").unwrap().with_timezone(&Utc);

    let entry = PostgresMemory::entry_from_values(
        "id-2".to_string(),
        "key".to_string(),
        "content".to_string(),
        "custom".to_string(),
        timestamp,
        None,
        None,
    );

    assert_eq!(entry.category, MemoryCategory::Custom("custom".into()));
    assert!(entry.session_id.is_none());
    assert!(entry.score.is_none());
}

#[test]
fn entry_from_values_normalizes_timezone_and_preserves_boundary_values() {
    let timestamp: DateTime<Utc> =
        DateTime::parse_from_rfc3339("2026-06-13T10:09:10.123+02:00").unwrap().with_timezone(&Utc);

    let entry = PostgresMemory::entry_from_values(
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        timestamp,
        Some(String::new()),
        Some(0.0),
    );

    assert!(entry.id.is_empty());
    assert!(entry.key.is_empty());
    assert!(entry.content.is_empty());
    assert_eq!(entry.category, MemoryCategory::Custom(String::new()));
    assert_eq!(entry.timestamp, "2026-06-13T08:09:10.123+00:00");
    assert_eq!(entry.session_id.as_deref(), Some(""));
    assert_eq!(entry.score, Some(0.0));
}

#[test]
fn row_to_entry_maps_real_postgres_row_with_optional_score() -> Result<()> {
    let Ok(database_url) = std::env::var("VIBEWINDOW_TEST_POSTGRES_URL") else {
        eprintln!(
            "skipping PostgreSQL row mapping coverage: VIBEWINDOW_TEST_POSTGRES_URL is not configured"
        );
        return Ok(());
    };
    let mut client = postgres::Client::connect(&database_url, postgres::NoTls)?;
    let row = client.query_one(
        "
        SELECT
            'row-id'::text,
            'row-key'::text,
            'row content'::text,
            'conversation'::text,
            '2026-06-13T08:09:10Z'::timestamptz,
            NULL::text,
            0.625::float8
        ",
        &[],
    )?;

    let entry = PostgresMemory::row_to_entry(&row)?;

    assert_eq!(entry.id, "row-id");
    assert_eq!(entry.key, "row-key");
    assert_eq!(entry.content, "row content");
    assert_eq!(entry.category, MemoryCategory::Conversation);
    assert_eq!(entry.timestamp, "2026-06-13T08:09:10+00:00");
    assert_eq!(entry.session_id, None);
    assert_eq!(entry.score, Some(0.625));
    Ok(())
}
