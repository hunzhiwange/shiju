use serde_json::json;

#[test]
fn session_tool_read_bounds_output() {
    assert_eq!(super::MAX_SESSION_READ_CHARS, 64 * 1024);
}

#[test]
fn session_tool_read_bounds_serialized_output() {
    let output = super::bounded_read_output(
        "task",
        vec![json!({"text": "x".repeat(super::MAX_SESSION_READ_CHARS)}), json!({"text":"recent"})],
        512,
    )
    .expect("bounded output");
    assert!(output.chars().count() <= 512);
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid bounded JSON");
    assert_eq!(value["truncated"], true);
    assert_eq!(value["messages"][0]["text"], "recent");
}
