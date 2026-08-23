//! 验证 PostgreSQL 项目存储对可选 JSON 数据的解析边界。

use super::parse_optional_json;

#[test]
fn parse_optional_json_preserves_absent_database_value() {
    let parsed = parse_optional_json::<serde_json::Value>(None).expect("parse absent value");

    assert_eq!(parsed, None);
}

#[test]
fn parse_optional_json_deserializes_present_value() {
    let parsed = parse_optional_json::<Vec<String>>(Some(r#"["alpha","beta"]"#.to_string()))
        .expect("parse string list");

    assert_eq!(parsed, Some(vec!["alpha".to_string(), "beta".to_string()]));
}

#[test]
fn parse_optional_json_keeps_json_null_distinct_from_sql_null() {
    let parsed = parse_optional_json::<serde_json::Value>(Some("null".to_string()))
        .expect("parse JSON null");

    assert_eq!(parsed, Some(serde_json::Value::Null));
}

#[test]
fn parse_optional_json_rejects_malformed_payload() {
    let error = parse_optional_json::<serde_json::Value>(Some("{".to_string()))
        .expect_err("malformed JSON should fail");

    assert!(error.to_string().contains("EOF"));
}

#[test]
fn parse_optional_json_rejects_unexpected_shape() {
    let error = parse_optional_json::<Vec<String>>(Some(r#"{"name":"plugin"}"#.to_string()))
        .expect_err("object should not deserialize as string list");

    assert!(error.to_string().contains("sequence"));
}
