//! 验证通用 DTO 的序列化形状及可选字段省略规则。

use super::common::{
    JsonMap, OperationAck, PaginatedResponse, PaginationRequest, StringMap, TimestampMs,
};
use serde_json::json;

// 共享类型 fixture 覆盖时间戳透明编码、可选字段省略及空映射默认值。
#[test]
fn common_types_serialize_with_expected_optional_fields() {
    assert_eq!(serde_json::to_value(TimestampMs(5)).expect("timestamp"), json!(5));
    assert_eq!(
        serde_json::to_value(OperationAck { ok: true, message: None }).expect("ack"),
        json!({"ok": true})
    );
    assert_eq!(
        serde_json::to_value(PaginationRequest { cursor: None, limit: Some(10) }).expect("page"),
        json!({"limit": 10})
    );

    let response = PaginatedResponse { items: vec![1], next_cursor: Some("n".into()) };
    assert_eq!(serde_json::to_value(response).expect("response")["next_cursor"], "n");
    assert!(StringMap::default().values.is_empty());
    assert!(JsonMap::default().values.is_empty());
}
