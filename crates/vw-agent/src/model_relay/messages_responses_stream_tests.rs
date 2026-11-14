use super::messages_responses_stream::{Decoder, response_body};
use axum::{
    body::{Body, to_bytes},
    http::StatusCode,
    response::Response,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

fn created() -> Value {
    json!({"type":"response.created","response":{"id":"resp_43"}})
}
fn terminal(output: Value) -> Value {
    json!({"type":"response.completed","response":{"id":"resp_43","status":"completed",
        "output":output,"usage":{"input_tokens":12,"output_tokens":4}}})
}
fn message(text: &str) -> Value {
    json!({"type":"message","id":"msg","role":"assistant","status":"completed",
        "content":[{"type":"output_text","text":text,"annotations":[]}]})
}
fn tool() -> Value {
    json!({"type":"function_call","id":"fc","call_id":"call_43","name":"lookup",
        "arguments":"{\"x\":1}","status":"completed"})
}
fn frame(decoder: &mut Decoder, value: Value) -> Vec<Value> {
    events(&decoder.frame(&value.to_string()).unwrap())
}
fn events(s: &str) -> Vec<Value> {
    s.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}
fn wire(value: Value) -> String {
    format!("data: {value}\n\n")
}

#[test]
fn cc_route_043_messages_responses_stream_expected_behavior() {
    let mut d = Decoder::new("client".into());
    let start = frame(&mut d, created());
    assert_eq!(start[0]["message"]["model"], "client");
    frame(
        &mut d,
        json!({"type":"response.output_item.added","output_index":0,
        "item":{"type":"message","id":"msg","role":"assistant","content":[]}}),
    );
    let delta = frame(
        &mut d,
        json!({"type":"response.output_text.delta","output_index":0,
        "content_index":0,"item_id":"msg","delta":"你"}),
    );
    assert_eq!(delta[0]["type"], "content_block_start");
    assert_eq!(delta[1]["delta"]["text"], "你");
    let end = frame(&mut d, terminal(json!([message("你好")])));
    assert_eq!(end[0]["delta"]["text"], "好");
    assert_eq!(end[1], json!({"type":"content_block_stop","index":0}));
    assert_eq!(end[2]["delta"]["stop_reason"], "end_turn");
    assert_eq!(end[2]["usage"], json!({"input_tokens":12,"output_tokens":4}));
    assert_eq!(end[3]["type"], "message_stop");
    assert!(d.done);
}

#[test]
fn cc_route_043_parallel_tool_and_reasoning_indices_remain_stable() {
    let mut d = Decoder::new("client".into());
    frame(&mut d, created());
    // Arrival order differs from output order, and output indices are not Messages indices.
    let mut added_tool = tool();
    added_tool["arguments"] = json!("");
    added_tool["status"] = json!("in_progress");
    let mut out = frame(
        &mut d,
        json!({"type":"response.output_item.added","output_index":1,"item":added_tool}),
    );
    assert_eq!(out[0]["content_block"]["id"], "call_43");
    let reasoning =
        json!({"type":"reasoning","id":"r","summary":[{"type":"summary_text","text":"think"}]});
    frame(
        &mut d,
        json!({"type":"response.output_item.added","output_index":0,
        "item":{"type":"reasoning","id":"r","summary":[]}}),
    );
    out.extend(frame(
        &mut d,
        json!({"type":"response.reasoning_summary_text.delta",
        "output_index":0,"summary_index":0,"delta":"think"}),
    ));
    out.extend(frame(
        &mut d,
        json!({"type":"response.function_call_arguments.delta",
        "output_index":1,"delta":"{\"x\":"}),
    ));
    out.extend(frame(
        &mut d,
        json!({"type":"response.function_call_arguments.delta",
        "output_index":1,"delta":"1}"}),
    ));
    // No arguments.done or output_item.done: terminal must close every open block.
    out.extend(frame(&mut d, terminal(json!([reasoning, tool()]))));
    let starts: Vec<_> = out.iter().filter(|e| e["type"] == "content_block_start").collect();
    assert_eq!(starts.len(), 2);
    assert_eq!(starts[0]["index"], 0);
    assert_eq!(starts[1]["index"], 1);
    assert_eq!(starts[1]["content_block"]["type"], "thinking");
    for delta in out.iter().filter(|e| e["delta"]["type"] == "input_json_delta") {
        assert_eq!(delta["index"], 0);
    }
    let stops: Vec<_> = out.iter().filter(|e| e["type"] == "content_block_stop").collect();
    assert_eq!(stops.len(), 2);
    assert!(stops.iter().any(|e| e["index"] == 0));
    assert!(stops.iter().any(|e| e["index"] == 1));
    assert_eq!(out[out.len() - 2]["delta"]["stop_reason"], "tool_use");
}

#[test]
fn cc_route_043_done_snapshots_do_not_duplicate_or_reopen_blocks() {
    let mut d = Decoder::new("m".into());
    frame(&mut d, created());
    frame(
        &mut d,
        json!({"type":"response.output_item.added","output_index":0,"item":{
        "type":"function_call","id":"fc","call_id":"call_43","name":"lookup","arguments":""}}),
    );
    let done = frame(
        &mut d,
        json!({"type":"response.function_call_arguments.done","output_index":0,"arguments":"{\"x\":1}"}),
    );
    assert_eq!(done.len(), 2);
    assert_eq!(done[1]["type"], "content_block_stop");
    assert!(
        frame(&mut d, json!({"type":"response.output_item.done","output_index":0,"item":tool()}))
            .is_empty()
    );
    let end = frame(&mut d, terminal(json!([tool()])));
    assert_eq!(end.len(), 2);
}

#[test]
fn cc_route_043_incomplete_and_failed_have_distinct_endings() {
    let mut value = terminal(json!([message("partial")]));
    value["type"] = json!("response.incomplete");
    value["response"]["status"] = json!("incomplete");
    value["response"]["output"][0]["status"] = json!("incomplete");
    value["response"]["incomplete_details"] = json!({"reason":"max_output_tokens"});
    let end = frame(&mut Decoder::new("m".into()), value.clone());
    assert_eq!(end[end.len() - 2]["delta"]["stop_reason"], "max_tokens");
    value["response"]["incomplete_details"]["reason"] = json!("content_filter");
    assert_eq!(Decoder::new("m".into()).frame(&value.to_string()), Err(StatusCode::BAD_GATEWAY));
    for ty in ["response.failed", "error", "response.cancelled"] {
        assert!(
            Decoder::new("m".into())
                .frame(&json!({"type":ty,"error":{"message":"private"}}).to_string())
                .is_err()
        );
    }
}

#[test]
fn cc_route_043_malformed_tools_and_conflicting_snapshots_fail_closed() {
    for arguments in ["{", "[]", "null"] {
        let mut bad = tool();
        bad["arguments"] = json!(arguments);
        assert!(Decoder::new("m".into()).frame(&terminal(json!([bad])).to_string()).is_err());
    }
    let mut d = Decoder::new("m".into());
    frame(&mut d, created());
    frame(
        &mut d,
        json!({"type":"response.output_item.added","output_index":0,
        "item":{"type":"message","id":"msg","content":[]}}),
    );
    frame(
        &mut d,
        json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"first"}),
    );
    assert!(d.frame(&terminal(json!([message("different")])).to_string()).is_err());
    assert!(d.frame(&json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"item_id":"wrong","delta":"x"}).to_string()).is_err());
}

