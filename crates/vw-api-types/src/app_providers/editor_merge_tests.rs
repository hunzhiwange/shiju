use super::editor_merge::*;
#[test]
fn cc04_editor_merge_inputs_reject_baselines_and_conceal_drafts() {
    let input = serde_json::json!({"targetHostId":"host","context":"opaque","edits":[{"path":["x"],"value":{"kind":"set","value":"fixture-secret"}}],"resolution":null});
    let request: EditorSave = serde_json::from_value(input.clone()).unwrap();
    assert!(!format!("{request:?}").contains("fixture-secret"));
    let mut invalid = input;
    invalid["base"] = serde_json::json!({});
    assert!(serde_json::from_value::<EditorSave>(invalid).is_err());
    assert!(serde_json::from_value::<EditorChoice>(serde_json::json!("force")).is_err());
}
