use super::*;
#[test]
fn video_edit_ui_editor_presentation_does_not_change_capabilities() {
    let mut root: Value = serde_yaml::from_str(DEFAULT_YAML).unwrap();
    root["workflow"]["environment_variables"] = serde_json::json!([]);
    root["workflow"]["conversation_variables"] = serde_json::json!([]);
    root["workflow"]["graph"]["viewport"] = serde_json::json!({"x":120,"y":120,"zoom":1});
    for node in root["workflow"]["graph"]["nodes"].as_array_mut().unwrap() {
        node["sourcePosition"] = Value::from("right");
        node["targetPosition"] = Value::from("left");
        node["zIndex"] = Value::from(0);
        node["data"]["selected"] = Value::from(false);
    }
    for (index, edge) in
        root["workflow"]["graph"]["edges"].as_array_mut().unwrap().iter_mut().enumerate()
    {
        edge["id"] = Value::from(format!("edge-{index}"));
        edge["type"] = Value::from("custom");
        edge["selected"] = Value::from(false);
        edge["zIndex"] = Value::from(0);
        edge["data"] = serde_json::json!({"sourceType":"tool","targetType":"tool"});
    }
    assert!(
        validate(&serde_yaml::to_string(&root).unwrap()).is_ok(),
        "editor presentation rejected"
    );
    root["workflow"]["graph"]["nodes"][1]["data"]["tool_name"] = Value::from("bash");
    assert!(
        validate(&serde_yaml::to_string(&root).unwrap()).is_err(),
        "capability bypass accepted"
    );
}
