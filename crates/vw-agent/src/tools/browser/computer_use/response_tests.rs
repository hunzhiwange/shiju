//! 验证计算机操作响应的可选字段与结构化数据保留。

use super::*;
use serde_json::json;

#[test]
fn response_deserializes_missing_optional_fields() {
    let response: ComputerUseResponse = serde_json::from_value(json!({})).unwrap();
    assert_eq!(response.success, None);
    assert_eq!(response.data, None);
    assert_eq!(response.error, None);
}

#[test]
fn response_preserves_error_and_data() {
    let response: ComputerUseResponse = serde_json::from_value(json!({
        "success": false,
        "data": {"path": "out.png"},
        "error": "denied"
    }))
    .unwrap();
    assert_eq!(response.success, Some(false));
    assert_eq!(response.data.unwrap()["path"], "out.png");
    assert_eq!(response.error.as_deref(), Some("denied"));
}
