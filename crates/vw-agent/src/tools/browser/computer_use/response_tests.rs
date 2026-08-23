//! 验证计算机操作响应的可选字段与结构化数据保留。
//! 缺省响应字段保持 None，结构化错误和任意数据不得被默认成功值覆盖。

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
