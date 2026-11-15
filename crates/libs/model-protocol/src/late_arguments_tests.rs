use super::late_arguments::Repair;
use serde_json::{Value, json};

fn frame(value: Value) -> String {
    format!("event: {}\ndata: {}\n\n", value["type"].as_str().unwrap(), value)
}

#[test]
fn cc_route_062_late_arguments_expected_behavior() {
    let mut repair = Repair::default();
    assert!(
        repair
            .push(frame(
                json!({"type":"response.function_call_arguments.done","item_id":"a","arguments":""})
            ))
            .unwrap()
            .is_empty()
    );
    assert!(repair.push(frame(json!({"type":"response.output_item.done","item":{"type":"function_call","id":"a","arguments":""}}))).unwrap().is_empty());
    let delta =
        frame(json!({"type":"response.function_call_arguments.delta","item_id":"a","delta":"{}"}));
    assert_eq!(repair.push(delta.clone()).unwrap(), delta);
    let output =
        repair.push(frame(json!({"type":"response.completed","response":{"output":[]}}))).unwrap();
    assert_eq!(output.matches("\"arguments\":\"{}\"").count(), 2);
    assert!(!output.contains("function_call_arguments.delta"));
    assert!(
        output.find("function_call_arguments.done").unwrap()
            < output.find("output_item.done").unwrap()
    );
}

#[test]
fn cc_route_062_completed_fallback_and_normal_events() {
    let mut repair = Repair::default();
    repair.push(frame(json!({"type":"response.output_item.done","item":{"type":"function_call","id":"a","arguments":""}}))).unwrap();
    let output = repair.push(frame(json!({"type":"response.completed","response":{"output":[{"id":"a","arguments":"{}"}]}}))).unwrap();
    assert_eq!(output.matches("\"arguments\":\"{}\"").count(), 2);
    let normal = frame(
        json!({"type":"response.function_call_arguments.done","item_id":"b","arguments":"{}"}),
    );
    assert_eq!(repair.push(normal.clone()).unwrap(), normal);
}

#[test]
fn cc_route_062_failed_terminal_discards_held_completion() {
    let mut repair = Repair::default();
    repair.push(frame(json!({"type":"response.output_item.done","item":{"type":"function_call","id":"a","arguments":""}}))).unwrap();
    let failed = frame(json!({"type":"response.failed","response":{"status":"failed"}}));
    assert_eq!(repair.push(failed.clone()).unwrap(), failed);
}

#[test]
fn cc_route_062_retained_arguments_are_bounded() {
    let mut repair = Repair::default();
    assert!(repair.push(frame(json!({"type":"response.function_call_arguments.delta","item_id":"a","delta":"x".repeat(4 * 1024 * 1024)}))).is_err());
}
