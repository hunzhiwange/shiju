//! 内存领域基础数据结构的序列化契约测试。
//!
//! 这些测试锁定 `MemoryCategory` 与 `MemoryEntry` 面向存储层和外部接口的 JSON
//! 表达，确保新增类别或可选字段时不会破坏既有持久化数据。

use super::*;

#[test]
fn memory_category_display_outputs_expected_values() {
    assert_eq!(MemoryCategory::Core.to_string(), "core");
    assert_eq!(MemoryCategory::Daily.to_string(), "daily");
    assert_eq!(MemoryCategory::Conversation.to_string(), "conversation");
    assert_eq!(MemoryCategory::Custom("project_notes".into()).to_string(), "project_notes");
}

#[test]
fn memory_category_serde_roundtrip_uses_plain_strings() {
    let core = serde_json::to_string(&MemoryCategory::Core).unwrap();
    let daily = serde_json::to_string(&MemoryCategory::Daily).unwrap();
    let conversation = serde_json::to_string(&MemoryCategory::Conversation).unwrap();
    let custom = serde_json::to_string(&MemoryCategory::Custom("travel".into())).unwrap();

    assert_eq!(core, "\"core\"");
    assert_eq!(daily, "\"daily\"");
    assert_eq!(conversation, "\"conversation\"");
    assert_eq!(custom, "\"travel\"");

    // 类别在存储层使用普通字符串，便于不同后端共享同一份迁移/导出格式。
    assert_eq!(serde_json::from_str::<MemoryCategory>("\"core\"").unwrap(), MemoryCategory::Core);
    assert_eq!(serde_json::from_str::<MemoryCategory>("\"daily\"").unwrap(), MemoryCategory::Daily);
    assert_eq!(
        serde_json::from_str::<MemoryCategory>("\"conversation\"").unwrap(),
        MemoryCategory::Conversation
    );
    assert_eq!(
        serde_json::from_str::<MemoryCategory>("\"travel\"").unwrap(),
        MemoryCategory::Custom("travel".into())
    );
}

#[test]
fn memory_category_deserialization_preserves_custom_strings_verbatim() {
    for raw in ["", " Core ", "CORE", "conversation "] {
        let json = serde_json::to_string(raw).unwrap();
        let category = serde_json::from_str::<MemoryCategory>(&json).unwrap();

        assert_eq!(category, MemoryCategory::Custom(raw.to_owned()));
    }
}

#[test]
fn memory_category_deserialization_rejects_non_string_values() {
    for json in ["null", "1", "true", "[]", "{}"] {
        assert!(serde_json::from_str::<MemoryCategory>(json).is_err());
    }
}

#[test]
fn memory_entry_roundtrip_preserves_optional_fields() {
    let entry = MemoryEntry {
        id: "id-1".into(),
        key: "favorite_language".into(),
        content: "Rust".into(),
        category: MemoryCategory::Core,
        timestamp: "2026-02-16T00:00:00Z".into(),
        session_id: Some("session-abc".into()),
        score: Some(0.98),
    };

    let json = serde_json::to_string(&entry).unwrap();
    let parsed: MemoryEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, "id-1");
    assert_eq!(parsed.key, "favorite_language");
    assert_eq!(parsed.content, "Rust");
    assert_eq!(parsed.category, MemoryCategory::Core);
    assert_eq!(parsed.session_id.as_deref(), Some("session-abc"));
    assert_eq!(parsed.score, Some(0.98));
}

#[test]
fn memory_entry_serializes_custom_category_as_plain_string() {
    let entry = MemoryEntry {
        id: "id-2".into(),
        key: "trip".into(),
        content: "booked a flight".into(),
        category: MemoryCategory::Custom("travel".into()),
        timestamp: "2026-03-04T00:00:00Z".into(),
        session_id: None,
        score: None,
    };

    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json.get("category").unwrap(), "travel");
}

#[test]
fn memory_entry_deserialization_defaults_omitted_optional_fields() {
    let json = serde_json::json!({
        "id": "id-optional",
        "key": "optional_fields",
        "content": "no session or score",
        "category": "conversation",
        "timestamp": "2026-04-01T00:00:00Z"
    });

    let entry = serde_json::from_value::<MemoryEntry>(json).unwrap();

    assert_eq!(entry.category, MemoryCategory::Conversation);
    assert_eq!(entry.session_id, None);
    assert_eq!(entry.score, None);
}

#[test]
fn memory_entry_deserialization_rejects_missing_required_fields() {
    let json = serde_json::json!({
        "id": "id-incomplete",
        "key": "missing_content",
        "category": "daily",
        "timestamp": "2026-04-02T00:00:00Z",
        "session_id": null,
        "score": null
    });

    assert!(serde_json::from_value::<MemoryEntry>(json).is_err());
}

#[test]
fn memory_entry_debug_includes_public_fields_and_omits_session_id() {
    let entry = MemoryEntry {
        id: "id-3".into(),
        key: "debug_key".into(),
        content: "debug content".into(),
        category: MemoryCategory::Daily,
        timestamp: "2026-04-05T00:00:00Z".into(),
        session_id: Some("session-secret".into()),
        score: None,
    };

    let debug = format!("{entry:?}");

    assert!(debug.contains("MemoryEntry"));
    assert!(debug.contains("id-3"));
    assert!(debug.contains("debug_key"));
    assert!(debug.contains("debug content"));
    assert!(debug.contains("Daily"));
    assert!(debug.contains("2026-04-05T00:00:00Z"));
    assert!(debug.contains("score"));
    assert!(!debug.contains("session_id"));
    assert!(!debug.contains("session-secret"));
}