#[test]
fn cc_route_043_fragmented_utf8_and_terminal_trailing_bytes() {
    let wire = wire(terminal(json!([message("你好")])));
    let mut d = Decoder::new("m".into());
    let mut out = String::new();
    for byte in wire.as_bytes() {
        out.push_str(&d.push(&[*byte]).unwrap());
    }
    assert!(d.done);
    assert!(out.contains("你好"));
    assert!(d.push(b"\xff\n").unwrap().is_empty());
}

#[tokio::test]
async fn cc_route_043_disconnect_and_failure_emit_error_without_success() {
    for wire in [
        wire(created()),
        wire(created())
            + &wire(
                json!({"type":"response.failed","response":{"error":{"message":"private upstream payload"}}}),
            ),
    ] {
        let response = Response::builder()
            .header("content-type", "text/event-stream")
            .body(Body::from(wire))
            .unwrap();
        let response = response_body(response, "m".into(), CancellationToken::new()).await.unwrap();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        let out = std::str::from_utf8(&bytes).unwrap();
        let events = events(out);
        assert_eq!(events.last().unwrap()["type"], "error");
        assert!(!events.iter().any(|e| e["type"] == "message_stop"));
        assert!(!out.contains("private upstream payload"));
    }
}

#[test]
fn cc_route_043_terminal_only_refusal_and_cache_accounting() {
    let mut end = terminal(json!([{"type":"message","role":"assistant","content":[
        {"type":"refusal","refusal":"declined"}]}]));
    end["response"]["usage"]["input_tokens_details"] =
        json!({"cached_tokens":5,"cache_write_tokens":2});
    let out = frame(&mut Decoder::new("client".into()), end);
    assert_eq!(out[2]["delta"]["text"], "declined");
    assert_eq!(
        out[out.len() - 2]["usage"],
        json!({"input_tokens":5,"output_tokens":4,
        "cache_read_input_tokens":5,"cache_creation_input_tokens":2})
    );
}

#[tokio::test]
async fn cc_route_043_successful_response_body_and_cancellation() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(wire(created()) + &wire(terminal(json!([message("answer")])))))
        .unwrap();
    let converted =
        response_body(response, "client".into(), CancellationToken::new()).await.unwrap();
    let bytes = to_bytes(converted.into_body(), 65536).await.unwrap();
    let out = events(std::str::from_utf8(&bytes).unwrap());
    assert_eq!(out.first().unwrap()["type"], "message_start");
    assert_eq!(out.last().unwrap()["type"], "message_stop");
    assert!(!out.iter().any(|event| event["type"] == "error"));

    let cancel = CancellationToken::new();
    cancel.cancel();
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from_stream(futures_util::stream::pending::<
            Result<axum::body::Bytes, std::io::Error>,
        >()))
        .unwrap();
    let converted = response_body(response, "client".into(), cancel).await.unwrap();
    assert!(to_bytes(converted.into_body(), 65536).await.unwrap().is_empty());
}
