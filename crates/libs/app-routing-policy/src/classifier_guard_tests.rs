use super::classifier_guard::classifier_retry_loop;
use serde_json::json;

fn failed_turn() -> Value {
    json!({"role":"user", "content":[{
        "type":"tool_result", "tool_use_id":"call_1", "is_error":true,
        "content":"claude-opus-5[1m] is temporarily unavailable, so auto mode cannot determine the safety of Bash right now."
    }]})
}

fn request(turns: usize) -> Value {
    let mut messages = vec![json!({"role":"user", "content":"Check the connection"})];
    for _ in 0..turns {
        messages.push(json!({"role":"assistant", "content":[{"type":"tool_use", "id":"call_1", "name":"Bash", "input":{}}]}));
        messages.push(failed_turn());
    }
    json!({"tools":[{"name":"Bash", "input_schema":{"type":"object"}}],"messages":messages})
}

#[test]
fn cc_route_066_classifier_guard_expected_behavior() {
    assert!(!classifier_retry_loop(&request(2)));
    assert!(classifier_retry_loop(&request(3)));
    assert!(classifier_retry_loop(&request(50)));
}

#[test]
fn cc_route_066_new_user_message_or_success_resets_the_streak() {
    for content in [
        json!("I fixed the mapping. Try again."),
        json!([{"type":"text","text":"Try again"}]),
        json!([{"type":"tool_result","is_error":false,"content":"ok"}]),
    ] {
        let mut body = request(3);
        body["messages"]
            .as_array_mut()
            .unwrap()
            .push(json!({"role":"user","content":content}));
        assert!(!classifier_retry_loop(&body));
    }
}

#[test]
fn cc_route_066_classifier_calls_and_other_tool_errors_are_not_blocked() {
    let mut body = request(3);
    body["tools"] = json!([]);
    assert!(!classifier_retry_loop(&body));
    let mut body = request(3);
    body["messages"][6]["content"][0]["content"] = json!("Permission denied");
    assert!(!classifier_retry_loop(&body));
    body["messages"][6]["content"][0] = failed_turn()["content"][0].clone();
    body["messages"][6]["content"][0]["is_error"] = json!(false);
    assert!(!classifier_retry_loop(&body));
}

#[test]
fn cc_route_066_parallel_tool_failures_count_as_one_turn() {
    let mut body = request(1);
    body["messages"][2]["content"] = json!([
        failed_turn()["content"][0],
        failed_turn()["content"][0],
        failed_turn()["content"][0]
    ]);
    assert!(!classifier_retry_loop(&body));
    body["messages"]
        .as_array_mut()
        .unwrap()
        .extend([failed_turn(), failed_turn()]);
    assert!(!classifier_retry_loop(&body));
}

#[test]
fn cc_route_066_recognizes_text_blocks_but_not_successful_or_mixed_results() {
    let mut body = request(3);
    body["messages"][6]["content"][0]["content"] = json!([{"type":"text","text":"The auto mode classifier is temporarily unavailable, so Bash cannot be used right now."}]);
    assert!(classifier_retry_loop(&body));
    body["messages"][6]["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"tool_result","content":"ok"}));
    assert!(!classifier_retry_loop(&body));
}
