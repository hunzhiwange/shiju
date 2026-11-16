use super::planning::*;
#[test]
fn video_edit_plan_json_rejects_boolean_time_and_unstructured_assets() {
    let effect = super::merge_tests::effect(0, VisualType::Keyword);
    let mut value = serde_json::to_value(effect).unwrap();
    value["start"] = serde_json::json!(true);
    assert!(serde_json::from_value::<VideoEditEffect>(value.clone()).is_err());
    value["start"] = serde_json::json!(0);
    value["assets"] = serde_json::json!([{"prompt":"x"}]);
    assert!(serde_json::from_value::<VideoEditEffect>(value).is_err());
}
