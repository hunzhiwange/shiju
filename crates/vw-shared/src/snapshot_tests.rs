//! 文件快照差异类型的线格式测试。
//!
//! 验证状态使用小写标签，并保留可选差异状态的 JSON 往返语义。

#[test]
fn diff_status_uses_lowercase_wire_values() {
    let serialized = serde_json::to_string(&super::DiffStatus::Modified).unwrap();
    let parsed: super::DiffStatus = serde_json::from_str(r#""deleted""#).unwrap();

    assert_eq!(serialized, r#""modified""#);
    assert_eq!(parsed, super::DiffStatus::Deleted);
}

#[test]
fn file_diff_round_trips_optional_status() {
    let diff = super::FileDiff {
        file: "src/lib.rs".to_string(),
        before: "old".to_string(),
        after: "new".to_string(),
        additions: 2,
        deletions: 1,
        status: Some(super::DiffStatus::Added),
    };

    let json = serde_json::to_string(&diff).unwrap();
    let parsed: super::FileDiff = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.file, "src/lib.rs");
    assert_eq!(parsed.status, Some(super::DiffStatus::Added));
}
