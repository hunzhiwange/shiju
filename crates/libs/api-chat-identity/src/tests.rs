use super::ReasoningEffortDto;
use serde_json::json;

#[test]
fn reasoning_effort_rejects_unknown_protocol_values() {
    assert!(serde_json::from_value::<ReasoningEffortDto>(json!("invalid")).is_err());
}
