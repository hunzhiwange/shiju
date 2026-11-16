use crate::routes::PtyConnectQuery;
#[test]
fn pty_connect_query_deserializes_directory_and_cursor() {
    let query: PtyConnectQuery = serde_json::from_value(serde_json::json!({
        "directory": "/workspace/project",
        "cursor": 42
    }))
    .expect("valid query");

    assert_eq!(query.directory.as_deref(), Some("/workspace/project"));
    assert_eq!(query.cursor, Some(42));
}

#[test]
fn pty_connect_query_allows_missing_optional_fields() {
    let query: PtyConnectQuery =
        serde_json::from_value(serde_json::json!({})).expect("valid query");

    assert!(query.directory.is_none());
    assert!(query.cursor.is_none());
}

#[test]
fn pty_connect_query_rejects_non_integer_cursor() {
    let result = serde_json::from_value::<PtyConnectQuery>(serde_json::json!({"cursor": "latest"}));

    assert!(result.is_err());
}

#[test]
fn pty_connect_query_preserves_cursor_boundaries_and_blank_directory() {
    let minimum: PtyConnectQuery = serde_json::from_value(serde_json::json!({
        "directory": "   ",
        "cursor": i64::MIN
    }))
    .expect("minimum cursor should deserialize");
    let maximum: PtyConnectQuery = serde_json::from_value(serde_json::json!({"cursor": i64::MAX}))
        .expect("maximum cursor should deserialize");

    assert_eq!(minimum.directory.as_deref(), Some("   "));
    assert_eq!(minimum.cursor, Some(i64::MIN));
    assert_eq!(maximum.cursor, Some(i64::MAX));
}
