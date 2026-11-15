use super::messages_gemini_result::convert;
use axum::http::StatusCode;
use serde_json::{Value, json};

fn response() -> Value {
    json!({"responseId":"reply","modelVersion":"upstream",
        "candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"answer"}]},"finishReason":"STOP"}],
        "usageMetadata":{"promptTokenCount":20,"candidatesTokenCount":3,
            "thoughtsTokenCount":2,"totalTokenCount":25,"cachedContentTokenCount":8}})
}

#[test]
fn cc_route_048_messages_gemini_result_expected_behavior() {
    let mut body = response();
    body["candidates"][0]["content"]["parts"] = json!([
        {"text":"before"},
        {"functionCall":{"id":"call-a","name":"lookup","args":{"city":"杭州"}}},
        {"text":"after"},
        {"functionCall":{"name":"next","args":{}}}]);
    let result = convert(&body, "client").unwrap();
    assert_eq!(result["model"], "client");
    assert_eq!(result["id"], "reply");
    assert_eq!(result["stop_reason"], "tool_use");
    assert_eq!(result["stop_sequence"], Value::Null);
    assert_eq!(result["content"][0], json!({"type":"text","text":"before"}));
    assert_eq!(
        result["content"][1],
        json!({"type":"tool_use","id":"call-a","name":"lookup","input":{"city":"杭州"}})
    );
    assert_eq!(result["content"][2], json!({"type":"text","text":"after"}));
    assert_eq!(result["content"][3]["type"], "tool_use");
    assert!(result["content"][3]["id"].as_str().unwrap().starts_with("toolu_"));
    assert_eq!(
        result["usage"],
        json!({"input_tokens":12,"output_tokens":5,"cache_read_input_tokens":8})
    );
}

#[test]
fn cc_route_048_stops_usage_and_generated_ids() {
    for (reason, expected) in [("STOP", "end_turn"), ("MAX_TOKENS", "max_tokens")] {
        let mut body = response();
        body["candidates"][0]["finishReason"] = json!(reason);
        body.as_object_mut().unwrap().remove("responseId");
        body["usageMetadata"].as_object_mut().unwrap().remove("totalTokenCount");
        let result = convert(&body, "m").unwrap();
        assert_eq!(result["stop_reason"], expected);
        assert_eq!(result["usage"]["output_tokens"], 5);
        assert!(result["id"].as_str().unwrap().starts_with("msg_"));
    }
    let mut body = response();
    body.as_object_mut().unwrap().remove("usageMetadata");
    assert_eq!(convert(&body, "m").unwrap()["usage"], json!({"input_tokens":0,"output_tokens":0}));
    body["candidates"][0]["content"]["parts"] = json!([
        {"functionCall":{"id":"","name":"lookup"}}]);
    let result = convert(&body, "m").unwrap();
    assert!(result["content"][0]["id"].as_str().unwrap().starts_with("toolu_"));
    assert_eq!(result["content"][0]["input"], json!({}));
}

#[test]
fn cc_route_048_safety_is_refusal_and_never_executes_tools() {
    for reason in ["SAFETY", "RECITATION", "SPII", "BLOCKLIST", "PROHIBITED_CONTENT"] {
        let mut body = response();
        body["candidates"][0]["finishReason"] = json!(reason);
        body["candidates"][0]["content"]["parts"] = json!([
            {"functionCall":{"id":"danger","name":"run","args":{}}}]);
        let result = convert(&body, "m").unwrap();
        assert_eq!(result["stop_reason"], "refusal");
        assert_eq!(result["content"], json!([]));
    }
    let body = json!({"promptFeedback":{"blockReason":"SAFETY"}});
    assert!(
        super::protocol_bridge::response(vw_shared::provider::ModelProtocol::Gemini, &body)
            .is_err()
    );
    let result = convert(&body, "m").unwrap();
    assert_eq!(result["stop_reason"], "refusal");
    assert_eq!(result["content"], json!([]));
    let mut body = response();
    body["promptFeedback"] = json!({"blockReason":"SAFETY"});
    assert_eq!(convert(&body, "m").unwrap()["content"], json!([]));
}

#[test]
fn cc_route_048_malformed_and_unsupported_results_fail_closed() {
    for patch in [
        json!({"candidates":[]}),
        json!({"candidates":[{},{}]}),
        json!({"candidates":[null]}),
        json!({"candidates":[{"finishReason":"UNKNOWN","content":{"parts":[]}}]}),
        json!({"candidates":[{"finishReason":"STOP"}]}),
        json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"inlineData":{}}]}}]}),
        json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"hidden","thought":true}]}}]}),
        json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"signed","thoughtSignature":"opaque"}]}}]}),
        json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"functionCall":{"id":42,"name":"f","args":{}}}]}}]}),
        json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"functionCall":{"id":"a","name":"f","args":"{}"}}]}}]}),
        json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"functionCall":{"id":"a","name":"f"}},{"functionCall":{"id":"a","name":"g"}}]}}]}),
        json!({"usageMetadata":{"promptTokenCount":1,"cachedContentTokenCount":2}}),
        json!({"usageMetadata":{"promptTokenCount":2,"totalTokenCount":1}}),
        json!({"usageMetadata":{"promptTokenCount":-1}}),
        json!({"usageMetadata":null}),
        json!({"responseId":42}),
        json!({"promptFeedback":{"blockReason":false}}),
        json!({"error":{"message":"private upstream failure"}}),
    ] {
        let mut body = response();
        body.as_object_mut().unwrap().extend(patch.as_object().unwrap().clone());
        assert_eq!(convert(&body, "m"), Err(StatusCode::BAD_GATEWAY));
    }
    assert_eq!(convert(&json!(42), "m"), Err(StatusCode::BAD_GATEWAY));
}

#[tokio::test]
async fn cc_route_048_response_preserves_cancellation_and_size_boundary() {
    let cancel = tokio_util::sync::CancellationToken::new();
    cancel.cancel();
    let upstream = axum::response::Response::new(axum::body::Body::from(response().to_string()));
    assert_eq!(
        super::messages_gemini_result::response(upstream, "m", cancel, 4096).await.unwrap_err(),
        StatusCode::REQUEST_TIMEOUT
    );
    let upstream = axum::response::Response::new(axum::body::Body::from(response().to_string()));
    assert_eq!(
        super::messages_gemini_result::response(
            upstream,
            "m",
            tokio_util::sync::CancellationToken::new(),
            1
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_GATEWAY
    );
    let upstream = axum::response::Response::new(axum::body::Body::from(response().to_string()));
    let result = super::messages_gemini_result::response(
        upstream,
        "client",
        tokio_util::sync::CancellationToken::new(),
        4096,
    )
    .await
    .unwrap();
    let bytes = axum::body::to_bytes(result.into_body(), 4096).await.unwrap();
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["model"], "client");
    assert_eq!(result["stop_reason"], "end_turn");
}
