use super::{conditions_match, select_case_handle};
use crate::workflow::variables::VariablePool;
use serde_json::{Value, json};

#[test]
fn select_case_handle_supports_dify_common_operators() {
    let mut pool = VariablePool::default();
    pool.insert_selector(
        &["start".to_string(), "text".to_string()],
        Value::String("订单查询".to_string()),
    );
    pool.insert_selector(&["code".to_string(), "success".to_string()], Value::Bool(true));

    let cases = json!([
        {
            "case_id": "first",
            "logical_operator": "and",
            "conditions": [
                {
                    "comparison_operator": "contains",
                    "variable_selector": ["start", "text"],
                    "value": "订单"
                },
                {
                    "comparison_operator": "=",
                    "variable_selector": ["code", "success"],
                    "value": "1"
                }
            ]
        }
    ]);

    assert_eq!(
        select_case_handle(cases.as_array().expect("cases"), &pool).expect("selected"),
        "first"
    );
}

#[test]
fn select_case_handle_supports_nested_selector_paths() {
    let mut pool = VariablePool::default();
    pool.insert_selector(
        &["code".to_string(), "payload".to_string()],
        json!({
            "orders": [
                {
                    "status": "paid",
                    "amount": 128
                }
            ]
        }),
    );

    let cases = json!([
        {
            "case_id": "paid-order",
            "conditions": [
                {
                    "comparison_operator": "=",
                    "variable_selector": ["code", "payload", "orders[0]", "status"],
                    "value": "paid"
                },
                {
                    "comparison_operator": ">=",
                    "variable_selector": ["code", "payload", "orders[0].amount"],
                    "value": 100
                }
            ]
        }
    ]);

    assert_eq!(
        select_case_handle(cases.as_array().expect("cases"), &pool).expect("selected"),
        "paid-order"
    );
}

#[test]
fn select_case_handle_rejects_unsupported_operator() {
    let pool = VariablePool::default();
    let cases = json!([
        {
            "case_id": "bad",
            "conditions": [
                {
                    "comparison_operator": "around",
                    "variable_selector": ["start", "text"],
                    "value": "订单"
                }
            ]
        }
    ]);

    let error = select_case_handle(cases.as_array().expect("cases"), &pool).expect_err("error");
    assert!(error.contains("不支持的 if-else 比较运算符"));
}

#[test]
fn select_case_handle_defaults_to_false_for_no_matching_or_empty_handle() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("start", "query", Value::String("invoice".into()));

    let no_match = json!([
        {
            "case_id": "orders",
            "conditions": [{
                "comparison_operator": "contains",
                "variable_selector": ["start", "query"],
                "value": "order"
            }]
        }
    ]);
    assert_eq!(
        select_case_handle(no_match.as_array().expect("cases"), &pool).expect("handle"),
        "false"
    );

    let empty_handle = json!([
        {
            "case_id": "   ",
            "conditions": []
        }
    ]);
    assert_eq!(
        select_case_handle(empty_handle.as_array().expect("cases"), &pool).expect("handle"),
        "false"
    );
}

#[test]
fn conditions_match_supports_or_and_negative_string_operators() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("start", "text", Value::String("hello world".into()));

    let conditions = json!([
        {
            "comparison_operator": "start with",
            "variable_selector": ["start", "text"],
            "value": "nope"
        },
        {
            "comparison_operator": "end with",
            "variable_selector": ["start", "text"],
            "value": "world"
        }
    ]);

    assert!(conditions_match(conditions.as_array().expect("conditions"), "or", &pool).expect("or"));
    assert!(
        !conditions_match(conditions.as_array().expect("conditions"), "and", &pool).expect("and")
    );

    let negatives = json!([
        {
            "comparison_operator": "not contains",
            "variable_selector": ["start", "text"],
            "value": "missing"
        },
        {
            "comparison_operator": "is not",
            "variable_selector": ["start", "text"],
            "value": "other"
        }
    ]);
    assert!(
        conditions_match(negatives.as_array().expect("conditions"), "and", &pool)
            .expect("negative")
    );
}

#[test]
fn conditions_match_supports_empty_null_in_and_numeric_comparisons() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("node", "empty_text", Value::String("  ".into()));
    pool.insert_node_output("node", "items", json!(["paid", "shipped"]));
    pool.insert_node_output("node", "count", Value::String("10".into()));
    pool.insert_node_output("node", "flag", Value::Number(0.into()));

    let conditions = json!([
        {
            "comparison_operator": "empty",
            "variable_selector": ["node", "empty_text"]
        },
        {
            "comparison_operator": "not empty",
            "variable_selector": ["node", "items"]
        },
        {
            "comparison_operator": "in",
            "variable_selector": ["node", "flag"],
            "value": [false, true]
        },
        {
            "comparison_operator": "not in",
            "variable_selector": ["node", "count"],
            "value": [1, 2]
        },
        {
            "comparison_operator": ">",
            "variable_selector": ["node", "count"],
            "value": 9
        },
        {
            "comparison_operator": "<=",
            "variable_selector": ["node", "count"],
            "value": 10
        },
        {
            "comparison_operator": "null",
            "variable_selector": ["node", "missing"]
        }
    ]);

    assert!(
        conditions_match(conditions.as_array().expect("conditions"), "and", &pool)
            .expect("conditions")
    );

    let not_null = json!([{
        "comparison_operator": "not null",
        "variable_selector": ["node", "items"]
    }]);
    assert!(
        conditions_match(not_null.as_array().expect("conditions"), "and", &pool).expect("not null")
    );
}

#[test]
fn numeric_comparison_returns_false_for_non_numeric_values() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("node", "value", Value::String("abc".into()));

    let conditions = json!([{
        "comparison_operator": ">=",
        "variable_selector": ["node", "value"],
        "value": 1
    }]);

    assert!(
        !conditions_match(conditions.as_array().expect("conditions"), "and", &pool)
            .expect("numeric")
    );
}

#[test]
fn select_case_handle_uses_legacy_id_and_default_equality() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("node", "flag", Value::String("TRUE".into()));
    let cases = json!([{
        "id": "legacy",
        "conditions": [{
            "variable_selector": ["node", "flag"],
            "value": 1
        }]
    }]);

    assert_eq!(
        select_case_handle(cases.as_array().expect("cases"), &pool).expect("handle"),
        "legacy"
    );
}

#[test]
fn conditions_match_distinguishes_empty_collection_and_scalar_values() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("node", "empty_object", json!({}));
    pool.insert_node_output("node", "empty_array", json!([]));
    pool.insert_node_output("node", "number", json!(0));
    pool.insert_node_output("node", "boolean", json!(false));

    let conditions = json!([
        {
            "comparison_operator": "empty",
            "variable_selector": ["node", "empty_object"]
        },
        {
            "comparison_operator": "empty",
            "variable_selector": ["node", "empty_array"]
        },
        {
            "comparison_operator": "not empty",
            "variable_selector": ["node", "number"]
        },
        {
            "comparison_operator": "not empty",
            "variable_selector": ["node", "boolean"]
        }
    ]);

    assert!(
        conditions_match(conditions.as_array().expect("conditions"), "and", &pool)
            .expect("collection and scalar emptiness")
    );
}
