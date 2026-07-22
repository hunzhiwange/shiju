use super::*;

use serde_json::json;

#[test]
fn merge_deep_value_recursively_merges_objects() {
    let mut target = json!({
        "temperature": 0.2,
        "nested": {
            "keep": true,
            "replace": "old",
            "inner": {
                "left": 1
            }
        }
    });
    let source = json!({
        "top_p": 0.9,
        "nested": {
            "replace": "new",
            "inner": {
                "right": 2
            }
        }
    });

    merge_deep_value(&mut target, &source);

    assert_eq!(
        target,
        json!({
            "temperature": 0.2,
            "top_p": 0.9,
            "nested": {
                "keep": true,
                "replace": "new",
                "inner": {
                    "left": 1,
                    "right": 2
                }
            }
        })
    );
}

#[test]
fn merge_deep_value_replaces_non_object_values() {
    let mut target = json!({
        "array": [1, 2],
        "scalar": "old",
        "object_to_null": {
            "nested": true
        }
    });
    let source = json!({
        "array": [3],
        "scalar": {
            "now": "object"
        },
        "object_to_null": null
    });

    merge_deep_value(&mut target, &source);

    assert_eq!(
        target,
        json!({
            "array": [3],
            "scalar": {
                "now": "object"
            },
            "object_to_null": null
        })
    );
}

#[test]
fn merge_deep_value_replaces_non_object_target_with_object_source() {
    let mut target = json!("old");
    let source = json!({
        "new": true
    });

    merge_deep_value(&mut target, &source);

    assert_eq!(target, json!({ "new": true }));
}

#[test]
fn merge_deep_value_leaves_target_unchanged_for_empty_object_source() {
    let mut target = json!({
        "keep": {
            "nested": 1
        }
    });

    merge_deep_value(&mut target, &json!({}));

    assert_eq!(target, json!({ "keep": { "nested": 1 } }));
}

#[test]
fn merge_deep_value_leaves_nested_target_unchanged_for_empty_nested_object_source() {
    let mut target = json!({
        "nested": {
            "keep": 1
        },
        "top_level": true
    });
    let source = json!({
        "nested": {}
    });

    merge_deep_value(&mut target, &source);

    assert_eq!(
        target,
        json!({
            "nested": {
                "keep": 1
            },
            "top_level": true
        })
    );
}

#[test]
fn merge_deep_value_clones_values_inserted_for_missing_keys() {
    let mut target = json!({});
    let source = json!({
        "inserted": {
            "items": [1, 2]
        }
    });

    merge_deep_value(&mut target, &source);
    target["inserted"]["items"][0] = json!(99);

    assert_eq!(source, json!({ "inserted": { "items": [1, 2] } }));
    assert_eq!(target, json!({ "inserted": { "items": [99, 2] } }));
}
