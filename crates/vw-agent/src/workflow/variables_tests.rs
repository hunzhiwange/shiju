//! 验证工作流变量池对嵌套对象、数组路径与选择器格式的读写。
//! 非法下标、空选择器和非字符串路径片段不得产生部分匹配或幽灵变量。

use super::{VariablePool, selector_from_value, selector_key};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn get_selector_reads_nested_object_fields() {
    let mut pool = VariablePool::default();
    pool.insert_node_output(
        "code",
        "payload",
        json!({
            "order": {
                "id": "A-100"
            }
        }),
    );

    let selector = ["code", "payload", "order", "id"].map(str::to_string);

    assert_eq!(pool.get_selector(&selector), Some(&Value::String("A-100".to_string())));
}

#[test]
fn get_selector_reads_nested_array_items() {
    let mut pool = VariablePool::default();
    pool.insert_node_output(
        "code",
        "payload",
        json!({
            "items": [
                { "name": "first" },
                { "name": "second" }
            ]
        }),
    );

    let selector = ["code", "payload", "items[1]", "name"].map(str::to_string);

    assert_eq!(pool.get_selector(&selector), Some(&Value::String("second".to_string())));
}

#[test]
fn get_selector_reads_array_root_with_bracket_only_segment() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("code", "items", json!([{"name": "first"}, {"name": "second"}]));

    let selector = ["code", "items", "[1]", "name"].map(str::to_string);

    assert_eq!(pool.get_selector(&selector), Some(&Value::String("second".to_string())));
}

#[test]
fn get_selector_reads_dotted_path_parts() {
    let mut pool = VariablePool::default();
    pool.insert_node_output(
        "start",
        "profile",
        json!({
            "addresses": [
                { "city": "Hangzhou" }
            ]
        }),
    );

    let selector = ["start", "profile", "addresses[0].city"].map(str::to_string);

    assert_eq!(pool.get_selector(&selector), Some(&Value::String("Hangzhou".to_string())));
}

#[test]
fn from_values_and_values_clone_roundtrip() {
    let values = BTreeMap::from([("node.value".to_string(), Value::String("demo".into()))]);
    let pool = VariablePool::from_values(values.clone());

    assert_eq!(pool.values(), values);
    assert_eq!(
        pool.get_selector(&["node".to_string(), "value".to_string()]),
        Some(&Value::String("demo".into()))
    );
}

#[test]
fn insert_selector_ignores_empty_selector_and_node_outputs_are_prefixed() {
    let mut pool = VariablePool::default();
    pool.insert_selector(&[], Value::String("ignored".into()));
    pool.insert_node_output("node", "a", Value::Number(1.into()));
    pool.insert_node_output("other", "a", Value::Number(2.into()));

    assert!(pool.get_selector(&[]).is_none());
    assert_eq!(
        pool.node_outputs("node"),
        BTreeMap::from([("a".to_string(), Value::Number(1.into()))])
    );
}

#[test]
fn selector_helpers_filter_non_string_parts() {
    assert_eq!(selector_from_value(&json!(["a", 1, "b"])), vec!["a".to_string(), "b".to_string()]);
    assert!(selector_from_value(&json!("not array")).is_empty());
    assert_eq!(selector_key(&["a".to_string(), "b".to_string()]), "a.b");
}

#[test]
fn get_selector_returns_none_for_invalid_nested_paths() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("node", "payload", json!({"items": [{"name": "first"}]}));

    assert!(pool.get_selector(&["node", "payload", "items[abc]"].map(str::to_string)).is_none());
    assert!(pool.get_selector(&["node", "payload", "items[4]"].map(str::to_string)).is_none());
    assert!(pool.get_selector(&["node", "payload", "items[0"].map(str::to_string)).is_none());
    assert!(pool.get_selector(&["node", "payload", "missing"].map(str::to_string)).is_none());
}
