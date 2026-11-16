//! 验证供应商上下文溢出识别与流式错误载荷解析。

use super::*;

#[test]
// 不同供应商没有统一错误码，因此用常见消息形态识别上下文溢出。
fn overflow_detection_matches_common_provider_messages() {
    assert!(is_overflow("context length exceeded for this model"));
    assert!(!is_overflow("temporary network error"));
}

#[test]
// 流式接口可能把错误包在 JSON 数据帧中，解析器需提取内部错误对象。
fn parses_stream_error_json_payload() {
    let parsed = parse_stream_error(r#"{"error":{"message":"too many tokens","type":"invalid_request_error"}}"#);
    assert!(parsed.is_some());
}
