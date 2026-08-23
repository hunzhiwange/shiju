use super::*;
use prost::Message as _;

#[test]
fn pb_frame_header_value_returns_match_or_empty_string() {
    let frame = PbFrame {
        seq_id: 1,
        log_id: 2,
        service: 3,
        method: 4,
        headers: vec![PbHeader { key: "k".to_string(), value: "v".to_string() }],
        payload: None,
    };

    assert_eq!(frame.header_value("k"), "v");
    assert_eq!(frame.header_value("missing"), "");
}

#[test]
fn pb_frame_round_trips_through_protobuf_encoding() {
    let frame = PbFrame {
        seq_id: 7,
        log_id: 8,
        service: 9,
        method: 1,
        headers: vec![PbHeader { key: "type".to_string(), value: "event".to_string() }],
        payload: Some(b"payload".to_vec()),
    };

    let decoded = PbFrame::decode(frame.encode_to_vec().as_slice()).expect("decode frame");

    assert_eq!(decoded.seq_id, 7);
    assert_eq!(decoded.log_id, 8);
    assert_eq!(decoded.service, 9);
    assert_eq!(decoded.method, 1);
    assert_eq!(decoded.header_value("type"), "event");
    assert_eq!(decoded.payload.as_deref(), Some(&b"payload"[..]));
}

#[test]
fn ws_endpoint_response_deserializes_optional_fields() {
    let response: WsEndpointResp = serde_json::from_value(serde_json::json!({
        "code": 0,
        "data": { "URL": "wss://example", "ClientConfig": { "PingInterval": 1000 } }
    }))
    .expect("endpoint response");

    assert_eq!(response.code, 0);
    assert_eq!(
        response.data.expect("data").client_config.expect("config").ping_interval,
        Some(1000)
    );
}

#[test]
fn ws_endpoint_response_allows_missing_optional_data_and_message() {
    let response: WsEndpointResp =
        serde_json::from_value(serde_json::json!({"code": 1})).expect("endpoint response");

    assert_eq!(response.code, 1);
    assert!(response.msg.is_none());
    assert!(response.data.is_none());
}

#[test]
fn lark_event_and_message_payload_deserialize_defaults() {
    let event: LarkEvent = serde_json::from_value(serde_json::json!({
        "header": { "event_type": "im.message.receive_v1", "event_id": "evt-1" },
        "event": {
            "sender": {
                "sender_type": "user",
                "sender_id": {}
            },
            "message": {
                "message_id": "om_1",
                "chat_id": "oc_1",
                "chat_type": "p2p",
                "message_type": "text"
            }
        }
    }))
    .expect("event");

    assert_eq!(event.header.event_type, "im.message.receive_v1");
    let payload: MsgReceivePayload = serde_json::from_value(event.event).expect("payload");
    assert_eq!(payload.sender.sender_type, "user");
    assert_eq!(payload.sender.sender_id.open_id, None);
    assert_eq!(payload.message.content, "");
    assert!(payload.message.mentions.is_empty());
}

#[test]
fn ws_client_config_defaults_to_no_ping_interval() {
    let config = WsClientConfig::default();

    assert_eq!(config.ping_interval, None);
}
// 本文件验证 Lark WebSocket 协议帧、响应载荷与客户端默认配置。
