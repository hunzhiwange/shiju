use super::*;
use crate::app::agent::util::MaybeSet;
use serde_json::json;

#[test]
fn parse_string_list_accepts_csv_and_arrays() {
    assert_eq!(
        ModelRoutingConfigTool::parse_string_list(&json!("a, b,,c"), "items").unwrap(),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        ModelRoutingConfigTool::parse_string_list(&json!(["a", " ", "b"]), "items").unwrap(),
        vec!["a", "b"]
    );
}

#[test]
fn parse_non_empty_string_rejects_missing_and_blank() {
    assert_eq!(
        ModelRoutingConfigTool::parse_non_empty_string(&json!({"name":" agent "}), "name").unwrap(),
        "agent"
    );
    assert!(
        ModelRoutingConfigTool::parse_non_empty_string(&json!({"name":"   "}), "name").is_err()
    );
}

#[test]
fn optional_updates_distinguish_unset_null_and_values() {
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_string_update(&json!({}), "model").unwrap(),
        MaybeSet::Unset
    );
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_string_update(&json!({"model": null}), "model")
            .unwrap(),
        MaybeSet::Null
    );
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_string_update(&json!({"model":" gpt "}), "model")
            .unwrap(),
        MaybeSet::Set("gpt".into())
    );
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_bool(&json!({"enabled": true}), "enabled").unwrap(),
        Some(true)
    );
}

#[test]
fn parse_string_inputs_reject_wrong_types_and_non_string_array_items() {
    let scalar_error = ModelRoutingConfigTool::parse_string_list(&json!(42), "items").unwrap_err();
    assert_eq!(scalar_error.to_string(), "'items' must be a string or string[]");

    let item_error =
        ModelRoutingConfigTool::parse_string_list(&json!(["valid", 42]), "items").unwrap_err();
    assert_eq!(item_error.to_string(), "'items' array must only contain strings");

    let missing_error =
        ModelRoutingConfigTool::parse_non_empty_string(&json!({}), "name").unwrap_err();
    assert_eq!(missing_error.to_string(), "Missing 'name'");
}

#[test]
fn optional_numeric_updates_parse_values_and_reject_invalid_ranges() {
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_f64_update(
            &json!({"temperature": 0.5}),
            "temperature"
        )
        .unwrap(),
        MaybeSet::Set(0.5)
    );
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_usize_update(&json!({"count": 3}), "count").unwrap(),
        MaybeSet::Set(3)
    );
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_u32_update(&json!({"depth": 4}), "depth").unwrap(),
        MaybeSet::Set(4)
    );
    assert_eq!(
        ModelRoutingConfigTool::parse_optional_i32_update(&json!({"priority": -2}), "priority")
            .unwrap(),
        MaybeSet::Set(-2)
    );

    assert!(
        ModelRoutingConfigTool::parse_optional_f64_update(
            &json!({"temperature": "warm"}),
            "temperature"
        )
        .unwrap_err()
        .to_string()
        .contains("must be a number or null")
    );
    assert!(
        ModelRoutingConfigTool::parse_optional_usize_update(&json!({"count": -1}), "count")
            .unwrap_err()
            .to_string()
            .contains("non-negative integer")
    );
    assert!(
        ModelRoutingConfigTool::parse_optional_u32_update(
            &json!({"depth": u64::from(u32::MAX) + 1}),
            "depth"
        )
        .unwrap_err()
        .to_string()
        .contains("must fit in u32")
    );
    assert!(
        ModelRoutingConfigTool::parse_optional_i32_update(
            &json!({"priority": i64::from(i32::MAX) + 1}),
            "priority"
        )
        .unwrap_err()
        .to_string()
        .contains("must fit in i32")
    );
}

#[test]
fn optional_updates_reject_non_matching_json_types() {
    assert!(
        ModelRoutingConfigTool::parse_optional_string_update(&json!({"model": 7}), "model")
            .is_err()
    );
    assert!(
        ModelRoutingConfigTool::parse_optional_bool(&json!({"enabled": null}), "enabled").is_err()
    );
}
