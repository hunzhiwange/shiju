use super::Decoder;
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn chat_stream_accepts_empty_reasoning_and_completes_tools() {
    for empty in [serde_json::Value::Null, json!("")] {
        let mut decoder = Decoder { model: "selected".into(), ..Default::default() };
        decoder
            .frame(
                &json!({"id":"reply","choices":[{"index":0,"delta":{
                "role":"assistant","reasoning_content":empty,"tool_calls":null}}]})
                .to_string(),
            )
            .unwrap();
        decoder
            .frame(
                &json!({"id":"reply","choices":[{"index":0,"delta":{
            "tool_calls":[{"index":0,"id":"read-1","type":"function",
                "function":{"name":"Read","arguments":"{}"}}]},
            "finish_reason":"tool_calls"}],"usage":{"prompt_tokens":1,"completion_tokens":1}})
                .to_string(),
            )
            .unwrap();
        let result = decoder.frame("[DONE]").unwrap();
        assert!(result.contains("read-1"));
        assert!(result.contains("message_stop"));
        assert!(decoder.done);
    }
}

#[test]
fn chat_stream_rejects_nonempty_reasoning_in_nonthinking_mode() {
    let mut decoder = Decoder::default();
    assert_eq!(
        decoder.frame(
            &json!({"id":"reply","choices":[{"index":0,
            "delta":{"reasoning_content":"requires replay"}}]})
            .to_string()
        ),
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
}
