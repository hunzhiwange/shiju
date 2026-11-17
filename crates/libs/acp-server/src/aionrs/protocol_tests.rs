use super::protocol::Frame;
#[test]
fn aionrs_ready_requires_approval_capabilities() {
    let frame: Frame = serde_json::from_str(r#"{"type":"ready","version":"0.2.0","capabilities":{"tool_approval":true,"current_mode":"default"}}"#).unwrap();
    assert_eq!(frame.version.as_deref(), Some("0.2.0"));
    assert!(frame.capabilities.unwrap().tool_approval);
}
#[test]
fn aionrs_tool_request_retains_turn_and_call_identity() {
    let frame: Frame = serde_json::from_str(r#"{"type":"tool_request","msg_id":"m1","call_id":"t1","tool":{"name":"Write","args":{"secret":"redacted"}}}"#).unwrap();
    assert_eq!(frame.msg_id.as_deref(), Some("m1"));
    assert_eq!(frame.call_id.as_deref(), Some("t1"));
    assert_eq!(frame.tool.unwrap().name, "Write");
}
