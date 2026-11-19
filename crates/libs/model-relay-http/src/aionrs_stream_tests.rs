use super::{Lines, normalize, responses_chat_stream};
use serde_json::{Value, json};

#[test]
fn aionrs_split_reasoning_preserves_the_answer_and_tool_delta() {
    let mut lines = Lines::default();
    let mut state = responses_chat_stream::State::default();
    let tool_calls =
        json!([{"index":0,"id":"call_1","type":"function","function":{"name":"Read"}}]);
    let inputs = [
        json!({"content":"<thi"}),
        json!({"content":"nk>internal</th"}),
        json!({"content":"ink>MCP_NOT_AVAILABLE"}),
        json!({"tool_calls":tool_calls}),
    ];
    let mut answer = String::new();
    let mut reasoning = String::new();
    let mut actual_calls = Value::Null;
    for delta in inputs {
        let frame = format!(
            "data: {}\n\n",
            json!({
                "choices":[{"index":0,"delta":delta,"finish_reason":null}]
            })
        );
        let (first, second) = frame.as_bytes().split_at(10);
        assert!(normalize(&mut lines, &mut state, first).unwrap().is_empty());
        let output = normalize(&mut lines, &mut state, second).unwrap();
        let line = output.trim().strip_prefix("data: ").unwrap();
        let event: Value = serde_json::from_str(line).unwrap();
        let delta = &event["choices"][0]["delta"];
        answer.push_str(delta["content"].as_str().unwrap_or_default());
        reasoning.push_str(delta["reasoning_content"].as_str().unwrap_or_default());
        if let Some(calls) = delta.get("tool_calls") {
            actual_calls = calls.clone();
        }
    }
    assert_eq!(answer, "MCP_NOT_AVAILABLE");
    assert_eq!(reasoning, "internal");
    assert_eq!(actual_calls, tool_calls);
}
