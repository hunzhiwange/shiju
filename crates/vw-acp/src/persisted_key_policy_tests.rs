//! 验证持久化键名策略、嵌套路径检查与工具结果豁免边界。
//!
//! 受控输入、临时路径和策略 fixture 用于验证默认拒绝的安全边界。

use super::*;
use serde_json::json;

// 下列行为用例覆盖允许路径、拒绝路径以及错误输入的安全处理。
#[test]
fn persisted_key_policy_allows_snake_case_and_zed_tags() {
    let value = json!({
        "session_id": "s1",
        "messages": [{"Agent": {"tool_results": [{"CamelCaseOutput": "allowed under tag"}]}}],
        "agent_capabilities": {"CamelCaseOpaque": true}
    });

    assert!(assert_persisted_key_policy(&value).is_ok());
}

#[test]
fn persisted_key_policy_reports_nested_violations() {
    let violations = find_persisted_key_policy_violations(&json!({
        "badKey": true,
        "nested": {"alsoBad": 1}
    }));

    assert_eq!(violations, vec!["badKey", "nested.alsoBad"]);
}

#[test]
fn assert_persisted_key_policy_returns_joined_error_message() {
    let error = assert_persisted_key_policy(&json!({
        "badKey": true,
        "nested": {"alsoBad": 1}
    }))
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Persisted key policy violation (expected snake_case keys): badKey, nested.alsoBad"
    );
}

#[test]
fn persisted_key_policy_descends_short_output_and_result_paths() {
    let violations = find_persisted_key_policy_violations(&json!({
        "output": {
            "badKey": true
        },
        "result": {
            "alsoBad": true
        }
    }));

    assert_eq!(violations, vec!["output.badKey", "result.alsoBad"]);
}

#[test]
fn persisted_key_policy_only_skips_exact_tool_result_payload_paths() {
    let violations = find_persisted_key_policy_violations(&json!({
        "messages": {
            "Agent": {
                "tool_results": {
                    "tool-1": {
                        "output": {
                            "camelCaseAllowed": true
                        },
                        "result": {
                            "alsoCamelCaseAllowed": true
                        },
                        "nested": {
                            "output": {
                                "badKey": true
                            }
                        }
                    }
                }
            }
        },
        "other": {
            "Agent": {
                "tool_results": {
                    "tool-2": {
                        "output": {
                            "wrongPrefixBadKey": true
                        }
                    }
                }
            }
        }
    }));

    assert_eq!(
        violations,
        vec![
            "messages.Agent.tool_results.tool-1.nested.output.badKey",
            "other.Agent.tool_results.tool-2",
            "other.Agent.tool_results.tool-2.output.wrongPrefixBadKey",
        ]
    );
}

#[test]
fn persisted_key_policy_checks_array_entries_and_allows_scalar_inputs() {
    let violations = find_persisted_key_policy_violations(&json!([
        {
            "valid_key": true
        },
        {
            "badKey": true,
            "nested": [
                {
                    "alsoBad": true
                }
            ]
        },
        null
    ]));

    assert_eq!(violations, vec!["badKey", "nested.alsoBad"]);
    assert!(find_persisted_key_policy_violations(&json!(null)).is_empty());
}

#[test]
fn snake_case_key_rejects_empty_and_uppercase_starts() {
    assert!(is_snake_case_key("valid_key_1"));
    assert!(!is_snake_case_key(""));
    assert!(!is_snake_case_key("Bad"));
}

#[test]
fn snake_case_key_rejects_non_snake_case_boundaries() {
    assert!(!is_snake_case_key("1_bad"));
    assert!(!is_snake_case_key("_bad"));
    assert!(!is_snake_case_key("bad-key"));
    assert!(!is_snake_case_key("badKey"));
}

#[test]
fn tool_result_field_path_helper_rejects_short_paths() {
    let path = vec!["messages".to_string(), "Agent".to_string()];

    assert!(!matches_tool_result_field_path(&path));
}

#[test]
fn tool_result_field_path_helper_rejects_missing_marker_and_wrong_prefix() {
    let missing_marker_path = vec![
        "messages".to_string(),
        "Agent".to_string(),
        "other_results".to_string(),
        "tool-1".to_string(),
        "output".to_string(),
    ];
    let wrong_prefix_path = vec![
        "other".to_string(),
        "Agent".to_string(),
        "tool_results".to_string(),
        "tool-1".to_string(),
        "output".to_string(),
    ];
    let extra_segment_path = vec![
        "messages".to_string(),
        "Agent".to_string(),
        "tool_results".to_string(),
        "tool-1".to_string(),
        "nested".to_string(),
        "output".to_string(),
    ];

    assert!(!matches_tool_result_field_path(&missing_marker_path));
    assert!(!matches_tool_result_field_path(&wrong_prefix_path));
    assert!(!matches_tool_result_field_path(&extra_segment_path));
}
