use super::*;

#[test]
fn reasoning_profiles_only_expose_effective_levels() {
    for (provider, model, expected) in [
        ("shiju", "deepseek-v4-flash", vec!["none", "high", "max"]),
        ("token-plan", "deepseek-v4-flash-0731", vec!["none", "low", "high", "max"]),
        ("token-plan", "glm-5.2", vec!["none", "high", "max"]),
        ("token-plan", "qwen3.8-max", vec!["none", "low", "medium", "xhigh"]),
        ("token-plan", "qwen3.7-plus", vec![]),
        ("token-plan", "wan2.7-image", vec![]),
        ("custom", "unknown-reasoner", vec![]),
        ("custom", "gpt-5.4", vec!["none", "low", "medium", "high", "xhigh"]),
        ("custom", "gpt-5.4-pro", vec![]),
        ("custom", "gpt-5", vec!["minimal", "low", "medium", "high"]),
    ] {
        assert_eq!(
            reasoning_profile(model, "openai-compatible", provider, "").levels,
            expected,
            "{provider}/{model}"
        );
    }
    assert!(reasoning_profile("gpt-5.4", "unknown", "custom", "").levels.is_empty());
    assert_eq!(
        reasoning_profile("claude-sonnet-4-6", "anthropic", "anthropic", "").levels,
        ["low", "medium", "high", "max"]
    );
    assert!(reasoning_profile("claude-sonnet-4-5", "anthropic", "anthropic", "").levels.is_empty());
}

#[test]
fn declared_reasoning_levels_override_inference_and_preserve_empty_controls() {
    let options = vec![ReasoningOption::Effort { values: vec!["high".into(), "max".into()] }];
    assert_eq!(
        reasoning_levels("glm-5.2", "openai-compatible", "shiju", "", Some(&options)),
        ["high", "max"]
    );
    assert!(reasoning_levels("gpt-5", "openai-compatible", "custom", "", Some(&[])).is_empty());
    assert!(
        reasoning_levels(
            "gpt-5",
            "openai-compatible",
            "custom",
            "",
            Some(&[ReasoningOption::Toggle])
        )
        .is_empty()
    );
    assert_eq!(
        reasoning_levels("gpt-5", "openai-compatible", "custom", "", None),
        ["minimal", "low", "medium", "high"]
    );
}

#[test]
fn reasoning_budget_metadata_accepts_provider_auto_sentinel() {
    let option: ReasoningOption = serde_json::from_value(
        serde_json::json!({"type": "budget_tokens", "min": -1, "max": 32768}),
    )
    .unwrap();
    assert_eq!(option, ReasoningOption::BudgetTokens { min: Some(-1), max: Some(32768) });
}
