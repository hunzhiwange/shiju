use super::*;
fn frame(v: Value) -> String {
    format!("data: {v}\n\n")
}
fn wire(p: P) -> String {
    match p{
        P::Messages=>[
            json!({"type":"message_start","message":{"id":"m","type":"message","role":"assistant","model":"api","usage":{"input_tokens":2,"output_tokens":0},"content":[]}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"中文"}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}),
            json!({"type":"message_stop"})].into_iter().map(frame).collect(),
        P::Responses=>[
            json!({"type":"response.created","response":{"model":"api","status":"in_progress"}}),
            json!({"type":"response.output_text.delta","delta":"中文"}),
            json!({"type":"response.completed","response":{"id":"r","model":"api","status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"中文"}]}],"usage":{"input_tokens":2,"output_tokens":3}}})].into_iter().map(frame).collect(),
        P::Gemini=>[
            json!({"modelVersion":"api","candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"中文"}]}}]}),
            json!({"modelVersion":"api","candidates":[{"index":0,"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":2,"candidatesTokenCount":3}})].into_iter().map(frame).collect(),
        P::Chat=>panic!("native fixture")
    }
}
#[test]
fn model_protocol_023_native_streams_emit_unicode_before_verified_terminal() {
    for p in [P::Messages, P::Responses, P::Gemini] {
        let mut d = Decoder::new(p, "bound".into());
        let mut out = String::new();
        for b in wire(p).bytes() {
            out.push_str(&d.push(&[b]).unwrap());
        }
        assert!(out.contains("中文"));
        assert!(!out.contains("[DONE]"));
        let tail = d.finish().unwrap();
        assert!(tail.contains("[DONE]"));
        assert!(tail.contains("\"total_tokens\":5"));
    }
}
#[test]
fn model_protocol_023_native_truncated_streams_never_complete() {
    for p in [P::Messages, P::Responses, P::Gemini] {
        let data = wire(p);
        let last = data.rfind("data:").unwrap();
        let mut d = Decoder::new(p, "bound".into());
        d.push(&data.as_bytes()[..last]).unwrap();
        assert!(d.finish().is_err());
    }
}
#[test]
fn model_protocol_023_responses_final_text_must_match_forwarded_deltas() {
    let mut d = Decoder::new(P::Responses, "bound".into());
    d.push(wire(P::Responses).replace("\"text\":\"中文\"", "\"text\":\"different\"").as_bytes())
        .unwrap();
    assert!(d.finish().is_err());
}
#[test]
fn model_protocol_023_gemini_opaque_signature_is_never_dropped() {
    let mut d = Decoder::new(P::Gemini, "bound".into());
    let mut value = json!({"modelVersion":"api","candidates":[{"content":{"role":"model","parts":[{"text":"中文","thoughtSignature":"opaque"}]},"finishReason":"STOP"}]});
    assert!(d.push(frame(value.clone()).as_bytes()).is_err());
    value["candidates"][0]["content"]["parts"][0]["thought"] = true.into();
    assert!(Decoder::new(P::Gemini, "bound".into()).push(frame(value).as_bytes()).is_err());
}

#[tokio::test]
async fn model_protocol_023_cancelled_native_response_has_no_successful_terminal() {
    let cancel = CancellationToken::new();
    let native = reqwest::Response::from(
        axum::http::Response::builder()
            .header("content-type", "text/event-stream")
            .body(wire(P::Messages))
            .unwrap(),
    );
    let adapted =
        response(native, P::Messages, "bound".into(), true, cancel.clone()).await.unwrap();
    cancel.cancel();
    assert!(adapted.bytes().await.is_err());
}
#[tokio::test]
async fn model_protocol_023_nonstream_native_response_is_bounded_and_cancellable() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let native = reqwest::Response::from(axum::http::Response::new("{}"));
    assert_eq!(
        response(native, P::Messages, "bound".into(), false, cancel).await.unwrap_err(),
        StatusCode::REQUEST_TIMEOUT
    );
    let native = reqwest::Response::from(axum::http::Response::new(" ".repeat(LIMIT + 1)));
    assert_eq!(
        response(native, P::Messages, "bound".into(), false, CancellationToken::new())
            .await
            .unwrap_err(),
        BAD
    );
}
#[test]
fn model_protocol_023_native_stream_rejects_oversized_frames() {
    let mut decoder = Decoder::new(P::Messages, "bound".into());
    assert!(decoder.push(&vec![b'x'; LIMIT + 1]).is_err());
}
