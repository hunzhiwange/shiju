use super::responses_anthropic_request::convert;
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn cc_route_057_responses_anthropic_request_expected_behavior() {
    let (body, _) = convert(
        json!({"model":"fixture", "instructions":"First",
        "max_output_tokens":120, "input":[
            {"role":"developer","content":[{"type":"input_text","text":"Second"}]},
            {"role":"assistant","content":"previous answer  "}
        ]}),
        4096,
    )
    .unwrap();
    assert_eq!(body["system"], "First\n\nSecond");
    assert_eq!(body["max_tokens"], 120);
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][1]["content"][0]["text"], "previous answer");
}

#[test]
fn cc_route_057_media_order_and_tool_identity() {
    let (body, _) = convert(json!({"model":"fixture", "tools":[
        {"type":"namespace","name":"repo","tools":[
            {"type":"custom","name":"read","description":"read input"}]}],
        "input":[
            {"role":"user","content":[
                {"type":"input_text","text":"inspect"},
                {"type":"input_image","image_url":"data:image/png;base64,YQ=="},
                {"type":"input_file","filename":"report.pdf","file_url":"https://example.com/report.pdf"}]},
            {"type":"custom_tool_call","call_id":"a","name":"read","namespace":"repo","input":"raw"},
            {"type":"custom_tool_call_output","call_id":"a","output":[
                {"type":"input_text","text":"result"},
                {"type":"input_image","image_url":"https://example.com/image.png"}]}]}), 4096).unwrap();
    assert_eq!(body["messages"][0]["content"][1]["type"], "image");
    assert_eq!(body["messages"][0]["content"][2]["title"], "report.pdf");
    assert_eq!(body["messages"][1]["content"][0]["name"], body["tools"][0]["name"]);
    assert_eq!(body["messages"][1]["content"][0]["input"], json!({"input":"raw"}));
    assert_eq!(body["messages"][2]["content"][0]["tool_use_id"], "a");
    assert_eq!(body["messages"][2]["content"][0]["content"][1]["type"], "image");
}

#[test]
fn cc_route_057_drops_entire_partial_parallel_round() {
    let (body, _) = convert(
        json!({"model":"fixture","input":[
            {"type":"function_call","call_id":"a","name":"read","arguments":"{}"},
            {"type":"function_call","call_id":"b","name":"read","arguments":"{}"},
            {"type":"function_call_output","call_id":"a","output":"partial"},
            {"role":"user","content":"continue"},
            {"type":"function_call_output","call_id":"orphan","output":"stale"}
        ]}),
        4096,
    )
    .unwrap();
    assert_eq!(
        body["messages"],
        json!([{"role":"user","content":[{"type":"text","text":"continue"}]}])
    );
    assert!(
        convert(
            json!({"model":"fixture","input":[
        {"type":"function_call_output","call_id":"orphan","output":"stale"}]}),
            4096
        )
        .is_err()
    );
}

#[test]
fn cc_route_057_rejects_unsupported_media_and_preserves_session_policy() {
    for part in [
        json!({"type":"input_file","file_id":"file_secret"}),
        json!({"type":"input_image","image_url":"file:///tmp/private"}),
        json!({"type":"input_file","file_data":"data:application/pdf;base64,%%%"}),
    ] {
        assert!(
            convert(json!({"model":"fixture","input":[{"role":"user","content":[part]}]}), 4096)
                .is_err()
        );
    }
    let body = json!({"model":"fixture","input":[{"role":"assistant","content":"history"}]});
    let (chat, _) = super::responses::to_chat(&body, false).unwrap();
    assert_eq!(chat["messages"][0]["role"], "assistant");
    assert_eq!(
        super::responses_compaction::Request::from_responses_anthropic(
            json!({"model":"fixture","stream":true,"input":"hello"})
        )
        .err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
}
