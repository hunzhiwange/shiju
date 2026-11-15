use super::reasoning_bridge::*;
use serde_json::json;

#[test]
fn cc_route_060_reasoning_bridge_expected_behavior() {
    let item = json!({"type":"reasoning","id":"r","summary":[{"type":"summary_text","text":"推理"}],"encrypted_content":"opaque","status":"completed"});
    let block = anthropic_block_from_openai_reasoning_item(&item).unwrap();
    let restored = openai_reasoning_item_from_anthropic_block(&block).unwrap();
    assert_eq!(restored["summary"], item["summary"]);
    assert_eq!(restored["encrypted_content"], "opaque");
    assert!(restored.get("status").is_none());
}

#[test]
fn cc_route_060_anthropic_signature_and_redacted_roundtrip() {
    for block in [
        json!({"type":"thinking","thinking":"分析","signature":"signed"}),
        json!({"type":"redacted_thinking","data":"opaque"}),
    ] {
        let item = responses_reasoning_item_from_anthropic_block("r", &block).unwrap();
        assert_eq!(
            decode_anthropic_thinking_block(item["encrypted_content"].as_str().unwrap()),
            Some(block)
        );
    }
    assert!(decode_anthropic_thinking_block("arbitrary base64").is_none());
    assert!(decode_openai_reasoning_item("arbitrary base64").is_none());
}

#[test]
fn cc_route_060_resident_request_replays_only_reasoning_fields() {
    let item = json!({"type":"reasoning","summary":[],"encrypted_content":"cipher","authorization":"forged"});
    let block = anthropic_block_from_openai_reasoning_item(&item).unwrap();
    let request = json!({"model":"fixture","max_tokens":100,"messages":[{"role":"assistant","content":[block]},{"role":"user","content":"continue"}]});
    let output = super::messages_responses_request::to_responses(&request).unwrap();
    assert_eq!(output["input"][0]["type"], "reasoning");
    assert!(output["input"][0].get("authorization").is_none());
}
