use super::*;
use crate::app::App;
use crate::app::state::QueryClassificationRuleInput;

fn app() -> App {
    App::new().0
}

#[test]
fn rules_for_config_filters_blank_rows_and_normalizes_values() {
    let mut app = app();
    app.query_classification_settings.rules = vec![
        QueryClassificationRuleInput {
            pattern: "   ".to_string(),
            category: String::new(),
            priority_input: String::new(),
        },
        QueryClassificationRuleInput {
            pattern: "  rust async  ".to_string(),
            category: "  code  ".to_string(),
            priority_input: "  7  ".to_string(),
        },
        QueryClassificationRuleInput {
            pattern: String::new(),
            category: "fallback".to_string(),
            priority_input: "not-a-number".to_string(),
        },
    ];

    let rules = rules_for_config(&app);

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].hint, "code");
    assert_eq!(rules[0].keywords, vec!["rust async".to_string()]);
    assert!(rules[0].patterns.is_empty());
    assert_eq!(rules[0].min_length, None);
    assert_eq!(rules[0].max_length, None);
    assert_eq!(rules[0].priority, 7);
    assert_eq!(rules[1].hint, "fallback");
    assert!(rules[1].keywords.is_empty());
    assert_eq!(rules[1].priority, 0);
}

#[test]
fn rules_for_config_keeps_priority_only_rows_with_default_text_fields() {
    let mut app = app();
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "  ".to_string(),
        category: "  ".to_string(),
        priority_input: " -2147483648 ".to_string(),
    }];

    let rules = rules_for_config(&app);

    assert_eq!(rules.len(), 1);
    assert!(rules[0].hint.is_empty());
    assert!(rules[0].keywords.is_empty());
    assert_eq!(rules[0].priority, i32::MIN);
}

#[test]
fn rule_input_from_config_prefers_keywords_then_patterns() {
    let keyword_rule = ClassificationRule {
        hint: "code".to_string(),
        keywords: vec!["rust".to_string(), "cargo".to_string()],
        patterns: vec!["*.rs".to_string()],
        min_length: Some(2),
        max_length: Some(200),
        priority: -4,
    };
    let pattern_rule = ClassificationRule {
        hint: "docs".to_string(),
        keywords: Vec::new(),
        patterns: vec!["*.md".to_string(), "*.txt".to_string()],
        min_length: None,
        max_length: None,
        priority: 9,
    };
    let empty_rule = ClassificationRule {
        hint: String::new(),
        keywords: Vec::new(),
        patterns: Vec::new(),
        min_length: None,
        max_length: None,
        priority: 0,
    };

    let keyword_input = rule_input_from_config(keyword_rule);
    assert_eq!(keyword_input.pattern, "rust");
    assert_eq!(keyword_input.category, "code");
    assert_eq!(keyword_input.priority_input, "-4");

    let pattern_input = rule_input_from_config(pattern_rule);
    assert_eq!(pattern_input.pattern, "*.md");
    assert_eq!(pattern_input.category, "docs");
    assert_eq!(pattern_input.priority_input, "9");

    let empty_input = rule_input_from_config(empty_rule);
    assert!(empty_input.pattern.is_empty());
    assert!(empty_input.category.is_empty());
    assert_eq!(empty_input.priority_input, "0");
}

#[test]
fn validate_accepts_empty_rows_and_trimmed_complete_rules() {
    let mut app = app();
    app.query_classification_settings.rules = vec![
        QueryClassificationRuleInput {
            pattern: "  ".to_string(),
            category: "  ".to_string(),
            priority_input: "  ".to_string(),
        },
        QueryClassificationRuleInput {
            pattern: " rust ".to_string(),
            category: " code ".to_string(),
            priority_input: " -3 ".to_string(),
        },
    ];

    assert_eq!(validate(&app), Ok(()));
}

