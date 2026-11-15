use super::*;
use serde_json::json;

fn from_config(value: &Value) -> Ruleset {
    super::from_config(value, str::to_string)
}

#[test]
fn wildcard_match_respects_simple_globs() {
    assert!(wildcard_match("shell.read", "shell.*"));
    assert!(wildcard_match("abc", "a*c"));
    assert!(!wildcard_match("abc", "a*d"));
}

#[test]
fn wildcard_match_rejects_invalid_glob_patterns() {
    assert!(!wildcard_match("anything", "["));
}

#[test]
fn evaluate_defaults_to_ask_without_matching_rule() {
    let ruleset = from_config(&json!({"shell.rm": "deny", "shell.read": "ask"}));
    assert_eq!(evaluate("shell.rm", "anything", &[ruleset.clone()]).action, Action::Deny);
    assert_eq!(evaluate("shell.read", "anything", &[ruleset]).action, Action::Ask);
}

#[test]
// 解析只接受对象规则，未知动作按 Ask 收敛，避免配置错误变成静默允许。
fn from_config_rejects_non_objects_and_defaults_unknown_actions_to_ask() {
    assert!(from_config(&json!(null)).is_empty());
    assert!(from_config(&json!(["allow"])).is_empty());

    let ruleset = from_config(&json!({
        "trimmed": " allow ",
        "unknown": "ALLOW",
        "patterns": {
            "accepted": " deny ",
            "ignored": false
        }
    }));

    assert!(
        ruleset.iter().any(|r| {
            r.permission == "trimmed" && r.pattern == "*" && r.action == Action::Allow
        })
    );
    assert!(
        ruleset
            .iter()
            .any(|r| { r.permission == "unknown" && r.pattern == "*" && r.action == Action::Ask })
    );
    assert!(ruleset.iter().any(|r| {
        r.permission == "patterns" && r.pattern == "accepted" && r.action == Action::Deny
    }));
    assert!(!ruleset.iter().any(|r| r.pattern == "ignored"));
}

#[test]
fn merge_preserves_ruleset_and_rule_order() {
    let first =
        Rule { permission: "first".to_string(), pattern: "one".to_string(), action: Action::Allow };
    let second =
        Rule { permission: "second".to_string(), pattern: "two".to_string(), action: Action::Deny };

    let merged = merge(&[vec![first], Vec::new(), vec![second]]);

    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].permission, "first");
    assert_eq!(merged[1].permission, "second");
}

#[test]
fn evaluate_uses_later_matching_rule_priority() {
    let base = vec![Rule {
        permission: "edit".to_string(),
        pattern: "src/**".to_string(),
        action: Action::Deny,
    }];
    let override_rules = vec![Rule {
        permission: "edit".to_string(),
        pattern: "src/main.rs".to_string(),
        action: Action::Allow,
    }];

    assert_eq!(evaluate("edit", "src/main.rs", &[base, override_rules]).action, Action::Allow);
}

#[test]
fn evaluate_returns_a_complete_default_rule_without_a_match() {
    let rule = evaluate("read", "notes.txt", &[]);

    assert_eq!(rule.action, Action::Ask);
    assert_eq!(rule.permission, "read");
    assert_eq!(rule.pattern, "*");
}

#[test]
fn evaluate_matches_permission_globs_and_prefers_the_last_match() {
    let ruleset = vec![
        Rule { permission: "shell.*".to_string(), pattern: "*".to_string(), action: Action::Allow },
        Rule {
            permission: "shell.read".to_string(),
            pattern: "*.key".to_string(),
            action: Action::Deny,
        },
    ];

    assert_eq!(evaluate("shell.write", "notes.txt", &[ruleset.clone()]).action, Action::Allow);
    assert_eq!(evaluate("shell.read", "secret.key", &[ruleset]).action, Action::Deny);
}

#[test]
fn disabled_maps_edit_tools_and_requires_global_deny() {
    let ruleset = vec![
        Rule { permission: "edit".to_string(), pattern: "*".to_string(), action: Action::Deny },
        Rule { permission: "bash".to_string(), pattern: "rm *".to_string(), action: Action::Deny },
        Rule { permission: "read".to_string(), pattern: "*".to_string(), action: Action::Allow },
    ];
    let tools =
        vec!["write".to_string(), "patch".to_string(), "bash".to_string(), "read".to_string()];

    let disabled = disabled(&tools, &ruleset);
    assert!(disabled.contains("write"));
    assert!(disabled.contains("patch"));
    assert!(!disabled.contains("bash"));
    assert!(!disabled.contains("read"));
}

#[test]
fn disabled_uses_the_last_permission_rule() {
    let tools = vec!["edit".to_string(), "write".to_string(), "unknown".to_string()];
    let allow_override = vec![
        Rule { permission: "edit".to_string(), pattern: "*".to_string(), action: Action::Deny },
        Rule { permission: "edit".to_string(), pattern: "*".to_string(), action: Action::Allow },
    ];
    let deny_override = allow_override.iter().cloned().rev().collect::<Vec<_>>();

    assert!(disabled(&tools, &allow_override).is_empty());

    let disabled = disabled(&tools, &deny_override);
    assert!(disabled.contains("edit"));
    assert!(disabled.contains("write"));
    assert!(!disabled.contains("unknown"));
}

