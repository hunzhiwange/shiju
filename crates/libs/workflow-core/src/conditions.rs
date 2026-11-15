//! Dify if-else 条件判断。

use super::model::{array_field, string_field};
use super::template::value_to_text;
use super::variables::{VariablePool, selector_from_value};
use serde_json::Value;

pub fn select_case_handle(cases: &[Value], pool: &VariablePool) -> Result<String, String> {
    for case in cases {
        if case_matches(case, pool)? {
            if let Some(handle) = case_handle(case) {
                return Ok(handle);
            }
            break;
        }
    }
    Ok("false".to_string())
}

fn case_matches(case: &Value, pool: &VariablePool) -> Result<bool, String> {
    let conditions = array_field(case, "conditions");
    if conditions.is_empty() {
        return Ok(true);
    }
    conditions_match(conditions, string_field(case, "logical_operator").unwrap_or("and"), pool)
}

pub fn conditions_match(
    conditions: &[Value],
    logical_operator: &str,
    pool: &VariablePool,
) -> Result<bool, String> {
    let all = !logical_operator.eq_ignore_ascii_case("or");
    if all {
        for condition in conditions {
            if !condition_matches(condition, pool)? {
                return Ok(false);
            }
        }
        Ok(true)
    } else {
        for condition in conditions {
            if condition_matches(condition, pool)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn condition_matches(condition: &Value, pool: &VariablePool) -> Result<bool, String> {
    let selector = condition.get("variable_selector").map(selector_from_value).unwrap_or_default();
    let actual = pool.get_selector(&selector).unwrap_or(&Value::Null);
    let expected = condition.get("value").unwrap_or(&Value::Null);
    let operator =
        string_field(condition, "comparison_operator").unwrap_or("is").trim().to_ascii_lowercase();
    let matches = match operator.as_str() {
        "contains" => value_contains(actual, expected),
        "not contains" => !value_contains(actual, expected),
        "start with" => value_to_text(actual).starts_with(&value_to_text(expected)),
        "end with" => value_to_text(actual).ends_with(&value_to_text(expected)),
        "empty" => value_empty(actual),
        "not empty" => !value_empty(actual),
        "null" => actual.is_null(),
        "not null" => !actual.is_null(),
        "is" | "=" => values_equal(actual, expected),
        "is not" | "!=" | "≠" => !values_equal(actual, expected),
        "in" => value_contains(expected, actual),
        "not in" => !value_contains(expected, actual),
        ">" => compare_number(actual, expected, |left, right| left > right),
        "<" => compare_number(actual, expected, |left, right| left < right),
        ">=" | "≥" => compare_number(actual, expected, |left, right| left >= right),
        "<=" | "≤" => compare_number(actual, expected, |left, right| left <= right),
        other => return Err(format!("不支持的 if-else 比较运算符: {other}")),
    };
    Ok(matches)
}

fn case_handle(case: &Value) -> Option<String> {
    case.get("case_id")
        .or_else(|| case.get("id"))
        .map(value_to_text)
        .filter(|value| !value.trim().is_empty())
}

fn values_equal(actual: &Value, expected: &Value) -> bool {
    if let (Some(actual), Some(expected)) = (boolish_value(actual), boolish_value(expected)) {
        return actual == expected;
    }
    actual == expected || value_to_text(actual).trim() == value_to_text(expected).trim()
}

fn boolish_value(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) if value.as_i64() == Some(1) => Some(true),
        Value::Number(value) if value.as_i64() == Some(0) => Some(false),
        Value::String(value) if value.eq_ignore_ascii_case("true") || value == "1" => Some(true),
        Value::String(value) if value.eq_ignore_ascii_case("false") || value == "0" => Some(false),
        _ => None,
    }
}

fn value_contains(actual: &Value, expected: &Value) -> bool {
    match actual {
        Value::Array(items) => items.iter().any(|item| values_equal(item, expected)),
        other => value_to_text(other).contains(&value_to_text(expected)),
    }
}

fn value_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.trim().is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    }
}

fn compare_number(actual: &Value, expected: &Value, predicate: impl Fn(f64, f64) -> bool) -> bool {
    let left = value_to_text(actual).parse::<f64>().ok();
    let right = value_to_text(expected).parse::<f64>().ok();
    left.zip(right).is_some_and(|(left, right)| predicate(left, right))
}

#[cfg(test)]
#[path = "conditions_tests.rs"]
mod conditions_tests;
