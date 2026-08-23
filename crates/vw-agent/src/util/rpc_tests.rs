//! 验证 RPC 请求、结果与事件在线路格式中的编码解码契约。
//! 往返断言同时固定消息变体和请求标识，避免序列化时丢失关联信息。

use super::rpc::{Wire, decode, encode, event, request, result};
use serde_json::json;

#[test]
fn encodes_and_decodes_wire_messages() {
    let wire = request("sum", json!({"a": 1}), 9);
    let decoded = decode(&encode(&wire).expect("encode")).expect("decode");
    assert!(matches!(decoded, Wire::Request { id: 9, .. }));

    assert!(matches!(result(json!(42), 9), Wire::Result { id: 9, .. }));
    assert!(matches!(event("ready", json!({})), Wire::Event { .. }));
}