#[test]
fn validate_reports_first_invalid_rule_with_one_based_index() {
    let mut app = app();
    app.query_classification_settings.rules = vec![
        QueryClassificationRuleInput {
            pattern: "rust".to_string(),
            category: "code".to_string(),
            priority_input: String::new(),
        },
        QueryClassificationRuleInput {
            pattern: String::new(),
            category: "chat".to_string(),
            priority_input: String::new(),
        },
    ];
    assert_eq!(validate(&app), Err("第 2 条分类规则缺少 pattern".to_string()));

    app.query_classification_settings.rules[1].pattern = "hello".to_string();
    app.query_classification_settings.rules[1].category.clear();
    assert_eq!(validate(&app), Err("第 2 条分类规则缺少 category".to_string()));

    app.query_classification_settings.rules[1].category = "chat".to_string();
    app.query_classification_settings.rules[1].priority_input = "high".to_string();
    assert_eq!(validate(&app), Err("第 2 条分类规则的 priority 不是有效数字".to_string()));
}

#[test]
fn invalid_edits_update_the_field_and_expose_validation_error() {
    let mut app = app();
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput::default()];

    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::CategoryChanged(
            0,
            "code".to_string(),
        )),
    );
    assert_eq!(app.query_classification_settings.rules[0].category, "code");
    assert_eq!(
        app.query_classification_settings.save_error.as_deref(),
        Some("第 1 条分类规则缺少 pattern")
    );

    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::PatternChanged(
            0,
            "  ".to_string(),
        )),
    );
    assert_eq!(app.query_classification_settings.rules[0].pattern, "  ");
    assert_eq!(
        app.query_classification_settings.save_error.as_deref(),
        Some("第 1 条分类规则缺少 pattern")
    );

    app.query_classification_settings.rules[0].pattern = "rust".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::PriorityChanged(
            0,
            "high".to_string(),
        )),
    );
    assert_eq!(app.query_classification_settings.rules[0].priority_input, "high");
    assert_eq!(
        app.query_classification_settings.save_error.as_deref(),
        Some("第 1 条分类规则的 priority 不是有效数字")
    );
}

#[test]
fn add_remove_and_toggle_messages_update_local_state_before_sync_result() {
    let mut app = app();
    app.query_classification_settings.enabled = false;
    app.query_classification_settings.save_error = Some("old".to_string());
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "rust".to_string(),
        category: "code".to_string(),
        priority_input: "1".to_string(),
    }];

    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::EnabledToggled(true)),
    );
    assert!(app.query_classification_settings.enabled);

    let _ =
        update(&mut app, SettingsMessage::QueryClassification(QueryClassificationMessage::AddRule));
    assert_eq!(app.query_classification_settings.rules.len(), 2);
    assert!(app.query_classification_settings.rules[1].pattern.is_empty());

    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::RemoveRule(0)),
    );
    assert_eq!(app.query_classification_settings.rules.len(), 1);
    assert!(app.query_classification_settings.rules[0].pattern.is_empty());

    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::RemoveRule(99)),
    );
    assert_eq!(app.query_classification_settings.rules.len(), 1);
}

#[test]
fn valid_field_edits_clear_validation_error_and_preserve_updated_values() {
    let mut app = app();
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "rust".to_string(),
        category: "code".to_string(),
        priority_input: "bad".to_string(),
    }];
    app.query_classification_settings.save_error = Some("old".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::PriorityChanged(
            0,
            "-2".to_string(),
        )),
    );

    assert_eq!(app.query_classification_settings.rules[0].priority_input, "-2");
    assert_ne!(
        app.query_classification_settings.save_error.as_deref(),
        Some("第 1 条分类规则的 priority 不是有效数字")
    );
}

#[test]
fn refresh_reloads_default_config_and_clears_error_when_gateway_is_unavailable() {
    let mut app = app();
    app.query_classification_settings.enabled = true;
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "stale".to_string(),
        category: "stale".to_string(),
        priority_input: "9".to_string(),
    }];
    app.query_classification_settings.save_error = Some("old".to_string());

    let _ =
        update(&mut app, SettingsMessage::QueryClassification(QueryClassificationMessage::Refresh));

    assert!(app.query_classification_settings.save_error.is_none());
    assert_eq!(
        app.query_classification_settings.enabled,
        load_query_classification_config().enabled
    );
}

#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = app();
    app.query_classification_settings.enabled = true;
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput::default()];
    app.query_classification_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.query_classification_settings.enabled);
    assert_eq!(app.query_classification_settings.rules.len(), 1);
    assert_eq!(app.query_classification_settings.save_error.as_deref(), Some("keep"));
}
