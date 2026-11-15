//! 持久化记录键名策略的检查与约束。

use serde_json::Value;

// Map keys are protocol/user identifiers; nested values still obey local field rules.
const MAP_OBJECT_PATHS: &[&str] = &[
    "request_token_usage",
    "messages.Agent.tool_results",
    "agent_config.env",
    "vwacp.desired_config_options",
];
const OPAQUE_VALUE_PATHS: &[&str] =
    &["agent_capabilities", "messages.Agent.content.ToolUse.input", "vwacp.config_options"];
const ZED_TAG_KEYS: &[&str] = &[
    "User",
    "Agent",
    "Resume",
    "Text",
    "Mention",
    "Image",
    "Thinking",
    "RedactedThinking",
    "ToolUse",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Persisted key policy violation (expected snake_case keys): {violations}")]
pub struct PersistedKeyPolicyError {
    violations: String,
}

impl PersistedKeyPolicyError {
    pub fn new(violations: Vec<String>) -> Self {
        Self { violations: violations.join(", ") }
    }
}

fn join_path(path: &[String]) -> String {
    path.join(".")
}

fn is_allowed_key(key: &str) -> bool {
    ZED_TAG_KEYS.contains(&key)
}

fn should_skip_key_rule(path: &[String]) -> bool {
    MAP_OBJECT_PATHS.contains(&join_path(path).as_str())
}

fn should_skip_descend(path: &[String]) -> bool {
    OPAQUE_VALUE_PATHS.contains(&join_path(path).as_str())
        || is_tool_result_output_path(path)
        || is_tool_result_result_path(path)
}

fn is_tool_result_output_path(path: &[String]) -> bool {
    if path.len() < 5 || path.last().is_none_or(|segment| segment != "output") {
        return false;
    }

    matches_tool_result_field_path(path)
}

fn is_tool_result_result_path(path: &[String]) -> bool {
    if path.len() < 5 || path.last().is_none_or(|segment| segment != "result") {
        return false;
    }

    matches_tool_result_field_path(path)
}

fn matches_tool_result_field_path(path: &[String]) -> bool {
    if path.len() < 5 {
        return false;
    }

    let Some(tool_results_index) = path.iter().rposition(|segment| segment == "tool_results")
    else {
        return false;
    };

    if tool_results_index + 2 != path.len() - 1 {
        return false;
    }

    join_path(&path[..=tool_results_index]) == "messages.Agent.tool_results"
}

fn is_snake_case_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn collect_violations(value: &Value, path: &[String], violations: &mut Vec<String>) {
    match value {
        Value::Array(entries) => {
            for entry in entries {
                collect_violations(entry, path, violations);
            }
        }
        Value::Object(record) => {
            let skip_key_rule = should_skip_key_rule(path);
            for (key, child) in record {
                if !skip_key_rule && !is_snake_case_key(key) && !is_allowed_key(key) {
                    let child_path = if path.is_empty() {
                        key.to_string()
                    } else {
                        format!("{}.{}", join_path(path), key)
                    };
                    violations.push(child_path);
                }

                let child_path =
                    path.iter().cloned().chain(std::iter::once(key.clone())).collect::<Vec<_>>();
                if should_skip_descend(&child_path) {
                    continue;
                }
                collect_violations(child, &child_path, violations);
            }
        }
        _ => {}
    }
}

pub fn find_persisted_key_policy_violations(value: &Value) -> Vec<String> {
    let mut violations = Vec::new();
    collect_violations(value, &[], &mut violations);
    violations
}

pub fn assert_persisted_key_policy(value: &Value) -> Result<(), PersistedKeyPolicyError> {
    let violations = find_persisted_key_policy_violations(value);
    if violations.is_empty() {
        return Ok(());
    }
    Err(PersistedKeyPolicyError::new(violations))
}

#[cfg(test)]
#[path = "persisted_key_policy_tests.rs"]
mod persisted_key_policy_tests;
