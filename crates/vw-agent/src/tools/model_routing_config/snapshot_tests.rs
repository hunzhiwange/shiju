use super::*;
use crate::app::agent::config::DelegateAgentConfig;

fn route(hint: &str) -> ModelRouteConfig {
    ModelRouteConfig {
        hint: hint.to_string(),
        provider: "openai".to_string(),
        model: format!("{hint}-model"),
        max_tokens: None,
        api_key: None,
    }
}

fn rule(hint: &str, priority: i32) -> ClassificationRule {
    ClassificationRule {
        hint: hint.to_string(),
        keywords: vec![hint.to_string()],
        priority,
        ..ClassificationRule::default()
    }
}

#[test]
fn snapshot_sorts_scenarios_and_exposes_classification_only_rules() {
    let mut cfg = Config::default();
    let mut beta = route("beta");
    beta.api_key = Some(" key ".to_string());
    cfg.model_routes = vec![beta, route("alpha")];
    cfg.query_classification.enabled = true;
    cfg.query_classification.rules = vec![rule("orphan", 20), rule("beta", 5)];

    let snapshot = ModelRoutingConfigTool::snapshot(&cfg);

    assert_eq!(snapshot["scenarios"][0]["hint"], "alpha");
    assert!(snapshot["scenarios"][0]["classification"].is_null());
    assert_eq!(snapshot["scenarios"][1]["hint"], "beta");
    assert_eq!(snapshot["scenarios"][1]["api_key_configured"], true);
    assert_eq!(snapshot["scenarios"][1]["classification"]["priority"], 5);
    assert_eq!(snapshot["classification_only_rules"][0]["hint"], "orphan");
    assert_eq!(snapshot["query_classification"]["rules_count"], 2);
}

#[test]
fn snapshot_sorts_agents_and_treats_blank_keys_as_unconfigured() {
    let mut cfg = Config::default();
    cfg.agents.insert(
        "zeta".to_string(),
        DelegateAgentConfig {
            provider: "openai".to_string(),
            model: "zeta-model".to_string(),
            api_key: Some("   ".to_string()),
            ..DelegateAgentConfig::default()
        },
    );
    cfg.agents.insert(
        "alpha".to_string(),
        DelegateAgentConfig {
            provider: "anthropic".to_string(),
            model: "alpha-model".to_string(),
            api_key: Some("secret".to_string()),
            allowed_tools: vec!["file_read".to_string()],
            allowed_skills: vec!["review".to_string()],
            ..DelegateAgentConfig::default()
        },
    );

    let snapshot = ModelRoutingConfigTool::snapshot(&cfg);
    let names = snapshot["agents"].as_object().unwrap().keys().cloned().collect::<Vec<_>>();

    assert_eq!(names, vec!["alpha", "zeta"]);
    assert_eq!(snapshot["agents"]["alpha"]["api_key_configured"], true);
    assert_eq!(snapshot["agents"]["alpha"]["allowed_tools"], json!(["file_read"]));
    assert_eq!(snapshot["agents"]["zeta"]["api_key_configured"], false);
}

#[test]
fn normalize_helpers_drop_blank_hints_and_apply_stable_ordering() {
    let mut routes = vec![route("zeta"), route("  "), route("alpha")];
    ModelRoutingConfigTool::normalize_and_sort_routes(&mut routes);
    assert_eq!(
        routes.iter().map(|route| route.hint.as_str()).collect::<Vec<_>>(),
        vec!["alpha", "zeta"]
    );

    let mut rules = vec![rule("beta", 4), rule("  ", 99), rule("alpha", 4), rule("low", 1)];
    ModelRoutingConfigTool::normalize_and_sort_rules(&mut rules);
    assert_eq!(
        rules.iter().map(|rule| rule.hint.as_str()).collect::<Vec<_>>(),
        vec!["alpha", "beta", "low"]
    );
}

#[test]
fn rule_matcher_recognizes_each_supported_criterion() {
    let empty = ClassificationRule::default();
    assert!(!ModelRoutingConfigTool::has_rule_matcher(&empty));

    for rule in [
        ClassificationRule { keywords: vec!["code".to_string()], ..ClassificationRule::default() },
        ClassificationRule { patterns: vec!["```".to_string()], ..ClassificationRule::default() },
        ClassificationRule { min_length: Some(1), ..ClassificationRule::default() },
        ClassificationRule { max_length: Some(10), ..ClassificationRule::default() },
    ] {
        assert!(ModelRoutingConfigTool::has_rule_matcher(&rule));
    }
}

#[test]
fn ensure_rule_defaults_only_fills_rules_without_matchers() {
    let mut empty = ClassificationRule::default();
    ModelRoutingConfigTool::ensure_rule_defaults(&mut empty, "coding");
    assert_eq!(empty.keywords, vec!["coding"]);

    let mut existing =
        ClassificationRule { patterns: vec!["```".to_string()], ..ClassificationRule::default() };
    ModelRoutingConfigTool::ensure_rule_defaults(&mut existing, "coding");
    assert!(existing.keywords.is_empty());
    assert_eq!(existing.patterns, vec!["```"]);
}
